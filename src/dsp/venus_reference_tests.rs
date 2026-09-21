use super::VenusResampler;

/// Opt-in: no personal recordings are stored in the repository.
/// VENUS_REFERENCE_BANK=/path VENUS_REFERENCE_OUTPUT=/path cargo test
/// venus_reference_corpus --lib --release -- --ignored --nocapture
#[test]
#[ignore = "requires a local reference voicebank"]
fn venus_reference_corpus() {
    let bank = crate::oto::Voicebank::new(
        std::env::var("VENUS_REFERENCE_BANK").expect("reference bank path"),
    )
    .unwrap();
    let output = std::path::PathBuf::from(
        std::env::var("VENUS_REFERENCE_OUTPUT").expect("output directory"),
    );
    std::fs::create_dir_all(&output).unwrap();
    let mut rows = Vec::new();
    for (index, alias) in ["` a", "a -", "b a", "a f", "AP a"].iter().enumerate() {
        let entry = bank.entries.get(*alias).expect("reference alias missing");
        let (samples, sr) = crate::renderer::TrackRenderer::load_wav_samples(
            bank.root_path.join(&entry.wav_filename),
        )
        .unwrap();
        for (level_name, gain) in [("original", 1.0f32), ("soft", 0.05)] {
            let input: Vec<f32> = samples.iter().map(|s| s * gain).collect();
            for hz in [110.0, 220.0, 440.0] {
                let start = std::time::Instant::now();
                let rendered = VenusResampler::render_sample(
                    &input,
                    sr,
                    entry.offset,
                    entry.consonant,
                    entry.consonant.min(180.0),
                    entry.cutoff,
                    600.0,
                    hz,
                    &[],
                    None,
                    None,
                    None,
                    0.0,
                    0.0,
                );
                let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
                assert_eq!(rendered.len(), (sr as f64 * 0.6).round() as usize);
                assert!(rendered.iter().all(|s| s.is_finite()));
                let peak = rendered.iter().fold(0.0f32, |p, s| p.max(s.abs()));
                assert!(peak <= 0.9);
                let rms = (rendered.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>()
                    / rendered.len() as f64)
                    .sqrt();
                let step = rendered
                    .windows(2)
                    .fold(0.0f32, |p, s| p.max((s[1] - s[0]).abs()));
                let path = output.join(format!("{index}-{level_name}-{hz:.0}.wav"));
                let mut writer = hound::WavWriter::create(
                    path,
                    hound::WavSpec {
                        channels: 1,
                        sample_rate: sr,
                        bits_per_sample: 32,
                        sample_format: hound::SampleFormat::Float,
                    },
                )
                .unwrap();
                for sample in rendered {
                    writer.write_sample(sample).unwrap();
                }
                writer.finalize().unwrap();
                rows.push(serde_json::json!({"alias":alias,"level":level_name,"hz":hz,"peak":peak,"rms":rms,"max_step":step,"render_ms":elapsed_ms}));
            }
        }
    }
    std::fs::write(
        output.join("metrics.json"),
        serde_json::to_string_pretty(&rows).unwrap(),
    )
    .unwrap();
    println!(
        "Reference corpus: {} cases at {}",
        rows.len(),
        output.display()
    );
}
