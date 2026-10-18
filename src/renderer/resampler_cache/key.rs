use super::CACHE_SCHEMA;
use crate::drivers::{ResamplerArgs, ResamplerDriver};
use std::hash::{DefaultHasher, Hash, Hasher};

fn hash_f64(value: f64, hasher: &mut impl Hasher) {
    value.to_bits().hash(hasher);
}

fn hash_optional_f64(value: Option<f64>, hasher: &mut impl Hasher) {
    value.map(f64::to_bits).hash(hasher);
}

fn file_fingerprint(path: &std::path::Path, hasher: &mut impl Hasher) {
    path.hash(hasher);
    if let Ok(metadata) = std::fs::metadata(path) {
        metadata.len().hash(hasher);
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                duration.as_nanos().hash(hasher);
            }
        }
    }
}

fn source_fingerprint(args: &ResamplerArgs, raw_samples: &[f32], hasher: &mut impl Hasher) {
    let input = &args.input_wav;
    file_fingerprint(input, hasher);
    if !input.is_file() {
        // Some preview paths provide synthesized source samples without a
        // persistent WAV. Include those samples so a reused temporary path
        // cannot return audio from a previous voicebank/alias.
        raw_samples.len().hash(hasher);
        for sample in raw_samples {
            sample.to_bits().hash(hasher);
        }
    }

    let extension = input
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let stem = input
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let without_extension = input.with_extension("");
    let mut sidecars = vec![
        input.with_file_name(format!("{stem}_{extension}.frq")),
        input.with_extension(format!("{extension}.llsm")),
        input.with_extension(format!("{extension}.uspec")),
        input.with_extension(format!("{extension}.dio")),
        input.with_extension(format!("{extension}.star")),
        input.with_extension(format!("{extension}.platinum")),
        input.with_extension(format!("{extension}.frc")),
        input.with_extension(format!("{extension}.pmk")),
        input.with_extension(format!("{extension}.vs4ufrq")),
        without_extension.with_extension("venus"),
        without_extension.with_extension("rudb"),
        without_extension.with_extension("sc.npz"),
        without_extension.with_extension("sc"),
        without_extension.with_extension("hifi.npz"),
    ];
    if let Some(parent) = input.parent() {
        sidecars.push(parent.join("desc.mrq"));
    }
    for sidecar in sidecars.into_iter().filter(|path| path.is_file()) {
        file_fingerprint(&sidecar, hasher);
    }
}

pub(super) fn cache_key(
    driver: &dyn ResamplerDriver,
    sample_rate: u32,
    args: &ResamplerArgs,
    raw_samples: &[f32],
) -> u64 {
    let mut hasher = DefaultHasher::new();
    CACHE_SCHEMA.hash(&mut hasher);
    driver.cache_identity().hash(&mut hasher);
    sample_rate.hash(&mut hasher);
    source_fingerprint(args, raw_samples, &mut hasher);
    args.pitch_name.hash(&mut hasher);
    hash_f64(args.pitch_freq, &mut hasher);
    hash_f64(args.velocity, &mut hasher);
    args.flags.hash(&mut hasher);
    hash_f64(args.offset_ms, &mut hasher);
    hash_f64(args.duration_ms, &mut hasher);
    hash_f64(args.source_consonant_ms, &mut hasher);
    hash_f64(args.consonant_ms, &mut hasher);
    hash_f64(args.cutoff_ms, &mut hasher);
    hash_f64(args.volume, &mut hasher);
    hash_f64(args.modulation, &mut hasher);
    hash_f64(args.tempo, &mut hasher);
    args.pitch_bend_str.hash(&mut hasher);
    for point in &args.pitch_points {
        hash_f64(point.time_offset_ms, &mut hasher);
        hash_f64(point.pitch_offset_cents, &mut hasher);
        point.shape.hash(&mut hasher);
    }
    hash_optional_f64(args.loop_start_ms, &mut hasher);
    hash_optional_f64(args.loop_end_ms, &mut hasher);
    hash_optional_f64(args.tail_start_ms, &mut hasher);
    hasher.finish()
}
