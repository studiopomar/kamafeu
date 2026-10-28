#pragma once
#include "tainah.h"
#include <algorithm>
#include <cmath>

namespace tainah {
// Timing behavior implemented by Tainah, informed by Lessampler's CheckPara
// and Lessampler-NG's AudioProcess::TimeStretch (see NOTICE.md).
inline double source_time(const lt_render_options *o, double time, double source_duration) {
    const double marked_begin = o->loop_start_ms - o->offset_ms;
    const double marked_end = o->loop_end_ms - o->offset_ms;
    const double tail = o->tail_start_ms - o->offset_ms;
    const bool valid_loop = std::isfinite(marked_begin) && std::isfinite(marked_end) &&
        marked_begin >= 0 && marked_end > marked_begin && marked_end <= source_duration;
    // oto.ini consonant is a length after offset, not an absolute WAV position.
    // Velocity and flag C change output time, never the source boundary.
    const double begin = valid_loop ? marked_begin : std::clamp(o->consonant_ms, 0.0, source_duration);
    const double end = valid_loop ? marked_end :
        (std::isfinite(tail) && tail > begin && tail <= source_duration ? tail : source_duration);
    const bool has_target = o->struct_size >=
        offsetof(lt_render_options, has_target_consonant) + sizeof(o->has_target_consonant) &&
        o->has_target_consonant != 0;
    const bool has_adjustment = o->struct_size >=
        offsetof(lt_render_options, consonant_adjustment) + sizeof(o->consonant_adjustment);
    double attack = has_target ? o->target_consonant_ms : begin;
    if (valid_loop && has_target) {
        attack = o->consonant_ms > 0 ? begin * attack / o->consonant_ms : begin;
    }
    attack *= has_adjustment ? 1.0 + o->consonant_adjustment / 100.0 : 1.0;
    if (time < attack) return std::clamp(time * begin / attack, 0.0, source_duration);
    const double tail_begin = std::isfinite(tail) && tail >= end && tail <= source_duration ? tail : source_duration;
    const double release = source_duration - tail_begin;
    // Preserve explicitly marked release audio where it fits, without shortening
    // the fixed attack to accommodate a short note.
    const double target_tail = std::max(attack, o->duration_ms - release);
    if (release > 0 && time >= target_tail && target_tail < o->duration_ms) {
        return std::clamp(tail_begin + time - target_tail, 0.0, source_duration);
    }
    // Short vowels use the available source region without accelerating later
    // recording and its transitions.
    const double rate = std::min(1.0, std::max(0.0, end - begin) /
        std::max(target_tail - attack, 1e-9));
    return std::clamp(begin + (time - attack) * rate, 0.0, source_duration);
}
}
