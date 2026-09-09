use super::{PhrasePitchNote, TrackRenderer};
use crate::project::model::UNote;

impl TrackRenderer {
    pub(super) fn phrase_pitch_notes(notes: &[UNote]) -> Vec<PhrasePitchNote> {
        let mut result = notes
            .iter()
            .enumerate()
            .map(|(index, note)| {
                let previous = index.checked_sub(1).and_then(|index| notes.get(index));
                let adjacent = previous.is_some_and(|previous| {
                    (note.position_ms - (previous.position_ms + previous.duration_ms)).abs() <= 60.0
                });
                let is_plus = note.lyric.trim() == "+" || note.lyric.trim().starts_with("+ ");
                let points = if is_plus && adjacent {
                    // Nota '+' com legato contínuo herda / faz transição contínua
                    note.pitch_bend.effective_points(
                        previous.map(UNote::midi_key),
                        note.midi_key(),
                        true,
                    )
                } else {
                    note.pitch_bend.effective_points(
                        previous.map(UNote::midi_key),
                        note.midi_key(),
                        adjacent,
                    )
                };
                PhrasePitchNote {
                    position_ms: note.position_ms,
                    duration_ms: note.duration_ms,
                    midi: note.midi_key(),
                    // A note owns its base pitch from its musical start. Only
                    // a negative first point may begin that ownership earlier
                    // to form a portamento from the previous note.
                    curve_start_ms: note.position_ms
                        + points
                            .first()
                            .map(|point| point.time_offset_ms.min(0.0))
                            .unwrap_or(0.0),
                    points,
                    vibrato: note.vibrato.clone(),
                    pitch_delta: note.expressions.pitch_delta,
                }
            })
            .collect::<Vec<_>>();
        result.sort_by(|left, right| {
            left.curve_start_ms
                .partial_cmp(&right.curve_start_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        result
    }

    pub(super) fn phrase_pitch_cents_at(
        pitch_notes: &[PhrasePitchNote],
        absolute_time_ms: f64,
        tone_shift: f64,
    ) -> f64 {
        use crate::dsp::pitch_bend::PitchBendSolver;
        if pitch_notes.is_empty() {
            return (60.0 + tone_shift) * 100.0;
        }
        let upper = pitch_notes.partition_point(|note| note.curve_start_ms <= absolute_time_ms);
        let index = upper.saturating_sub(1).min(pitch_notes.len() - 1);
        let note = &pitch_notes[index];
        let relative_time_ms = absolute_time_ms - note.position_ms;
        let bend = PitchBendSolver::get_pitch_offset_cents_sorted(relative_time_ms, &note.points);
        let vibrato = note
            .vibrato
            .pitch_offset_cents_at(relative_time_ms, note.duration_ms);
        (f64::from(note.midi) + tone_shift) * 100.0 + note.pitch_delta + bend + vibrato
    }

    pub(super) fn combined_pitch_points(
        phone: &crate::phonemizer::RenderPhone,
        pitch_notes: &[PhrasePitchNote],
        segment_start_ms: f64,
        duration_ms: f64,
        tone_shift: f64,
    ) -> Vec<crate::project::model::UPitchBendPoint> {
        use crate::dsp::pitch_bend::PitchBendSolver;
        use crate::project::model::UPitchBendPoint;

        let step_ms = 5.0;
        let count = (duration_ms.max(1.0) / step_ms).ceil() as usize + 1;
        let mut points = Vec::with_capacity(count);
        for index in 0..count {
            let time_ms = (index as f64 * step_ms).min(duration_ms);
            let absolute_pitch =
                Self::phrase_pitch_cents_at(pitch_notes, segment_start_ms + time_ms, tone_shift);
            let cents = absolute_pitch - f64::from(phone.midi_key()) * 100.0;
            points.push(UPitchBendPoint {
                time_offset_ms: time_ms,
                pitch_offset_cents: cents,
                shape: "l".to_string(),
            });
        }
        PitchBendSolver::simplify_pitch_points(&points, 0.25)
    }
}
