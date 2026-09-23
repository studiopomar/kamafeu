//! Render and measure a real voicebank with VENUS at several piano-roll notes.

use kamafeu::dsp::{midi_to_freq, venus::WorldF0Extractor, venus_analysis, VenusResampler};
use kamafeu::oto::{OtoEntry, Voicebank};
use kamafeu::renderer::{AudioDiagnostics, TrackRenderer};
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Serialize)]
struct CorpusMeasurement {
    alias: String,
    wav: String,
    target_midi: u8,
    target_hz: f64,
    measured_f0_hz: Option<f32>,
    f0_error_cents: Option<f32>,
    input_rms: f32,
    output_rms: f32,
    output_peak: f32,
    max_step: f32,
    spectral_centroid_hz: f32,
    spectral_tilt_db: f32,
    harmonicity: f32,
}

fn usage() -> ! {
    eprintln!(
        "Uso: venus-corpus <pasta-do-voicebank> <relatorio.json> [limite-de-aliases]\n\nExemplo: cargo run --bin venus-corpus -- /caminho/voicebank venus-corpus.json 40"
    );
    std::process::exit(2);
}

fn mean(values: impl Iterator<Item = f32>) -> f32 {
    let values = values.filter(|value| value.is_finite()).collect::<Vec<_>>();
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}

fn render_measurement(
    root: &std::path::Path,
    entry: &OtoEntry,
    target_midi: u8,
) -> Result<CorpusMeasurement, String> {
    let wav_path = root.join(&entry.wav_filename);
    let (input, sample_rate) = TrackRenderer::load_wav_samples(&wav_path)?;
    let target_hz = midi_to_freq(f64::from(target_midi));
    let rendered = VenusResampler::render_sample(
        &input,
        sample_rate,
        entry.offset,
        entry.consonant.max(0.0),
        entry.consonant.max(0.0),
        entry.cutoff,
        600.0,
        target_hz,
        &[],
        entry.loop_start,
        entry.loop_end,
        entry.tail_start,
        0.0,
        0.0,
    );
    let input_diagnostics = AudioDiagnostics::analyze(&input, 1);
    let output_diagnostics = AudioDiagnostics::analyze(&rendered, 1);
    let extractor = WorldF0Extractor::default();
    let (f0, _) = extractor.extract_f0(&rendered, sample_rate);
    let measured_f0_hz = {
        let voiced = f0
            .into_iter()
            .filter(|value| *value > 50.0)
            .collect::<Vec<_>>();
        (!voiced.is_empty()).then(|| voiced.iter().sum::<f32>() / voiced.len() as f32)
    };
    let f0_error_cents =
        measured_f0_hz.map(|actual| 1_200.0 * (actual as f64 / target_hz).log2() as f32);
    let output_analysis = venus_analysis::analyze_samples(&rendered, sample_rate);

    Ok(CorpusMeasurement {
        alias: entry.alias.clone(),
        wav: entry.wav_filename.clone(),
        target_midi,
        target_hz,
        measured_f0_hz,
        f0_error_cents,
        input_rms: input_diagnostics.rms,
        output_rms: output_diagnostics.rms,
        output_peak: output_diagnostics.peak,
        max_step: output_diagnostics.max_step,
        spectral_centroid_hz: mean(
            output_analysis
                .frames
                .iter()
                .filter(|frame| frame.voiced)
                .map(|frame| frame.spectral_centroid_hz),
        ),
        spectral_tilt_db: mean(
            output_analysis
                .frames
                .iter()
                .filter(|frame| frame.voiced)
                .map(|frame| frame.spectral_tilt_db),
        ),
        harmonicity: mean(
            output_analysis
                .frames
                .iter()
                .filter(|frame| frame.voiced)
                .map(|frame| frame.harmonicity),
        ),
    })
}

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let root = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| usage());
    let report_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| usage());
    let limit = arguments
        .next()
        .and_then(|value| value.into_string().ok())
        .map(|value| value.parse::<usize>().unwrap_or_else(|_| usage()))
        .unwrap_or(usize::MAX);

    let result = Voicebank::new(&root)
        .map_err(|error| error.to_string())
        .and_then(|voicebank| {
            let mut entries = voicebank.entries.values().cloned().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.alias.cmp(&right.alias));
            let mut seen_wavs = HashSet::new();
            entries.retain(|entry| seen_wavs.insert(entry.wav_filename.clone()));
            let mut report = Vec::new();
            for entry in entries.into_iter().take(limit) {
                for midi in [48, 60, 72] {
                    match render_measurement(&voicebank.root_path, &entry, midi) {
                        Ok(measurement) => report.push(measurement),
                        Err(error) => eprintln!("Ignorado '{}': {error}", entry.alias),
                    }
                }
            }
            let json = serde_json::to_string_pretty(&report)
                .map_err(|error| format!("não foi possível serializar relatório: {error}"))?;
            std::fs::write(&report_path, json).map_err(|error| {
                format!("não foi possível salvar {}: {error}", report_path.display())
            })?;
            println!(
                "Corpus VENUS: {} medições em {}",
                report.len(),
                report_path.display()
            );
            Ok(())
        });

    if let Err(error) = result {
        eprintln!("venus-corpus: {error}");
        std::process::exit(1);
    }
}
