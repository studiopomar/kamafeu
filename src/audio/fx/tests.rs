use super::*;

#[test]
fn cached_equalizer_matches_original_across_blocks_and_gain_changes() {
    use super::biquad::BiquadState;
    use super::equalizer::Equalizer;

    for sample_rate in [8000, 44100, 48000, 96000] {
        let mut gains = [0.0; 31];
        let mut equalizer = Equalizer::new(gains, sample_rate as f32);
        let mut reference = [BiquadState::default(); 31];
        for block in 0..12 {
            match block {
                1 => gains[12] = 6.0,
                3 => gains[20] = -4.0,
                5 => gains = [0.0; 31],
                6 => gains[12] = 6.0,
                8 => gains = [2.5; 31],
                10 => gains[12] = 0.049,
                11 => gains[12] = 0.05,
                _ => {}
            }
            // The old algorithm recomputed every coefficient and checked every band
            // per stereo frame. Keep it here as a differential audio reference.
            let mut expected: Vec<f32> = (0..257)
                .map(|i| ((i + block * 257) as f32 * 0.071).sin() * 0.2)
                .collect();
            let mut actual = expected.clone();
            if gains.iter().any(|gain| gain.abs() >= 0.05) {
                for i in 0..31 {
                    reference[i].set_peaking(
                        ISO_31_BAND_FREQS[i],
                        gains[i],
                        4.318,
                        sample_rate as f32,
                    );
                }
                for frame in expected.chunks_exact_mut(2) {
                    for i in 0..31 {
                        if gains[i].abs() >= 0.05 {
                            (frame[0], frame[1]) = reference[i].process(frame[0], frame[1]);
                        }
                    }
                }
            }
            equalizer.process(&mut actual, &gains);
            assert_eq!(actual, expected, "rate={sample_rate}, block={block}");
        }
    }
}

#[test]
fn test_fx_rack_bypass() {
    let config = FxRackConfig {
        master_enabled: false,
        ..Default::default()
    };
    let mut processor = FxRackProcessor::new(config, 44100);
    let mut samples = vec![0.5f32; 100];
    let original = samples.clone();
    processor.process_interleaved(&mut samples, 44100);
    assert_eq!(samples, original);
}

#[test]
fn test_fx_rack_processing_does_not_panic() {
    let mut config = FxRackConfig::default();
    config.master_enabled = true;
    config.eq.enabled = true;
    config.compressor.enabled = true;
    config.delay.enabled = true;
    config.reverb.enabled = true;

    let mut processor = FxRackProcessor::new(config, 44100);
    let mut samples = vec![0.3f32; 1000];
    processor.process_interleaved(&mut samples, 44100);
    for s in &samples {
        assert!(s.is_finite());
    }
}
