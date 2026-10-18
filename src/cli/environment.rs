use super::*;

pub(super) fn handle(
    command: Commands,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::PlatformInfo => {
            let matrix = platform_capability_matrix();
            if json_output {
                print_json(&json!({
                    "current_platform": kamafeu::extensions::current_platform(),
                    "capabilities": matrix,
                }))?;
            } else {
                kamafeu::terminal_log::out_header(
                    "Platform",
                    kamafeu::extensions::current_platform(),
                );
                if kamafeu::terminal_log::is_verbose() {
                    for capability in matrix {
                        kamafeu::terminal_log::out_status(
                            if capability.desktop { "OK" } else { "WARN" },
                            capability.id,
                            format!(
                                "desktop={} android={} wasm={} | {}",
                                capability.desktop,
                                capability.android,
                                capability.wasm,
                                capability.note
                            ),
                        );
                    }
                } else {
                    let available = matrix
                        .iter()
                        .filter(|capability| capability.desktop)
                        .count();
                    kamafeu::terminal_log::out_status(
                        "OK",
                        "Capabilities",
                        format!(
                            "{available}/{} available; use KAMAFEU_LOG=verbose for details",
                            matrix.len()
                        ),
                    );
                }
            }
        }

        Commands::CacheInfo => {
            let report = automation::cache_info_json();
            if json_output {
                print_json(&report)?;
            } else {
                kamafeu::terminal_log::out_header(
                    "Render cache",
                    report["directory"].as_str().unwrap_or(""),
                );
                kamafeu::terminal_log::out_kv("Disk files", &report["disk_file_count"]);
                kamafeu::terminal_log::out_kv(
                    "Disk size",
                    format!("{} bytes", report["disk_bytes"]),
                );
                kamafeu::terminal_log::out_kv(
                    "Memory",
                    format!(
                        "{} entries / {} samples",
                        report["memory_entries"], report["memory_samples"]
                    ),
                );
                kamafeu::terminal_log::out_kv(
                    "Stats",
                    format!(
                        "{} hit(s), {} miss(es), {} render(s)",
                        report["hits"], report["misses"], report["renders"]
                    ),
                );
            }
        }

        Commands::CachePrune { days } => {
            let removed = kamafeu::renderer::resampler_cache::cleanup_older_than(days)?;
            if json_output {
                print_json(&json!({ "days": days, "removed": removed }))?;
            } else {
                kamafeu::terminal_log::out_status(
                    "OK",
                    "Cache",
                    format!("removed {removed} file(s) older than {days} day(s)"),
                );
            }
        }

        _ => unreachable!("environment command dispatcher received another command"),
    }
    Ok(())
}
