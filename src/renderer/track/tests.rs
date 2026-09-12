use super::{ResamplerSlots, TrackRenderer};
use crate::drivers::{NativeResamplerDriver, NativeWavtoolDriver};
use crate::oto::Voicebank;
use crate::phonemizer::PhonemizerMode;
use crate::project::model::UNote;
use crate::renderer::RenderOptions;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn resampler_instances_cap_concurrent_synthesis() {
    let slots = Arc::new(ResamplerSlots::new(2));
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));

    std::thread::scope(|scope| {
        for _ in 0..8 {
            let slots = Arc::clone(&slots);
            let active = Arc::clone(&active);
            let peak = Arc::clone(&peak);
            scope.spawn(move || {
                let _slot = slots.acquire(None).unwrap();
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(5));
                active.fetch_sub(1, Ordering::SeqCst);
            });
        }
    });

    assert_eq!(peak.load(Ordering::SeqCst), 2);
}

#[test]
fn unavailable_phonemes_are_silent_without_muting_valid_notes() {
    for (lyric, oto, broken_wav) in [
        ("missing_alias", "", false),
        (
            "missing_sample",
            "missing.wav=missing_sample,0,0,-500,0,0\n",
            false,
        ),
        (
            "broken_sample",
            "broken.wav=broken_sample,0,0,-500,0,0\n",
            true,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let source: Vec<f32> = (0..44100)
            .map(|i| (i as f32 * std::f32::consts::TAU * 261.63 / 44100.0).sin() * 0.2)
            .collect();
        TrackRenderer::save_wav_samples(directory.path().join("valid.wav"), &source, 44100)
            .unwrap();
        std::fs::write(
            directory.path().join("oto.ini"),
            format!("valid.wav=valid,0,50,-800,0,0\n{oto}"),
        )
        .unwrap();
        if broken_wav {
            std::fs::write(directory.path().join("broken.wav"), b"not a WAV").unwrap();
        }
        let voicebank = Voicebank::new(directory.path()).unwrap();
        let options = RenderOptions {
            phonemizer_mode: PhonemizerMode::BasicCV,
            ..Default::default()
        };
        let warnings = std::sync::Mutex::new(Vec::new());
        let log = |_: f32, message: &str| warnings.lock().unwrap().push(message.to_owned());
        let invalid = UNote::new(lyric, "C4", 0.0, 500.0);
        let render = |notes: &[UNote]| {
            TrackRenderer::try_render_track_with_progress_cancellable(
                notes,
                &voicebank,
                44100,
                120.0,
                &NativeResamplerDriver,
                &NativeWavtoolDriver,
                Some(&options),
                Some(&log),
                None,
            )
            .unwrap()
        };
        let silent = render(std::slice::from_ref(&invalid));
        assert!(!silent.is_empty());
        assert!(silent.iter().all(|sample| *sample == 0.0), "{lyric}");
        let mixed = render(&[invalid, UNote::new("valid", "C4", 1500.0, 500.0)]);
        assert!(
            mixed[..44100].iter().all(|sample| *sample == 0.0),
            "{lyric}"
        );
        assert!(
            mixed[66150..88200].iter().any(|sample| sample.abs() > 0.01),
            "valid note was muted"
        );
        assert!(mixed.iter().all(|sample| sample.is_finite()));
        assert!(warnings
            .lock()
            .unwrap()
            .iter()
            .any(|message| message.contains("silenciado") && message.contains(lyric)));
    }
}

#[test]
fn reads_32_bit_pcm_without_sign_overflow() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("pcm32.wav");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44_100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&path, spec).unwrap();
    writer.write_sample(i32::MAX).unwrap();
    writer.write_sample(i32::MIN).unwrap();
    writer.finalize().unwrap();

    let (samples, _) = TrackRenderer::load_wav_samples(path).unwrap();
    assert!(samples[0] > 0.99);
    assert!(samples[1] <= -1.0);
}

#[test]
fn converts_sample_rate_and_preserves_duration() {
    let input = vec![0.0; 44_100];
    let output = TrackRenderer::convert_sample_rate(&input, 44_100, 48_000);
    assert_eq!(output.len(), 48_000);
}

#[test]
fn mixer_adds_pre_enveloped_segments_without_a_second_fade() {
    let mut track = vec![0.0; 10];
    for (index, sample) in track[5..10].iter_mut().enumerate() {
        *sample = 1.0 - index as f32 / 4.0;
    }
    track.resize(15, 0.0);
    let mut next = vec![1.0; 10];
    for (index, sample) in next[..5].iter_mut().enumerate() {
        *sample = index as f32 / 4.0;
    }

    let end = TrackRenderer::mix_phase_aligned(&mut track, &next, 5, 10, 0, 261.63, 44_100);

    assert_eq!(end, 15);
    assert!((track[5] - 1.0).abs() < 1e-6);
    assert!((track[9] - 1.0).abs() < 1e-6);
    assert!(track[5..10].iter().all(|sample| *sample <= 1.000_001));
    assert!(track[5..10].windows(2).all(|pair| {
        let jump = (pair[1] - pair[0]).abs();
        jump < 1e-5
    }));
}

#[test]
fn adjacent_notes_crossfade_smoothly_without_discontinuities() {
    let sample_rate = 44_100;
    let mut track = vec![0.0f32; 1_000];
    // Note 1: 0..600ms, fades out from 400..600ms (samples 400..600)
    for i in 0..600 {
        let gain = if i < 400 {
            1.0
        } else {
            (600 - i) as f32 / 200.0
        };
        let carrier = (i as f32 * std::f32::consts::TAU * 220.0 / sample_rate as f32).sin();
        track[i] = carrier * gain;
    }

    // Note 2: 500..1000ms with 100ms preutterance and 50ms overlap (starts at sample 400, fades in 400..600)
    let mut note2 = vec![0.0f32; 600];
    for i in 0..600 {
        let gain = if i < 200 { i as f32 / 200.0 } else { 1.0 };
        let carrier = (i as f32 * std::f32::consts::TAU * 220.0 / sample_rate as f32).sin();
        note2[i] = carrier * gain;
    }

    TrackRenderer::mix_phase_aligned(&mut track, &note2, 400, 600, 200, 220.0, sample_rate);

    // Verify that across the entire transition 400..600 there are no pops/spikes
    let max_jump = track[390..610]
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_jump < 0.15,
        "Discontinuity / pop detected at transition: max jump was {max_jump}"
    );
}

#[test]
fn oto_enveloped_vc_overlap_is_not_faded_twice() {
    // The wavtool has already made these gains complementary. Reapplying an
    // equal-power crossfade in the mixer made the middle of a VC transition
    // dip to ~0.707, which is heard as a chopped consonant boundary.
    let mut track = vec![0.0f32; 160];
    let mut incoming = vec![0.0f32; 100];
    for index in 0..100 {
        track[30 + index] = 1.0 - index as f32 / 100.0;
        incoming[index] = index as f32 / 100.0;
    }

    TrackRenderer::mix_phase_aligned(&mut track, &incoming, 30, 130, 100, 220.0, 44_100);

    for (index, sample) in track[30..130].iter().enumerate() {
        assert!(
            (sample - 1.0).abs() < 1e-5,
            "double fade caused a VC energy dip at sample {index}: {sample}"
        );
    }
}

#[test]
fn generated_vcv_fixture_has_no_silent_transition_hole() {
    let directory = tempfile::tempdir().unwrap();
    let source: Vec<f32> = (0..44100)
        .map(|i| (i as f32 * std::f32::consts::TAU * 220.0 / 44100.0).sin() * 0.2)
        .collect();
    for name in ["ka.wav", "ki.wav"] {
        TrackRenderer::save_wav_samples(directory.path().join(name), &source, 44100).unwrap();
    }
    std::fs::write(
        directory.path().join("oto.ini"),
        "ka.wav=- ka,10,300,-900,300,100\nki.wav=a ki,10,300,-900,300,100\n",
    )
    .unwrap();
    let voicebank = Voicebank::new(directory.path()).unwrap();
    let notes = vec![
        UNote::new("ka", "C4", 0.0, 500.0),
        UNote::new("ki", "D4", 500.0, 500.0),
    ];
    let options = RenderOptions {
        phonemizer_mode: PhonemizerMode::VCV,
        ..RenderOptions::default()
    };
    let audio = TrackRenderer::render_track_with_drivers(
        &notes,
        &voicebank,
        44_100,
        120.0,
        &NativeResamplerDriver,
        &NativeWavtoolDriver,
        Some(&options),
    );

    // With 300 ms preutterance and 100 ms overlap the VCV transition is
    // 200..300 ms. The previous bug positioned the next segment at 400 ms,
    // leaving this interval silent or ending it with a hard onset.
    for center_ms in [220.0, 250.0, 280.0] {
        let center = (center_ms * 44.1) as usize;
        let radius = 220usize;
        let window = &audio[center - radius..center + radius];
        let rms = (window
            .iter()
            .map(|sample| f64::from(*sample) * f64::from(*sample))
            .sum::<f64>()
            / window.len() as f64)
            .sqrt();
        assert!(
            rms > 0.005,
            "silent VCV transition at {center_ms} ms: {rms}"
        );
    }
}

#[test]
fn phrase_portamento_is_shared_across_adjacent_phonemes() {
    let notes = vec![
        UNote::new("ka", "C4", 0.0, 500.0),
        UNote::new("ki", "D4", 500.0, 500.0),
    ];
    let curve = TrackRenderer::phrase_pitch_notes(&notes);
    let at = |time| TrackRenderer::phrase_pitch_cents_at(&curve, time, 0.0);
    assert!((at(450.0) - 6000.0).abs() < 1e-6);
    assert!((at(460.0) - 6000.0).abs() < 1e-6);
    assert!((at(500.0) - 6100.0).abs() < 1e-6);
    assert!((at(540.0) - 6200.0).abs() < 1e-6);
}

#[test]
fn vc_transitions_render_smoothly_without_discontinuities_or_empty_holes() {
    let directory = tempfile::tempdir().unwrap();
    let sample_rate = 44_100;
    let source: Vec<f32> = (0..sample_rate)
        .map(|i| (i as f32 * std::f32::consts::TAU * 220.0 / sample_rate as f32).sin() * 0.25)
        .collect();
    for name in ["ka.wav", "at.wav", "ta.wav"] {
        TrackRenderer::save_wav_samples(directory.path().join(name), &source, sample_rate).unwrap();
    }
    std::fs::write(
        directory.path().join("oto.ini"),
        "ka.wav=ka,10,100,-500,10,0\nat.wav=a t,50,90,-140,70,35\nta.wav=ta,20,80,-600,70,35\n",
    )
    .unwrap();
    let voicebank = Voicebank::new(directory.path()).unwrap();
    // Manual mode: user places ka -> a t -> ta
    let notes = vec![
        UNote::new("ka", "C4", 0.0, 400.0),
        UNote::new("a t", "C4", 400.0, 80.0),
        UNote::new("ta", "D4", 480.0, 400.0),
    ];
    let audio = TrackRenderer::render_track_with_drivers(
        &notes,
        &voicebank,
        sample_rate,
        120.0,
        &NativeResamplerDriver,
        &NativeWavtoolDriver,
        None,
    );

    assert!(
        !audio.is_empty(),
        "Rendered track buffer should not be empty"
    );
    let max_amp = audio.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_amp > 0.05,
        "Audio must not be silent, max_amp={max_amp}"
    );

    // Check transition interval around 380ms..500ms: no sudden discontinuity or harsh spike
    let start_sample = (380.0 * 44.1) as usize;
    let end_sample = (500.0 * 44.1) as usize;
    let max_jump = audio[start_sample..end_sample]
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_jump < 0.25,
        "Detected discontinuity in VC transition: jump was {max_jump}"
    );
}
