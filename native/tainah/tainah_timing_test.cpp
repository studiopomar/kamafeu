#ifdef NDEBUG
#undef NDEBUG
#endif
#include "tainah_timing.h"
#include <cassert>
#include <limits>

static void near(double actual, double expected) {
    assert(std::abs(actual - expected) < 1e-7);
}
int main() {
    lt_render_options o{};
    o.struct_size = sizeof(o);
    o.offset_ms = 700; // source coordinates have already had offset removed
    o.consonant_ms = 100;
    o.loop_start_ms = o.loop_end_ms = o.tail_start_ms = std::nan("");
    o.has_target_consonant = 1;
    for (double velocity : {50., 100., 150., 200.}) {
        const double speed = std::pow(2., velocity / 100. - 1.);
        o.target_consonant_ms = 100 / speed;
        for (double duration : {40., 150., 500., 1500.}) {
            o.duration_ms = duration;
            // Reference behavior: Lessampler CheckPara and Lessampler-NG
            // AudioProcess::TimeStretch; attribution and links are in NOTICE.md.
            const double base = 100 / speed;
            const double rate = std::min(1., 400 / std::max(duration - base, 1e-9));
            for (double t = 0; t <= duration; t += 0.5) {
                const double expected = t < base ? t * speed : 100 + (t - base) * rate;
                near(tainah::source_time(&o, t, 500), expected);
            }
            if (base <= duration) near(tainah::source_time(&o, base, 500), 100);
        }
    }
    o.duration_ms = 150; o.target_consonant_ms = 100;
    near(tainah::source_time(&o, 150, 500), 150); // do not pull later transitions forward
    o.consonant_adjustment = 100;
    near(tainah::source_time(&o, 100, 500), 50); // C changes time, not source boundary
    o.consonant_adjustment = -100;
    near(tainah::source_time(&o, 0, 500), 100);
    o.consonant_adjustment = 0; o.target_consonant_ms = 0;
    near(tainah::source_time(&o, 0, 500), 100); // explicit zero attack
    o.has_target_consonant = 0;
    near(tainah::source_time(&o, 50, 500), 50); // zero-initialized extension
    o.struct_size = offsetof(lt_render_options, formant_shift);
    near(tainah::source_time(&o, 50, 500), 50); // old ABI 2 caller
    o.struct_size = sizeof(o); o.has_target_consonant = 1; o.target_consonant_ms = 100;
    o.duration_ms = 600; o.loop_start_ms = 800; o.loop_end_ms = 1000; o.tail_start_ms = 1150;
    near(tainah::source_time(&o, 100, 500), 100);
    near(tainah::source_time(&o, 550, 500), 450);
    near(tainah::source_time(&o, 600, 500), 500);
    o.target_consonant_ms = 50;
    near(tainah::source_time(&o, 50, 500), 100);
    // An exhausted fixed region must not rescale the entire source recording.
    o.loop_start_ms = o.loop_end_ms = o.tail_start_ms = std::nan("");
    o.consonant_ms = 500; o.target_consonant_ms = 500; o.duration_ms = 50;
    near(tainah::source_time(&o, 25, 500), 25);
}
