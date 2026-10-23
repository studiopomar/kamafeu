//! Reproducible paired KFV 2 SOLA/WORLD render for the engine spike.
//! This is an engineering tool; it does not select a product backend.

use kamafeu::voicebank::{
    native::NativeBank,
    planner::{plan_phrase, PhoneEvent, PhrasePlan, PhraseRequest},
    render::{render_phrase_sola, render_phrase_world, RenderedPhrase},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
    time::Instant,
};

const OUTPUT_RATE: u32 = 48_000;

#[derive(Serialize)]
struct EngineReport {
    requested_backend: String,
    phrase_engine_id: String,
    unit_traces: Vec<UnitTrace>,
    render_latency_ms: f64,
    render_latency_runs_ms: Vec<f64>,
    duration_ms: f64,
    sample_count: usize,
    peak_dbfs: Option<f64>,
    rms_dbfs: Option<f64>,
    clipped_samples: usize,
    audible_window_ratio: f64,
    leading_silence_ms: f64,
    trailing_silence_ms: f64,
    estimated_output_buffer_bytes: usize,
    sample_f32_le_sha256: String,
    wav_path: String,
}

#[derive(Serialize)]
struct PairedReport {
    report_schema: String,
    bank_id: String,
    bank_version: String,
    package_revision: u32,
    bank_root: String,
    style_id: String,
    case_id: String,
    source_assets: Vec<SourceAssetReport>,
    requested_event_durations_ms: Vec<f64>,
    output_sample_rate_hz: u32,
    normalization: String,
    phrase_plan: PhrasePlan,
    sola: EngineReport,
    world: EngineReport,
    interpretation_limit: String,
}

struct TimedPhrase {
    phrase: RenderedPhrase,
    latencies_ms: Vec<f64>,
}

#[derive(Debug, Deserialize)]
struct SpikeCorpus {
    format: String,
    schema_version: String,
    cases: Vec<SpikeCase>,
}

#[derive(Debug, Deserialize)]
struct SpikeCase {
    id: String,
    style_id: String,
    events: Vec<PhoneEvent>,
}

#[derive(Serialize)]
struct SourceAssetReport {
    path: String,
    sha256: String,
}

#[derive(Serialize)]
struct UnitTrace {
    unit_id: String,
    asset_path: String,
    requested_engine: String,
    effective_engine: String,
    stages: Vec<String>,
    fallback_reasons: Vec<String>,
    adjustments: Vec<String>,
}

#[derive(Serialize)]
struct CorpusSummary {
    report_schema: String,
    bank_id: String,
    cases: Vec<CaseSummary>,
    interpretation_limit: String,
}

#[derive(Serialize)]
struct CaseSummary {
    case_id: String,
    report_path: String,
    sola_sample_f32_le_sha256: String,
    world_sample_f32_le_sha256: String,
    sola_effective_engines: Vec<String>,
    world_effective_engines: Vec<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("kfv2-engine-spike: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        return Err("usage: kfv2-engine-spike <bank-folder-or.kfv> <unit-id> <midi> <duration-ms> <output-dir>\n       kfv2-engine-spike <bank-folder-or.kfv> --corpus <corpus.json> <output-dir>".into());
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Err("usage: kfv2-engine-spike <bank-folder-or.kfv> <unit-id> <midi> <duration-ms> <output-dir>\n       kfv2-engine-spike <bank-folder-or.kfv> --corpus <corpus.json> <output-dir>".into());
    }
    let bank_path = PathBuf::from(&args[0]);
    let bank = NativeBank::load_path(&bank_path)?;
    let cache = &mut BTreeMap::new();
    if args.len() == 4 && args[1] == "--corpus" {
        let corpus_bytes = fs::read(&args[2]).map_err(|e| format!("cannot read corpus: {e}"))?;
        if corpus_bytes.len() > 16 * 1024 * 1024 {
            return Err("corpus exceeds the 16 MiB safety limit".into());
        }
        let corpus: SpikeCorpus = serde_json::from_slice(&corpus_bytes)
            .map_err(|e| format!("invalid corpus JSON: {e}"))?;
        if corpus.format != "kamafeu.kfv2-engine-spike-corpus" || corpus.schema_version != "1.0.0" {
            return Err("unsupported engine-spike corpus format or version".into());
        }
        if corpus.cases.is_empty() || corpus.cases.len() > 100 {
            return Err("corpus must contain 1 to 100 cases".into());
        }
        let output_dir = PathBuf::from(&args[3]);
        fs::create_dir_all(&output_dir)
            .map_err(|e| format!("cannot create output directory: {e}"))?;
        let mut seen = HashSet::new();
        let mut summaries = Vec::with_capacity(corpus.cases.len());
        for case in &corpus.cases {
            if !valid_case_id(&case.id) || !seen.insert(case.id.clone()) {
                return Err(format!("invalid or duplicate case id: {}", case.id));
            }
            if case.events.is_empty() || case.events.len() > 128 {
                return Err(format!("case '{}' must contain 1 to 128 events", case.id));
            }
            summaries.push(run_case(&bank, case, &output_dir.join(&case.id), cache)?);
        }
        let summary = CorpusSummary {
            report_schema: "kamafeu.kfv2-engine-spike-corpus-report/1.0.0".into(),
            bank_id: bank.manifest.bank_id.clone(), cases: summaries,
            interpretation_limit: "Technical renders do not establish intelligibility, naturalness, license, or backend suitability. Human review and listening remain required.".into(),
        };
        let path = output_dir.join("corpus_report.json");
        fs::write(
            &path,
            serde_json::to_vec_pretty(&summary).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("cannot save corpus summary: {e}"))?;
        println!("Corpus report: {}", path.display());
        return Ok(());
    }
    if args.len() != 5 {
        return Err("usage: kfv2-engine-spike <bank-folder-or.kfv> <unit-id> <midi> <duration-ms> <output-dir>\n       kfv2-engine-spike <bank-folder-or.kfv> --corpus <corpus.json> <output-dir>".into());
    }
    let unit_id = &args[1];
    let midi: u8 = args[2]
        .parse()
        .map_err(|_| "midi must be an integer from 0 to 127")?;
    let duration_ms: f64 = args[3]
        .parse()
        .map_err(|_| "duration-ms must be a positive number")?;
    if midi > 127 || !duration_ms.is_finite() || duration_ms <= 0.0 || duration_ms > 300_000.0 {
        return Err("midi or duration-ms is outside the supported range".into());
    }
    let (layer, units) = bank
        .layers
        .iter()
        .find(|(_, file)| file.units.iter().any(|unit| unit.id == *unit_id))
        .ok_or_else(|| format!("unit not found in bank: {unit_id}"))?;
    let unit = units
        .units
        .iter()
        .find(|unit| unit.id == *unit_id)
        .ok_or("selected unit disappeared")?;
    let case = SpikeCase {
        id: unit.id.clone(),
        style_id: layer.style_id.clone(),
        events: vec![PhoneEvent {
            phones: unit.phones.clone(),
            kind: unit.kind.clone(),
            position_ms: 0.0,
            duration_ms,
            midi,
            pitch_points: Vec::new(),
            expressions: kamafeu::project::model::UExpressions::default(),
            flags: String::new(),
            unit_override_id: Some(unit.id.clone()),
            layer_override_id: Some(layer.id.clone()),
        }],
    };
    let output_dir = PathBuf::from(&args[4]);
    fs::create_dir_all(&output_dir).map_err(|e| format!("cannot create output directory: {e}"))?;
    let summary = run_case(&bank, &case, &output_dir, cache)?;
    println!("SOLA/WORLD: {}", summary.report_path);
    Ok(())
}

fn valid_case_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn run_case(
    bank: &NativeBank,
    case: &SpikeCase,
    output_dir: &Path,
    source_hashes: &mut BTreeMap<String, String>,
) -> Result<CaseSummary, String> {
    fs::create_dir_all(output_dir).map_err(|e| format!("cannot create case directory: {e}"))?;
    let plan = plan_phrase(
        bank,
        &PhraseRequest {
            style_id: case.style_id.clone(),
            events: case.events.clone(),
        },
    )?;
    if plan.units.len() != case.events.len() {
        return Err(format!(
            "planner event count mismatch for case '{}'",
            case.id
        ));
    }
    let mut assets = BTreeMap::new();
    for planned in &plan.units {
        let (_, unit_file) = bank
            .layers
            .iter()
            .find(|(layer, _)| layer.id == planned.layer_id)
            .ok_or("planned layer is missing")?;
        let unit = unit_file
            .units
            .iter()
            .find(|unit| unit.id == planned.unit_id)
            .ok_or("planned unit is missing")?;
        let asset = unit_file
            .assets
            .iter()
            .find(|asset| asset.id == unit.asset_id)
            .ok_or("planned unit asset is missing")?;
        if !source_hashes.contains_key(&asset.path) {
            let path = bank.manifest.resolve_existing(&bank.root, &asset.path)?;
            source_hashes.insert(asset.path.clone(), hash_file(&path)?);
        }
        assets.insert(asset.path.clone(), source_hashes[&asset.path].clone());
    }
    fs::write(
        output_dir.join("phrase_plan.json"),
        serde_json::to_vec_pretty(&plan).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("cannot save phrase plan: {e}"))?;
    let sola = render_timed(bank, &plan, false)?;
    let world = render_timed(bank, &plan, true)?;
    let sola_path = output_dir.join("sola.wav");
    let world_path = output_dir.join("world.wav");
    write_wav(&sola_path, &sola.phrase)?;
    write_wav(&world_path, &world.phrase)?;
    let report = PairedReport {
        report_schema: "kamafeu.kfv2-engine-spike/1.0.0".into(),
        bank_id: bank.manifest.bank_id.clone(), bank_version: bank.manifest.bank_version.clone(),
        package_revision: bank.manifest.package_revision, bank_root: bank.root.display().to_string(),
        style_id: case.style_id.clone(), case_id: case.id.clone(),
        source_assets: assets.into_iter().map(|(path, sha256)| SourceAssetReport { path, sha256 }).collect(),
        requested_event_durations_ms: case.events.iter().map(|event| event.duration_ms).collect(),
        output_sample_rate_hz: OUTPUT_RATE,
        normalization: "none; identical output rate, phrase plan and gain; loudness is not normalized".into(),
        phrase_plan: plan,
        sola: report_engine("kamafeu.sola", &sola.phrase, &sola.latencies_ms, &sola_path),
        world: report_engine("kamafeu.tainah-motor", &world.phrase, &world.latencies_ms, &world_path),
        interpretation_limit: "Technical paired render only. Latency is the median of five sequential renders per backend; scheduling and thermal state can still affect it. Buffer bytes estimate only the returned mono f32 phrase, not peak process memory. Migration marks may be unreviewed and the source license may be undefined. This report does not establish intelligibility, naturalness, authorization, or a public product backend.".into(),
    };
    let report_path = output_dir.join("report.json");
    fs::write(
        &report_path,
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("cannot save report: {e}"))?;
    Ok(CaseSummary {
        case_id: case.id.clone(),
        report_path: report_path.display().to_string(),
        sola_sample_f32_le_sha256: report.sola.sample_f32_le_sha256.clone(),
        world_sample_f32_le_sha256: report.world.sample_f32_le_sha256.clone(),
        sola_effective_engines: report
            .sola
            .unit_traces
            .iter()
            .map(|trace| trace.effective_engine.clone())
            .collect(),
        world_effective_engines: report
            .world
            .unit_traces
            .iter()
            .map(|trace| trace.effective_engine.clone())
            .collect(),
    })
}

fn render_timed(
    bank: &NativeBank,
    plan: &kamafeu::voicebank::planner::PhrasePlan,
    world: bool,
) -> Result<TimedPhrase, String> {
    let mut latencies_ms = Vec::with_capacity(5);
    let mut phrase = None;
    for _ in 0..5 {
        let start = Instant::now();
        let rendered = if world {
            render_phrase_world(bank, plan, OUTPUT_RATE)?
        } else {
            render_phrase_sola(bank, plan, OUTPUT_RATE)?
        };
        latencies_ms.push(start.elapsed().as_secs_f64() * 1_000.0);
        phrase = Some(rendered);
    }
    Ok(TimedPhrase {
        phrase: phrase.ok_or("render loop produced no phrase")?,
        latencies_ms,
    })
}

fn report_engine(
    requested: &str,
    phrase: &RenderedPhrase,
    latencies_ms: &[f64],
    path: &Path,
) -> EngineReport {
    let samples = &phrase.samples;
    let peak = samples.iter().fold(0.0f32, |v, sample| v.max(sample.abs()));
    let rms = (samples
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>()
        / samples.len().max(1) as f64)
        .sqrt();
    let block = (OUTPUT_RATE as usize / 100).max(1); // 10 ms windows
    let windows: Vec<f32> = samples
        .chunks(block)
        .map(|chunk| chunk.iter().fold(0.0f32, |v, s| v.max(s.abs())))
        .collect();
    let audible = |value: f32| value > 0.001;
    let first = windows.iter().position(|v| audible(*v));
    let last = windows.iter().rposition(|v| audible(*v));
    let unit_traces = phrase
        .units
        .iter()
        .map(|unit| UnitTrace {
            unit_id: unit.unit_id.clone(),
            asset_path: unit.asset_path.clone(),
            requested_engine: unit.requested_engine.clone(),
            effective_engine: unit.effective_engine.clone(),
            stages: unit.stages.clone(),
            fallback_reasons: unit.fallback_reasons.clone(),
            adjustments: unit.adjustments.clone(),
        })
        .collect();
    let pcm: Vec<u8> = samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    let mut sorted_latencies = latencies_ms.to_vec();
    sorted_latencies.sort_by(f64::total_cmp);
    EngineReport {
        requested_backend: requested.into(),
        phrase_engine_id: phrase.engine_id.clone(),
        unit_traces,
        render_latency_ms: sorted_latencies[sorted_latencies.len() / 2],
        render_latency_runs_ms: latencies_ms.to_vec(),
        duration_ms: samples.len() as f64 * 1_000.0 / OUTPUT_RATE as f64,
        sample_count: samples.len(),
        peak_dbfs: dbfs(f64::from(peak)),
        rms_dbfs: dbfs(rms),
        clipped_samples: samples
            .iter()
            .filter(|sample| sample.abs() >= 0.999)
            .count(),
        audible_window_ratio: windows.iter().filter(|v| audible(**v)).count() as f64
            / windows.len().max(1) as f64,
        leading_silence_ms: first
            .map_or(samples.len() as f64 * 1_000.0 / OUTPUT_RATE as f64, |i| {
                (i * block) as f64 * 1_000.0 / OUTPUT_RATE as f64
            }),
        trailing_silence_ms: last
            .map_or(samples.len() as f64 * 1_000.0 / OUTPUT_RATE as f64, |i| {
                ((windows.len() - 1 - i) * block) as f64 * 1_000.0 / OUTPUT_RATE as f64
            }),
        estimated_output_buffer_bytes: samples.len() * std::mem::size_of::<f32>(),
        sample_f32_le_sha256: hex(&Sha256::digest(pcm)),
        wav_path: path.display().to_string(),
    }
}

fn dbfs(value: f64) -> Option<f64> {
    (value > 0.0).then(|| 20.0 * value.log10())
}
fn hash_file(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let mut file =
        fs::File::open(path).map_err(|e| format!("cannot open source for fingerprinting: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| format!("cannot hash source asset: {e}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex(&hasher.finalize()))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn write_wav(path: &Path, phrase: &RenderedPhrase) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: phrase.sample_rate_hz,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    for sample in &phrase.samples {
        let pcm = (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
        writer
            .write_sample(pcm)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    }
    writer
        .finalize()
        .map_err(|e| format!("cannot finish {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_corpus_case_preserves_multi_event_plan_and_reports_each_unit() {
        let root = tempfile::tempdir().unwrap();
        let write_json = |relative: &str, contents: &str| {
            let path = root.path().join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        };
        write_json(
            "kamafeu_voicebank.json",
            include_str!("../../docs/kfv2/examples/minimal/kamafeu_voicebank.json"),
        );
        write_json(
            "phonetics/pt-BR.json",
            include_str!("../../docs/kfv2/examples/minimal/phonetics/pt-BR.json"),
        );
        let mut profile: serde_json::Value = serde_json::from_str(include_str!(
            "../../docs/kfv2/examples/minimal/phonetics/pt-BR.json"
        ))
        .unwrap();
        profile["symbols"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"id":"s","ipa":"s","class":"fricative","voiced":false}));
        write_json(
            "phonetics/pt-BR.json",
            &serde_json::to_string(&profile).unwrap(),
        );
        let mut units: serde_json::Value = serde_json::from_str(include_str!(
            "../../docs/kfv2/examples/minimal/units/natural-c4.json"
        ))
        .unwrap();
        units["units"].as_array_mut().unwrap().push(serde_json::json!({
            "id":"as-1", "asset_id":"take-1", "kind":"VC", "phones":["a","s"],
            "left_context":{"type":"any"}, "right_context":{"type":"boundary"},
            "source":{"start_frame":0,"end_frame":4800,"anchor_frame":1200,"fixed_end_frame":2400},
            "timing":{"min_target_ms":35.0,"preferred_crossfade_ms":8.0,"max_crossfade_ms":20.0}
        }));
        write_json(
            "units/natural-c4.json",
            &serde_json::to_string(&units).unwrap(),
        );
        let wav_path = root.path().join("audio/natural/c4/take-1.wav");
        fs::create_dir_all(wav_path.parent().unwrap()).unwrap();
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut wav = hound::WavWriter::create(wav_path, spec).unwrap();
        for frame in 0..4_800 {
            let sample = (0.15
                * (f64::from(frame) * std::f64::consts::TAU * 220.0 / 48_000.0).sin()
                * i16::MAX as f64) as i16;
            wav.write_sample(sample).unwrap();
        }
        wav.finalize().unwrap();

        let bank = NativeBank::load_dir(root.path()).unwrap();
        let case = SpikeCase {
            id: "two-event-test".into(),
            style_id: "natural".into(),
            events: vec![
                PhoneEvent {
                    phones: vec!["k".into(), "a".into()],
                    kind: "CV".into(),
                    position_ms: 0.0,
                    duration_ms: 100.0,
                    midi: 60,
                    pitch_points: vec![],
                    expressions: crate::project::model::UExpressions::default(),
                    flags: String::new(),
                    unit_override_id: Some("ka-1".into()),
                    layer_override_id: None,
                },
                PhoneEvent {
                    phones: vec!["a".into(), "s".into()],
                    kind: "VC".into(),
                    position_ms: 100.0,
                    duration_ms: 100.0,
                    midi: 60,
                    pitch_points: vec![],
                    expressions: crate::project::model::UExpressions::default(),
                    flags: String::new(),
                    unit_override_id: Some("as-1".into()),
                    layer_override_id: None,
                },
            ],
        };
        let output = tempfile::tempdir().unwrap();
        let summary = run_case(&bank, &case, output.path(), &mut BTreeMap::new()).unwrap();
        assert_eq!(
            summary.sola_effective_engines,
            vec!["kamafeu.sola", "kamafeu.sola"]
        );
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(output.path().join("report.json")).unwrap()).unwrap();
        assert_eq!(report["phrase_plan"]["units"].as_array().unwrap().len(), 2);
        assert_eq!(report["sola"]["unit_traces"].as_array().unwrap().len(), 2);
        assert_eq!(report["world"]["unit_traces"].as_array().unwrap().len(), 2);
        assert!(output.path().join("sola.wav").is_file());
        assert!(output.path().join("world.wav").is_file());

        let mut authoring =
            kamafeu::copaiba::native_project::CopaibaNativeProject::load_dir(root.path()).unwrap();
        let package_dir = tempfile::tempdir().unwrap();
        let package_path = package_dir.path().join("synthetic.kfv");
        authoring.package_kfv2_draft(&package_path).unwrap();
        let packaged_bank = NativeBank::load_path(&package_path).unwrap();
        let packaged_plan = plan_phrase(
            &packaged_bank,
            &PhraseRequest {
                style_id: case.style_id.clone(),
                events: case.events.clone(),
            },
        )
        .unwrap();
        let folder_plan: PhrasePlan =
            serde_json::from_slice(&fs::read(output.path().join("phrase_plan.json")).unwrap())
                .unwrap();
        assert_eq!(
            folder_plan, packaged_plan,
            "folder and package must make the same unit decisions"
        );
        let packaged_output = tempfile::tempdir().unwrap();
        let packaged_summary = run_case(
            &packaged_bank,
            &case,
            packaged_output.path(),
            &mut BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(
            summary.sola_sample_f32_le_sha256,
            packaged_summary.sola_sample_f32_le_sha256
        );
        assert_eq!(
            summary.world_sample_f32_le_sha256,
            packaged_summary.world_sample_f32_le_sha256
        );
    }
}
