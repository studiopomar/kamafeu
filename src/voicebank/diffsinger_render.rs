//! DiffSinger inference over ONNX Runtime. Sessions are keyed by model path
//! and execution provider and reused for preview/export renders.
use crate::project::model::UNote;
use crate::voicebank::diffsinger::{DiffSingerBank, ExecutionProvider};
use ort::execution_providers::{cpu::CPUExecutionProvider, ExecutionProvider as _};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::{DynValue, Tensor};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

static SESSIONS: LazyLock<Mutex<HashMap<String, Session>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

enum InputData {
    I64(Vec<i64>, Vec<usize>),
    F32(Vec<f32>, Vec<usize>),
    Bool(Vec<bool>, Vec<usize>),
}

impl InputData {
    fn into_tensor(self) -> Result<DynValue, String> {
        match self {
            Self::I64(data, shape) => Tensor::<i64>::from_array((shape, data))
                .map(Tensor::into_dyn)
                .map_err(|e| e.to_string()),
            Self::F32(data, shape) => Tensor::<f32>::from_array((shape, data))
                .map(Tensor::into_dyn)
                .map_err(|e| e.to_string()),
            Self::Bool(data, shape) => Tensor::<bool>::from_array((shape, data))
                .map(Tensor::into_dyn)
                .map_err(|e| e.to_string()),
        }
    }
}

fn session_key(path: &Path, provider: ExecutionProvider) -> String {
    format!("{}::{provider:?}", path.display())
}

fn with_session<T>(
    path: &Path,
    provider: ExecutionProvider,
    f: impl FnOnce(&mut Session) -> Result<T, String>,
) -> Result<T, String> {
    let key = session_key(path, provider);
    let mut sessions = SESSIONS
        .lock()
        .map_err(|_| "ONNX session cache lock poisoned".to_string())?;
    if !sessions.contains_key(&key) {
        #[cfg(test)]
        eprintln!("[DiffSingerTest] loading {} ({provider:?})", path.display());
        let mut builder = Session::builder()
            .map_err(|e| format!("ONNX session setup failed: {e}"))?
            .with_optimization_level(GraphOptimizationLevel::Disable)
            .map_err(|e| format!("ONNX optimization setup failed: {e}"))?;
        match provider {
            ExecutionProvider::Cpu => {
                builder = builder
                    .with_execution_providers([CPUExecutionProvider::default().build()])
                    .map_err(|e| format!("CPU execution provider setup failed: {e}"))?;
            }
            ExecutionProvider::CoreMl => {
                #[cfg(target_os = "macos")]
                {
                    use ort::execution_providers::coreml::CoreMLExecutionProvider;
                    let coreml = CoreMLExecutionProvider::default();
                    if !coreml.is_available().map_err(|e| e.to_string())? {
                        return Err(
                            "CoreML execution provider is unavailable in this ONNX Runtime build"
                                .into(),
                        );
                    }
                    coreml.register(&mut builder).map_err(|e| {
                        format!("CoreML execution provider registration failed: {e}")
                    })?;
                    builder = builder
                        .with_execution_providers([CPUExecutionProvider::default().build()])
                        .map_err(|e| format!("CPU fallback setup failed: {e}"))?;
                }
                #[cfg(not(target_os = "macos"))]
                return Err("CoreML is available only on macOS".into());
            }
        }
        let session = builder
            .commit_from_file(path)
            .map_err(|e| format!("cannot load ONNX model {}: {e}", path.display()))?;
        #[cfg(test)]
        eprintln!(
            "[DiffSingerTest] loaded {} inputs={:?} outputs={:?}",
            path.display(),
            session
                .inputs
                .iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>(),
            session
                .outputs
                .iter()
                .map(|o| o.name.as_str())
                .collect::<Vec<_>>()
        );
        sessions.insert(key.clone(), session);
    }
    f(sessions.get_mut(&key).expect("session inserted above"))
}

fn run_model(
    path: &Path,
    provider: ExecutionProvider,
    inputs: HashMap<String, InputData>,
) -> Result<Vec<f32>, String> {
    with_session(path, provider, |session| {
        let model_inputs: Vec<String> = session.inputs.iter().map(|i| i.name.clone()).collect();
        let mut named = Vec::with_capacity(model_inputs.len());
        for name in &model_inputs {
            let data = inputs
                .get(name)
                .ok_or_else(|| {
                    format!(
                        "ONNX model {} requires unsupported or unavailable input '{name}'",
                        path.display()
                    )
                })?
                .clone_data();
            named.push((name.as_str(), data.into_tensor()?));
        }
        #[cfg(test)]
        eprintln!(
            "[DiffSingerTest] running {} names={model_inputs:?}",
            path.display()
        );
        let output_name = session
            .outputs
            .first()
            .map(|output| output.name.clone())
            .ok_or_else(|| format!("ONNX model {} returned no outputs", path.display()))?;
        let output = session
            .run(named)
            .map_err(|e| format!("ONNX inference failed for {}: {e}", path.display()))?;
        let first = output
            .get(output_name)
            .ok_or_else(|| format!("ONNX model {} returned no outputs", path.display()))?;
        let (_, values) = first.try_extract_tensor::<f32>().map_err(|e| {
            format!(
                "ONNX model {} returned a non-float output: {e}",
                path.display()
            )
        })?;
        #[cfg(test)]
        eprintln!(
            "[DiffSingerTest] finished {} length={}",
            path.display(),
            values.len()
        );
        Ok(values.to_vec())
    })
}

fn run_linguistic_model(
    path: &Path,
    provider: ExecutionProvider,
    inputs: HashMap<String, InputData>,
) -> Result<(Vec<f32>, Vec<bool>), String> {
    with_session(path, provider, |session| {
        let input_names: Vec<String> = session.inputs.iter().map(|input| input.name.clone()).collect();
        let mut named = Vec::with_capacity(input_names.len());
        for input_name in &input_names {
            let data = inputs.get(input_name).ok_or_else(|| format!(
                "linguistic ONNX model {} requires unsupported or unavailable input '{}'",
                path.display(), input_name
            ))?.clone_data();
            named.push((input_name.as_str(), data.into_tensor()?));
        }
        let outputs = session.run(named).map_err(|e| format!(
            "linguistic ONNX inference failed for {}: {e}", path.display()
        ))?;
        let encoder = outputs.get("encoder_out").ok_or_else(|| format!(
            "linguistic model {} has no encoder_out output", path.display()
        ))?;
        let (_, encoder) = encoder.try_extract_tensor::<f32>().map_err(|e| format!(
            "linguistic model {} returned invalid encoder_out: {e}", path.display()
        ))?;
        let masks = outputs.get("x_masks").ok_or_else(|| format!(
            "linguistic model {} has no x_masks output", path.display()
        ))?;
        let (_, masks) = masks.try_extract_tensor::<bool>().map_err(|e| format!(
            "linguistic model {} returned invalid x_masks: {e}", path.display()
        ))?;
        Ok((encoder.to_vec(), masks.to_vec()))
    })
}

impl InputData {
    fn clone_data(&self) -> Self {
        match self {
            Self::I64(data, shape) => Self::I64(data.clone(), shape.clone()),
            Self::F32(data, shape) => Self::F32(data.clone(), shape.clone()),
            Self::Bool(data, shape) => Self::Bool(data.clone(), shape.clone()),
        }
    }
}

pub(crate) fn phone_sequence(
    bank: &DiffSingerBank,
    note: &UNote,
    language: &str,
    phonemizer: crate::voicebank::diffsinger::DiffSingerPhonemizer,
) -> Result<Vec<String>, String> {
    let source = note
        .phonetic_hint
        .as_deref()
        .unwrap_or(note.lyric.as_str())
        .trim();
    if source.is_empty() || source == "-" || source.eq_ignore_ascii_case("rest") {
        return Ok(vec!["SP".into()]);
    }
    if language == "pt"
        && phonemizer == crate::voicebank::diffsinger::DiffSingerPhonemizer::BrapaPhonetic
    {
        return brapa_phonetic_sequence(bank, source);
    }
    let pieces: Vec<&str> = source.split_whitespace().collect();
    let explicit =
        note.phonetic_hint.is_some() || pieces.len() > 1 || bank.phonemes.contains_key(source);
    let phones = if explicit {
        pieces
            .into_iter()
            .map(|piece| -> Result<String, String> {
                if bank.phonemes.contains_key(piece) {
                    Ok(piece.to_owned())
                } else {
                    let tagged = format!("{language}/{piece}");
                    if bank.phonemes.contains_key(&tagged) {
                        Ok(tagged)
                    } else {
                        return Err(format!(
                            "phone '{piece}' is not present in the DiffSinger vocabulary"
                        ));
                    }
                }
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        grapheme_to_phones(source, language, bank)?
    };
    if phones.is_empty() {
        return Err(format!("no valid phones generated from lyric '{source}'"));
    }
    Ok(phones)
}

fn brapa_phonetic_sequence(bank: &DiffSingerBank, source: &str) -> Result<Vec<String>, String> {
    let pieces: Vec<&str> = source.split_whitespace().collect();
    let mut phones = Vec::new();
    for piece in pieces {
        if let Some(phone) = resolve_brapa_token(bank, piece) {
            phones.push(phone);
            continue;
        }

        // The BRAPA phonetic plugin accepts both spaced phones and compact
        // input such as "sa". Greedy longest-match keeps digraph phones
        // (nh, lh, ch, rr, eh, oh, ...) intact while splitting compact input.
        let clean = piece.trim_matches(['!', '?', '.', ',', ';', ':', '"', '\'', '(', ')']);
        if clean.is_empty() {
            continue;
        }
        let mut rest = clean.to_ascii_lowercase();
        while !rest.is_empty() {
            let mut candidates: Vec<&str> = bank
                .phonemes
                .keys()
                .filter_map(|token| {
                    token
                        .strip_prefix("pt/")
                        .or_else(|| (!token.contains('/')).then_some(token.as_str()))
                })
                .filter(|token| !matches!(*token, "SP" | "AP"))
                .collect();
            candidates.sort_by_key(|token| std::cmp::Reverse(token.len()));
            let Some(token) = candidates
                .into_iter()
                .find(|token| rest.starts_with(*token))
            else {
                return Err(format!(
                    "Fone BRAPA inválido ou não suportado pelo banco: '{piece}'"
                ));
            };
            let resolved = resolve_brapa_token(bank, token)
                .ok_or_else(|| format!("phone '{token}' is not supported by this bank"))?;
            rest.drain(..token.len());
            phones.push(resolved);
        }
    }
    if phones.is_empty() {
        return Err("nenhum fone BRAPA válido foi informado".into());
    }
    Ok(phones)
}

fn resolve_brapa_token(bank: &DiffSingerBank, raw: &str) -> Option<String> {
    let clean = raw.trim().to_ascii_lowercase();
    let token = match clean.as_str() {
        "sp" | "-" => "SP",
        "ap" | "br" => "AP",
        "c" | "q" => "k",
        "tx" | "tch" => "ch",
        "dx" => "dj",
        "ah" => "ax",
        "wn" => "w",
        "yn" => "y",
        _ => clean.strip_prefix("pt/").unwrap_or(&clean),
    };
    if token == "SP" || token == "AP" {
        return bank.phonemes.contains_key(token).then(|| token.to_owned());
    }
    let tagged = format!("pt/{token}");
    bank.validate_phone(&tagged)
        .ok()
        .map(str::to_owned)
        .or_else(|| bank.validate_phone(token).ok().map(str::to_owned))
}

/// Return model-predicted phone durations and the model-predicted consonant
/// pre-onset for one lyric. This follows OpenUtau's duration predictor
/// contract: a leading SP plus consonants, an anchor at the vowel/note start,
/// and the note end as the final alignment point.
pub(crate) fn predict_phone_timing(
    bank: &DiffSingerBank,
    note: &UNote,
    phones: &[String],
    language: &str,
    speaker: &str,
    provider: ExecutionProvider,
) -> Result<(Vec<f64>, f64), String> {
    if phones.is_empty() {
        return Ok((Vec::new(), 0.0));
    }
    if !bank.predict_dur {
        return Err("voicebank does not enable the DiffSinger duration predictor".into());
    }
    let linguistic = bank.predictor_models.get("dsdur.linguistic")
        .ok_or("duration linguistic model is not configured")?;
    let duration_model = bank.predictor_models.get("dsdur.dur")
        .ok_or("duration predictor model is not configured")?;
    let pause = if bank.phonemes.contains_key("SP") { "SP" } else { "AP" };
    let mut tokens = Vec::with_capacity(phones.len() + 1);
    tokens.push(*bank.phonemes.get(pause).ok_or("DiffSinger vocabulary has no SP/AP token")?);
    for phone in phones {
        tokens.push(*bank.phonemes.get(phone).ok_or_else(|| format!("phone '{phone}' is not in the duration vocabulary"))?);
    }

    let is_vowel = |phone: &str| {
        bank.phone_types.get(phone).is_some_and(|kind| kind.eq_ignore_ascii_case("vowel"))
            || phone.rsplit('/').next().is_some_and(|symbol| matches!(symbol,
                "a"|"ae"|"an"|"ax"|"e"|"eh"|"en"|"i"|"i0"|"ih"|"iy"|"in"|
                "o"|"oh"|"on"|"ow"|"u"|"uw"|"u0"|"un"))
    };
    let is_glide = |phone: &str| {
        bank.phone_types.get(phone).is_some_and(|kind| kind.eq_ignore_ascii_case("semivowel"))
            || phone.rsplit('/').next().is_some_and(|symbol| matches!(symbol, "w" | "y"))
    };
    let first_vowel = phones.iter().position(|phone| is_vowel(phone));
    let initial_count = first_vowel.map(|index| {
        if index >= 2 && is_glide(&phones[index - 1]) && !is_vowel(&phones[index - 2]) {
            index // C-G-V: the glide begins at the note anchor.
        } else {
            index + 1
        }
    }).unwrap_or(1);
    let body_count = tokens.len() - initial_count;
    let frame_ms = bank.hop_size as f64 * 1000.0 / bank.sample_rate as f64;
    let note_frames = (note.duration_ms.max(0.0) / frame_ms).round() as i64;
    // OpenUtau gives the first consonant group 500 ms of phrase lead-in.
    let word_div = vec![initial_count as i64, body_count as i64];
    let word_dur = vec![(500.0 / frame_ms).round() as i64, note_frames];
    let mut linguistic_inputs = HashMap::new();
    linguistic_inputs.insert("tokens".into(), InputData::I64(tokens.clone(), vec![1, tokens.len()]));
    linguistic_inputs.insert("word_div".into(), InputData::I64(word_div.clone(), vec![1, 2]));
    linguistic_inputs.insert("word_dur".into(), InputData::I64(word_dur.clone(), vec![1, 2]));
    if bank.use_lang_id {
        let phone_languages: Vec<i64> = std::iter::once(0).chain(phones.iter().map(|phone| {
            let code = phone.split_once('/').map(|(lang, _)| lang).unwrap_or(language);
            bank.language_ids.get(code).copied().unwrap_or(0)
        })).collect();
        linguistic_inputs.insert("languages".into(), InputData::I64(phone_languages, vec![1, tokens.len()]));
    }
    let (encoder, masks) = run_linguistic_model(linguistic, provider, linguistic_inputs)?;
    let hidden = encoder.len() / tokens.len();
    if hidden == 0 || encoder.len() % tokens.len() != 0 || masks.len() != tokens.len() {
        return Err(format!("duration linguistic model returned incompatible tensors (encoder={}, masks={}, phones={})", encoder.len(), masks.len(), tokens.len()));
    }
    let mut inputs = HashMap::new();
    inputs.insert("encoder_out".into(), InputData::F32(encoder, vec![1, tokens.len(), hidden]));
    inputs.insert("x_masks".into(), InputData::Bool(masks, vec![1, tokens.len()]));
    inputs.insert("ph_midi".into(), InputData::I64(vec![i64::from(note.midi_key()); tokens.len()], vec![1, tokens.len()]));
    inputs.insert("word_div".into(), InputData::I64(word_div, vec![1, 2]));
    inputs.insert("word_dur".into(), InputData::I64(word_dur, vec![1, 2]));
    if bank.duration_uses_speakers {
        let embedding_path = bank.duration_speaker_embeddings.get(speaker)
            .or_else(|| bank.duration_speaker_embeddings.values().next())
            .ok_or("duration predictor declares speakers but has no embedding files")?;
        let bytes = std::fs::read(embedding_path).map_err(|e| format!("cannot read duration speaker embedding {}: {e}", embedding_path.display()))?;
        if bytes.len() % 4 != 0 || bytes.is_empty() {
            return Err(format!("invalid duration speaker embedding: {}", embedding_path.display()));
        }
        let embedding: Vec<f32> = bytes.chunks_exact(4).map(|b| f32::from_le_bytes([b[0],b[1],b[2],b[3]])).collect();
        let mut repeated = Vec::with_capacity(tokens.len() * embedding.len());
        for _ in &tokens { repeated.extend_from_slice(&embedding); }
        inputs.insert("spk_embed".into(), InputData::F32(repeated, vec![1, tokens.len(), embedding.len()]));
    }
    let predicted = run_model(duration_model, provider, inputs)?;
    if predicted.len() != tokens.len() || predicted.iter().any(|x| !x.is_finite() || *x <= 0.0) {
        return Err(format!("duration predictor returned invalid values ({} for {} phones)", predicted.len(), tokens.len()));
    }
    let prefix = &predicted[1..initial_count]; // the initial SP itself is not rendered
    let onset_ms = -(prefix.iter().sum::<f32>() as f64 * frame_ms);
    let prefix_durations: Vec<f64> = prefix.iter().map(|frames| *frames as f64 * frame_ms).collect();
    let body = &predicted[initial_count..];
    let body_sum = body.iter().sum::<f32>() as f64;
    if body_sum <= 0.0 { return Err("duration predictor returned an empty aligned vowel group".into()); }
    let body_durations: Vec<f64> = body.iter().map(|frames| note.duration_ms * *frames as f64 / body_sum).collect();
    let mut durations = prefix_durations;
    durations.extend(body_durations);
    Ok((durations, onset_ms))
}

/// Timing used for display and rendering. Authored phone boundaries take
/// priority; otherwise use the voicebank's actual duration ONNX graphs.
pub(crate) fn phoneme_durations_ms(
    bank: &DiffSingerBank,
    note: &UNote,
    phones: &[String],
) -> Vec<f64> {
    if phones.is_empty() {
        return Vec::new();
    }
    if note.phoneme_durations_ms.len() == phones.len()
        && note
            .phoneme_durations_ms
            .iter()
            .all(|duration| duration.is_finite() && *duration > 0.0)
    {
        let sum: f64 = note.phoneme_durations_ms.iter().sum();
        if sum > 0.0 {
            return note
                .phoneme_durations_ms
                .iter()
                .map(|duration| note.duration_ms * duration / sum)
                .collect();
        }
    }
    let language = phones.first().and_then(|p| p.split_once('/').map(|(lang, _)| lang)).unwrap_or("en");
    let speaker = bank.duration_speaker_embeddings.keys().next().map(String::as_str)
        .or_else(|| bank.speakers.first().map(String::as_str)).unwrap_or_default();
    if let Ok((durations, _)) = predict_phone_timing(bank, note, phones, language, speaker, ExecutionProvider::Cpu) {
        return durations;
    }
    fallback_phoneme_durations_ms(bank, note, phones)
}

fn fallback_phoneme_durations_ms(
    bank: &DiffSingerBank,
    note: &UNote,
    phones: &[String],
) -> Vec<f64> {
    let symbols: Vec<String> = phones
        .iter()
        .map(|phone| {
            phone
                .split_once('/')
                .map(|(_, symbol)| symbol)
                .unwrap_or(phone)
                .to_ascii_lowercase()
        })
        .collect();
    let weights: Vec<f64> = symbols
        .iter()
        .map(|symbol| {
            if matches!(
                symbol.as_str(),
                "a" | "ae"
                    | "an"
                    | "ax"
                    | "e"
                    | "eh"
                    | "en"
                    | "i"
                    | "i0"
                    | "in"
                    | "o"
                    | "oh"
                    | "on"
                    | "u"
                    | "u0"
                    | "un"
            ) {
                10.0
            } else if matches!(
                symbol.as_str(),
                "w" | "y" | "m" | "n" | "nh" | "ng" | "l" | "ly" | "r" | "rr"
            ) {
                2.0
            } else {
                let _ = bank;
                1.5
            }
        })
        .collect();
    let minimums: Vec<f64> = symbols
        .iter()
        .map(|symbol| {
            if matches!(
                symbol.as_str(),
                "s" | "sh" | "f" | "v" | "z" | "j" | "h" | "ch" | "th"
            ) {
                65.0
            } else if matches!(symbol.as_str(), "SP" | "AP") {
                0.0
            } else if matches!(
                symbol.as_str(),
                "a" | "ae"
                    | "an"
                    | "ax"
                    | "e"
                    | "eh"
                    | "en"
                    | "i"
                    | "i0"
                    | "in"
                    | "o"
                    | "oh"
                    | "on"
                    | "u"
                    | "u0"
                    | "un"
            ) {
                0.0
            } else {
                45.0
            }
        })
        .collect();
    let total_minimum: f64 = minimums.iter().sum();
    let floors = if total_minimum > note.duration_ms && total_minimum > 0.0 {
        minimums
            .iter()
            .map(|minimum| note.duration_ms * minimum / total_minimum)
            .collect::<Vec<_>>()
    } else {
        minimums
    };
    let remaining = (note.duration_ms - floors.iter().sum::<f64>()).max(0.0);
    let weight_sum: f64 = weights.iter().sum();
    weights
        .iter()
        .enumerate()
        .map(|(index, weight)| floors[index] + remaining * weight / weight_sum)
        .collect()
}

pub(crate) fn phoneme_durations_with_offsets(
    bank: &DiffSingerBank,
    note: &UNote,
    phones: &[String],
    frame_ms: f64,
    language: &str,
    speaker: &str,
    provider: ExecutionProvider,
) -> Result<(Vec<f64>, f64), String> {
    let (base, model_onset) = if note.phoneme_durations_ms.len() == phones.len()
        && note.phoneme_durations_ms.iter().all(|d| d.is_finite() && *d > 0.0)
    {
        (phoneme_durations_ms(bank, note, phones), 0.0)
    } else if bank.predict_dur {
        predict_phone_timing(bank, note, phones, language, speaker, provider)?
    } else {
        (fallback_phoneme_durations_ms(bank, note, phones), 0.0)
    };
    if phones.is_empty() {
        return Ok((base, 0.0));
    }
    let offsets: Vec<f64> = (0..phones.len())
        .map(|index| {
            note.phoneme_overrides
                .iter()
                .find(|item| item.index == index)
                .and_then(|item| item.offset_ms)
                .filter(|value| value.is_finite())
                .unwrap_or(0.0)
                .clamp(-2_000.0, 2_000.0)
        })
        .collect();
    let adjusted: Vec<f64> = base
        .iter()
        .enumerate()
        .map(|(index, duration)| {
            let next = offsets.get(index + 1).copied().unwrap_or(0.0);
            (duration + next - offsets[index]).max(frame_ms)
        })
        .collect();
    Ok((adjusted, model_onset + offsets[0]))
}

fn grapheme_to_phones(
    word: &str,
    language: &str,
    bank: &DiffSingerBank,
) -> Result<Vec<String>, String> {
    // Small deterministic G2P for simple English and BR Portuguese input;
    // explicit multi-phone hints remain available for names and exceptions.
    let word = word.to_lowercase();
    let chars: Vec<char> = word.chars().collect();
    let mut raw = Vec::<String>::new();
    let mut i = 0;
    while i < chars.len() {
        let rest: String = chars[i..].iter().collect();
        let (phone, consumed) = if language == "pt" {
            if rest.starts_with("nh") {
                ("nh", 2)
            } else if rest.starts_with("lh") {
                ("ly", 2)
            } else if rest.starts_with("ch") {
                ("sh", 2)
            } else if rest.starts_with("rr") {
                ("h", 2)
            } else if rest.starts_with("ss") {
                ("s", 2)
            } else if rest.starts_with("qu") {
                ("k", 2)
            } else if rest.starts_with("gu") {
                ("g", 2)
            } else {
                (
                    match chars[i] {
                        'a' => "a",
                        'e' => "eh",
                        'i' => "i",
                        'o' => "oh",
                        'u' => "u",
                        'b' => "b",
                        'c' => {
                            if chars.get(i + 1).is_some_and(|c| "ei".contains(*c)) {
                                "s"
                            } else {
                                "k"
                            }
                        }
                        'd' => "d",
                        'f' => "f",
                        'g' => {
                            if chars.get(i + 1).is_some_and(|c| "ei".contains(*c)) {
                                "j"
                            } else {
                                "g"
                            }
                        }
                        'h' => "",
                        'j' => "j",
                        'l' => "l",
                        'm' => "m",
                        'n' => "n",
                        'p' => "p",
                        'q' => "k",
                        'r' => "r",
                        's' => "s",
                        't' => "t",
                        'v' => "v",
                        'w' => "w",
                        'x' => "sh",
                        'y' => "i",
                        'z' => "z",
                        'á' => "a",
                        'â' => "a",
                        'ã' => "an",
                        'à' => "a",
                        'é' => "eh",
                        'ê' => "e",
                        'í' => "i",
                        'ó' => "oh",
                        'ô' => "o",
                        'õ' => "on",
                        'ú' => "u",
                        _ => {
                            return Err(format!(
                                "cannot phonemize character '{}' in Portuguese lyric '{word}'",
                                chars[i]
                            ))
                        }
                    },
                    1,
                )
            }
        } else if language == "en" {
            if rest.starts_with("sh") {
                ("sh", 2)
            } else if rest.starts_with("ch") {
                ("ch", 2)
            } else if rest.starts_with("th") {
                ("th", 2)
            } else if rest.starts_with("ng") {
                ("ng", 2)
            } else if rest.starts_with("oo") {
                ("uw", 2)
            } else if rest.starts_with("ee") {
                ("iy", 2)
            } else if rest.starts_with("oo") {
                ("uw", 2)
            } else {
                (
                    match chars[i] {
                        'a' => "ae",
                        'e' => "eh",
                        'i' => "ih",
                        'o' => "ow",
                        'u' => "ah",
                        'b' => "b",
                        'c' => "k",
                        'd' => "d",
                        'f' => "f",
                        'g' => "g",
                        'h' => "hh",
                        'j' => "jh",
                        'k' => "k",
                        'l' => "l",
                        'm' => "m",
                        'n' => "n",
                        'p' => "p",
                        'q' => "k",
                        'r' => "r",
                        's' => "s",
                        't' => "t",
                        'v' => "v",
                        'w' => "w",
                        'x' => "k",
                        'y' => "y",
                        'z' => "z",
                        _ => {
                            return Err(format!(
                                "cannot phonemize character '{}' in English lyric '{word}'",
                                chars[i]
                            ))
                        }
                    },
                    1,
                )
            }
        } else {
            return Err(format!(
                "automatic G2P is not implemented for language '{language}'; enter explicit phones"
            ));
        };
        if !phone.is_empty() {
            raw.push(format!("{language}/{phone}"));
        }
        i += consumed;
    }
    raw.into_iter()
        .map(|phone| bank.validate_phone(&phone).map(str::to_owned))
        .collect()
}

/// Render a monophonic track through the bank's acoustic model and vocoder.
pub fn render_track(
    bank: &DiffSingerBank,
    notes: &[UNote],
    language: &str,
    phonemizer: crate::voicebank::diffsinger::DiffSingerPhonemizer,
    speaker: &str,
    provider: ExecutionProvider,
    output_rate: u32,
    cancel: Option<&AtomicBool>,
) -> Result<Vec<f32>, String> {
    if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
        return Ok(Vec::new());
    }
    if !bank.loadable {
        return Err(bank
            .diagnostic
            .clone()
            .unwrap_or_else(|| "DiffSinger bank is not loadable".into()));
    }
    if output_rate == 0 || bank.sample_rate == 0 || bank.hop_size == 0 {
        return Err("invalid DiffSinger sample rate or hop size".into());
    }
    let acoustic = bank
        .acoustic_model
        .as_deref()
        .ok_or("acoustic model is not configured")?;
    let vocoder = bank
        .vocoder_model
        .as_deref()
        .ok_or("vocoder model is not configured")?;
    let lang_id = if bank.use_lang_id {
        bank.validate_language(language)?
    } else {
        0
    };
    let embedding_path = bank.validate_speaker(speaker)?;
    let bytes = std::fs::read(embedding_path).map_err(|e| {
        format!(
            "cannot read speaker embedding {}: {e}",
            embedding_path.display()
        )
    })?;
    if bytes.len() < 4 || bytes.len() % 4 != 0 {
        return Err(format!(
            "speaker embedding has invalid float data: {}",
            embedding_path.display()
        ));
    }
    let embedding: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    let embed_size = embedding.len();
    if embed_size != bank.hidden_size {
        return Err(format!(
            "speaker embedding size {embed_size} differs from model hidden_size {}",
            bank.hidden_size
        ));
    }

    let frame_ms = bank.hop_size as f64 * 1000.0 / bank.sample_rate as f64;
    let mut tokens = Vec::<i64>::new();
    let mut durations = Vec::<i64>::new();
    let mut frame_f0 = Vec::<f32>::new();
    let mut phone_lang = Vec::<i64>::new();
    const HEAD_FRAMES: usize = 8;
    const TAIL_FRAMES: usize = 8;
    let pause = bank
        .phonemes
        .get("SP")
        .or_else(|| bank.phonemes.get("AP"))
        .copied();
    let pause_id = pause.ok_or_else(|| {
        "DiffSinger vocabulary has no SP or AP token for phrase padding".to_string()
    })?;
    tokens.push(pause_id);
    durations.push(HEAD_FRAMES as i64);
    frame_f0.extend(std::iter::repeat_n(0.0, HEAD_FRAMES));
    phone_lang.push(0);
    let mut total_frames = HEAD_FRAMES;
    let mut cursor_frames = 0usize;
    let mut has_note = false;
    for note in notes {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Ok(Vec::new());
        }
        if !note.duration_ms.is_finite() || note.duration_ms <= 0.0 {
            continue;
        }
        let phones = phone_sequence(bank, note, language, phonemizer)?;
        let (mut phone_durations, mut first_phone_offset_ms) =
            phoneme_durations_with_offsets(bank, note, &phones, frame_ms, language, speaker, provider)?;
        // A phrase cannot begin before the audio starts. In that case clamp
        // the onset to zero and restore the phone time that would be pre-roll.
        if note.position_ms + first_phone_offset_ms < 0.0 {
            if let Some(first) = phone_durations.first_mut() {
                *first = (*first + first_phone_offset_ms).max(frame_ms);
            }
            first_phone_offset_ms = 0.0;
        }
        let note_start =
            ((note.position_ms + first_phone_offset_ms).max(0.0) / frame_ms).round() as usize;
        if note_start < cursor_frames {
            // A negative onset advances the consonant into the preceding
            // note's tail. DiffSinger phones are sequential, so trim that many
            // frames from the rendered tail before appending the new onset.
            let mut trim = cursor_frames - note_start;
            while trim > 0 && durations.len() > 1 {
                let last = durations.len() - 1;
                let available = durations[last].max(0) as usize;
                if available > trim {
                    durations[last] -= trim as i64;
                    trim = 0;
                } else {
                    trim -= available;
                    durations.pop();
                    tokens.pop();
                    phone_lang.pop();
                }
            }
            if trim > 0 {
                return Err(format!(
                    "phone onset offset exceeds preceding audio by {trim} frames"
                ));
            }
            total_frames = total_frames.saturating_sub(cursor_frames - note_start);
            frame_f0.truncate(total_frames);
            cursor_frames = note_start;
        }
        if note_start > cursor_frames {
            let gap = note_start - cursor_frames;
            tokens.push(pause_id);
            durations.push(gap as i64);
            frame_f0.extend(std::iter::repeat_n(0.0, gap));
            phone_lang.push(0);
            total_frames += gap;
            cursor_frames += gap;
        }
        has_note = true;
        let adjusted_duration_ms =
            (note.duration_ms - first_phone_offset_ms).max(frame_ms * phones.len() as f64);
        let frames = (adjusted_duration_ms / frame_ms)
            .round()
            .max(phones.len() as f64) as usize;
        let key = f64::from(note.midi_key());
        let hz = (440.0 * 2.0_f64.powf((key - 69.0) / 12.0)) as f32;
        let mut remain = frames;
        for (i, phone) in phones.iter().enumerate() {
            let remaining_phones = phones.len() - i - 1;
            let count = if i + 1 == phones.len() {
                remain
            } else {
                ((frames as f64 * phone_durations[i] / phone_durations.iter().sum::<f64>()).round()
                    as usize)
                    .max(1)
                    .min(remain.saturating_sub(remaining_phones))
            };
            let id = bank
                .phonemes
                .get(phone)
                .copied()
                .ok_or_else(|| format!("phone '{phone}' is not in vocabulary"))?;
            tokens.push(id);
            durations.push(count as i64);
            frame_f0.extend(std::iter::repeat_n(hz, count));
            phone_lang.push(lang_id);
            remain -= count;
        }
        total_frames += frames;
        cursor_frames = note_start.max(cursor_frames) + frames;
    }
    if !has_note {
        return Ok(Vec::new());
    }
    tokens.push(pause_id);
    durations.push(TAIL_FRAMES as i64);
    frame_f0.extend(std::iter::repeat_n(0.0, TAIL_FRAMES));
    phone_lang.push(0);
    total_frames += TAIL_FRAMES;
    let mut inputs = HashMap::<String, InputData>::new();
    let phone_count = durations.len();
    inputs.insert(
        "tokens".into(),
        InputData::I64(tokens, vec![1, phone_count]),
    );
    inputs.insert(
        "durations".into(),
        InputData::I64(durations, vec![1, phone_count]),
    );
    inputs.insert(
        "f0".into(),
        InputData::F32(frame_f0.clone(), vec![1, total_frames]),
    );
    inputs.insert(
        "languages".into(),
        InputData::I64(phone_lang, vec![1, phone_count]),
    );
    let mut spk = Vec::with_capacity(total_frames * embed_size);
    for _ in 0..total_frames {
        spk.extend_from_slice(&embedding);
    }
    inputs.insert(
        "spk_embed".into(),
        InputData::F32(spk, vec![1, total_frames, embed_size]),
    );
    if bank.use_key_shift_embed {
        inputs.insert(
            "gender".into(),
            InputData::F32(vec![0.0; total_frames], vec![1, total_frames]),
        );
    }
    if bank.use_speed_embed {
        inputs.insert(
            "velocity".into(),
            InputData::F32(vec![1.0; total_frames], vec![1, total_frames]),
        );
    }
    if bank.use_continuous_acceleration {
        if bank.use_variable_depth {
            inputs.insert("depth".into(), InputData::F32(vec![0.6], vec![1]));
        }
        inputs.insert("steps".into(), InputData::I64(vec![20], vec![1]));
    } else {
        inputs.insert("speedup".into(), InputData::I64(vec![50], vec![1]));
    }
    for key in ["energy", "breathiness", "voicing", "tension"] {
        inputs.insert(
            key.into(),
            InputData::F32(vec![0.0; total_frames], vec![1, total_frames]),
        );
    }
    let mel = run_model(acoustic, provider, inputs)?;
    if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
        return Ok(Vec::new());
    }
    if mel.is_empty() || mel.len() % bank.mel_bins != 0 {
        return Err(format!(
            "acoustic output has {} values, incompatible with {} mel bins",
            mel.len(),
            bank.mel_bins
        ));
    }
    let mel_frames = mel.len() / bank.mel_bins;
    let mut vocoder_inputs = HashMap::new();
    vocoder_inputs.insert(
        "mel".into(),
        InputData::F32(mel, vec![1, mel_frames, bank.mel_bins]),
    );
    vocoder_inputs.insert("f0".into(), InputData::F32(frame_f0, vec![1, total_frames]));
    let mut audio = run_model(vocoder, provider, vocoder_inputs)?;
    if audio.iter().any(|sample| !sample.is_finite()) {
        return Err("DiffSinger returned non-finite audio samples".into());
    }
    let head_samples = HEAD_FRAMES * bank.hop_size;
    if audio.len() > head_samples {
        audio.drain(..head_samples);
    } else {
        audio.clear();
    }
    if bank.sample_rate != output_rate {
        audio = resample_linear(&audio, bank.sample_rate, output_rate);
    }
    Ok(audio)
}

fn resample_linear(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if input.is_empty() || from == to {
        return input.to_vec();
    }
    let len = (input.len() as u64 * to as u64 / from as u64) as usize;
    (0..len)
        .map(|i| {
            let position = i as f64 * from as f64 / to as f64;
            let left = (position.floor() as usize).min(input.len() - 1);
            let right = (left + 1).min(input.len() - 1);
            let fraction = (position - left as f64) as f32;
            input[left] * (1.0 - fraction) + input[right] * fraction
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_and_portuguese_g2p_tokens_resolve_in_reference_vocabularies() {
        for path in [
            "/Users/xiao/Library/OpenUtau/Singers/Evelyn_DS_v100",
            "/Users/xiao/Library/OpenUtau/Singers/MIRACLE",
            "/Users/xiao/Library/OpenUtau/Singers/SAiFA_v170",
        ] {
            if !Path::new(path).exists() {
                continue;
            }
            let bank = DiffSingerBank::inspect(path).unwrap();
            assert!(
                grapheme_to_phones("see", "en", &bank).is_ok(),
                "{}",
                bank.name
            );
            if bank.language_ids.contains_key("pt") {
                assert!(
                    grapheme_to_phones("amor", "pt", &bank).is_ok(),
                    "{}",
                    bank.name
                );
            }
        }
    }

    #[test]
    fn brapa_phonetic_accepts_compact_spaced_and_plugin_alias_phones() {
        let path = "/Users/xiao/Library/OpenUtau/Singers/MIRACLE";
        if !Path::new(path).exists() {
            return;
        }
        let bank = DiffSingerBank::inspect(path).unwrap();
        let mode = crate::voicebank::diffsinger::DiffSingerPhonemizer::BrapaPhonetic;
        let compact = UNote::new("sa", "C4", 0.0, 1000.0);
        let spaced = UNote::new("s a", "C4", 0.0, 1000.0);
        let expected = vec!["pt/s".to_owned(), "pt/a".to_owned()];
        assert_eq!(
            phone_sequence(&bank, &compact, "pt", mode).unwrap(),
            expected
        );
        assert_eq!(
            phone_sequence(&bank, &spaced, "pt", mode).unwrap(),
            expected
        );
        let aliases = brapa_phonetic_sequence(&bank, "tch ah").unwrap();
        assert_eq!(aliases, vec!["pt/ch", "pt/ax"]);
        let timings = phoneme_durations_ms(&bank, &spaced, &expected);
        let (predicted, onset) = predict_phone_timing(
            &bank, &spaced, &expected, "pt", "miracle", ExecutionProvider::Cpu,
        ).unwrap();
        assert!(onset < 0.0, "leading consonant should precede vowel anchor: {onset}");
        assert!((predicted.iter().sum::<f64>() - (spaced.duration_ms - onset)).abs() < 1e-5);
        assert!(timings[0] > 0.0 && timings[1] > 0.0, "predicted timings: {timings:?}");
        let short_note = UNote::new("s a", "C4", 0.0, 200.0);
        let short_timings = phoneme_durations_ms(&bank, &short_note, &expected);
        assert!(short_timings.iter().all(|duration| *duration > 0.0), "short note timings: {short_timings:?}");
        let mut offset_note = spaced.clone();
        offset_note.phoneme_durations_ms = vec![300.0, 700.0];
        offset_note
            .phoneme_overrides
            .push(crate::project::model::UPhonemeOverride {
                index: 0,
                offset_ms: Some(-60.0),
                ..Default::default()
            });
        let (offset_timings, onset) =
            phoneme_durations_with_offsets(&bank, &offset_note, &expected, 10.0, "pt", "miracle", ExecutionProvider::Cpu).unwrap();
        assert_eq!(onset, -60.0);
        assert!((offset_timings[0] - 360.0).abs() < 1e-6);
        assert!((offset_timings.iter().sum::<f64>() - 1_060.0).abs() < 1e-6);
    }

    /// Set KAMAFEU_DIFFSINGER_SMOKE_BANK to one local voicebank directory to
    /// exercise actual ONNX acoustic + vocoder inference on CPU.
    #[test]
    fn local_cpu_onnx_render_smoke_when_bank_is_configured() {
        let Ok(root) = std::env::var("KAMAFEU_DIFFSINGER_SMOKE_BANK") else {
            return;
        };
        let bank = DiffSingerBank::inspect(&root).unwrap();
        let language =
            std::env::var("KAMAFEU_DIFFSINGER_SMOKE_LANGUAGE").unwrap_or_else(|_| "en".into());
        let provider = match std::env::var("KAMAFEU_DIFFSINGER_SMOKE_PROVIDER").as_deref() {
            Ok("coreml") => ExecutionProvider::CoreMl,
            _ => ExecutionProvider::Cpu,
        };
        let speaker = std::env::var("KAMAFEU_DIFFSINGER_SMOKE_SPEAKER")
            .ok()
            .or_else(|| bank.speakers.first().cloned())
            .unwrap();
        let explicit_phones =
            std::env::var_os("KAMAFEU_DIFFSINGER_SMOKE_EXPLICIT_PHONES").is_some();
        let phonemizer = if language == "pt" {
            crate::voicebank::diffsinger::DiffSingerPhonemizer::BrapaPhonetic
        } else {
            crate::voicebank::diffsinger::DiffSingerPhonemizer::Automatic
        };
        let mut notes = if let Ok(lyric) = std::env::var("KAMAFEU_DIFFSINGER_SMOKE_LYRIC") {
            vec![UNote::new(lyric, "C4", 0.0, 1000.0)]
        } else if language == "pt" && explicit_phones {
            let mut first = UNote::new("a", "C4", 0.0, 500.0);
            first.phonetic_hint = Some("pt/a".into());
            let mut second = UNote::new("mor", "D4", 500.0, 500.0);
            second.phonetic_hint = Some("pt/m pt/oh pt/r".into());
            vec![first, second]
        } else if language == "pt" {
            vec![UNote::new("amor", "C4", 0.0, 1000.0)]
        } else if explicit_phones {
            let mut note = UNote::new("see", "C4", 0.0, 500.0);
            note.phonetic_hint = Some("en/s en/iy".into());
            vec![note, UNote::new("me", "D4", 500.0, 500.0)]
        } else {
            vec![
                UNote::new("see", "C4", 0.0, 500.0),
                UNote::new("me", "D4", 500.0, 500.0),
            ]
        };
        if let Ok(value) = std::env::var("KAMAFEU_DIFFSINGER_SMOKE_ONSET_OFFSET_MS") {
            if let Ok(offset) = value.parse::<f64>() {
                let note_index = std::env::var("KAMAFEU_DIFFSINGER_SMOKE_ONSET_NOTE_INDEX")
                    .ok()
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(0);
                if let Some(note) = notes.get_mut(note_index) {
                    note.phoneme_overrides
                        .push(crate::project::model::UPhonemeOverride {
                            index: 0,
                            offset_ms: Some(offset),
                            ..Default::default()
                        });
                }
            }
        }
        let samples = render_track(
            &bank, &notes, &language, phonemizer, &speaker, provider, 44_100, None,
        )
        .unwrap_or_else(|error| panic!("{} {provider:?} inference: {error}", bank.name));
        assert!(!samples.is_empty(), "{} returned no samples", bank.name);
        assert!(
            samples.iter().all(|sample| sample.is_finite()),
            "{} returned non-finite samples",
            bank.name
        );
        assert!(
            samples.iter().any(|sample| sample.abs() > 1e-7),
            "{} returned silence",
            bank.name
        );
        if let Ok(path) = std::env::var("KAMAFEU_DIFFSINGER_SMOKE_WAV") {
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: 44_100,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut writer = hound::WavWriter::create(path, spec).unwrap();
            for sample in &samples {
                writer
                    .write_sample((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                    .unwrap();
            }
            writer.finalize().unwrap();
        }
        let cached_sessions = SESSIONS.lock().unwrap().len();
        let cached_render = render_track(
            &bank, &notes, &language, phonemizer, &speaker, provider, 44_100, None,
        )
        .unwrap();
        assert_eq!(
            samples.len(),
            cached_render.len(),
            "cached session changed render length"
        );
        assert_eq!(
            SESSIONS.lock().unwrap().len(),
            cached_sessions,
            "acoustic and vocoder sessions should be reused"
        );
    }
}
