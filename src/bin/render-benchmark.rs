//! Repeatable end-to-end renderer benchmark for an APS project and voicebank.
//!
//! Usage: cargo run --release --bin render-benchmark -- project.aps voicebank [iterations]
//! Optional backends: --world, --resampler PATH, --wavtool PATH

use kamafeu::{
    ApsFormat, ExternalResamplerDriver, ExternalWavtoolDriver, GalapagosWavtoolDriver,
    NativeResamplerDriver, NativeWorldResamplerDriver, RenderOptions, ResamplerDriver,
    TrackRenderer, UNote, UProject, Voicebank, WavtoolDriver,
};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};

fn percentile(samples: &[Duration], percentile: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut values = samples
        .iter()
        .map(Duration::as_secs_f64)
        .collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    let index = ((values.len() - 1) as f64 * percentile).ceil() as usize;
    values[index.min(values.len() - 1)] * 1000.0
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().collect::<Vec<_>>();
    let synthetic = args.get(1).is_some_and(|value| value == "--synthetic");
    if !synthetic && args.len() < 3 {
        return Err("uso: render-benchmark <projeto.aps> <voicebank> [iterações] [--resampler PATH] [--wavtool PATH], ou --synthetic [iterações]".into());
    }
    let mut iterations = 7usize;
    let mut resampler_path = None;
    let mut wavtool_path = None;
    let mut world = false;
    let mut uncached = false;
    let mut index = if synthetic { 2 } else { 3 };
    while index < args.len() {
        match args[index].to_string_lossy().as_ref() {
            "--world" => {
                world = true;
                index += 1;
            }
            "--resampler" if index + 1 < args.len() => {
                resampler_path = Some(args[index + 1].clone());
                index += 2;
            }
            "--wavtool" if index + 1 < args.len() => {
                wavtool_path = Some(args[index + 1].clone());
                index += 2;
            }
            "--uncached" => {
                uncached = true;
                index += 1;
            }
            value if !value.starts_with('-') => {
                iterations = value.parse()?;
                index += 1;
            }
            _ => {
                return Err(
                    format!("argumento desconhecido: {}", args[index].to_string_lossy()).into(),
                )
            }
        }
    }
    if world && resampler_path.is_some() {
        return Err("--world e --resampler são opções mutuamente exclusivas".into());
    }
    let iterations = iterations.clamp(2, 100);
    let mut synthetic_fixture = None;
    let (project, voicebank, project_load_ms, voicebank_load_ms) = if synthetic {
        let directory = tempfile::tempdir()?;
        let source: Vec<f32> = (0..72_000)
            .map(|index| {
                let time = index as f32 / 48_000.0;
                (std::f32::consts::TAU * (180.0 + 40.0 * time) * time).sin() * 0.2
            })
            .collect();
        let voicebank_path = directory.path().join("voicebank");
        std::fs::create_dir_all(&voicebank_path)?;
        TrackRenderer::save_wav_samples(voicebank_path.join("fixture.wav"), &source, 48_000)?;
        std::fs::write(
            voicebank_path.join("oto.ini"),
            "fixture.wav=a,0,100,-500,35,10\n",
        )?;
        let mut fixture_project = UProject::default();
        fixture_project.parts[0].notes = vec![UNote::new("a", "C4", 0.0, 900.0)];
        let project_path = directory.path().join("fixture.aps");
        ApsFormat::save_file(&fixture_project, &project_path)?;
        let project_started = Instant::now();
        let loaded_project = ApsFormat::load_file(project_path)?;
        let project_load_ms = project_started.elapsed().as_secs_f64() * 1000.0;
        let voicebank_started = Instant::now();
        let voicebank = Voicebank::new(&voicebank_path)?;
        let voicebank_load_ms = voicebank_started.elapsed().as_secs_f64() * 1000.0;
        synthetic_fixture = Some(directory);
        (
            loaded_project,
            voicebank,
            project_load_ms,
            voicebank_load_ms,
        )
    } else {
        let project_started = Instant::now();
        let project = ApsFormat::load_file(&args[1])?;
        let project_load_ms = project_started.elapsed().as_secs_f64() * 1000.0;
        let voicebank_started = Instant::now();
        let voicebank = Voicebank::new(&args[2])?;
        let voicebank_load_ms = voicebank_started.elapsed().as_secs_f64() * 1000.0;
        (project, voicebank, project_load_ms, voicebank_load_ms)
    };
    let isolated_cache = tempfile::Builder::new()
        .prefix("kamafeu-render-benchmark-cache-")
        .tempdir()?;
    kamafeu::renderer::resampler_cache::set_persistent_cache_dir_override(Some(
        isolated_cache.path().to_path_buf(),
    ));
    let resampler: Box<dyn ResamplerDriver> = match resampler_path {
        Some(path) => Box::new(ExternalResamplerDriver::new(path)),
        None if world => Box::new(NativeWorldResamplerDriver),
        None => Box::new(NativeResamplerDriver),
    };
    let wavtool: Box<dyn WavtoolDriver> = match wavtool_path {
        Some(path) => Box::new(ExternalWavtoolDriver::new(path)),
        None => Box::new(GalapagosWavtoolDriver),
    };
    let options = RenderOptions::default();
    let mut durations = Vec::with_capacity(iterations + 1);
    let mut pcm_hash = None;
    let mut cold_ms = 0.0;

    for iteration in 0..=iterations {
        if iteration == 0 || uncached {
            let _ = kamafeu::renderer::resampler_cache::clear_disk_cache()?;
        }
        let started = Instant::now();
        let rendered = kamafeu::ProjectRenderer::render_project_with_drivers(
            &project,
            &voicebank,
            48_000,
            0.0,
            resampler.as_ref(),
            wavtool.as_ref(),
            &options,
            None,
        );
        let elapsed = started.elapsed();
        if iteration == 0 {
            cold_ms = elapsed.as_secs_f64() * 1000.0;
        }
        if let Some(error) = rendered.error {
            return Err(format!("render #{iteration}: {error}").into());
        }
        let mut hash = Sha256::new();
        for sample in &rendered.samples {
            hash.update(sample.to_le_bytes());
        }
        let current_hash = format!("{:x}", hash.finalize());
        if let Some(expected) = &pcm_hash {
            if expected != &current_hash {
                return Err(format!(
                    "PCM divergiu na iteração {iteration}: {expected} != {current_hash}"
                )
                .into());
            }
        } else {
            pcm_hash = Some(current_hash);
        }
        if iteration > 0 {
            durations.push(elapsed);
        }
        eprintln!(
            "iteration={iteration} elapsed_ms={:.3} pcm_samples={}",
            elapsed.as_secs_f64() * 1000.0,
            rendered.samples.len()
        );
    }

    let pool_reuse = std::env::var_os("KAMAFEU_BENCH_DISABLE_POOL_REUSE").is_none();
    let cache = kamafeu::renderer::resampler_cache::cache_stats();
    println!("{{\"resampler\":\"{}\",\"wavtool\":\"{}\",\"iterations\":{iterations},\"uncached\":{uncached},\"pool_reuse\":{pool_reuse},\"project_load_ms\":{project_load_ms:.3},\"voicebank_load_ms\":{voicebank_load_ms:.3},\"cold_ms\":{cold_ms:.3},\"measured_median_ms\":{:.3},\"measured_p95_ms\":{:.3},\"cache_hits\":{},\"cache_misses\":{},\"resampler_renders\":{},\"pcm_sha256\":\"{}\",\"pcm_identical\":true}}",
        resampler.name(), wavtool.name(), percentile(&durations, 0.50), percentile(&durations, 0.95),
        cache.hits, cache.misses, cache.renders, pcm_hash.unwrap_or_default());
    drop(synthetic_fixture);
    Ok(())
}
