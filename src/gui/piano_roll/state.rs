pub use super::lyric::{active_phoneme_query, replace_active_phoneme, PHONEME_SEPARATORS};
pub use super::pitch_data::{canonical_pitch_shape, insert_pitch_point, smooth_pitch_points};
use crate::gui::types::{AutoScrollMode, EditTool, PitchSubTool};
#[cfg(test)]
use crate::project::model::UNote;
use eframe::egui::{Pos2, Rect};
use std::collections::HashSet;

#[path = "state/quantize.rs"]
mod quantize;
pub use quantize::{apply_snap, apply_snap_with_zoom};

#[path = "state/scale.rs"]
mod scale;
pub use scale::{MusicalScale, ROOT_NOTE_NAMES};

#[path = "state/autopitch.rs"]
mod autopitch;
pub use autopitch::{apply_autopitch_to_selection, AutoPitchStyle};

#[path = "state/waveform.rs"]
mod waveform;
pub use waveform::set_waveform_cache_resolution;

#[derive(Debug, Clone, Copy, Default)]
pub struct ArrangementMeterState {
    pub level: f32,
    pub peak: f32,
}

/// Render-time `oto.ini` geometry cached for one phoneme in the ruler.  A note can
/// expand into several aliases, so a note-level `oto.ini` cache is not enough to
/// show (or edit around) its real crossfades.
#[derive(Debug, Clone, Default)]
pub struct CachedPhoneme {
    pub alias: String,
    pub relative_position_ms: f64,
    pub duration_ms: f64,
    pub preutter_ms: f64,
    pub overlap_ms: f64,
    pub tail_intrude_ms: f64,
    pub tail_overlap_ms: f64,
    /// Five points resolved from the same OpenUtau timing calculation used by
    /// the renderer. X is relative to `relative_position_ms`, Y is gain.
    pub envelope_points: [(f64, f64); 5],
    pub auto_preutter_ms: f64,
    pub auto_overlap_ms: f64,
    pub consonant_velocity: f64,
}

/// Maps an absolute project time to a screen coordinate.  All horizontally
/// scrolling piano-roll layers use this same affine transform so waveform,
/// grid, notes and playhead cannot accumulate different origins.
pub(crate) fn timeline_time_to_screen_x(
    content_left: f32,
    time_ms: f64,
    px_per_ms: f32,
    scroll_offset: f32,
) -> f32 {
    content_left + (time_ms * px_per_ms as f64) as f32 - scroll_offset
}

/// Returns the absolute timeline interval covered by a screen rectangle.
/// Keeping this conversion beside the shared affine transform prevents a
/// footer layer from sampling waveform data outside its actual viewport.
pub(crate) fn timeline_visible_time_range(
    viewport: Rect,
    content_left: f32,
    px_per_ms: f32,
    scroll_offset: f32,
) -> Option<(f64, f64)> {
    if !viewport.is_positive() || px_per_ms <= 0.0 {
        return None;
    }

    let origin_x = timeline_time_to_screen_x(content_left, 0.0, px_per_ms, scroll_offset);
    let start_ms = ((viewport.min.x - origin_x) / px_per_ms) as f64;
    let end_ms = ((viewport.max.x - origin_x) / px_per_ms) as f64;
    Some((start_ms, end_ms))
}

/// Returns the absolute screen-space edge where time zero starts for every
/// fixed piano-roll lane. Bottom panels must use the ruler's origin instead of
/// their own allocated rectangle because egui layouts may insert item spacing
/// between their gutter and canvas.
pub(crate) fn timeline_content_left(ruler_left: f32, keyboard_width: f32) -> f32 {
    ruler_left + keyboard_width
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParameterTab {
    Timbre,
    VoicebankPitch,
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
    EnvelopeFadeIn,
    EnvelopeFadeOut,
    EnvelopeCrossfade,
    VibratoLength,
    VibratoDepth,
    VibratoPeriod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextMenuCategory {
    Selection,
    QuantizeGrade,
    LyricsPhonemes,
    Phonemizer,
    MusicalTransform,
    Expression,
    Transpose,
    AutoPitch,
    Vibrato,
}

pub struct PianoRollState {
    pub timbre_drag: Option<Pos2>,
    pub timbre_changed: bool,
    pub selected_note_index: Option<usize>,
    pub selected_note_indices: HashSet<usize>,
    pub active_scale: MusicalScale,
    pub scale_root_key: u8,
    pub show_minimap: bool,
    pub minimap_height: f32,
    pub active_sounding_keys: HashSet<u8>,
    pub editing_lyric_index: Option<usize>,
    pub lyric_buffer: String,
    pub editing_phoneme_index: Option<(usize, usize)>,
    pub phoneme_buffer: String,
    pub phoneme_needs_select_all: bool,
    pub selected_copaiba_alias: Option<(String, String)>,
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
    pub note_original_states: Vec<(usize, f64, f64, u8)>,
    pub marquee_start: Option<Pos2>,
    pub marquee_current: Option<Pos2>,
    pub creating_note_idx: Option<usize>,
    pub active_tool: EditTool,
    pub pitch_sub_tool: PitchSubTool,
    pub pitch_line_start: Option<(f64, f64)>, // (abs_time_ms, abs_midi)
    /// Anchor captured when a vibrato brush stroke starts. The vertical
    /// position chooses the centre pitch, never an accidental depth.
    pub vibrato_brush_center: Option<f64>, // abs_midi
    pub auto_scroll_mode: AutoScrollMode,
    pub default_note_duration_ms: f64,
    pub default_note_lyric: String,
    pub default_note_dynamics: f64,
    pub default_note_volume: f64,
    pub default_note_attack: f64,
    pub default_note_decay: f64,
    pub ui_animations_enabled: bool,
    pub ui_animation_speed: f32,
    pub ui_reduce_motion: bool,
    pub animation_quality_reduced: bool,
    pub vertical_pitch_follow: bool,
    pub pitch_follow_smoothed_midi: Option<f32>,
    pub is_scrubbing_ruler: bool,
    pub loop_enabled: bool,
    pub loop_start_ms: f64,
    pub loop_end_ms: f64,
    pub show_arrangement_view: bool,
    pub arrangement_height: f32,
    pub is_maximized: bool,
    pub show_parameters_drawer: bool,
    pub drawer_height: f32,
    pub parameter_sidebar_width: f32,
    pub parameter_sidebar_drag_start: Option<()>,
    pub show_phoneme_ruler: bool,
    pub show_inspector: bool,
    pub selected_parameter: ParameterTab,
    pub request_undo: bool,
    pub request_redo: bool,
    pub request_cut: bool,
    pub request_copy: bool,
    pub request_paste: bool,
    pub request_batch_lyrics: bool,
    pub request_quantize_snap: bool,
    pub request_quantize_durations: bool,
    pub request_fix_overlaps: bool,
    pub request_legato: bool,
    pub request_clean_pitch_suffixes: bool,
    pub request_rephonemize: bool,
    pub request_humanize: bool,
    pub request_invert_retrograde: bool,
    pub request_invert_intervals: bool,
    pub request_select_overlapping: bool,
    pub request_select_out_of_scale: bool,
    pub request_autopitch_all: bool,
    pub request_autopitch_window: bool,
    pub request_export_selection_audio: bool,
    pub request_clear_selected_render_cache: bool,
    pub horizontal_scroll_offset: f32,
    pub vertical_scroll_offset: f32,
    /// Arrangement is a separate viewport: playback follow in the piano roll
    /// must never seize its timeline while the user is editing tracks.
    pub arrangement_horizontal_scroll_offset: f32,
    pub arrangement_vertical_scroll_offset: f32,
    pub arrangement_drag_track: Option<usize>,
    pub arrangement_track_meters: Vec<ArrangementMeterState>,
    /// Horizontal offset captured when a minimap viewport drag begins.
    pub minimap_drag_scroll_origin: Option<f32>,
    pub minimap_navigation_active: bool,
    pub horizontal_follow_user_override: bool,
    pub vertical_follow_user_override: bool,
    pub initial_scrolled: bool,
    pub properties_window_for_note: Option<usize>,
    pub context_menu_note_idx: Option<usize>,
    pub context_menu_pos: Option<Pos2>,
    pub context_menu_hovered_category: Option<ContextMenuCategory>,
    /// Screen-space rectangle of the category currently anchoring the flyout.
    pub context_menu_category_anchor: Option<Rect>,
    /// Screen-space hit area of the open flyout, used to keep it alive while
    /// the pointer travels from the category to its submenu.
    pub context_menu_flyout_rect: Option<Rect>,
    pub dragging_envelope_pt: Option<(usize, usize)>, // (note_idx, pt_idx)
    /// Per-phoneme P1-P5 drag: (note_idx, phoneme_idx, point_idx).
    pub dragging_phoneme_envelope_pt: Option<(usize, usize, usize)>,
    pub dragging_pitch_pt: Option<(usize, usize)>, // (note_idx, pt_idx)
    pub continuous_edit_dirty: bool,
    pub lyric_needs_select_all: bool,
    /// High-resolution audio peak envelope: (time_ms, min_amplitude, max_amplitude) in range [-1.0, 1.0]
    pub rendered_waveform_peaks: Vec<(f32, f32, f32)>,
    pub show_envelope_handles: bool,
    pub vibrato_popover_note_idx: Option<usize>,
    pub phoneme_cache: Vec<String>,
    pub note_phonemes_cache: Vec<Vec<CachedPhoneme>>,
    pub oto_consonant_cache: Vec<f64>,
    pub oto_preutter_cache: Vec<f64>,
    pub oto_overlap_cache: Vec<f64>,
    pub phoneme_cache_hash: u64,
    pub pitch_brush_raw_stroke: Vec<(f64, f64)>, // (abs_time_ms, abs_midi)
    /// Blend applied by the interactive pitch smoother (0 = none, 1 = full).
    pub pitch_smooth_intensity: f32,
    pub dragging_phoneme_handle: Option<(usize, u8, f32, f64)>,
    pub dragging_subphoneme_boundary: Option<(usize, usize, f32, f64)>,
    pub right_click_reset_active: bool,
    pub shift_locked_drawer_norm: Option<f64>,
    pub is_middle_panning: bool,
    pub show_waveform_area: bool,
    pub is_edge_autoscrolling: bool,
    pub is_dragging_in_drawer: bool,
    pub auto_cut: bool,
}

impl PianoRollState {
    pub fn update_arrangement_meter(&mut self, track_count: usize, track_index: usize, level: f32) {
        self.arrangement_track_meters
            .resize(track_count, ArrangementMeterState::default());
        if let Some(meter) = self.arrangement_track_meters.get_mut(track_index) {
            let level = level.clamp(0.0, 1.0);
            meter.level = meter.level * 0.64 + level * 0.36;
            meter.peak = (meter.peak - 0.012).max(meter.level).clamp(0.0, 1.0);
        }
    }

    pub fn decay_arrangement_meters(&mut self, track_count: usize) {
        self.arrangement_track_meters
            .resize(track_count, ArrangementMeterState::default());
        for meter in &mut self.arrangement_track_meters {
            meter.level = (meter.level - 0.06).max(0.0);
            meter.peak = (meter.peak - 0.018).max(meter.level).max(0.0);
        }
    }
}

impl Default for PianoRollState {
    fn default() -> Self {
        Self {
            timbre_drag: None,
            timbre_changed: false,
            selected_note_index: None,
            selected_note_indices: HashSet::new(),
            active_scale: MusicalScale::Chromatic,
            scale_root_key: 0,
            show_minimap: true,
            minimap_height: 22.0,
            active_sounding_keys: HashSet::new(),
            editing_lyric_index: None,
            lyric_buffer: String::new(),
            lyric_needs_select_all: false,
            editing_phoneme_index: None,
            phoneme_buffer: String::new(),
            phoneme_needs_select_all: false,
            selected_copaiba_alias: None,
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
            vibrato_brush_center: None,
            auto_scroll_mode: AutoScrollMode::PageScroll,
            default_note_duration_ms: 480.0,
            default_note_lyric: "ka".to_string(),
            default_note_dynamics: 0.0,
            default_note_volume: 100.0,
            default_note_attack: 100.0,
            default_note_decay: 0.0,
            ui_animations_enabled: true,
            ui_animation_speed: 1.0,
            ui_reduce_motion: false,
            animation_quality_reduced: false,
            vertical_pitch_follow: false,
            pitch_follow_smoothed_midi: None,
            is_scrubbing_ruler: false,
            loop_enabled: false,
            loop_start_ms: 0.0,
            loop_end_ms: 8000.0,
            show_arrangement_view: false,
            arrangement_height: 95.0,
            is_maximized: false,
            show_parameters_drawer: false,
            drawer_height: 120.0,
            parameter_sidebar_width: 178.0,
            parameter_sidebar_drag_start: None,
            show_phoneme_ruler: false,
            show_inspector: false,
            selected_parameter: ParameterTab::Dynamics,
            request_undo: false,
            request_redo: false,
            request_cut: false,
            request_copy: false,
            request_paste: false,
            request_batch_lyrics: false,
            request_quantize_snap: false,
            request_quantize_durations: false,
            request_fix_overlaps: false,
            request_legato: false,
            request_clean_pitch_suffixes: false,
            request_rephonemize: false,
            request_humanize: false,
            request_invert_retrograde: false,
            request_invert_intervals: false,
            request_select_overlapping: false,
            request_select_out_of_scale: false,
            request_autopitch_all: false,
            request_autopitch_window: false,
            request_export_selection_audio: false,
            request_clear_selected_render_cache: false,
            horizontal_scroll_offset: 0.0,
            vertical_scroll_offset: 0.0,
            arrangement_horizontal_scroll_offset: 0.0,
            arrangement_vertical_scroll_offset: 0.0,
            arrangement_drag_track: None,
            arrangement_track_meters: Vec::new(),
            minimap_drag_scroll_origin: None,
            minimap_navigation_active: false,
            horizontal_follow_user_override: false,
            vertical_follow_user_override: false,
            initial_scrolled: false,
            properties_window_for_note: None,
            context_menu_note_idx: None,
            context_menu_pos: None,
            context_menu_hovered_category: None,
            context_menu_category_anchor: None,
            context_menu_flyout_rect: None,
            dragging_envelope_pt: None,
            dragging_pitch_pt: None,
            continuous_edit_dirty: false,
            rendered_waveform_peaks: Vec::new(),
            show_envelope_handles: false,
            vibrato_popover_note_idx: None,
            phoneme_cache: Vec::new(),
            note_phonemes_cache: Vec::new(),
            oto_consonant_cache: Vec::new(),
            oto_preutter_cache: Vec::new(),
            oto_overlap_cache: Vec::new(),
            phoneme_cache_hash: 0,
            pitch_brush_raw_stroke: Vec::new(),
            dragging_phoneme_envelope_pt: None,
            pitch_smooth_intensity: 0.30,
            dragging_phoneme_handle: None,
            dragging_subphoneme_boundary: None,
            right_click_reset_active: false,
            shift_locked_drawer_norm: None,
            is_middle_panning: false,
            show_waveform_area: true,
            is_edge_autoscrolling: false,
            is_dragging_in_drawer: false,
            auto_cut: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::model::UPitchBendPoint;

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

        assert_eq!(state.waveform_amplitude_at(50.0), Some(0.5));
        assert_eq!(state.waveform_amplitude_at(150.0), Some(0.25));
        assert_eq!(state.waveform_amplitude_at(250.0), None);
    }

    #[test]
    fn timeline_screen_mapping_is_shared_by_scrolled_lanes() {
        let left = 120.0;
        let px_per_ms = 0.25;
        let scroll = 500.0;

        assert_eq!(
            timeline_time_to_screen_x(left, 2_000.0, px_per_ms, scroll),
            120.0
        );
        assert_eq!(
            timeline_time_to_screen_x(left, 2_400.0, px_per_ms, scroll),
            220.0
        );
    }

    #[test]
    fn bottom_lanes_use_the_ruler_origin_instead_of_their_layout_rect() {
        let ruler_left = 14.0;
        let keyboard_width = 62.0;
        let lower_layout_left = 84.0; // Includes an egui item-spacing offset.
        let timeline_left = timeline_content_left(ruler_left, keyboard_width);

        assert_eq!(timeline_left, 76.0);
        assert_ne!(timeline_left, lower_layout_left);
        assert_eq!(
            timeline_time_to_screen_x(timeline_left, 2_000.0, 0.25, 125.0),
            451.0
        );
    }

    #[test]
    fn visible_time_range_uses_the_same_scroll_for_footer_and_notes() {
        let viewport = Rect::from_min_max(Pos2::new(76.0, 0.0), Pos2::new(276.0, 40.0));
        for px_per_ms in [0.05, 0.25, 1.0] {
            let range = timeline_visible_time_range(viewport, 76.0, px_per_ms, 125.0).unwrap();
            let expected_start = 125.0 / px_per_ms as f64;
            let expected_end = (125.0 + 200.0) / px_per_ms as f64;

            assert!((range.0 - expected_start).abs() < 0.01);
            assert!((range.1 - expected_end).abs() < 0.01);
            assert!(
                (timeline_time_to_screen_x(76.0, range.0, px_per_ms, 125.0) - viewport.min.x).abs()
                    < 0.01
            );
            assert!(
                (timeline_time_to_screen_x(76.0, range.1, px_per_ms, 125.0) - viewport.max.x).abs()
                    < 0.01
            );
        }
    }

    #[test]
    fn visible_time_range_rejects_empty_viewports_and_invalid_zoom() {
        let empty = Rect::from_min_max(Pos2::new(10.0, 10.0), Pos2::new(10.0, 20.0));
        assert_eq!(timeline_visible_time_range(empty, 0.0, 0.25, 0.0), None);

        let viewport = Rect::from_min_max(Pos2::new(10.0, 0.0), Pos2::new(20.0, 10.0));
        assert_eq!(timeline_visible_time_range(viewport, 0.0, 0.0, 0.0), None);
    }

    #[test]
    fn progressive_chunks_share_absolute_buckets_at_boundaries() {
        let mut state = PianoRollState::default();
        // At 1 kHz the default 2 ms bucket contains both chunks.  The second
        // chunk must merge into bucket 0 instead of creating a second point.
        state.merge_rendered_waveform(&[0.2, 0.2], 1_000, 1, 0.0);
        state.merge_rendered_waveform(&[-0.8], 1_000, 1, 1.0);

        assert_eq!(state.rendered_waveform_peaks.len(), 1);
        assert_eq!(state.rendered_waveform_peaks[0].0, 0.0);
        assert_eq!(state.rendered_waveform_peaks[0].1, -0.8);
        assert_eq!(state.rendered_waveform_peaks[0].2, 0.2);
    }

    #[test]
    fn waveform_queries_follow_absolute_time_when_scrolling_both_directions() {
        let mut state = PianoRollState::default();
        state.merge_rendered_waveform(&vec![0.4; 1_000], 1_000, 1, 0.0);
        state.merge_rendered_waveform(&vec![0.8; 1_000], 1_000, 1, 1_000.0);
        let left = 76.0;
        let zoom = 0.25;
        for scroll in [0.0, 125.0, 250.0, 125.0, 0.0] {
            let timestamp = 1_200.0;
            let x = timeline_time_to_screen_x(left, timestamp, zoom, scroll);
            let recovered = ((x - left + scroll) / zoom) as f32;
            assert!((recovered - timestamp as f32).abs() < 0.01);
            assert_eq!(
                state.waveform_min_max_between(recovered, recovered + 1.0),
                Some((0.8, 0.8))
            );
        }
    }

    #[test]
    fn waveform_does_not_draw_buckets_across_silent_or_unrendered_gaps() {
        let mut state = PianoRollState::default();
        state.merge_rendered_waveform(&[0.6; 4], 1_000, 1, 0.0);
        state.merge_rendered_waveform(&[0.9; 4], 1_000, 1, 100.0);

        assert_eq!(state.waveform_min_max_between(0.0, 4.0), Some((0.6, 0.6)));
        assert_eq!(state.waveform_min_max_between(40.0, 60.0), None);
        assert_eq!(state.waveform_min_max_between(0.0, 104.0), None);
        assert_eq!(
            state.waveform_min_max_between(100.0, 104.0),
            Some((0.9, 0.9))
        );
    }

    #[test]
    fn duplicate_progressive_chunk_does_not_duplicate_buckets() {
        let mut state = PianoRollState::default();
        state.merge_rendered_waveform(&[0.5; 4], 1_000, 1, 0.0);
        let first = state.rendered_waveform_peaks.clone();
        state.merge_rendered_waveform(&[0.5; 4], 1_000, 1, 0.0);

        assert_eq!(state.rendered_waveform_peaks, first);
    }

    #[test]
    fn waveform_rejects_zero_channel_chunks_instead_of_treating_them_as_mono() {
        let mut state = PianoRollState::default();
        state.merge_rendered_waveform(&[0.8, -0.8], 1_000, 0, 0.0);
        assert!(state.rendered_waveform_peaks.is_empty());
    }

    #[test]
    fn pitch_brush_normalizes_reverse_strokes_without_an_endpoint_jump() {
        let points =
            smooth_pitch_points(&[(120.0, 0.0), (80.0, 300.0), (40.0, 0.0), (40.0, 900.0)]);

        assert!(!points.is_empty());
        assert!(points
            .windows(2)
            .all(|pair| { pair[0].time_offset_ms < pair[1].time_offset_ms }));
        assert!(points
            .iter()
            .all(|point| point.pitch_offset_cents.abs() <= 2400.0));
        assert!(points
            .last()
            .is_none_or(|point| (point.time_offset_ms - 120.0).abs() < 0.001));
    }

    #[test]
    fn openutau_pitch_point_insertion_keeps_the_authored_point_and_order() {
        let mut note = UNote::new("a", "C4", 0.0, 480.0);
        note.pitch_bend.points = vec![
            UPitchBendPoint {
                time_offset_ms: 0.0,
                pitch_offset_cents: 0.0,
                shape: "io".to_string(),
            },
            UPitchBendPoint {
                time_offset_ms: 300.0,
                pitch_offset_cents: 0.0,
                shape: "io".to_string(),
            },
        ];

        let index = insert_pitch_point(&mut note, 120.0, 275.0);

        assert_eq!(index, 1);
        assert_eq!(note.pitch_bend.points[index].time_offset_ms, 120.0);
        assert_eq!(note.pitch_bend.points[index].pitch_offset_cents, 275.0);
        assert!(note
            .pitch_bend
            .points
            .windows(2)
            .all(|pair| pair[0].time_offset_ms <= pair[1].time_offset_ms));
    }

    #[test]
    fn openutau_pitch_shapes_are_normalized_without_changing_the_file_model() {
        assert_eq!(canonical_pitch_shape("s"), "io");
        assert_eq!(canonical_pitch_shape("linear"), "l");
        assert_eq!(canonical_pitch_shape("ease-in"), "i");
        assert_eq!(canonical_pitch_shape("r"), "o");
        assert_eq!(canonical_pitch_shape("hermite"), "sp");
    }

    #[test]
    fn test_parameter_tab_reset_defaults() {
        let mut note = crate::project::model::UNote::new("a", "C4", 0.0, 480.0);
        note.expressions.dynamics = 45.0;
        note.expressions.gender = -30.0;
        note.expressions.volume = 150.0;
        note.vibrato.length_pct = 75.0;

        // Reset Dynamics
        note.expressions.dynamics = 0.0;
        assert_eq!(note.expressions.dynamics, 0.0);

        // Reset Gender
        note.expressions.gender = 0.0;
        assert_eq!(note.expressions.gender, 0.0);

        // Reset Volume
        note.expressions.volume = 100.0;
        assert_eq!(note.expressions.volume, 100.0);

        // Reset Vibrato
        note.vibrato.length_pct = 0.0;
        assert_eq!(note.vibrato.length_pct, 0.0);
    }
}
