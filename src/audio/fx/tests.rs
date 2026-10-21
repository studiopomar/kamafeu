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
    let mut config = FxRackConfig {
        master_enabled: true,
        ..Default::default()
    };
    config.eq.enabled = true;
    config.compressor.enabled = true;
    config.delay.enabled = true;
    config.reverb.enabled = true;
    config.saturation.enabled = true;
    config.limiter.enabled = true;
    config.chorus.enabled = true;
    config.gate.enabled = true;
    config.auto_pan.enabled = true;

    let mut processor = FxRackProcessor::new(config, 44100);
    let mut samples = vec![0.3f32; 1000];
    processor.process_interleaved(&mut samples, 44100);
    for s in &samples {
        assert!(s.is_finite());
    }
}

#[test]
fn limiter_enforces_configured_ceiling() {
    let mut config = FxRackConfig {
        master_enabled: true,
        ..Default::default()
    };
    config.limiter.enabled = true;
    config.limiter.ceiling_db = -6.0;
    let mut processor = FxRackProcessor::new(config, 44_100);
    let mut samples = vec![1.0, -1.0];
    processor.process_interleaved(&mut samples, 44_100);
    assert!(samples.iter().all(|sample| sample.abs() <= 0.502));
}

fn rack_with(configure: impl FnOnce(&mut FxRackConfig)) -> FxRackProcessor {
    let mut config = FxRackConfig {
        master_enabled: true,
        ..Default::default()
    };
    config.eq.enabled = false;
    configure(&mut config);
    FxRackProcessor::new(config, 44_100)
}

fn stereo_sine(frequency: f32, amplitude: f32, frames: usize) -> Vec<f32> {
    (0..frames)
        .flat_map(|index| {
            let value =
                (std::f32::consts::TAU * frequency * index as f32 / 44_100.0).sin() * amplitude;
            [value, value]
        })
        .collect()
}

#[test]
fn limiter_rides_the_gain_instead_of_clipping() {
    let mut processor = rack_with(|config| {
        config.limiter.enabled = true;
        config.limiter.ceiling_db = -6.0;
    });
    let input = stereo_sine(220.0, 1.0, 22_050);
    let mut output = input.clone();
    processor.process_interleaved(&mut output, 44_100);

    assert!(output.iter().all(|sample| sample.abs() <= 0.502));
    // After the first peak the waveform is scaled, not flattened: the
    // output/input ratio is the same near zero crossings and at the peaks.
    for (out, reference) in output.iter().zip(&input).skip(800) {
        if reference.abs() > 0.05 {
            let ratio = out / reference;
            assert!((0.49..=0.52).contains(&ratio), "ratio {ratio}");
        }
    }
}

#[test]
fn limiter_recovers_after_an_over() {
    let mut processor = rack_with(|config| {
        config.limiter.enabled = true;
        config.limiter.ceiling_db = -6.0;
        config.limiter.release_ms = 50.0;
    });
    let mut samples = stereo_sine(220.0, 1.0, 4_410);
    samples.extend(stereo_sine(220.0, 0.2, 44_100));
    processor.process_interleaved(&mut samples, 44_100);
    let tail = &samples[samples.len() - 4_410..];
    let peak = tail
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    assert!((peak - 0.2).abs() < 0.002, "gain did not recover: {peak}");
}

#[test]
fn compressor_follows_the_soft_knee_curve() {
    let steady_gain_db = |level_db: f32, knee_db: f32| {
        let mut processor = rack_with(|config| {
            config.compressor.enabled = true;
            config.compressor.threshold_db = -20.0;
            config.compressor.ratio = 4.0;
            config.compressor.knee_db = knee_db;
            config.compressor.makeup_gain_db = 0.0;
            config.compressor.attack_ms = 1.0;
            config.compressor.release_ms = 10.0;
        });
        let level = 10.0f32.powf(level_db / 20.0);
        let mut samples = vec![level; 44_100];
        processor.process_interleaved(&mut samples, 44_100);
        20.0 * (samples[samples.len() - 1] / level).log10()
    };
    // Far above the threshold the knee does not matter: 10 dB over -> -7.5 dB.
    assert!((steady_gain_db(-10.0, 0.0) + 7.5).abs() < 0.01);
    assert!((steady_gain_db(-10.0, 6.0) + 7.5).abs() < 0.01);
    // At the threshold a 10 dB knee already compresses by 0.9375 dB...
    assert!((steady_gain_db(-20.0, 10.0) + 0.9375).abs() < 0.01);
    // ...while the hard knee does not, and below the knee nothing happens.
    assert!(steady_gain_db(-20.0, 0.0).abs() < 0.01);
    assert!(steady_gain_db(-30.0, 10.0).abs() < 0.01);
}

#[test]
fn compressor_makeup_no_longer_hard_clips() {
    let mut processor = rack_with(|config| {
        config.compressor.enabled = true;
        config.compressor.threshold_db = 0.0;
        config.compressor.knee_db = 0.0;
        config.compressor.makeup_gain_db = 12.0;
    });
    let input = stereo_sine(440.0, 0.9, 4_410);
    let mut output = input.clone();
    processor.process_interleaved(&mut output, 44_100);
    let makeup = 10.0f32.powf(12.0 / 20.0);
    for (out, reference) in output.iter().zip(&input) {
        assert!((out - reference * makeup).abs() < 1e-4);
    }
}

#[test]
fn gate_stays_open_on_a_low_note_near_the_threshold() {
    let mut processor = rack_with(|config| {
        config.gate.enabled = true;
        config.gate.threshold_db = -30.0;
        config.gate.attack_ms = 4.0;
        config.gate.release_ms = 120.0;
    });
    // 80 Hz at -27 dBFS: its instantaneous value is below the threshold for
    // half of every cycle, which made the old gate modulate the note.
    let amplitude = 10.0f32.powf(-27.0 / 20.0);
    let input = stereo_sine(80.0, amplitude, 44_100);
    let mut output = input.clone();
    processor.process_interleaved(&mut output, 44_100);
    for (out, reference) in output.iter().zip(&input).skip(2 * 8_820) {
        if reference.abs() > amplitude * 0.1 {
            let ratio = out / reference;
            assert!((ratio - 1.0).abs() < 1e-3, "gate gain modulated: {ratio}");
        }
    }
}

#[test]
fn gate_closes_after_the_hold_and_release() {
    let mut processor = rack_with(|config| {
        config.gate.enabled = true;
        config.gate.threshold_db = -30.0;
        config.gate.release_ms = 20.0;
    });
    let mut samples = stereo_sine(200.0, 0.5, 4_410);
    samples.extend(stereo_sine(200.0, 0.005, 22_050));
    processor.process_interleaved(&mut samples, 44_100);
    let tail = &samples[samples.len() - 4_410..];
    assert!(tail.iter().all(|sample| sample.abs() < 1e-5));
}

#[test]
fn saturation_latency_is_compensated_for_complete_renders() {
    let input = stereo_sine(330.0, 0.5, 8_192);

    let mut dry_only = rack_with(|config| {
        config.saturation.enabled = true;
        config.saturation.mix = 0.0;
    });
    assert!(dry_only.latency_frames() > 0);
    let mut output = input.clone();
    dry_only.process_render(&mut output, 44_100);
    assert_eq!(output, input);

    // In the linear region the wet path is a gain of 1 / tanh(1).
    let mut wet_only = rack_with(|config| {
        config.saturation.enabled = true;
        config.saturation.drive_db = 0.0;
        config.saturation.mix = 1.0;
    });
    let quiet = stereo_sine(330.0, 0.01, 8_192);
    let mut output = quiet.clone();
    wet_only.process_render(&mut output, 44_100);
    let expected_gain = 1.0 / 1.0f32.tanh();
    for (out, reference) in output.iter().zip(&quiet).skip(400).take(15_000) {
        assert!(
            (out - reference * expected_gain).abs() < 2e-4,
            "{out} vs {}",
            reference * expected_gain
        );
    }
}

#[test]
fn chorus_widens_a_mono_source() {
    let mut processor = rack_with(|config| {
        config.chorus.enabled = true;
        config.chorus.mix = 0.5;
    });
    let mut samples = stereo_sine(440.0, 0.5, 44_100);
    processor.process_interleaved(&mut samples, 44_100);
    let difference: f32 = samples
        .chunks_exact(2)
        .skip(4_410)
        .map(|frame| (frame[0] - frame[1]).abs())
        .sum::<f32>()
        / 39_690.0;
    assert!(
        difference > 0.01,
        "chorus output is still mono ({difference})"
    );
}

#[test]
fn auto_pan_keeps_constant_power_with_unity_at_the_centre() {
    let mut centred = rack_with(|config| {
        config.auto_pan.enabled = true;
        config.auto_pan.depth = 0.0;
    });
    let input = stereo_sine(440.0, 0.5, 1_000);
    let mut output = input.clone();
    centred.process_interleaved(&mut output, 44_100);
    assert_eq!(output, input);

    let mut moving = rack_with(|config| {
        config.auto_pan.enabled = true;
        config.auto_pan.depth = 1.0;
        config.auto_pan.rate_hz = 2.0;
    });
    let mut samples = vec![0.5f32; 2 * 44_100];
    moving.process_interleaved(&mut samples, 44_100);
    for frame in samples.chunks_exact(2) {
        let power = frame[0] * frame[0] + frame[1] * frame[1];
        assert!((power - 0.5).abs() < 1e-4, "power {power}");
    }
}

#[test]
fn reverb_pre_delay_postpones_the_wet_signal() {
    let first_wet_frame = |pre_delay_ms: f32| {
        let mut processor = rack_with(|config| {
            config.reverb.enabled = true;
            config.reverb.dry_level = 0.0;
            config.reverb.pre_delay_ms = pre_delay_ms;
        });
        let mut samples = vec![0.0f32; 2 * 8_820];
        samples[0] = 1.0;
        samples[1] = 1.0;
        processor.process_interleaved(&mut samples, 44_100);
        samples
            .chunks_exact(2)
            .position(|frame| frame[0] != 0.0 || frame[1] != 0.0)
            .unwrap()
    };
    let without = first_wet_frame(0.0);
    let with = first_wet_frame(50.0);
    assert_eq!(with - without, 2_205);
}

#[test]
fn reverb_tail_decays_to_exact_silence_without_subnormals() {
    let mut processor = rack_with(|config| {
        config.reverb.enabled = true;
        config.reverb.room_size = 0.0;
        config.reverb.dry_level = 0.0;
    });
    let mut samples = vec![0.0f32; 2 * 44_100 * 6];
    samples[0] = 1.0;
    samples[1] = 1.0;
    processor.process_interleaved(&mut samples, 44_100);
    assert!(samples.iter().all(|sample| !sample.is_subnormal()));
    assert!(samples[samples.len() - 2_000..]
        .iter()
        .all(|sample| *sample == 0.0));
}

#[test]
fn reset_clears_tails_between_independent_renders() {
    let mut processor = rack_with(|config| {
        config.reverb.enabled = true;
        config.delay.enabled = true;
    });
    let mut first = stereo_sine(440.0, 0.5, 4_410);
    processor.process_interleaved(&mut first, 44_100);
    processor.reset();
    let mut silence = vec![0.0f32; 2 * 4_410];
    processor.process_interleaved(&mut silence, 44_100);
    assert!(silence.iter().all(|sample| *sample == 0.0));
}

#[test]
fn old_fx_settings_deserialize_with_new_defaults() {
    let json = r#"{
        "master_enabled": true,
        "eq": {"enabled": false},
        "compressor": {"enabled": true, "threshold_db": -18.0, "ratio": 4.0,
                        "attack_ms": 15.0, "release_ms": 120.0, "makeup_gain_db": 3.0},
        "delay": {"enabled": false, "time_ms": 250.0, "feedback": 0.35,
                   "wet_level": 0.25, "ping_pong": true},
        "reverb": {"enabled": true, "room_size": 0.6, "damping": 0.4, "width": 1.0,
                    "wet_level": 0.2, "dry_level": 1.0},
        "limiter": {"enabled": true, "ceiling_db": -1.0}
    }"#;
    let config: FxRackConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.compressor.knee_db, 6.0);
    assert_eq!(config.reverb.pre_delay_ms, 0.0);
    assert_eq!(config.limiter.release_ms, 80.0);
}
