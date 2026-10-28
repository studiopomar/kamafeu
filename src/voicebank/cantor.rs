//! Native Kamafeu singer format, identified only by `cantor.toml`.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cantor {
    pub versao: u32,
    pub id: String,
    pub nome: String,
    #[serde(default)]
    pub idiomas: Vec<String>,
    #[serde(default = "default_engine")]
    pub engine: String,
    #[serde(default)]
    pub ajustes: Ajustes,
    #[serde(default)]
    pub tons: Vec<Tom>,
    #[serde(default)]
    pub imagem: Option<String>,
    #[serde(default)]
    pub portrait: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CantorBank {
    pub root: PathBuf,
    pub cantor: Cantor,
    pub(crate) _workspace: Option<Arc<tempfile::TempDir>>,
}

fn default_engine() -> String {
    "tainah".into()
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn has_transparency(image: &image::DynamicImage) -> bool {
    image.to_rgba8().pixels().any(|pixel| pixel.0[3] < u8::MAX)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Ajustes {
    #[serde(default)]
    pub ganho_db: f32,
    #[serde(default)]
    pub transicao_ms: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tom {
    pub id: String,
    pub midi: u8,
    #[serde(default)]
    pub gravacoes: Vec<Gravacao>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Gravacao {
    pub id: String,
    pub arquivo: String,
    /// Phoneme sequence, e.g. ["k", "a", "m", "a"].
    pub fonemas: Vec<String>,
    /// Boundaries in milliseconds, one per phoneme boundary, including 0 and end.
    pub marcacoes_ms: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unidade {
    pub tom_id: String,
    pub gravacao_id: String,
    pub arquivo: PathBuf,
    pub tipo: UnidadeTipo,
    pub fonemas: [String; 2],
    pub inicio_ms: f64,
    pub fim_ms: f64,
}

#[derive(Debug, Clone)]
pub struct NotaCantor {
    pub fonemas_cv: [String; 2],
    pub inicio_ms: f64,
    pub duracao_ms: f64,
    pub midi: u8,
}

#[derive(Debug, Clone)]
pub struct PlanoCantor {
    pub plano: super::planner::PhrasePlan,
    pub avisos: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnidadeTipo {
    Cv,
    Vc,
}

impl Cantor {
    pub fn load_dir(root: &Path) -> Result<Self, String> {
        let config = root.join("cantor.toml");
        let text =
            std::fs::read_to_string(&config).map_err(|e| format!("{}: {e}", config.display()))?;
        let cantor: Self = toml::from_str(&text)
            .map_err(|e| format!("{}: TOML inválido: {e}", config.display()))?;
        cantor.validate(root)?;
        Ok(cantor)
    }

    pub fn validate(&self, root: &Path) -> Result<(), String> {
        if self.versao != 1 {
            return Err(format!("versão cantor.toml não suportada: {}", self.versao));
        }
        if !safe_id(&self.id) || self.nome.trim().is_empty() {
            return Err("cantor.toml exige id e nome".into());
        }
        if self.engine.trim().is_empty() {
            return Err("cantor.toml exige uma engine nativa".into());
        }
        if !self.ajustes.ganho_db.is_finite()
            || !(-24.0..=24.0).contains(&self.ajustes.ganho_db)
            || !self.ajustes.transicao_ms.is_finite()
            || self.ajustes.transicao_ms < 0.0
            || self.ajustes.transicao_ms > 500.0
        {
            return Err("ganho deve estar entre -24 e +24 dB; transicao_ms entre 0 e 500".into());
        }
        let mut tones = std::collections::HashSet::new();
        for tone in &self.tons {
            if !safe_id(&tone.id) || tone.midi > 127 || !tones.insert(&tone.id) {
                return Err(format!("tom inválido ou duplicado: {}", tone.id));
            }
            let mut recordings = std::collections::HashSet::new();
            for recording in &tone.gravacoes {
                if !safe_id(&recording.id) || !recordings.insert(&recording.id) {
                    return Err(format!("gravação inválida ou duplicada: {}", recording.id));
                }
                let rel = Path::new(&recording.arquivo);
                if !super::native::safe_relative_path(&recording.arquivo) {
                    return Err(format!("caminho de áudio inseguro: {}", recording.arquivo));
                }
                let path = root.join(rel);
                if !path.is_file() {
                    return Err(format!("áudio não encontrado: {}", recording.arquivo));
                }
                if !matches!(
                    path.extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase()
                        .as_str(),
                    "wav" | "mp3"
                ) {
                    return Err(format!(
                        "formato de áudio não aceito: {}",
                        recording.arquivo
                    ));
                }
                let count = recording.fonemas.len();
                if count < 2 || recording.marcacoes_ms.len() != count + 1 {
                    return Err(format!(
                        "{}: forneça ao menos dois fonemas e uma marcação em cada limite",
                        recording.id
                    ));
                }
                if recording.fonemas.iter().any(|p| p.trim().is_empty()) {
                    return Err(format!("{}: fonema vazio", recording.id));
                }
                if recording.fonemas.iter().any(|p| {
                    p.chars()
                        .any(|c| c.is_whitespace() || matches!(c, ',' | '=' | '\n' | '\r'))
                }) {
                    return Err(format!(
                        "{}: cada fonema deve ser um token sem espaço, vírgula ou '='",
                        recording.id
                    ));
                }
                if recording
                    .fonemas
                    .windows(2)
                    .any(|pair| is_vowel(&pair[0]) == is_vowel(&pair[1]))
                {
                    return Err(format!(
                        "{}: a lista de fonemas deve alternar consoante e vogal para formar CV/VC",
                        recording.id
                    ));
                }
                let mut prior = -1.0;
                for mark in &recording.marcacoes_ms {
                    if !mark.is_finite() || *mark < 0.0 || *mark <= prior {
                        return Err(format!(
                            "{}: marcações devem ser finitas, não negativas e crescentes",
                            recording.id
                        ));
                    }
                    prior = *mark;
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let info = crate::audio::probe_audio_file(&path).ok_or_else(|| {
                        format!("não foi possível decodificar {}", recording.arquivo)
                    })?;
                    if *recording.marcacoes_ms.last().unwrap() > info.duration_ms {
                        return Err(format!(
                            "{}: marcação excede a duração do áudio",
                            recording.id
                        ));
                    }
                }
            }
        }
        for p in [self.imagem.as_deref(), self.portrait.as_deref()]
            .into_iter()
            .flatten()
        {
            let rel = Path::new(p);
            if !super::native::safe_relative_path(p) || !root.join(rel).is_file() {
                return Err(format!("asset de apresentação ausente ou inseguro: {p}"));
            }
        }
        if let Some(path) = &self.imagem {
            let (width, height) = image::image_dimensions(root.join(path))
                .map_err(|e| format!("imagem do cantor inválida: {e}"))?;
            if width != 200 || height != 200 {
                return Err("imagem do cantor deve medir 200×200 pixels".into());
            }
        }
        if let Some(path) = &self.portrait {
            let image =
                image::open(root.join(path)).map_err(|e| format!("portrait inválido: {e}"))?;
            if !has_transparency(&image) {
                return Err("portrait do piano roll precisa conter pixels transparentes".into());
            }
        }
        Ok(())
    }

    /// CVC recordings produce CV and VC slices over the same original file.
    pub fn unidades(&self, root: &Path) -> Vec<Unidade> {
        let mut out = Vec::new();
        for tone in &self.tons {
            for rec in &tone.gravacoes {
                for i in 0..rec.fonemas.len() - 1 {
                    let (a, b) = (&rec.fonemas[i], &rec.fonemas[i + 1]);
                    if is_vowel(a) == is_vowel(b) {
                        continue;
                    }
                    let kind = if is_vowel(a) {
                        UnidadeTipo::Vc
                    } else {
                        UnidadeTipo::Cv
                    };
                    out.push(Unidade {
                        tom_id: tone.id.clone(),
                        gravacao_id: rec.id.clone(),
                        arquivo: root.join(&rec.arquivo),
                        tipo: kind,
                        fonemas: [a.clone(), b.clone()],
                        inicio_ms: rec.marcacoes_ms[i],
                        fim_ms: rec.marcacoes_ms[i + 2],
                    });
                }
            }
        }
        out
    }
}

impl CantorBank {
    pub fn install_package_atomic(
        package: &Path,
        destination: &Path,
    ) -> Result<(String, String), String> {
        if !package
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("kfv"))
        {
            return Err("o pacote do cantor deve usar .kfv".into());
        }
        let parent = destination
            .parent()
            .ok_or("destino de instalação sem pasta pai")?;
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("falha ao criar pasta de instalação: {e}"))?;
        if destination.exists() {
            return Err(format!("o destino já existe: {}", destination.display()));
        }
        let stage = tempfile::Builder::new()
            .prefix(".cantor-install-")
            .tempdir_in(parent)
            .map_err(|e| format!("falha ao criar estágio de instalação: {e}"))?;
        crate::oto::voicebank::extract_kfv(package, stage.path())
            .map_err(|e| format!("pacote .kfv inválido: {e}"))?;
        let cantor = Cantor::load_dir(stage.path())?;
        let snapshot = Self {
            root: stage.path().to_path_buf(),
            cantor: cantor.clone(),
            _workspace: None,
        }
        .native_snapshot()?;
        let hash = snapshot.content_sha256()?;
        std::fs::rename(stage.path(), destination)
            .map_err(|e| format!("falha ao instalar cantor: {e}"))?;
        let _ = stage.keep();
        Ok((cantor.nome, hash))
    }

    pub fn load_path(path: &Path) -> Result<Self, String> {
        if path.is_dir() {
            return Self::load_dir(path, None);
        }
        if !path.is_file()
            || !path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("kfv"))
        {
            return Err(format!(
                "banco cantor não é uma pasta ou pacote .kfv: {}",
                path.display()
            ));
        }
        let workspace = tempfile::Builder::new()
            .prefix("kamafeu_cantor_")
            .tempdir()
            .map_err(|e| format!("não foi possível criar área temporária: {e}"))?;
        crate::oto::voicebank::extract_kfv(path, workspace.path())
            .map_err(|e| format!("pacote .kfv inválido: {e}"))?;
        let root = workspace.path().to_path_buf();
        Self::load_dir(&root, Some(Arc::new(workspace)))
    }

    fn load_dir(path: &Path, workspace: Option<Arc<tempfile::TempDir>>) -> Result<Self, String> {
        let root = std::fs::canonicalize(path)
            .map_err(|e| format!("não foi possível resolver pasta do cantor: {e}"))?;
        if root.join("oto.ini").exists() {
            return Err("conflito de formato: o banco contém cantor.toml e oto.ini".into());
        }
        let cantor = Cantor::load_dir(&root)?;
        Ok(Self {
            root,
            cantor,
            _workspace: workspace,
        })
    }

    /// Adapts authored marks to the shared phrase planner. MP3 sources are
    /// decoded into temporary PCM WAVs; originals remain untouched.
    pub fn native_snapshot(&self) -> Result<super::native::NativeBank, String> {
        use super::native::*;
        let audio_workspace = tempfile::Builder::new()
            .prefix("kamafeu_cantor_pcm_")
            .tempdir()
            .map_err(|e| format!("falha ao criar cache temporário de áudio: {e}"))?;
        std::fs::create_dir_all(audio_workspace.path().join("recordings"))
            .map_err(|e| format!("falha ao preparar cache temporário de áudio: {e}"))?;
        let presentation = if self.cantor.imagem.is_some() || self.cantor.portrait.is_some() {
            std::fs::create_dir_all(audio_workspace.path().join("assets"))
                .map_err(|e| format!("falha ao preparar assets de apresentação: {e}"))?;
            for relative in [
                self.cantor.imagem.as_deref(),
                self.cantor.portrait.as_deref(),
            ]
            .into_iter()
            .flatten()
            {
                let source = self.root.join(relative);
                let target = audio_workspace.path().join(relative);
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::copy(source, target)
                    .map_err(|e| format!("falha ao copiar asset de apresentação: {e}"))?;
            }
            Some(Presentation {
                avatar: self.cantor.imagem.clone(),
                portrait: self.cantor.portrait.clone(),
                accent_color: None,
            })
        } else {
            None
        };
        let mut layers = Vec::new();
        let mut phones = BTreeSet::new();
        for (tone_index, tone) in self.cantor.tons.iter().enumerate() {
            let mut assets = Vec::new();
            let mut units = Vec::new();
            for (recording_index, recording) in tone.gravacoes.iter().enumerate() {
                let source = self.root.join(&recording.arquivo);
                let ext = source
                    .extension()
                    .and_then(|x| x.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let stem = format!("t{tone_index}_r{recording_index}.wav");
                let asset_path = audio_workspace.path().join("recordings").join(&stem);
                let info = if ext == "mp3" {
                    let decoded = crate::audio::load_audio_file(&source)
                        .map_err(|e| format!("falha ao decodificar {}: {e}", recording.arquivo))?;
                    if decoded.channels == 0
                        || decoded.channels > 2
                        || decoded.sample_rate == 0
                        || decoded.samples.is_empty()
                    {
                        return Err(format!(
                            "áudio MP3 com formato não aceito: {}",
                            recording.arquivo
                        ));
                    }
                    let spec = hound::WavSpec {
                        channels: decoded.channels,
                        sample_rate: decoded.sample_rate,
                        bits_per_sample: 16,
                        sample_format: hound::SampleFormat::Int,
                    };
                    let mut writer = hound::WavWriter::create(&asset_path, spec)
                        .map_err(|e| format!("falha ao preparar PCM: {e}"))?;
                    for sample in &decoded.samples {
                        writer
                            .write_sample((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                            .map_err(|e| format!("falha ao escrever PCM: {e}"))?;
                    }
                    writer
                        .finalize()
                        .map_err(|e| format!("falha ao finalizar PCM: {e}"))?;
                    crate::audio::AudioFileInfo {
                        duration_ms: decoded.duration_ms,
                        sample_rate: decoded.sample_rate,
                        channels: decoded.channels,
                    }
                } else {
                    let info = crate::audio::probe_audio_file(&source).ok_or_else(|| {
                        format!("não foi possível decodificar {}", recording.arquivo)
                    })?;
                    std::fs::copy(&source, &asset_path)
                        .map_err(|e| format!("falha ao preparar cópia de áudio: {e}"))?;
                    info
                };
                let wav = hound::WavReader::open(&asset_path)
                    .map_err(|e| format!("áudio não pode ser lido para síntese: {e}"))?;
                let frames = wav.duration() as u64;
                let id = recording.id.clone();
                assets.push(Asset {
                    id: id.clone(),
                    path: format!("recordings/{stem}"),
                    sample_rate_hz: info.sample_rate,
                    channels: info.channels,
                    frame_count: frames,
                });
                phones.extend(recording.fonemas.iter().cloned());
                for i in 0..recording.fonemas.len() - 1 {
                    let first = recording.fonemas[i].clone();
                    let second = recording.fonemas[i + 1].clone();
                    if is_vowel(&first) == is_vowel(&second) {
                        continue;
                    }
                    let is_vc = is_vowel(&first);
                    let start = (recording.marcacoes_ms[i] * info.sample_rate as f64 / 1000.0)
                        .round() as u64;
                    let anchor = (recording.marcacoes_ms[i + 1] * info.sample_rate as f64 / 1000.0)
                        .round() as u64;
                    let end = (recording.marcacoes_ms[i + 2] * info.sample_rate as f64 / 1000.0)
                        .round() as u64;
                    let kind = if is_vc { "VC" } else { "CV" };
                    // In CV, preserve the initial consonant and let the vowel
                    // carry note duration. In VC, stretch the vowel and keep
                    // the final consonant as a WORLD release region.
                    let (anchor_frame, fixed_end_frame, release_start_frame) = if is_vc {
                        (start, start, Some(anchor))
                    } else {
                        (anchor, anchor, None)
                    };
                    units.push(Unit {
                        id: format!("{}-{}", recording.id, i),
                        asset_id: id.clone(),
                        kind: kind.into(),
                        phones: vec![first, second],
                        left_context: Context {
                            kind: "any".into(),
                            phone: None,
                        },
                        right_context: Context {
                            kind: "any".into(),
                            phone: None,
                        },
                        legacy_aliases: vec![],
                        source: SourceRegion {
                            start_frame: start,
                            end_frame: end,
                            anchor_frame,
                            fixed_end_frame,
                            sustain: None,
                            release_start_frame,
                        },
                        timing: UnitTiming {
                            min_target_ms: 1.0,
                            preferred_crossfade_ms: f64::from(
                                self.cantor.ajustes.transicao_ms.max(0.0),
                            ),
                            max_crossfade_ms: f64::from(self.cantor.ajustes.transicao_ms.max(0.0)),
                        },
                        render: RenderTuning {
                            gain_db: f64::from(self.cantor.ajustes.ganho_db),
                            ..RenderTuning::default()
                        },
                        quality: Quality {
                            reviewed: true,
                            priority: 0,
                        },
                    });
                }
            }
            let layer_id = tone.id.clone();
            layers.push((
                Layer {
                    id: layer_id.clone(),
                    style_id: "natural".into(),
                    recorded_midi: tone.midi,
                    units_path: format!("units/{layer_id}.json"),
                },
                UnitFile {
                    format: "kamafeu.units".into(),
                    schema_version: "1.0.0".into(),
                    layer_id,
                    assets,
                    units,
                },
            ));
        }
        if phones.is_empty() {
            return Err("o cantor ainda não tem gravações com fonemas".into());
        }
        let profile = PhoneticProfile {
            format: "kamafeu.phonetics".into(),
            schema_version: "1.0.0".into(),
            id: "cantor-phones".into(),
            language: self
                .cantor
                .idiomas
                .first()
                .cloned()
                .unwrap_or_else(|| "pt-BR".into()),
            symbols: phones
                .iter()
                .map(|p| PhoneSymbol {
                    id: p.clone(),
                    ipa: p.clone(),
                    class: if is_vowel(p) { "vowel" } else { "consonant" }.into(),
                    voiced: true,
                })
                .collect(),
            lexicon: BTreeMap::new(),
            unknown_word_policy: "ask-for-pronunciation".into(),
        };
        profile.validate()?;
        let profiles = vec![profile];
        let manifest = Manifest {
            format: "kamafeu.voicebank".into(),
            schema_version: "2.0.0".into(),
            voice_id: self.cantor.id.clone(),
            bank_id: self.cantor.id.clone(),
            bank_version: "1.0.0".into(),
            package_revision: 1,
            display_name: self.cantor.nome.clone(),
            publisher: "Kamafeu KAMUSE".into(),
            description: toml::to_string(&self.cantor)
                .map_err(|e| format!("falha ao normalizar cantor.toml para identidade: {e}"))?,
            languages: vec![profiles[0].language.clone()],
            phonetic_profiles: vec![PhoneticProfileRef {
                id: profiles[0].id.clone(),
                language: profiles[0].language.clone(),
                path: "phonetics/generated.json".into(),
            }],
            engine: EngineRequirement {
                id: "kamafeu.tainah-world".into(),
                api_major: 1,
                required_features: vec![],
            },
            styles: vec![Style {
                id: "natural".into(),
                display_name: "Natural".into(),
            }],
            layers: layers.iter().map(|(l, _)| l.clone()).collect(),
            defaults: Defaults {
                phonetic_profile_id: profiles[0].id.clone(),
                style_id: "natural".into(),
                layer_policy: "nearest-recorded".into(),
                missing_unit_policy: "error".into(),
            },
            presentation,
            license: None,
        };
        let workspace = Arc::new(audio_workspace);
        Ok(NativeBank {
            root: workspace.path().to_path_buf(),
            manifest,
            profiles,
            layers,
            temp_dir: Some(workspace),
        })
    }

    pub fn project_reference(
        &self,
        source_path_hint: Option<String>,
    ) -> Result<crate::project::NativeVoicebankReference, String> {
        self.native_snapshot()?.project_reference(source_path_hint)
    }

    pub fn plan_notes(&self, notes: &[NotaCantor]) -> Result<PlanoCantor, String> {
        if self.cantor.engine != "tainah" {
            return Err(format!(
                "engine nativa não suportada: {}",
                self.cantor.engine
            ));
        }
        if notes.is_empty() {
            return Err("adicione ao menos uma nota CV para gerar a prévia".into());
        }
        let bank = self.native_snapshot()?;
        let events = notes
            .iter()
            .enumerate()
            .map(|(index, note)| {
                if !note.inicio_ms.is_finite()
                    || !note.duracao_ms.is_finite()
                    || note.duracao_ms <= 0.0
                    || note.midi > 127
                {
                    return Err(format!("nota {index} tem posição, duração ou tom inválido"));
                }
                Ok(super::planner::PhoneEvent {
                    phones: note.fonemas_cv.to_vec(),
                    kind: "CV".into(),
                    position_ms: note.inicio_ms,
                    duration_ms: note.duracao_ms,
                    midi: note.midi,
                    pitch_points: Vec::new(),
                    unit_override_id: None,
                    layer_override_id: None,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let plan = super::planner::plan_phrase(
            &bank,
            &super::planner::PhraseRequest {
                style_id: bank.manifest.defaults.style_id.clone(),
                events,
            },
        )?;
        Ok(PlanoCantor {
            plano: plan,
            avisos: Vec::new(),
        })
    }
}

fn is_vowel(phone: &str) -> bool {
    matches!(
        phone.to_lowercase().as_str(),
        "a" | "e" | "i" | "o" | "u" | "á" | "é" | "í" | "ó" | "ú" | "â" | "ê" | "ô" | "ã" | "õ"
    )
}
