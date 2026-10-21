use super::phone_result::PhoneResult;
use super::TrackRenderer;
use crate::drivers::{WavtoolDriver, WavtoolMixStrategy};
use crate::renderer::timing::{PhonemeTiming, PhonemeTimingInput};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) struct TrackMergeContext<'a> {
    pub(super) track_buffer: Vec<f32>,
    pub(super) phone_results: Result<Vec<PhoneResult>, String>,
    pub(super) timing_inputs: &'a [PhonemeTimingInput],
    pub(super) timings: &'a [PhonemeTiming],
    pub(super) wavtool_driver: &'a dyn WavtoolDriver,
    pub(super) temp_dir_path: &'a Path,
    pub(super) sample_rate: u32,
    pub(super) cancel: Option<&'a AtomicBool>,
    pub(super) mix_strategy: WavtoolMixStrategy,
    pub(super) total_phones: usize,
    pub(super) log: &'a (dyn Fn(f32, &str) + Sync),
}

pub(super) fn merge_track_results(context: TrackMergeContext<'_>) -> Result<Vec<f32>, String> {
    let TrackMergeContext {
        mut track_buffer,
        phone_results,
        timing_inputs,
        timings,
        wavtool_driver,
        temp_dir_path,
        sample_rate,
        cancel,
        mix_strategy,
        total_phones,
        log,
    } = context;
    // ------------------------------------------------------------------
    // Phase 2: Sequential merge.
    //
    // Convergence is order-dependent: each correction contributes to the
    // phase used by the following phone, so this must run in idx order.
    // ------------------------------------------------------------------
    if cancel.is_some_and(|t| t.load(Ordering::Relaxed)) {
        log(1.0, "[Render] Cancelled");
        return Err("Renderização cancelada".to_string());
    }

    let mut phone_results = phone_results?;
    phone_results.sort_unstable_by_key(|r| r.idx);
    let mut previous_phone_end_sample = 0usize;

    if let Some(executable) = wavtool_driver.phrase_executable() {
        let mut begin = 0;
        while begin < phone_results.len() {
            let mut end = begin + 1;
            while end < phone_results.len() && phone_results[end].adjacent {
                end += 1;
            }
            let group = &phone_results[begin..end];
            let args: Vec<_> = group.iter().map(|r| r.wav_args.clone()).collect();
            let output = temp_dir_path.join(format!("phrase-{begin}.wav"));
            let rendered = crate::drivers::wavtool_driver::concatenate_external(
                &executable,
                &args,
                &output,
                sample_rate,
                cancel,
            )?;
            let origin =
                timing_inputs[group[0].idx].position_ms - timings[group[0].idx].preutter_ms;
            let skip = ((-origin).max(0.0) * sample_rate as f64 / 1000.0).round() as usize;
            let start = (origin.max(0.0) * sample_rate as f64 / 1000.0).round() as usize;
            TrackRenderer::mix_phase_aligned(
                &mut track_buffer,
                rendered.get(skip..).unwrap_or(&[]),
                start,
                0,
                0,
                0.0,
                sample_rate,
            );
            for item in group {
                for (p, message) in &item.logs {
                    log(*p, message);
                }
            }
            begin = end;
        }
        return Ok(track_buffer);
    }

    let mut previous_phase_tail = None;
    let mut previous_phase_correction = 0isize;
    let mut previous_lyric: Option<String> = None;
    for result in phone_results {
        // Replay per-phone log messages in-order now that we are sequential.
        for (progress, msg) in result.logs {
            log(progress, &msg);
        }

        let source_skip_samples =
            ((result.source_skip_ms / 1000.0) * sample_rate as f64).round() as usize;
        let audible_samples = result
            .note_rendered
            .get(source_skip_samples.min(result.note_rendered.len())..)
            .unwrap_or(&[]);
        let start_sample_idx =
            ((result.actual_start_ms / 1000.0) * sample_rate as f64).round() as usize;
        // The wavtool envelope is the authority for transition energy.
        // Do not apply RMS matching here: even a synthetic safety overlap
        // can otherwise raise a consonant by up to 1.5x and make it sound
        // like a separate, overly loud phoneme.
        match mix_strategy {
            WavtoolMixStrategy::OpenUtauConvergence => {
                if !result.adjacent {
                    previous_phase_correction = 0;
                }
                let correction = if result.adjacent {
                    crate::renderer::phase::openutau_convergence_correction(
                        previous_phase_tail,
                        result.phase_metadata.head,
                        previous_phase_correction,
                        sample_rate,
                    )
                } else {
                    0
                };
                let mixed = TrackRenderer::mix_with_resolved_correction(
                    &mut track_buffer,
                    audible_samples,
                    start_sample_idx,
                    previous_phone_end_sample,
                    correction,
                );
                previous_phone_end_sample = mixed.end_sample;
                let head = result.phase_metadata.head;
                let tail = previous_phase_tail;
                log(
                result.idx as f32 / total_phones as f32,
                &format!(
                    "  [Transition] strategy=OpenUtauConvergence from='{}' to='{}' adjacent={} tail_valid={} head_valid={} head_window={} tail_window={} correction_samples={} correction_ms={:.3} source_trim_samples={} envelope=linear-openutau",
                    previous_lyric.as_deref().unwrap_or("<none>"),
                    result.lyric,
                    result.adjacent,
                    tail.is_some(),
                    head.is_some(),
                    head.map_or_else(|| "none".to_string(), |signature| format!(
                        "{}+{}@{:.3}Hz",
                        signature.window_start, signature.window_len, signature.frequency
                    )),
                    tail.map_or_else(|| "none".to_string(), |signature| format!(
                        "{}+{}@{:.3}Hz",
                        signature.window_start, signature.window_len, signature.frequency
                    )),
                    mixed.correction_samples,
                    mixed.correction_samples as f64 * 1000.0 / sample_rate.max(1) as f64,
                    mixed.source_trim_samples,
                ),
            );
                previous_phase_correction = correction;
            }
            WavtoolMixStrategy::Wavtool4Vcv => {
                let head = result.phase_metadata.head;
                let tail = previous_phase_tail;
                let overlap_end = previous_phone_end_sample.min(track_buffer.len());
                let correction = if result.adjacent && head.is_some() && tail.is_some() {
                    let overlap_start = start_sample_idx.min(overlap_end);
                    let previous_overlap = &track_buffer[overlap_start..overlap_end];
                    crate::renderer::phase::wavtool4vcv_correction(
                        previous_overlap,
                        audible_samples,
                        sample_rate,
                        head.expect("checked above").frequency,
                    )
                } else {
                    0
                };
                let mixed = TrackRenderer::mix_with_resolved_correction(
                    &mut track_buffer,
                    audible_samples,
                    start_sample_idx,
                    previous_phone_end_sample,
                    correction,
                );
                previous_phone_end_sample = mixed.end_sample;
                log(
                    result.idx as f32 / total_phones as f32,
                    &format!(
                        "  [Transition] strategy=Wavtool4Vcv from='{}' to='{}' adjacent={} voiced_windows={} correction_samples={} correction_ms={:.3} source_trim_samples={} envelope=linear-openutau",
                        previous_lyric.as_deref().unwrap_or("<none>"),
                        result.lyric,
                        result.adjacent,
                        head.is_some() && tail.is_some(),
                        mixed.correction_samples,
                        mixed.correction_samples as f64 * 1000.0 / sample_rate.max(1) as f64,
                        mixed.source_trim_samples,
                    ),
                );
            }
            WavtoolMixStrategy::LegacyPhaseAligned => {
                let crossfade_samples =
                    ((result.crossfade_ms / 1000.0) * sample_rate as f64).round() as usize;
                previous_phone_end_sample = TrackRenderer::mix_phase_aligned(
                    &mut track_buffer,
                    audible_samples,
                    start_sample_idx,
                    previous_phone_end_sample,
                    crossfade_samples,
                    result.pitch_freq,
                    sample_rate,
                );
            }
            WavtoolMixStrategy::Plain => {
                previous_phone_end_sample = TrackRenderer::mix_with_resolved_correction(
                    &mut track_buffer,
                    audible_samples,
                    start_sample_idx,
                    previous_phone_end_sample,
                    0,
                )
                .end_sample;
            }
        }
        previous_phase_tail = result.phase_metadata.tail;
        previous_lyric = Some(result.lyric);
    }

    if let Some(target_peak) = crate::renderer::project::configured_render_limiter_peak() {
        let report =
            TrackRenderer::apply_render_limiter(&mut track_buffer, 1, sample_rate, target_peak);
        if report.limited_frames > 0 {
            log(
                0.95,
                &format!(
                    "[Render][Limiter] {} frames limited, max reduction {:.2} dB",
                    report.limited_frames,
                    report.max_reduction_db()
                ),
            );
        }
    }

    let diagnostics = crate::renderer::diagnostics::AudioDiagnostics::analyze(&track_buffer, 1);
    log(0.95, &diagnostics.render_log_line("Final track_buffer"));

    if let Some(last_nonzero) = track_buffer.iter().rposition(|&s| s.abs() > 1e-4) {
        track_buffer.truncate(last_nonzero + 1);
        log(
            1.0,
            &format!(
                "[Render] Truncated to {} non-silent samples",
                track_buffer.len()
            ),
        );
    } else {
        log(1.0, "[Render] WARNING: Entire track buffer is silent!");
    }

    let final_diagnostics =
        crate::renderer::diagnostics::AudioDiagnostics::analyze(&track_buffer, 1);
    log(
        1.0,
        &format!(
            "[Render][Complete] output_samples={} output_duration_ms={:.3} {}",
            track_buffer.len(),
            track_buffer.len() as f64 * 1000.0 / sample_rate.max(1) as f64,
            final_diagnostics.render_log_line("final output"),
        ),
    );

    Ok(track_buffer)
}
