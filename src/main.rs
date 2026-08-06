use clap::{Parser, Subcommand};
use eframe::NativeOptions;
use std::fs::{self, File};
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

use kamafeu::{
    drivers::{NativeResamplerDriver, NativeWavtoolDriver},
    formats::{MidiFormat, UstFormat, UstxFormat},
    gui::KamafeuStudioApp,
    oto::Voicebank,
    project::model::{UNote, UProject},
    renderer::{ProjectRenderer, RenderOptions},
};

#[derive(Parser)]
#[command(name = "kamafeu")]
#[command(about = "OpenUTAU-style voice synthesizer core and Piano Roll GUI in Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the OpenUTAU-style Piano Roll GUI Studio
    Gui,

    /// Inspect information about an UTAU voicebank directory
    VoicebankInfo {
        /// Path to the UTAU voicebank root folder
        path: PathBuf,
    },

    /// Generate a test UTAU voicebank with sample WAVs and oto.ini for quick testing
    GenSample {
        /// Destination directory to create the sample voicebank
        path: PathBuf,
    },

    /// Render a JSON, UST, USTX, or MIDI project to a WAV audio file
    Render {
        /// Path to UTAU voicebank directory
        #[arg(short, long)]
        voicebank: PathBuf,

        /// Path to project or score file (.ustx, .ust, .mid, .midi, .json)
        #[arg(short, long)]
        input: PathBuf,

        /// Path to output WAV file
        #[arg(short, long, default_value = "output.wav")]
        output: PathBuf,

        /// Sample rate (Hz)
        #[arg(short, long, default_value_t = 44100)]
        sample_rate: u32,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Gui) => {
            println!("Starting kamafeu - sintetizador de voz...");
            let options = NativeOptions {
                viewport: eframe::egui::ViewportBuilder::default()
                    .with_title("kamafeu - sintetizador de voz")
                    .with_inner_size([1280.0, 750.0])
                    .with_min_inner_size([800.0, 500.0]),
                ..Default::default()
            };

            eframe::run_native(
                "kamafeu - sintetizador de voz",
                options,
                Box::new(|cc| Ok(Box::new(KamafeuStudioApp::new(cc)))),
            )?;
        }

        Some(Commands::VoicebankInfo { path }) => {
            println!("Loading voicebank from: {:?}", path);
            let vb = Voicebank::new(&path)?;
            println!("=== Voicebank Info ===");
            println!("Name:        {}", vb.name);
            println!("Author:      {}", vb.author);
            println!("Total entries: {}", vb.entries.len());
            println!("\nSample Aliases (up to 10):");
            for (alias, entry) in vb.entries.iter().take(10) {
                println!(
                    "  - Alias: {:<10} -> WAV: {:<12} (offset: {}ms, preutter: {}ms)",
                    alias, entry.wav_filename, entry.offset, entry.preutterance
                );
            }
        }

        Some(Commands::GenSample { path }) => {
            println!("Generating test UTAU voicebank at: {:?}", path);
            fs::create_dir_all(&path)?;

            let char_txt = "name=Kamafeu Sample Synth\nauthor=Kamafeu Team\n";
            fs::write(path.join("character.txt"), char_txt)?;

            let vowels = [
                ("ka", 261.63),
                ("ki", 293.66),
                ("ku", 329.63),
                ("ke", 349.23),
                ("ko", 392.00),
            ];

            let sample_rate = 44100;
            let mut oto_lines = Vec::new();

            for &(name, freq) in &vowels {
                let filename = format!("{}.wav", name);
                let wav_path = path.join(&filename);

                let spec = hound::WavSpec {
                    channels: 1,
                    sample_rate: sample_rate as u32,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                };

                let mut writer = hound::WavWriter::create(&wav_path, spec)?;
                let duration_sec = 1.0;
                let total_samples = (sample_rate as f64 * duration_sec) as usize;

                for i in 0..total_samples {
                    let t = i as f64 / sample_rate as f64;
                    let sample_val = 0.6 * (2.0 * std::f64::consts::PI * freq * t).sin()
                        + 0.3 * (2.0 * std::f64::consts::PI * freq * 2.0 * t).sin()
                        + 0.1 * (2.0 * std::f64::consts::PI * freq * 3.0 * t).sin();

                    let sample_i16 = (sample_val * i16::MAX as f64) as i16;
                    writer.write_sample(sample_i16)?;
                }
                writer.finalize()?;

                oto_lines.push(format!("{}=,10,50,-100,30,15", filename));
            }

            fs::write(path.join("oto.ini"), oto_lines.join("\n"))?;

            println!("Sample voicebank created successfully!");
            println!("Contains: character.txt, oto.ini, and ka.wav..ko.wav samples.");
        }

        Some(Commands::Render {
            voicebank,
            input,
            output,
            sample_rate,
        }) => {
            println!("Loading voicebank from: {:?}", voicebank);
            let vb = Voicebank::new(&voicebank)?;

            println!("Reading input file: {:?}", input);
            let mut project = load_project(&input)?;
            project.normalize();
            let note_count: usize = project.parts.iter().map(|part| part.notes.len()).sum();
            if note_count == 0 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "o projeto não contém notas renderizáveis",
                )
                .into());
            }

            println!("Rendering {} notes at {}Hz...", note_count, sample_rate);
            let native_resampler = NativeResamplerDriver;
            let native_wavtool = NativeWavtoolDriver;
            let rendered = ProjectRenderer::render_project_with_drivers(
                &project,
                &vb,
                sample_rate,
                0.0,
                &native_resampler,
                &native_wavtool,
                &RenderOptions::default(),
                None,
            );

            println!("Writing rendered audio to: {:?}", output);
            let spec = hound::WavSpec {
                channels: rendered.channels,
                sample_rate,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };

            let file = File::create(&output)?;
            let writer = BufWriter::new(file);
            let mut wav_writer = hound::WavWriter::new(writer, spec)?;

            for &sample in &rendered.samples {
                let clamped = sample.clamp(-1.0, 1.0);
                let sample_i16 = (clamped * i16::MAX as f32) as i16;
                wav_writer.write_sample(sample_i16)?;
            }
            wav_writer.finalize()?;

            println!("Render complete! Audio written to {:?}", output);
        }
    }

    Ok(())
}

fn load_project(path: &PathBuf) -> Result<UProject, Box<dyn std::error::Error>> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "ust" => UstFormat::load_file(path),
        "ustx" => UstxFormat::load_file(path),
        "mid" | "midi" => MidiFormat::load_file(path),
        "json" => {
            let content = fs::read_to_string(path)?;
            if let Ok(project) = serde_json::from_str::<UProject>(&content) {
                return Ok(project);
            }
            let notes = serde_json::from_str::<Vec<UNote>>(&content)?;
            let mut project = UProject::default();
            project.parts[0].notes = notes;
            Ok(project)
        }
        _ => Err(Error::new(
            ErrorKind::InvalidInput,
            format!("formato de entrada não suportado: .{extension}"),
        )
        .into()),
    }
}
