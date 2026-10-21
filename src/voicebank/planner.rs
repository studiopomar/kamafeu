use super::native::{Layer, NativeBank, Unit};
use crate::project::model::UPitchBendPoint;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhoneEvent {
    /// Phoneme sequence resolved by the caller or a manual override.
    pub phones: Vec<String>,
    pub kind: String,
    /// Musical position aligned to the selected unit's anchor.
    pub position_ms: f64,
    pub duration_ms: f64,
    pub midi: u8,
    #[serde(default)]
    pub pitch_points: Vec<UPitchBendPoint>,
    pub unit_override_id: Option<String>,
    pub layer_override_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhraseRequest {
    pub style_id: String,
    pub events: Vec<PhoneEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionReason {
    Override,
    ExactContext,
    WildcardContext,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedUnit {
    pub event_index: usize,
    pub phones: Vec<String>,
    pub kind: String,
    pub layer_id: String,
    pub unit_id: String,
    pub asset_path: String,
    pub selected_midi: u8,
    pub requested_midi: u8,
    pub pitch_points: Vec<UPitchBendPoint>,
    pub reason: SelectionReason,
    pub source_start_frame: u64,
    pub source_end_frame: u64,
    pub anchor_frame: u64,
    pub fixed_end_frame: u64,
    pub sustain: Option<(u64, u64)>,
    pub release_start_frame: Option<u64>,
    pub sustain_mode: String,
    pub gain_db: f64,
    pub formant_shift_cents: f64,
    /// Audio placement in phrase time; may be negative when the anchor is
    /// after the source unit's beginning.
    pub output_start_ms: f64,
    pub target_duration_ms: f64,
    pub preferred_join_to_next_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhrasePlan {
    pub bank_id: String,
    pub bank_version: String,
    pub package_revision: u32,
    pub engine_id: String,
    pub engine_api_major: u32,
    pub style_id: String,
    pub units: Vec<PlannedUnit>,
}

/// Build a stable, inspectable unit plan. This stage selects source material
/// and timing anchors; it does not synthesize or modify audio.
pub fn plan_phrase(bank: &NativeBank, request: &PhraseRequest) -> Result<PhrasePlan, String> {
    if !bank
        .manifest
        .styles
        .iter()
        .any(|s| s.id == request.style_id)
    {
        return Err(format!("unknown KFV 2 style: {}", request.style_id));
    }
    if request.events.is_empty() {
        return Ok(PhrasePlan {
            bank_id: bank.manifest.bank_id.clone(),
            bank_version: bank.manifest.bank_version.clone(),
            package_revision: bank.manifest.package_revision,
            engine_id: bank.manifest.engine.id.clone(),
            engine_api_major: bank.manifest.engine.api_major,
            style_id: request.style_id.clone(),
            units: Vec::new(),
        });
    }
    for (i, event) in request.events.iter().enumerate() {
        if event.phones.is_empty()
            || event.midi > 127
            || !event.position_ms.is_finite()
            || !event.duration_ms.is_finite()
            || event.duration_ms <= 0.0
            || event.pitch_points.iter().any(|point| {
                !point.time_offset_ms.is_finite() || !point.pitch_offset_cents.is_finite()
            })
            || event
                .pitch_points
                .windows(2)
                .any(|pair| pair[0].time_offset_ms > pair[1].time_offset_ms)
            || (i > 0 && request.events[i - 1].position_ms > event.position_ms)
        {
            return Err(format!("invalid or out-of-order phonetic event {i}"));
        }
    }

    let mut selected = Vec::with_capacity(request.events.len());
    for (index, event) in request.events.iter().enumerate() {
        let previous = index
            .checked_sub(1)
            .and_then(|i| request.events[i].phones.last())
            .map(String::as_str);
        let next = request
            .events
            .get(index + 1)
            .and_then(|e| e.phones.first())
            .map(String::as_str);
        let mut layers: Vec<&(Layer, super::native::UnitFile)> = bank
            .layers
            .iter()
            .filter(|(layer, _)| layer.style_id == request.style_id)
            .collect();
        if let Some(layer_id) = &event.layer_override_id {
            layers.retain(|(layer, _)| &layer.id == layer_id);
            if layers.is_empty() {
                return Err(format!(
                    "layer override '{}' is unavailable for style '{}'",
                    layer_id, request.style_id
                ));
            }
        }
        layers.sort_by_key(|(layer, _)| {
            (
                layer.recorded_midi.abs_diff(event.midi),
                layer.recorded_midi,
                layer.id.as_str(),
            )
        });
        if layers.is_empty() {
            return Err(format!(
                "no layer available for style '{}'",
                request.style_id
            ));
        }
        let mut choice = None;
        for layer in layers {
            let mut candidates: Vec<(&Unit, bool, bool)> = layer
                .1
                .units
                .iter()
                .filter(|unit| unit.kind == event.kind && unit.phones == event.phones)
                .filter_map(|unit| {
                    let left_exact = context_matches(&unit.left_context, previous, index == 0)?;
                    let right_exact = context_matches(
                        &unit.right_context,
                        next,
                        index + 1 == request.events.len(),
                    )?;
                    Some((unit, left_exact, right_exact))
                })
                .collect();
            if let Some(override_id) = &event.unit_override_id {
                candidates.retain(|(unit, _, _)| &unit.id == override_id);
            }
            candidates.sort_by(|(a, al, ar), (b, bl, br)| {
                let a_context = u8::from(*al) + u8::from(*ar);
                let b_context = u8::from(*bl) + u8::from(*br);
                b.quality
                    .priority
                    .cmp(&a.quality.priority)
                    .then_with(|| b.quality.reviewed.cmp(&a.quality.reviewed))
                    .then_with(|| b_context.cmp(&a_context))
                    .then_with(|| a.id.cmp(&b.id))
            });
            if let Some(candidate) = candidates.first().copied() {
                choice = Some((layer, candidate));
                break;
            }
        }
        let (chosen_layer, (unit, left_exact, right_exact)) = choice.ok_or_else(|| {
            if let Some(override_id) = &event.unit_override_id {
                format!(
                    "unit override '{}' is missing or context-incompatible for event {index}",
                    override_id
                )
            } else {
                format!(
                    "no eligible KFV 2 unit for event {index}: {}",
                    event.phones.join(" ")
                )
            }
        })?;
        let reason = if event.unit_override_id.is_some() {
            SelectionReason::Override
        } else if left_exact || right_exact {
            SelectionReason::ExactContext
        } else {
            SelectionReason::WildcardContext
        };
        let asset = chosen_layer
            .1
            .assets
            .iter()
            .find(|asset| asset.id == unit.asset_id)
            .ok_or_else(|| format!("unit '{}' refers to missing asset", unit.id))?;
        let anchor_ms = (unit.source.anchor_frame - unit.source.start_frame) as f64 * 1000.0
            / asset.sample_rate_hz as f64;
        selected.push(PlannedUnit {
            event_index: index,
            phones: event.phones.clone(),
            kind: event.kind.clone(),
            layer_id: chosen_layer.0.id.clone(),
            unit_id: unit.id.clone(),
            asset_path: asset.path.clone(),
            selected_midi: chosen_layer.0.recorded_midi,
            requested_midi: event.midi,
            pitch_points: event.pitch_points.clone(),
            reason,
            source_start_frame: unit.source.start_frame,
            source_end_frame: unit.source.end_frame,
            anchor_frame: unit.source.anchor_frame,
            fixed_end_frame: unit.source.fixed_end_frame,
            sustain: unit
                .source
                .sustain
                .as_ref()
                .map(|r| (r.start_frame, r.end_frame)),
            release_start_frame: unit.source.release_start_frame,
            sustain_mode: unit.render.sustain_mode.clone(),
            gain_db: unit.render.gain_db,
            formant_shift_cents: unit.render.formant_shift_cents,
            output_start_ms: event.position_ms - anchor_ms,
            target_duration_ms: (event.duration_ms + anchor_ms).max(unit.timing.min_target_ms),
            preferred_join_to_next_ms: 0.0,
        });
    }
    for i in 0..selected.len().saturating_sub(1) {
        let current_event = &request.events[i];
        let next_event = &request.events[i + 1];
        let current = bank
            .layers
            .iter()
            .find(|(l, _)| l.id == selected[i].layer_id)
            .ok_or_else(|| format!("planned layer '{}' disappeared", selected[i].layer_id))?;
        let next = bank
            .layers
            .iter()
            .find(|(l, _)| l.id == selected[i + 1].layer_id)
            .ok_or_else(|| format!("planned layer '{}' disappeared", selected[i + 1].layer_id))?;
        let current_unit = current
            .1
            .units
            .iter()
            .find(|u| u.id == selected[i].unit_id)
            .ok_or_else(|| format!("planned unit '{}' disappeared", selected[i].unit_id))?;
        let next_unit = next
            .1
            .units
            .iter()
            .find(|u| u.id == selected[i + 1].unit_id)
            .ok_or_else(|| format!("planned unit '{}' disappeared", selected[i + 1].unit_id))?;
        let requested = current_unit
            .timing
            .preferred_crossfade_ms
            .min(next_unit.timing.preferred_crossfade_ms);
        let limit = current_unit
            .timing
            .max_crossfade_ms
            .min(next_unit.timing.max_crossfade_ms);
        let requested_join = requested.min(limit).max(0.0);
        let event_overlap_ms =
            current_event.position_ms + current_event.duration_ms - next_event.position_ms;
        let join_ms = if event_overlap_ms > 0.5 {
            requested_join.min(event_overlap_ms)
        } else if event_overlap_ms >= -0.5 {
            // A contiguous musical boundary still needs an audio overlap.
            // Units such as KAMUSE VC have no preutterance, so their source
            // intervals otherwise meet exactly at the boundary and the
            // renderer has nothing to crossfade.
            requested_join
        } else {
            0.0
        };
        selected[i].preferred_join_to_next_ms = join_ms;

        if join_ms > 0.0 {
            let current_end_ms = selected[i].output_start_ms + selected[i].target_duration_ms;
            let incoming_start_ms = selected[i + 1].output_start_ms;
            let existing_audio_overlap_ms = current_end_ms - incoming_start_ms;
            let missing_overlap_ms = join_ms - existing_audio_overlap_ms;
            if missing_overlap_ms > 0.0 {
                // Pull the incoming unit earlier and extend its duration by
                // the same amount. Its end remains aligned to the note end,
                // while the mixer gets real shared samples to crossfade.
                selected[i + 1].output_start_ms -= missing_overlap_ms;
                selected[i + 1].target_duration_ms += missing_overlap_ms;
            }
        }
    }
    Ok(PhrasePlan {
        bank_id: bank.manifest.bank_id.clone(),
        bank_version: bank.manifest.bank_version.clone(),
        package_revision: bank.manifest.package_revision,
        engine_id: bank.manifest.engine.id.clone(),
        engine_api_major: bank.manifest.engine.api_major,
        style_id: request.style_id.clone(),
        units: selected,
    })
}

fn context_matches(
    context: &super::native::Context,
    adjacent: Option<&str>,
    at_boundary: bool,
) -> Option<bool> {
    match context.kind.as_str() {
        "any" => Some(false),
        "boundary" if at_boundary => Some(true),
        "boundary" => None,
        "phone" if context.phone.as_deref() == adjacent => Some(true),
        "phone" => None,
        _ => None,
    }
}
