//! Explicit bridge from APS note data to the native singer phrase pipeline.
use crate::project::model::{UNote, UProject};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, OnceLock};

use super::native::{NativeBank, PhoneticProfile};
use super::planner::{plan_phrase, PhoneEvent, PhrasePlan, PhraseRequest};
use super::render::RenderedPhrase;

#[derive(Debug, Clone)]
pub struct NativeProjectPhrase {
    pub track_index: usize,
    pub plan: PhrasePlan,
    pub rendered: RenderedPhrase,
    /// Project-level warnings that do not prevent a WORLD render.
    pub warnings: Vec<String>,
}

const MAX_NATIVE_PHRASE_CACHE_ENTRIES: usize = 64;
const MAX_NATIVE_PHRASE_CACHE_SAMPLES: usize = 64 * 1024 * 1024;

#[derive(Default)]
struct NativePhraseCache {
    entries: HashMap<[u8; 32], NativeProjectPhrase>,
    insertion_order: VecDeque<[u8; 32]>,
    sample_count: usize,
}

fn native_phrase_cache() -> &'static Mutex<NativePhraseCache> {
    static CACHE: OnceLock<Mutex<NativePhraseCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(NativePhraseCache::default()))
}

fn native_phrase_cache_key(
    bank_hash: &str,
    track_index: usize,
    output_sample_rate: u32,
    plan: &PhrasePlan,
) -> Result<[u8; 32], String> {
    let plan_json = serde_json::to_vec(plan)
        .map_err(|error| format!("cannot fingerprint native phrase plan: {error}"))?;
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    hasher.update(b"kamafeu.native-phrase-cache.v1\0");
    hasher.update(bank_hash.as_bytes());
    hasher.update((track_index as u64).to_le_bytes());
    hasher.update(output_sample_rate.to_le_bytes());
    hasher.update(plan_json);
    Ok(hasher.finalize().into())
}

fn cached_native_phrase(key: &[u8; 32]) -> Option<NativeProjectPhrase> {
    native_phrase_cache().lock().ok()?.entries.get(key).cloned()
}

fn store_native_phrase(key: [u8; 32], phrase: NativeProjectPhrase) {
    let sample_count = phrase.rendered.samples.len();
    if sample_count > MAX_NATIVE_PHRASE_CACHE_SAMPLES {
        return;
    }
    let Ok(mut cache) = native_phrase_cache().lock() else {
        return;
    };
    if let Some(previous) = cache.entries.remove(&key) {
        cache.sample_count = cache
            .sample_count
            .saturating_sub(previous.rendered.samples.len());
        cache.insertion_order.retain(|existing| existing != &key);
    }
    while cache.entries.len() >= MAX_NATIVE_PHRASE_CACHE_ENTRIES
        || cache.sample_count.saturating_add(sample_count) > MAX_NATIVE_PHRASE_CACHE_SAMPLES
    {
        let Some(oldest) = cache.insertion_order.pop_front() else {
            break;
        };
        if let Some(removed) = cache.entries.remove(&oldest) {
            cache.sample_count = cache
                .sample_count
                .saturating_sub(removed.rendered.samples.len());
        }
    }
    cache.sample_count = cache.sample_count.saturating_add(sample_count);
    cache.insertion_order.push_back(key);
    cache.entries.insert(key, phrase);
}

/// Plan and render one APS vocal track with its explicitly bound native singer.
/// The project must contain direct phone sequences in `phonetic_hint`, lyric,
/// or the first native phoneme override. No G2P or classic voicebank fallback occurs.
pub fn render_project_track_world(
    bank: &NativeBank,
    project: &UProject,
    track_index: usize,
    output_sample_rate: u32,
) -> Result<NativeProjectPhrase, String> {
    render_project_track_world_cancellable(bank, project, track_index, output_sample_rate, None)
}

pub fn render_project_track_world_cancellable(
    bank: &NativeBank,
    project: &UProject,
    track_index: usize,
    output_sample_rate: u32,
    cancel: Option<&AtomicBool>,
) -> Result<NativeProjectPhrase, String> {
    let track = project
        .tracks
        .get(track_index)
        .ok_or_else(|| format!("native KFV 2 track index does not exist: {track_index}"))?;
    if cancel.is_some_and(|token| token.load(std::sync::atomic::Ordering::Relaxed)) {
        return Err("native project track render cancelled".into());
    }
    let reference = track
        .native_voicebank
        .as_ref()
        .ok_or_else(|| format!("track '{}' has no native voicebank reference", track.name))?;
    verify_binding(bank, reference, cancel)?;
    if output_sample_rate == 0 {
        return Err("output sample rate must be greater than zero".into());
    }

    let profile = bank
        .profiles
        .iter()
        .find(|profile| profile.id == reference.phonetic_profile_id)
        .ok_or_else(|| {
            format!(
                "native phonetic profile is unavailable: {}",
                reference.phonetic_profile_id
            )
        })?;
    let mut notes: Vec<(f64, &UNote)> = project
        .parts
        .iter()
        .filter(|part| part.track_index == track_index)
        .flat_map(|part| {
            part.notes
                .iter()
                .map(move |note| (part.position_ms + note.position_ms, note))
        })
        .collect();
    notes.sort_by(|left, right| {
        left.0
            .partial_cmp(&right.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if notes.is_empty() {
        return Err(format!("track '{}' has no notes to render", track.name));
    }

    let mut events = Vec::with_capacity(notes.len().saturating_mul(2));
    let mut warnings = Vec::new();
    for (index, (position_ms, note)) in notes.iter().enumerate() {
        if cancel.is_some_and(|token| token.load(std::sync::atomic::Ordering::Relaxed)) {
            return Err("native project track render cancelled".into());
        }
        if !position_ms.is_finite() || !note.duration_ms.is_finite() || note.duration_ms <= 0.0 {
            return Err(format!("track '{}' has invalid note timing", track.name));
        }
        let pronunciation_override = note
            .phoneme_overrides
            .iter()
            .find(|override_| override_.index == 0)
            .and_then(|override_| override_.phoneme.as_deref());
        let direct_phones = pronunciation_override
            .or(note.phonetic_hint.as_deref())
            .unwrap_or(note.lyric.as_str());
        let phones = resolve_note_phones(direct_phones, profile);
        if phones.is_empty() {
            return Err(format!(
                "note {index} has no direct phonetic sequence; native G2P is not enabled"
            ));
        }
        let explicit_segments = explicit_override_segments(note, profile, index)?;
        let final_phone = phones.last().cloned().expect("phones checked above");
        let (segments, offsets, durations) = if let Some(segments) = explicit_segments {
            let durations = note.resolved_phoneme_durations(segments.len());
            let mut offsets = Vec::with_capacity(segments.len());
            let mut elapsed = 0.0;
            for duration in &durations {
                offsets.push(elapsed);
                elapsed += duration;
            }
            (segments, offsets, durations)
        } else if phones.len() > 2 {
            let phone_durations = note.resolved_phoneme_durations(phones.len());
            let mut segments = Vec::with_capacity(phones.len() - 1);
            let mut offsets = Vec::with_capacity(phones.len() - 1);
            let mut durations = Vec::with_capacity(phones.len() - 1);
            let mut elapsed = 0.0;
            for phone_index in 0..phones.len() - 1 {
                segments.push(vec![
                    phones[phone_index].clone(),
                    phones[phone_index + 1].clone(),
                ]);
                offsets.push(elapsed);
                durations.push(phone_durations[phone_index] + phone_durations[phone_index + 1]);
                elapsed += phone_durations[phone_index];
            }
            (segments, offsets, durations)
        } else {
            (vec![phones], vec![0.0], vec![note.duration_ms])
        };

        let midi = note.midi_key();
        let previous_midi = index
            .checked_sub(1)
            .map(|previous| notes[previous].1.midi_key());
        let note_adjacent = previous_midi.is_some_and(|_| {
            let previous = notes[index - 1].1;
            UNote::are_adjacent(notes[index - 1].0 + previous.duration_ms, *position_ms)
        });
        let native_overrides: Vec<_> = note
            .phoneme_overrides
            .iter()
            .filter(|override_| {
                override_.native_unit_id.is_some() || override_.native_layer_id.is_some()
            })
            .collect();
        if native_overrides
            .iter()
            .any(|override_| override_.index >= segments.len())
        {
            return Err(format!(
                "native unit/layer override on note {index} references an event outside its {} KFV 2 events",
                segments.len()
            ));
        }
        for (event_index, event_phones) in segments.into_iter().enumerate() {
            let kind = infer_unit_kind(profile, &event_phones)
                .map_err(|error| {
                    if error.starts_with("unknown phone '") {
                        format!(
                            "nota {}, evento {}: {error}; este banco usa fonemas diretos separados por espaços (por exemplo: k a)",
                            index + 1,
                            event_index + 1
                        )
                    } else {
                        format!("note {index}, event {event_index}: {error}")
                    }
                })?;
            let native_override = native_overrides
                .iter()
                .find(|override_| override_.index == event_index)
                .copied();
            let mut pitch_points = note.pitch_bend.effective_points(
                previous_midi.filter(|_| note_adjacent && event_index == 0),
                midi,
                note_adjacent && event_index == 0,
            );
            for point in &mut pitch_points {
                point.time_offset_ms -= offsets[event_index];
            }
            events.push(PhoneEvent {
                phones: event_phones,
                kind,
                position_ms: *position_ms + offsets[event_index],
                duration_ms: durations[event_index],
                midi,
                pitch_points,
                unit_override_id: native_override.and_then(|value| value.native_unit_id.clone()),
                layer_override_id: native_override.and_then(|value| value.native_layer_id.clone()),
            });
        }

        // A connected note boundary can use a vowel-to-consonant unit from
        // the same recording, if the bank contains that coverage. Keep this
        // at the project bridge so every native project render follows it.
        if let Some((next_position_ms, next_note)) = notes.get(index + 1).copied() {
            let boundary_ms = *position_ms + note.duration_ms;
            if UNote::are_adjacent(boundary_ms, next_position_ms) {
                let next_override = next_note
                    .phoneme_overrides
                    .iter()
                    .find(|override_| override_.index == 0)
                    .and_then(|override_| override_.phoneme.as_deref());
                let next_direct = next_override
                    .or(next_note.phonetic_hint.as_deref())
                    .unwrap_or(next_note.lyric.as_str());
                let next_phones = resolve_note_phones(next_direct, profile);
                if let Some(consonant) = next_phones.first() {
                    let is_vowel = |phone: &str| {
                        profile
                            .symbols
                            .iter()
                            .any(|symbol| symbol.id == phone && symbol.class == "vowel")
                    };
                    if is_vowel(&final_phone) && !is_vowel(consonant) {
                        let vc_phones = vec![final_phone.clone(), consonant.clone()];
                        let vc_unit = bank.layers.iter().find_map(|(_, units)| {
                            units
                                .units
                                .iter()
                                .find(|unit| unit.kind == "VC" && unit.phones == vc_phones)
                        });
                        if let Some(unit) = vc_unit {
                            let duration_ms = unit
                                .timing
                                .preferred_crossfade_ms
                                .min(note.duration_ms)
                                .min(next_note.duration_ms)
                                .max(1.0);
                            events.push(PhoneEvent {
                                phones: vc_phones,
                                kind: "VC".into(),
                                position_ms: boundary_ms - duration_ms,
                                duration_ms,
                                midi: next_note.midi_key(),
                                pitch_points: Vec::new(),
                                unit_override_id: None,
                                layer_override_id: None,
                            });
                        } else {
                            warnings.push(format!(
                                "Sem cobertura VC para {} {} na junção entre as notas {} e {}; mantendo junção CV direta.",
                                final_phone,
                                consonant,
                                index + 1,
                                index + 2
                            ));
                        }
                    }
                }
            }
        }
    }
    events.sort_by(|left, right| {
        left.position_ms
            .partial_cmp(&right.position_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let request = PhraseRequest {
        style_id: reference.style_id.clone(),
        events,
    };
    let plan = plan_phrase(bank, &request)?;
    let cache_key = native_phrase_cache_key(
        &reference.content_sha256,
        track_index,
        output_sample_rate,
        &plan,
    )?;
    if cancel.is_some_and(|token| token.load(std::sync::atomic::Ordering::Relaxed)) {
        return Err("native project track render cancelled".into());
    }
    if let Some(cached) = cached_native_phrase(&cache_key) {
        return Ok(cached);
    }
    let rendered =
        super::render::render_phrase_world_cancellable(bank, &plan, output_sample_rate, cancel)?;
    let phrase = NativeProjectPhrase {
        track_index,
        plan,
        rendered,
        warnings,
    };
    if !cancel.is_some_and(|token| token.load(std::sync::atomic::Ordering::Relaxed)) {
        store_native_phrase(cache_key, phrase.clone());
    }
    Ok(phrase)
}

fn verify_binding(
    bank: &NativeBank,
    reference: &crate::project::NativeVoicebankReference,
    cancel: Option<&AtomicBool>,
) -> Result<(), String> {
    if reference.bank_id != bank.manifest.bank_id
        || reference.bank_version != bank.manifest.bank_version
        || reference.package_revision != bank.manifest.package_revision
        || reference.engine_id != bank.manifest.engine.id
        || reference.engine_api_major != bank.manifest.engine.api_major
        || reference.phonetic_profile_id != bank.manifest.defaults.phonetic_profile_id
        || !bank
            .manifest
            .styles
            .iter()
            .any(|style| style.id == reference.style_id)
    {
        return Err(
            "native voicebank identity in the project does not match the loaded bank".into(),
        );
    }
    if reference.planner_version != "kamafeu.planner.v1" {
        return Err(format!(
            "unsupported native planner version: {}",
            reference.planner_version
        ));
    }
    let current_hash = bank.content_sha256_cancellable(cancel)?;
    if reference.content_sha256 != current_hash {
        return Err(format!(
            "KFV 2 content hash changed for '{}': expected {}, found {}",
            reference.bank_id, reference.content_sha256, current_hash
        ));
    }
    Ok(())
}

/// Indexed phoneme overrides can provide explicit one- or two-phone segments.
/// A single index-zero override is still treated as the direct phrase sequence
/// and split into overlapping transitions when it contains more than two phones.
fn explicit_override_segments(
    note: &UNote,
    profile: &PhoneticProfile,
    note_index: usize,
) -> Result<Option<Vec<Vec<String>>>, String> {
    let has_later_pronunciation = note
        .phoneme_overrides
        .iter()
        .any(|override_| override_.index > 0 && override_.phoneme.is_some());
    if !has_later_pronunciation {
        return Ok(None);
    }
    let pronunciation_by_index: std::collections::BTreeMap<usize, &str> = note
        .phoneme_overrides
        .iter()
        .filter_map(|override_| {
            override_
                .phoneme
                .as_deref()
                .map(|phoneme| (override_.index, phoneme))
        })
        .collect();
    let max_index = pronunciation_by_index
        .keys()
        .next_back()
        .copied()
        .unwrap_or_default();
    let mut segments = Vec::with_capacity(max_index + 1);
    for index in 0..=max_index {
        let phoneme = pronunciation_by_index.get(&index).ok_or_else(|| {
            format!("note {note_index} has a gap before indexed KFV 2 pronunciation {index}")
        })?;
        let phones: Vec<String> = phoneme.split_whitespace().map(str::to_owned).collect();
        if phones.is_empty() || phones.len() > 2 {
            return Err(format!(
                "note {note_index}, event {index} must contain one or two direct phones"
            ));
        }
        for phone in &phones {
            if !profile.symbols.iter().any(|symbol| symbol.id == *phone) {
                return Err(format!(
                    "note {note_index}, event {index} references unknown phone '{phone}'"
                ));
            }
        }
        segments.push(phones);
    }
    Ok(Some(segments))
}

/// Resolve a native note's direct phonetic text. Whitespace keeps explicit
/// phone boundaries; compact text such as `ka` is segmented against the
/// profile inventory so a CV bank can be played from ordinary syllable input.
fn resolve_note_phones(value: &str, profile: &PhoneticProfile) -> Vec<String> {
    let explicit: Vec<String> = value.split_whitespace().map(str::to_owned).collect();
    if explicit.len() != 1 {
        return explicit;
    }
    let text = explicit[0].to_lowercase();
    let exact = profile
        .symbols
        .iter()
        .find(|symbol| symbol.id.to_lowercase() == text);
    if let Some(symbol) = exact {
        return vec![symbol.id.clone()];
    }

    let mut symbols: Vec<_> = profile.symbols.iter().collect();
    symbols.sort_by(|left, right| {
        right
            .id
            .len()
            .cmp(&left.id.len())
            .then_with(|| left.id.cmp(&right.id))
    });
    let mut best: Vec<Option<Vec<String>>> = vec![None; text.len() + 1];
    best[text.len()] = Some(Vec::new());
    for offset in (0..text.len()).rev() {
        if !text.is_char_boundary(offset) {
            continue;
        }
        for symbol in &symbols {
            let token = symbol.id.to_lowercase();
            let end = offset + token.len();
            if end > text.len()
                || !text.is_char_boundary(end)
                || !text[offset..].starts_with(&token)
            {
                continue;
            }
            let Some(suffix) = best[end].as_ref() else {
                continue;
            };
            let mut candidate = Vec::with_capacity(suffix.len() + 1);
            candidate.push(symbol.id.clone());
            candidate.extend(suffix.iter().cloned());
            if best[offset]
                .as_ref()
                .is_none_or(|current| candidate.len() < current.len())
            {
                best[offset] = Some(candidate);
            }
        }
    }
    best[0].clone().unwrap_or_else(|| vec![explicit[0].clone()])
}

fn infer_unit_kind(profile: &PhoneticProfile, phones: &[String]) -> Result<String, String> {
    let classes: Vec<&str> = phones
        .iter()
        .map(|phone| {
            profile
                .symbols
                .iter()
                .find(|symbol| symbol.id == *phone)
                .map(|symbol| symbol.class.as_str())
                .ok_or_else(|| format!("unknown phone '{phone}'"))
        })
        .collect::<Result<_, _>>()?;
    let is_vowel = |class: &str| class == "vowel";
    match classes.as_slice() {
        [class] if is_vowel(class) => Ok("V".into()),
        [class] if *class == "breath" => Ok("breath".into()),
        [class] if *class == "release" => Ok("release".into()),
        [left, right] if !is_vowel(left) && is_vowel(right) => Ok("CV".into()),
        [left, right] if is_vowel(left) && !is_vowel(right) => Ok("VC".into()),
        [left, right] if is_vowel(left) && is_vowel(right) => Ok("VV".into()),
        [_, _] => Ok("CC".into()),
        _ => Err("direct MVP events support one or two phones".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> PhrasePlan {
        PhrasePlan {
            bank_id: "test.voice".into(),
            bank_version: "1.0.0".into(),
            package_revision: 1,
            engine_id: "kamafeu.sola".into(),
            engine_api_major: 1,
            style_id: "natural".into(),
            units: Vec::new(),
        }
    }

    #[test]
    fn native_phrase_cache_key_tracks_content_plan_and_output_format() {
        let first_plan = plan();
        let first = native_phrase_cache_key("hash-a", 0, 48_000, &first_plan).unwrap();
        assert_eq!(
            first,
            native_phrase_cache_key("hash-a", 0, 48_000, &first_plan).unwrap()
        );
        assert_ne!(
            first,
            native_phrase_cache_key("hash-b", 0, 48_000, &first_plan).unwrap()
        );
        assert_ne!(
            first,
            native_phrase_cache_key("hash-a", 1, 48_000, &first_plan).unwrap()
        );
        assert_ne!(
            first,
            native_phrase_cache_key("hash-a", 0, 44_100, &first_plan).unwrap()
        );
        let mut changed_plan = first_plan;
        changed_plan.style_id = "soft".into();
        assert_ne!(
            first,
            native_phrase_cache_key("hash-a", 0, 48_000, &changed_plan).unwrap()
        );
    }
}
