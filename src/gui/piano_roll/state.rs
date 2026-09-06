use crate::gui::types::{AutoScrollMode, EditTool, GridSnapOption, PitchSubTool};
use crate::project::model::{UNote, UPitchBendPoint};
use eframe::egui::Pos2;
use std::collections::HashSet;

pub const PHONEME_SEPARATORS: [char; 3] = ['.', ',', ';'];

pub fn active_phoneme_query(lyric: &str) -> &str {
    lyric
        .rsplit_once(PHONEME_SEPARATORS)
        .map_or(lyric, |(_, active)| active)
        .trim()
}

pub fn replace_active_phoneme(lyric: &str, alias: &str) -> String {
    let Some((separator_index, separator)) = lyric
        .char_indices()
        .rev()
        .find(|(_, character)| PHONEME_SEPARATORS.contains(character))
    else {
        return alias.to_string();
    };

    let active_start = separator_index + separator.len_utf8();
    let active = &lyric[active_start..];
    let whitespace_bytes = active.len() - active.trim_start().len();
    format!(
        "{}{}{}",
        &lyric[..active_start],
        &active[..whitespace_bytes],
        alias
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParameterTab {
    #[default]
    Dynamics,
    PitchDelta,
    Gender,
    Velocity,
    Breathiness,
    Modulation,
    Volume,
    Attack,
    Decay,
    VibratoLength,
    VibratoDepth,
    VibratoPeriod,
}

pub struct PianoRollState {
    pub selected_note_index: Option<usize>,
    pub selected_note_indices: HashSet<usize>,
    pub editing_lyric_index: Option<usize>,
    pub lyric_buffer: String,
    pub editing_phoneme_index: Option<(usize, usize)>,
    pub phoneme_buffer: String,
    pub phoneme_needs_select_all: bool,
    pub autocomplete_selected_idx: usize,
    pub autocomplete_cache_key: String,
    pub autocomplete_candidates: Vec<String>,
    pub playhead_ms: f64,
    pub is_playing: bool,
    pub px_per_ms: f32,
    pub row_height: f32,
    pub min_midi: u8,
    pub max_midi: u8,
    pub dragging_note_idx: Option<usize>,
    pub dragging_is_resize: bool,
    pub dragging_is_left_resize: bool,
    pub drag_start_pos: Option<Pos2>,
    pub note_original_start_ms: f64,
    pub note_original_duration_ms: f64,
    pub note_original_midi: u8,
    pub note_original_states: Vec<(usize, f64, u8)>,
    pub marquee_start: Option<Pos2>,
    pub marquee_current: Option<Pos2>,
    pub creating_note_idx: Option<usize>,
    pub active_tool: EditTool,
    pub pitch_sub_tool: PitchSubTool,
    pub pitch_line_start: Option<(usize, f64, f64)>, // (note_idx, time_offset_ms, pitch_offset_cents)
    pub auto_scroll_mode: AutoScrollMode,
    pub is_scrubbing_ruler: bool,
    pub show_arrangement_view: bool,
    pub arrangement_height: f32,
    pub show_parameters_drawer: bool,
    pub drawer_height: f32,
    pub selected_parameter: ParameterTab,
    pub horizontal_scroll_offset: f32,
    pub vertical_scroll_offset: f32,
    pub initial_scrolled: bool,
    pub properties_window_for_note: Option<usize>,
    pub context_menu_note_idx: Option<usize>,
    pub context_menu_pos: Option<Pos2>,
    pub dragging_envelope_pt: Option<(usize, usize)>, // (note_idx, pt_idx)
    pub dragging_pitch_pt: Option<(usize, usize)>,    // (note_idx, pt_idx)
    pub continuous_edit_dirty: bool,
    pub lyric_needs_select_all: bool,
    pub rendered_waveform_peaks: Vec<(f32, f32)>,
    pub show_waveform: bool,
    pub show_envelope_handles: bool,
    pub vibrato_popover_note_idx: Option<usize>,
    pub phoneme_cache: Vec<String>,
    pub note_phonemes_cache: Vec<Vec<(String, f64, f64)>>,
    pub oto_consonant_cache: Vec<f64>,
    pub oto_preutter_cache: Vec<f64>,
    pub oto_overlap_cache: Vec<f64>,
    pub phoneme_cache_hash: u64,
    pub pitch_brush_raw_stroke: Vec<(usize, f64, f64)>,
    pub dragging_phoneme_handle: Option<(usize, u8, f32, f64)>,
    pub dragging_subphoneme_boundary: Option<(usize, usize, f32, f64)>,
    pub right_click_reset_active: bool,
}

pub fn smooth_pitch_points(raw_points: &[(f64, f64)]) -> Vec<UPitchBendPoint> {
    if raw_points.is_empty() {
        return Vec::new();
    }
    if raw_points.len() <= 2 {
        return raw_points
            .iter()
            .map(|&(t, c)| UPitchBendPoint {
                time_offset_ms: t,
                pitch_offset_cents: c,
                shape: "s".to_string(),
            })
            .collect();
    }

    let n = raw_points.len();
    let mut smoothed = Vec::with_capacity(n);
    let kernel = [0.06136, 0.24477, 0.38774, 0.24477, 0.06136]; // 5-tap Gaussian kernel

    for i in 0..n {
        let t_center = raw_points[i].0;
        let mut weighted_cents = 0.0;
        let mut weight_sum = 0.0;

        for (k_idx, &weight) in kernel.iter().enumerate() {
            let offset = k_idx as isize - 2;
            let sample_idx = (i as isize + offset).clamp(0, n as isize - 1) as usize;
            weighted_cents += raw_points[sample_idx].1 * weight;
            weight_sum += weight;
        }

        let smoothed_cents = if weight_sum > 0.0 {
            weighted_cents / weight_sum
        } else {
            raw_points[i].1
        };

        smoothed.push((t_center, smoothed_cents));
    }

    let mut relaxed = Vec::with_capacity(n);
    for i in 0..n {
        let prev = smoothed[i.saturating_sub(1)].1;
        let curr = smoothed[i].1;
        let next = smoothed[(i + 1).min(n - 1)].1;
        relaxed.push((smoothed[i].0, (prev + curr * 2.0 + next) * 0.25));
    }

    let mut result: Vec<UPitchBendPoint> = Vec::new();
    let min_spacing_ms = 18.0;

    for (t, c) in relaxed {
        if let Some(last) = result.last() {
            if (t - last.time_offset_ms).abs() < min_spacing_ms {
                continue;
            }
        }
        result.push(UPitchBendPoint {
            time_offset_ms: t,
            pitch_offset_cents: c,
            shape: "s".to_string(),
        });
    }

    if let Some(&(last_t, last_c)) = raw_points.last() {
        if result
            .last()
            .is_none_or(|p| (p.time_offset_ms - last_t).abs() > 5.0)
        {
            result.push(UPitchBendPoint {
                time_offset_ms: last_t,
                pitch_offset_cents: last_c,
                shape: "s".to_string(),
            });
        }
    }

    result
}

impl PianoRollState {
    pub fn update_rendered_waveform(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
        start_offset_ms: f64,
    ) {
        self.rendered_waveform_peaks.clear();
        if samples.is_empty() || sample_rate == 0 {
            self.rendered_waveform_peaks.shrink_to_fit();
            return;
        }

        let ch = usize::from(channels.max(1));
        let total_frames = samples.len() / ch;
        if total_frames == 0 {
            self.rendered_waveform_peaks.shrink_to_fit();
            return;
        }

        let bucket_ms = 2.0;
        let frames_per_bucket = ((bucket_ms * sample_rate as f64) / 1000.0).max(1.0) as usize;
        let ms_per_frame = 1000.0 / sample_rate as f64;

        let decay_coeff = (-1.0 / (0.016 * sample_rate as f64)).exp() as f32;
        let mut env: f32 = 0.0;
        let mut max_overall: f32 = 0.001;

        let num_buckets = (total_frames + frames_per_bucket - 1) / frames_per_bucket;
        let mut raw_peaks = Vec::with_capacity(num_buckets);

        for (bucket_idx, frame_chunk) in samples.chunks(frames_per_bucket * ch).enumerate() {
            let start_frame = bucket_idx * frames_per_bucket;
            let time_ms = (start_offset_ms + start_frame as f64 * ms_per_frame) as f32;

            for frame in frame_chunk.chunks(ch) {
                let mut frame_max: f32 = 0.0;
                for &s in frame {
                    let a = s.abs();
                    if a > frame_max {
                        frame_max = a;
                    }
                }
                if frame_max > env {
                    env = frame_max;
                } else {
                    env *= decay_coeff;
                }
            }
            if env > max_overall {
                max_overall = env;
            }
            raw_peaks.push((time_ms, env));
        }

        if raw_peaks.len() >= 3 {
            let mut smoothed = Vec::with_capacity(raw_peaks.len());
            smoothed.push(raw_peaks[0]);
            for i in 1..raw_peaks.len() - 1 {
                let t = raw_peaks[i].0;
                let v =
                    0.25 * raw_peaks[i - 1].1 + 0.5 * raw_peaks[i].1 + 0.25 * raw_peaks[i + 1].1;
                smoothed.push((t, v));
            }
            smoothed.push(raw_peaks[raw_peaks.len() - 1]);
            raw_peaks = smoothed;
        }

        let norm_factor = 1.0 / max_overall.max(0.001);
        self.rendered_waveform_peaks = raw_peaks
            .into_iter()
            .map(|(t, p)| (t, (p * norm_factor).min(1.0)))
            .collect();
        self.rendered_waveform_peaks.shrink_to_fit();
    }

    pub fn append_rendered_waveform(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
        start_offset_ms: f64,
    ) {
        let mut chunk_state = PianoRollState::default();
        chunk_state.update_rendered_waveform(samples, sample_rate, channels, start_offset_ms);
        if chunk_state.rendered_waveform_peaks.is_empty() {
            return;
        }
        let chunk_start = chunk_state.rendered_waveform_peaks[0].0;
        self.rendered_waveform_peaks
            .retain(|(time_ms, _)| *time_ms < chunk_start);
        self.rendered_waveform_peaks
            .extend(chunk_state.rendered_waveform_peaks);
    }

    pub fn waveform_amplitude_at(&self, time_ms: f32) -> Option<f32> {
        let first = self.rendered_waveform_peaks.first()?.0;
        let last = self.rendered_waveform_peaks.last()?.0;
        if time_ms < first || time_ms > last {
            return None;
        }
        let index = match self.rendered_waveform_peaks.binary_search_by(|(time, _)| {
            time.partial_cmp(&time_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            Ok(index) => index,
            Err(index) => index,
        };
        if index == 0 {
            return Some(self.rendered_waveform_peaks[0].1);
        }
        if index >= self.rendered_waveform_peaks.len() {
            return self
                .rendered_waveform_peaks
                .last()
                .map(|(_, amplitude)| *amplitude);
        }
        let (t0, a0) = self.rendered_waveform_peaks[index - 1];
        let (t1, a1) = self.rendered_waveform_peaks[index];
        if (t1 - t0).abs() < f32::EPSILON {
            return Some(a0);
        }
        let frac = ((time_ms - t0) / (t1 - t0)).clamp(0.0, 1.0);
        Some(a0 + frac * (a1 - a0))
    }
}

impl Default for PianoRollState {
    fn default() -> Self {
        Self {
            selected_note_index: None,
            selected_note_indices: HashSet::new(),
            editing_lyric_index: None,
            lyric_buffer: String::new(),
            lyric_needs_select_all: false,
            editing_phoneme_index: None,
            phoneme_buffer: String::new(),
            phoneme_needs_select_all: false,
            autocomplete_selected_idx: 0,
            autocomplete_cache_key: String::new(),
            autocomplete_candidates: Vec::new(),
            playhead_ms: 0.0,
            is_playing: false,
            px_per_ms: 0.25,
            row_height: 22.0,
            min_midi: 36,
            max_midi: 96,
            dragging_note_idx: None,
            dragging_is_resize: false,
            dragging_is_left_resize: false,
            drag_start_pos: None,
            note_original_start_ms: 0.0,
            note_original_duration_ms: 0.0,
            note_original_midi: 60,
            note_original_states: Vec::new(),
            marquee_start: None,
            marquee_current: None,
            creating_note_idx: None,
            active_tool: EditTool::Pointer,
            pitch_sub_tool: PitchSubTool::Freehand,
            pitch_line_start: None,
            auto_scroll_mode: AutoScrollMode::PageScroll,
            is_scrubbing_ruler: false,
            show_arrangement_view: true,
            arrangement_height: 120.0,
            show_parameters_drawer: true,
            drawer_height: 130.0,
            selected_parameter: ParameterTab::Dynamics,
            horizontal_scroll_offset: 0.0,
            vertical_scroll_offset: 0.0,
            initial_scrolled: false,
            properties_window_for_note: None,
            context_menu_note_idx: None,
            context_menu_pos: None,
            dragging_envelope_pt: None,
            dragging_pitch_pt: None,
            continuous_edit_dirty: false,
            rendered_waveform_peaks: Vec::new(),
            show_waveform: true,
            show_envelope_handles: false,
            vibrato_popover_note_idx: None,
            phoneme_cache: Vec::new(),
            note_phonemes_cache: Vec::new(),
            oto_consonant_cache: Vec::new(),
            oto_preutter_cache: Vec::new(),
            oto_overlap_cache: Vec::new(),
            phoneme_cache_hash: 0,
            pitch_brush_raw_stroke: Vec::new(),
            dragging_phoneme_handle: None,
            dragging_subphoneme_boundary: None,
            right_click_reset_active: false,
        }
    }
}

pub fn apply_snap(val_ms: f64, snap: GridSnapOption, bpm: f64) -> f64 {
    if let Some(step) = snap.step_ms(bpm) {
        (val_ms / step).round() * step
    } else {
        val_ms
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoPitchStyle {
    Natural,
    Expressive,
    SmoothPop,
}

pub fn apply_autopitch_to_selection(
    notes: &mut [UNote],
    selection: &HashSet<usize>,
    style: AutoPitchStyle,
) {
    if notes.is_empty() {
        return;
    }

    let preset = match style {
        AutoPitchStyle::Natural => crate::dsp::AutoPitchPreset::NaturalPop,
        AutoPitchStyle::Expressive => crate::dsp::AutoPitchPreset::BalladExpressive,
        AutoPitchStyle::SmoothPop => crate::dsp::AutoPitchPreset::JPopAnime,
    };

    let options = crate::dsp::AutoPitchOptions {
        preset,
        intensity: 1.0,
        enable_attack_scoop: true,
        enable_overshoot: true,
        enable_release_drop: true,
        enable_vibrato: true,
    };

    let sel_vec: Option<Vec<usize>> = if selection.is_empty() {
        None
    } else {
        Some(selection.iter().copied().collect())
    };

    crate::dsp::AutoPitchEngine::apply_to_notes(notes, sel_vec.as_deref(), &options);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autocomplete_uses_and_replaces_only_the_phoneme_after_a_separator() {
        for lyric in ["- aa.d ih", "- aa,d ih", "- aa;d ih"] {
            assert_eq!(active_phoneme_query(lyric), "d ih");
            let separator = lyric
                .chars()
                .find(|character| PHONEME_SEPARATORS.contains(character))
                .unwrap();
            assert_eq!(
                replace_active_phoneme(lyric, "d ih D4"),
                format!("- aa{separator}d ih D4")
            );
        }
        assert_eq!(active_phoneme_query("- aa."), "");
        assert_eq!(replace_active_phoneme("- aa. ", "d ih"), "- aa. d ih");
    }

    #[test]
    fn progressive_chunks_extend_waveform_without_repeating_last_peak() {
        let mut state = PianoRollState::default();
        state.update_rendered_waveform(&vec![0.5; 100], 1_000, 1, 0.0);
        state.append_rendered_waveform(&vec![0.25; 100], 1_000, 1, 100.0);

        assert!(state.waveform_amplitude_at(50.0).is_some());
        assert!(state.waveform_amplitude_at(150.0).is_some());
        assert_eq!(state.waveform_amplitude_at(250.0), None);
    }
}
