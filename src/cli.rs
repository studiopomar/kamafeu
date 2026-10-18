use clap::Parser;
use serde_json::{json, Value};
use std::fs;
use std::io::BufRead;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

use kamafeu::{
    drivers::{
        ExternalResamplerDriver, NativeResamplerDriver, NativeWavtoolDriver,
        NativeWorldResamplerDriver, ResamplerDriver, WavtoolDriver,
    },
    extensions::{
        built_in_capabilities, diagnose_project_extensions, diagnose_project_extensions_structured,
        discover, platform_capability_matrix, verify_wasm_extension_with_policy,
        DiscoveredExtension, ExtensionPermissionPolicy,
    },
    oto::{SingerScanner, Voicebank},
    project::model::{UNote, UProject},
    renderer::{
        AudioDiagnostics, AudioExportFormat, AudioExporter, DitherMode, ProjectRenderer,
        RenderOptions, RenderProvenance,
    },
};

mod args;
mod common;
mod sample;
mod startup;
use args::*;
use common::*;
pub use startup::run;

fn real_main() -> Result<(), Box<dyn std::error::Error>> {
    let Cli {
        command,
        json: json_output,
    } = Cli::parse();

    match command {
        None | Some(Commands::Gui) => {
            startup::run_gui()?;
        }

        Some(
            command @ (Commands::VoicebankInfo { .. }
            | Commands::Voicebanks { .. }
            | Commands::ValidateVoicebank { .. }
            | Commands::MigrateVoicebank { .. }),
        ) => voicebank::handle(command, json_output)?,

        Some(
            command @ (Commands::ProjectInfo { .. }
            | Commands::Diff { .. }
            | Commands::ProvenanceInfo { .. }
            | Commands::ValidateProject { .. }),
        ) => project::handle(command, json_output)?,

        Some(Commands::Convert {
            input,
            output,
            fail_on_loss,
        }) => {
            let report = conversion::convert_project(&input, &output)?;
            if json_output {
                print_json(&report)?;
            } else {
                kamafeu::terminal_log::out_header(
                    "Conversion",
                    format!("{} -> {}", input.display(), output.display()),
                );
                let warnings = report["warnings"].as_array().cloned().unwrap_or_default();
                if warnings.is_empty() {
                    kamafeu::terminal_log::out_status("OK", "Audit", "musical data preserved");
                } else {
                    kamafeu::terminal_log::out_status("WARN", "Audit", "conversion warnings");
                    for warning in warnings {
                        if let Some(warning) = warning.as_str() {
                            kamafeu::terminal_log::out_list(warning);
                        }
                    }
                }
            }
            if fail_on_loss && report["tracked_features_preserved"].as_bool() != Some(true) {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "a auditoria detectou dados musicais alterados; o arquivo de destino foi criado, mas a conversão falhou em modo --fail-on-loss",
                )
                .into());
            }
        }

        Some(command @ Commands::Extensions { .. }) => {
            extension_command::handle(command, json_output)?;
        }

        Some(
            command @ (Commands::PlatformInfo | Commands::CacheInfo | Commands::CachePrune { .. }),
        ) => environment::handle(command, json_output)?,

        Some(Commands::Automation) => automation::run_automation()?,

        Some(Commands::Mcp) => mcp::run_mcp()?,

        Some(Commands::GenSample { path }) => sample::generate(path)?,

        Some(Commands::Render {
            voicebank,
            input,
            output,
            format,
            dither,
            sample_rate,
            venus,
            resampler,
            vocal_only,
        }) => {
            let summary = render::render_project_to_file(
                &voicebank,
                &input,
                &output,
                format,
                dither,
                sample_rate,
                venus,
                resampler,
                vocal_only,
            )?;
            if json_output {
                print_json(&render::render_summary_json(&summary))?;
            } else {
                kamafeu::terminal_log::out_header("Render", summary.output.display());
                kamafeu::terminal_log::out_kv(
                    "Notes",
                    format!("{} at {}Hz", summary.note_count, summary.sample_rate),
                );
                kamafeu::terminal_log::out_kv(
                    "Output",
                    format!(
                        "{} ({} frames, {} channel(s))",
                        summary.format.display_name(),
                        summary.frame_count,
                        summary.channels
                    ),
                );
                kamafeu::terminal_log::out_kv(
                    "Audio",
                    format!(
                        "peak {:.4}, RMS {:.4}, {} clipped sample(s), {}",
                        summary.diagnostics.peak,
                        summary.diagnostics.rms,
                        summary.diagnostics.clipped_samples,
                        if summary.silent { "silent" } else { "audible" }
                    ),
                );
            }
        }
    }

    Ok(())
}

mod automation;

mod mcp;

mod conversion;
mod project;
mod render;
mod voicebank;

mod environment;
mod extension_command;
mod extensions;

#[cfg(test)]
mod tests;
