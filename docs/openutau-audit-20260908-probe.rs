// Standalone audit probe: link against the locally built Kamafeu rlib.
use kamafeu::dsp::envelope::UtauEnvelope;
use kamafeu::oto::Voicebank;
use kamafeu::phonemizer::{JapanesePhonemizer, PhonemizerMode};
use kamafeu::project::model::UNote;
use kamafeu::renderer::timing::{resolve_phoneme_timings, PhonemeTimingInput};

fn main() {
    let root = std::env::args().nth(1).expect("fixture directory");
    let vb = Voicebank::new(&root).unwrap();
    println!("Alias requested=a ka; resolved={:?}", vb.find_entry("a ka", "C4").map(|e| &e.alias));
    let notes = vec![UNote::new("a", "C4", 100.0, 400.0), UNote::new("a", "C4", 530.0, 400.0)];
    let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::VCV);
    println!("VCV with 30ms rest: {:?}", phones.iter().map(|p| (&p.lyric, p.position_ms)).collect::<Vec<_>>());
    let english = JapanesePhonemizer::apply_phonemizer(&[UNote::new("A", "C4", 100.0, 400.0)], &vb, PhonemizerMode::EnglishVCCV);
    println!("English VCCV authored A becomes: {:?}", english.iter().map(|p| &p.lyric).collect::<Vec<_>>());
    let inputs = [
        PhonemeTimingInput { position_ms: 100.0, duration_ms: 500.0, oto_preutter_ms: 0.0, oto_overlap_ms: 0.0, velocity: 100.0, preutter_delta_ms: 0.0, overlap_delta_ms: 0.0 },
        PhonemeTimingInput { position_ms: 600.0, duration_ms: 500.0, oto_preutter_ms: 100.0, oto_overlap_ms: 80.0, velocity: 200.0, preutter_delta_ms: 0.0, overlap_delta_ms: 0.0 },
    ];
    let timings = resolve_phoneme_timings(&inputs);
    let env = UtauEnvelope::default();
    let a = env.phoneme_points(timings[0].preutter_ms, 500.0, timings[0].tail_intrude_ms, timings[0].tail_overlap_ms, 0.0, 100.0, 100.0, 0.0);
    // track.rs restores authored crossfade=80 after timing scaled it to 40.
    let b = env.phoneme_points(timings[1].preutter_ms, 500.0, 0.0, 0.0, 80.0, 100.0, 100.0, 0.0);
    println!("Manual crossfade 80ms, velocity 200: previous fade={}ms, next fade={}ms, summed gain at 590ms={}", timings[0].tail_overlap_ms, b[1].0-b[0].0, UtauEnvelope::gain_at_points(490.0,&a)+UtauEnvelope::gain_at_points(-10.0,&b));
    let isolated = env.phoneme_points(300.0, 500.0, 0.0, 0.0, 100.0, 100.0, 100.0, 0.0);
    println!("Isolated phone: local attack={}ms; OpenUtau GetFadeIn=5ms", isolated[1].0-isolated[0].0);
    println!("External wavtool: musical duration=500ms; envelope extent={}ms", isolated[4].0-isolated[0].0);
}
