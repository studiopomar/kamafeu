//! Blind paired comparison of Tainah and straycat-rs on reviewed APS phrases.
//!
//! Usage: cargo run --bin tainah-ab -- corpus.json straycat-rs output-dir [wavtool]

use kamafeu::{
    ApsFormat, ExternalResamplerDriver, ExternalWavtoolDriver, GalapagosWavtoolDriver,
    KnownResampler, NativeWorldResamplerDriver, ProjectRenderer, RenderOptions, ResamplerDriver,
    TrackRenderer, Voicebank, WavtoolDriver,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const OUTPUT_RATE: u32 = 48_000;
const RENDER_REPEATS: usize = 5;

#[derive(Debug, Deserialize)]
struct Corpus {
    format: String,
    schema_version: String,
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    project: PathBuf,
    voicebank: PathBuf,
}

#[derive(Debug, Serialize)]
struct CorpusReport {
    schema: &'static str,
    generated_at_unix_seconds: u64,
    sample_rate_hz: u32,
    wavtool: String,
    level_matching: &'static str,
    note: &'static str,
    cases: Vec<CaseReport>,
}

#[derive(Debug, Serialize)]
struct CaseReport {
    id: String,
    project_sha256: String,
    oto_ini_sha256: Option<String>,
    tainah: AudioReport,
    straycat: AudioReport,
    output_frames_equal: bool,
    pairing_issue: Option<String>,
    blind_a_engine: String,
    blind_b_engine: String,
    blind_a_gain_db: f32,
    blind_b_gain_db: f32,
}

#[derive(Debug, Clone, Serialize)]
struct AudioReport {
    sample_count: usize,
    channels: u16,
    duration_ms: f64,
    peak: f32,
    rms: f32,
    clipped_samples: usize,
    non_finite_samples: usize,
    audible_10ms_windows_pct: f32,
    leading_silence_ms: f64,
    trailing_silence_ms: f64,
    median_render_ms: f64,
    pcm_sha256: String,
}

struct Rendered {
    audio: Vec<f32>,
    channels: u16,
    report: AudioReport,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("tainah-ab: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() < 3 || args.len() > 4 {
        return Err(
            "usage: cargo run --bin tainah-ab -- corpus.json straycat-rs output-dir [wavtool]"
                .into(),
        );
    }
    let corpus_path = PathBuf::from(&args[0]);
    let straycat_path = PathBuf::from(&args[1]);
    if !straycat_path.is_file() {
        return Err(format!(
            "straycat-rs executable not found: {}",
            straycat_path.display()
        ));
    }
    let output_dir = PathBuf::from(&args[2]);
    fs::create_dir(&output_dir).map_err(|error| {
        format!(
            "cannot create output directory {}: {error}",
            output_dir.display()
        )
    })?;
    let corpus_text = fs::read(&corpus_path)
        .map_err(|error| format!("cannot read corpus {}: {error}", corpus_path.display()))?;
    if corpus_text.len() > 4 * 1024 * 1024 {
        return Err("corpus exceeds the 4 MiB safety limit".into());
    }
    let corpus: Corpus = serde_json::from_slice(&corpus_text)
        .map_err(|error| format!("invalid corpus JSON: {error}"))?;
    if corpus.format != "kamafeu.tainah-ab-corpus" || corpus.schema_version != "1.0.0" {
        return Err("unsupported Tainah A/B corpus format or schema version".into());
    }
    if corpus.cases.is_empty() || corpus.cases.len() > 100 {
        return Err("corpus must contain 1 to 100 reviewed cases".into());
    }
    let mut ids = HashSet::new();
    if corpus.cases.iter().any(|case| {
        case.id.is_empty()
            || case.id.len() > 64
            || !case
                .id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            || !ids.insert(case.id.clone())
    }) {
        return Err("case IDs must be unique, nonempty ASCII letters, digits, '-' or '_'".into());
    }
    let base_dir = corpus_path.parent().unwrap_or_else(|| Path::new("."));
    let tainah = NativeWorldResamplerDriver;
    let straycat =
        ExternalResamplerDriver::for_known(KnownResampler::StraycatRs, Some(straycat_path.clone()));
    let wavtool: Box<dyn WavtoolDriver> = if let Some(path) = args.get(3) {
        let path = PathBuf::from(path);
        if !path.is_file() {
            return Err(format!("wavtool executable not found: {}", path.display()));
        }
        Box::new(ExternalWavtoolDriver::new(path))
    } else {
        Box::new(GalapagosWavtoolDriver)
    };
    let cache_dir =
        tempfile::tempdir().map_err(|error| format!("create isolated cache: {error}"))?;
    kamafeu::renderer::resampler_cache::set_persistent_cache_dir_override(Some(
        cache_dir.path().to_path_buf(),
    ));
    let mut rng = rand::thread_rng();
    let mut reports = Vec::with_capacity(corpus.cases.len());
    let mut answer_key = String::from("case_id,blind_a_engine,blind_b_engine\n");
    let mut ratings = String::from(
        "listener_id,case_id,preference(A|B|none),stretch_character(1-5),intelligibility(1-5),continuity(1-5),notes\n",
    );

    for case in corpus.cases {
        let project_path = resolve_path(base_dir, &case.project);
        let voicebank_path = resolve_path(base_dir, &case.voicebank);
        let project = ApsFormat::load_file(&project_path)
            .map_err(|error| format!("case '{}': load project: {error}", case.id))?;
        let voicebank = Voicebank::new(&voicebank_path)
            .map_err(|error| format!("case '{}': load voicebank: {error}", case.id))?;
        let project_sha256 = sha256_file(&project_path)?;
        let oto_ini_path = voicebank.root_path.join("oto.ini");
        let oto_ini_sha256 = oto_ini_path
            .is_file()
            .then(|| sha256_file(&oto_ini_path))
            .transpose()?;
        let case_dir = output_dir.join(&case.id);
        fs::create_dir(&case_dir)
            .map_err(|error| format!("case '{}': create output directory: {error}", case.id))?;

        let tainah_render =
            render_repeated(&project, &voicebank, &tainah, wavtool.as_ref(), &case.id)?;
        let straycat_render =
            render_repeated(&project, &voicebank, &straycat, wavtool.as_ref(), &case.id)?;
        let tainah_frames = tainah_render.audio.len() / usize::from(tainah_render.channels.max(1));
        let straycat_frames =
            straycat_render.audio.len() / usize::from(straycat_render.channels.max(1));
        let pair_issue = (tainah_frames != straycat_frames
            || tainah_render.channels != straycat_render.channels)
            .then(|| format!("unpaired output: Tainah {tainah_frames} frames/{}ch; straycat-rs {straycat_frames} frames/{}ch", tainah_render.channels, straycat_render.channels));
        write_wav(
            &case_dir.join("raw-tainah.wav"),
            &tainah_render.audio,
            tainah_render.channels,
        )?;
        write_wav(
            &case_dir.join("raw-straycat.wav"),
            &straycat_render.audio,
            straycat_render.channels,
        )?;
        if let Some(issue) = pair_issue {
            eprintln!("case '{}': {issue}", case.id);
            reports.push(CaseReport {
                id: case.id,
                project_sha256,
                oto_ini_sha256,
                tainah: tainah_render.report,
                straycat: straycat_render.report,
                output_frames_equal: false,
                pairing_issue: Some(issue),
                blind_a_engine: "unpaired".into(),
                blind_b_engine: "unpaired".into(),
                blind_a_gain_db: 0.0,
                blind_b_gain_db: 0.0,
            });
            continue;
        }
        let (blind_a, blind_b): (&Rendered, &Rendered) = if rand::Rng::gen::<bool>(&mut rng) {
            (&tainah_render, &straycat_render)
        } else {
            (&straycat_render, &tainah_render)
        };
        let (a_id, b_id) = if std::ptr::eq(blind_a, &tainah_render) {
            ("tainah", "straycat-rs")
        } else {
            ("straycat-rs", "tainah")
        };
        let (a_gain, b_gain) = matched_gains(blind_a, blind_b)?;
        let a_samples = apply_gain(&blind_a.audio, a_gain);
        let b_samples = apply_gain(&blind_b.audio, b_gain);
        write_wav(&case_dir.join("A.wav"), &a_samples, blind_a.channels)?;
        write_wav(&case_dir.join("B.wav"), &b_samples, blind_b.channels)?;
        answer_key.push_str(&format!("{},{a_id},{b_id}\n", case.id));
        ratings.push_str(&format!(",{},,,,,\n", case.id));
        reports.push(CaseReport {
            id: case.id,
            project_sha256,
            oto_ini_sha256,
            tainah: tainah_render.report.clone(),
            straycat: straycat_render.report.clone(),
            output_frames_equal: true,
            pairing_issue: None,
            blind_a_engine: a_id.into(),
            blind_b_engine: b_id.into(),
            blind_a_gain_db: gain_db(a_gain),
            blind_b_gain_db: gain_db(b_gain),
        });
    }

    fs::write(output_dir.join("answer-key.csv"), answer_key)
        .map_err(|error| format!("write answer key: {error}"))?;
    fs::write(output_dir.join("ratings-template.csv"), ratings)
        .map_err(|error| format!("write ratings template: {error}"))?;
    let wavtool_name = wavtool.name().to_string();
    let report = CorpusReport {
        schema: "kamafeu.tainah-ab-report/1.0.0",
        generated_at_unix_seconds: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        sample_rate_hz: OUTPUT_RATE,
        wavtool: wavtool_name,
        level_matching: "Listening A/B files use an RMS-matched gain, capped to +/-12 dB and a 0.95 peak. Raw WAVs and metrics are retained separately.",
        note: "Randomized blind A/B only. Technical metrics do not establish preference or general superiority; review authorization and phoneme marks before listening.",
        cases: reports,
    };
    fs::write(
        output_dir.join("report.json"),
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("write report: {error}"))?;
    println!("Tainah A/B renders: {}", output_dir.display());
    if report.cases.len() < 20 {
        println!(
            "Note: {} cases are below the planned 20-case evaluation corpus.",
            report.cases.len()
        );
    }
    Ok(())
}

fn resolve_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn render_repeated(
    project: &kamafeu::UProject,
    voicebank: &Voicebank,
    resampler: &dyn ResamplerDriver,
    wavtool: &dyn WavtoolDriver,
    case_id: &str,
) -> Result<Rendered, String> {
    let mut durations = Vec::with_capacity(RENDER_REPEATS);
    let mut final_audio: Option<kamafeu::RenderedAudio> = None;
    for repeat in 0..RENDER_REPEATS {
        kamafeu::renderer::resampler_cache::clear_memory_cache();
        kamafeu::renderer::resampler_cache::clear_disk_cache()
            .map_err(|error| format!("case '{case_id}': clear isolated cache: {error}"))?;
        let started = Instant::now();
        let rendered = ProjectRenderer::render_project_with_drivers(
            project,
            voicebank,
            OUTPUT_RATE,
            0.0,
            resampler,
            wavtool,
            &RenderOptions::default(),
            None,
        );
        durations.push(started.elapsed());
        if let Some(error) = rendered.error {
            return Err(format!("case '{case_id}' repeat {repeat}: {error}"));
        }
        if rendered.samples.iter().any(|sample| !sample.is_finite()) {
            return Err(format!("case '{case_id}' produced non-finite samples"));
        }
        if let Some(previous) = &final_audio {
            if previous.samples != rendered.samples {
                return Err(format!(
                    "case '{case_id}' is nondeterministic across uncached renders with {}",
                    resampler.name()
                ));
            }
        }
        final_audio = Some(rendered);
    }
    let rendered = final_audio.ok_or_else(|| format!("case '{case_id}' rendered no audio"))?;
    let median_ms = median_ms(&mut durations);
    let report = audio_metrics(
        &rendered.samples,
        rendered.sample_rate,
        rendered.channels,
        median_ms,
    );
    Ok(Rendered {
        audio: rendered.samples,
        channels: rendered.channels,
        report,
    })
}

fn audio_metrics(samples: &[f32], sample_rate: u32, channels: u16, median_ms: f64) -> AudioReport {
    let channels = channels.max(1);
    let frames = samples.len() / usize::from(channels);
    let mut peak = 0.0f32;
    let mut square_sum = 0.0f64;
    let mut clipped = 0usize;
    let mut non_finite = 0usize;
    for &sample in samples {
        if !sample.is_finite() {
            non_finite += 1;
            continue;
        }
        peak = peak.max(sample.abs());
        square_sum += f64::from(sample).powi(2);
        clipped += usize::from(sample.abs() >= 1.0);
    }
    let rms = (square_sum / samples.len().max(1) as f64).sqrt() as f32;
    let samples_per_10ms = ((sample_rate as usize * 10) / 1_000 * usize::from(channels)).max(1);
    let window_peaks = samples
        .chunks(samples_per_10ms)
        .map(|window| {
            window
                .iter()
                .filter(|value| value.is_finite())
                .fold(0.0f32, |peak, value| peak.max(value.abs()))
        })
        .collect::<Vec<_>>();
    let audible_count = window_peaks.iter().filter(|peak| **peak > 0.001).count();
    let first = window_peaks.iter().position(|peak| *peak > 0.001);
    let last = window_peaks.iter().rposition(|peak| *peak > 0.001);
    let leading = first.unwrap_or(window_peaks.len());
    let trailing = last.map_or(window_peaks.len(), |last| window_peaks.len() - last - 1);
    let pcm_hash = samples.iter().fold(Sha256::new(), |mut hash, sample| {
        hash.update(sample.to_le_bytes());
        hash
    });
    AudioReport {
        sample_count: samples.len(),
        channels,
        duration_ms: frames as f64 * 1_000.0 / sample_rate.max(1) as f64,
        peak,
        rms,
        clipped_samples: clipped,
        non_finite_samples: non_finite,
        audible_10ms_windows_pct: audible_count as f32 * 100.0 / window_peaks.len().max(1) as f32,
        leading_silence_ms: leading as f64 * 10.0,
        trailing_silence_ms: trailing as f64 * 10.0,
        median_render_ms: median_ms,
        pcm_sha256: format!("{:x}", pcm_hash.finalize()),
    }
}

fn matched_gains(a: &Rendered, b: &Rendered) -> Result<(f32, f32), String> {
    let target_rms = ((a.report.rms as f64 * b.report.rms as f64).sqrt()) as f32;
    let minimum_gain = 10.0f32.powf(-12.0 / 20.0);
    let maximum_gain = 10.0f32.powf(12.0 / 20.0);
    let gain = |render: &Rendered| -> Result<f32, String> {
        if render.report.rms <= 1e-9 || target_rms <= 1e-9 {
            return Ok(1.0);
        }
        let rms_gain = (target_rms / render.report.rms).clamp(minimum_gain, maximum_gain);
        if render.report.peak > 0.0 {
            let peak_safe_gain = 0.95 / render.report.peak;
            if peak_safe_gain < minimum_gain {
                return Err(
                    "cannot level-match within +/-12 dB without exceeding the 0.95 peak cap".into(),
                );
            }
            Ok(rms_gain.min(peak_safe_gain))
        } else {
            Ok(rms_gain)
        }
    };
    Ok((gain(a)?, gain(b)?))
}

fn apply_gain(samples: &[f32], gain: f32) -> Vec<f32> {
    samples
        .iter()
        .map(|sample| (sample * gain).clamp(-1.0, 1.0))
        .collect()
}

fn gain_db(gain: f32) -> f32 {
    20.0 * gain.max(1e-9).log10()
}

fn median_ms(durations: &mut [Duration]) -> f64 {
    durations.sort_unstable();
    durations[durations.len() / 2].as_secs_f64() * 1_000.0
}

fn write_wav(path: &Path, samples: &[f32], channels: u16) -> Result<(), String> {
    TrackRenderer::save_wav_samples_with_channels(path, samples, OUTPUT_RATE, channels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_metrics_report_duration_energy_and_pcm_hash() {
        let report = audio_metrics(&[0.0, 0.5, -0.5, 0.0], 1_000, 1, 12.0);
        assert_eq!(report.sample_count, 4);
        assert_eq!(report.duration_ms, 4.0);
        assert_eq!(report.peak, 0.5);
        assert_eq!(report.clipped_samples, 0);
        assert_eq!(report.non_finite_samples, 0);
        assert!(!report.pcm_sha256.is_empty());
    }

    #[test]
    fn listening_gain_matches_rms_without_exceeding_peak_or_gain_caps() {
        let make = |sample: f32| Rendered {
            audio: vec![sample; 16],
            channels: 1,
            report: audio_metrics(&vec![sample; 16], 48_000, 1, 1.0),
        };
        let quiet = make(0.1);
        let loud = make(0.8);
        let (quiet_gain, loud_gain) = matched_gains(&quiet, &loud).unwrap();
        assert!((quiet.report.rms * quiet_gain - loud.report.rms * loud_gain).abs() < 0.01);
        assert!(0.1 * quiet_gain <= 0.95);
        assert!(0.8 * loud_gain <= 0.95);
        assert!(gain_db(quiet_gain).abs() <= 12.0);
        assert!(gain_db(loud_gain).abs() <= 12.0);
    }
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot hash {}: {error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
