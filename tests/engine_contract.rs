use kamafeu::dsp::envelope::UtauEnvelope;
use kamafeu::renderer::timing::{plan_inputs, resolve_phoneme_timings};
use kamafeu::{
    drivers::*,
    oto::Voicebank,
    phonemizer::{JapanesePhonemizer, PhonemizerMode},
    project::model::{UNote, UProject},
    renderer::{ProjectRenderer, RenderOptions, TrackRenderer},
};
use std::sync::atomic::AtomicBool;

fn bank(aliases: &[&str]) -> (tempfile::TempDir, Voicebank) {
    let dir = tempfile::tempdir().unwrap();
    let samples: Vec<_> = (0..44100)
        .map(|i| (i as f32 * std::f32::consts::TAU / 200.0).sin() * 0.2)
        .collect();
    TrackRenderer::save_wav_samples(dir.path().join("sample.wav"), &samples, 44100).unwrap();
    std::fs::write(
        dir.path().join("oto.ini"),
        aliases
            .iter()
            .map(|a| format!("sample.wav={a},0,100,-900,100,40\n"))
            .collect::<String>(),
    )
    .unwrap();
    let vb = Voicebank::new(dir.path()).unwrap();
    (dir, vb)
}

#[test]
fn phonetic_lookup_never_drops_a_transition_or_changes_case() {
    let (_dir, vb) = bank(&["ka", "a", "A"]);
    assert!(vb.find_mapped_entry("a ka", "C4").is_none());
    assert_eq!(vb.find_mapped_entry("A", "C4").unwrap().alias, "A");
    assert_eq!(vb.find_mapped_entry("a", "C4").unwrap().alias, "a");
}

#[test]
fn vcv_respects_short_rests_and_explicit_alternatives() {
    let (_dir, vb) = bank(&["- a", "a a", "a", "ka", "a_ka"]);
    let notes = vec![
        UNote::new("a", "C4", 100.0, 400.0),
        UNote::new("a", "C4", 530.0, 400.0),
    ];
    let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::VCV);
    assert_eq!(phones[1].lyric, "- a");
    let notes = vec![
        UNote::new("a", "C4", 100.0, 400.0),
        UNote::new("ka", "C4", 500.0, 400.0),
    ];
    let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::VCV);
    assert_eq!(phones[1].lyric, "a_ka");
}

#[test]
fn manual_crossfade_is_complementary_at_multiple_velocities() {
    let (_dir, vb) = bank(&["a"]);
    for velocity in [50.0, 100.0, 200.0] {
        let mut notes = vec![
            UNote::new("a", "C4", 100.0, 500.0),
            UNote::new("a", "C4", 600.0, 500.0),
        ];
        notes[1].expressions.consonant_velocity = velocity;
        notes[1].envelope.crossfade_ms = 80.0;
        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::None);
        let timings = resolve_phoneme_timings(&plan_inputs(&phones, &vb, 0.0));
        let env = UtauEnvelope::default();
        let t = timings[1];
        let a = env.phoneme_points(
            timings[0].preutter_ms,
            500.0,
            timings[0].tail_intrude_ms,
            timings[0].tail_overlap_ms,
            0.0,
            100.0,
            100.0,
            0.0,
        );
        let b = env.phoneme_points(
            t.preutter_ms,
            500.0,
            0.0,
            0.0,
            t.overlap_ms,
            100.0,
            100.0,
            0.0,
        );
        for step in 0..=20 {
            let time = 600.0 - t.preutter_ms + t.overlap_ms * step as f64 / 20.0;
            let gain = UtauEnvelope::gain_at_points(time - 100.0, &a)
                + UtauEnvelope::gain_at_points(time - 600.0, &b);
            assert!(
                (gain - 1.0).abs() < 1e-6,
                "velocity {velocity}, time {time}, gain {gain}"
            );
        }
    }
}

#[test]
fn english_vccv_preserves_inventory_and_sustains_the_nucleus() {
    let (_dir, vb) = bank(&["-A", "A", "a", "A s", "stE", "E n", "n-"]);
    let notes = vec![
        UNote::new("A", "C4", 100.0, 500.0),
        UNote::new("s t E n", "C4", 600.0, 500.0),
    ];
    let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::EnglishVCCV);
    assert_eq!(
        phones.iter().map(|p| p.lyric.as_str()).collect::<Vec<_>>(),
        ["-A", "A s", "stE", "E n", "n-"]
    );
    assert!(phones[1].position_ms < 600.0);
    assert_eq!(phones[2].position_ms, 600.0);
    assert!(phones[2].duration_ms > phones[1].duration_ms);
    for pair in phones.windows(2) {
        assert!((pair[0].position_ms + pair[0].duration_ms - pair[1].position_ms).abs() < 1e-6);
    }
    assert!(
        (phones.last().unwrap().position_ms + phones.last().unwrap().duration_ms - 1100.0).abs()
            < 1e-6
    );
}

struct FailingResampler;
impl ResamplerDriver for FailingResampler {
    fn name(&self) -> &str {
        "contract-failure"
    }
    fn supports_persistent_cache(&self) -> bool {
        false
    }
    fn render_sample(
        &self,
        _: &[f32],
        _: u32,
        _: &ResamplerArgs,
        _: Option<&AtomicBool>,
    ) -> Result<Vec<f32>, String> {
        Err("intentional failure".into())
    }
}

#[test]
fn failed_phone_invalidates_the_project_instead_of_substituting_raw_audio() {
    let (_dir, vb) = bank(&["a"]);
    let mut project = UProject::default();
    project.parts[0].notes = vec![UNote::new("a", "C4", 100.0, 500.0)];
    let rendered = ProjectRenderer::render_project_with_drivers(
        &project,
        &vb,
        44100,
        0.0,
        &FailingResampler,
        &NativeWavtoolDriver,
        &RenderOptions::default(),
        None,
    );
    assert!(rendered.samples.is_empty());
    let error = rendered.error.unwrap();
    assert!(error.contains("'a'") && error.contains("intentional failure"));
}

#[cfg(unix)]
#[test]
fn external_wavtool_gets_one_output_and_corrected_lengths() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, _) = bank(&["a"]);
    let exe = dir.path().join("wavtool");
    std::fs::write(
        &exe,
        "#!/bin/sh\nprintf '%s|%s|%s\\n' \"$1\" \"$3\" \"$4\" >> \"$1.calls\"\ncp \"$2\" \"$1\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    let item = WavtoolArgs {
        output_wav: dir.path().join("unused.wav"),
        input_rendered_wav: dir.path().join("sample.wav"),
        skip_over_ms: 0.0,
        duration_ms: 800.0,
        envelope: UtauEnvelope::default(),
        overlap_ms: 40.0,
        phoneme_envelope: [(0.0, 0.0); 5],
        sample_time_zero_ms: -300.0,
    };
    let output = dir.path().join("phrase.wav");
    let mut second = item.clone();
    second.duration_ms = 550.0;
    second.skip_over_ms = 25.0;
    let audio = kamafeu::drivers::wavtool_driver::concatenate_external(
        &exe,
        &[item, second],
        &output,
        44100,
        None,
    )
    .unwrap();
    assert!(!audio.is_empty());
    let calls = std::fs::read_to_string(dir.path().join("phrase.wav.calls")).unwrap();
    let lines: Vec<_> = calls.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].ends_with("|0.000000|800.000000"));
    assert!(lines[1].ends_with("|25.000000|550.000000"));
    assert!(lines
        .iter()
        .all(|l| l.starts_with(output.to_str().unwrap())));
}
