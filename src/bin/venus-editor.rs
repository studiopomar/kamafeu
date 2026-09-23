//! Small, scriptable VENUS analysis editor.
//!
//! The graphical editor will use the same `.venus` format. This utility makes
//! the format useful immediately for batch work and voicebank maintenance.

use kamafeu::dsp::venus_analysis::{self, VenusAnalysis};
use kamafeu::renderer::TrackRenderer;
use std::path::PathBuf;

fn usage() -> ! {
    eprintln!(
        "Uso:\n  venus-editor <wav> analyze\n  venus-editor <wav> show\n  venus-editor <wav> set <gain-db|formant-cents|breathiness|notes> <valor>\n\nO arquivo de análise é salvo ao lado do WAV como .venus."
    );
    std::process::exit(2);
}

fn load_current(wav: &PathBuf) -> Result<VenusAnalysis, String> {
    let (samples, sample_rate) = TrackRenderer::load_wav_samples(wav)?;
    venus_analysis::load_or_analyze(wav, &samples, sample_rate)
}

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let wav = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| usage());
    let command = arguments
        .next()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| usage());

    let result = match command.as_str() {
        "analyze" => load_current(&wav).map(|analysis| {
            println!(
                "Análise criada: {} ({} frames, {} marcas de pitch)",
                venus_analysis::sidecar_path(&wav).display(),
                analysis.frames.len(),
                analysis.pitch_marks.len()
            );
        }),
        "show" => load_current(&wav).map(|analysis| {
            let voiced = analysis.frames.iter().filter(|frame| frame.voiced).count();
            println!("Arquivo: {}", venus_analysis::sidecar_path(&wav).display());
            println!(
                "Amostras: {} @ {} Hz",
                analysis.sample_count, analysis.sample_rate
            );
            println!("Frames: {} ({} vozeados)", analysis.frames.len(), voiced);
            println!("Marcas de pitch: {}", analysis.pitch_marks.len());
            println!("Ganho: {:.2} dB", analysis.gain_db);
            println!("Formante: {:.1} cents", analysis.formant_shift_cents);
            println!("Respiração adicional: {:.1}", analysis.breathiness);
            if !analysis.notes.trim().is_empty() {
                println!("Notas: {}", analysis.notes);
            }
        }),
        "set" => {
            let field = arguments
                .next()
                .and_then(|value| value.into_string().ok())
                .unwrap_or_else(|| usage());
            let value = arguments
                .next()
                .and_then(|value| value.into_string().ok())
                .unwrap_or_else(|| usage());
            load_current(&wav).and_then(|mut analysis| {
                match field.as_str() {
                    "gain-db" => {
                        analysis.gain_db = value
                            .parse::<f32>()
                            .map_err(|_| "gain-db precisa ser um número".to_string())?
                            .clamp(-24.0, 24.0);
                    }
                    "formant-cents" => {
                        analysis.formant_shift_cents = value
                            .parse::<f32>()
                            .map_err(|_| "formant-cents precisa ser um número".to_string())?
                            .clamp(-2_400.0, 2_400.0);
                    }
                    "breathiness" => {
                        analysis.breathiness = value
                            .parse::<f32>()
                            .map_err(|_| "breathiness precisa ser um número".to_string())?
                            .clamp(0.0, 100.0);
                    }
                    "notes" => analysis.notes = value,
                    _ => return Err(format!("campo VENUS desconhecido: {field}")),
                }
                venus_analysis::save(&venus_analysis::sidecar_path(&wav), &analysis)
            })
        }
        _ => usage(),
    };

    if let Err(error) = result {
        eprintln!("venus-editor: {error}");
        std::process::exit(1);
    }
}
