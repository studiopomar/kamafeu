#include "tainah.h"
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <cassert>
#include <cstdio>
#include <cmath>
#include <cstdint>
#include <algorithm>
#include <vector>
#include <thread>
#include <filesystem>
#include <fstream>
#include <cstdlib>
static int32_t cancel(void *context) { return *static_cast<bool *>(context) ? 1 : 0; }
int main() {
    const auto cache_dir = std::filesystem::temp_directory_path() / "tainah-rt-abi-cache";
    std::filesystem::remove_all(cache_dir);
#if defined(_WIN32)
    _putenv_s("TAINAH_ANALYSIS_CACHE_DIR", cache_dir.string().c_str());
#else
    setenv("TAINAH_ANALYSIS_CACHE_DIR", cache_dir.string().c_str(), 1);
#endif
    assert(lt_abi_version() == LT_ABI_VERSION);
    lt_pcm output{}; output.struct_size = sizeof(output);
    lt_error error{}; error.struct_size = sizeof(error);
    assert(lt_render(nullptr, &output, &error) == 1 && error.code == 1);
    std::vector<float> sample(22050);
    lt_render_options options{};
    options.struct_size = sizeof(options); options.sample_rate = 44100;
    options.input = sample.data(); options.input_length = sample.size();
    options.duration_ms = 1500; options.pitch_hz = 220;
    options.frame_period_ms = 5; options.f0_floor = 71; options.f0_ceiling = 1760;
    options.f0_method = 1;
    assert(lt_render(&options, &output, &error) == 5 && error.code == 5);
    bool stop = true; options.cancel = cancel; options.cancel_user_data = &stop;
    sample[1] = 0.5f;
    assert(lt_render(&options, &output, &error) == 2 && error.code == 2);
    stop = false;
    std::filesystem::remove_all(cache_dir);
    for (size_t i = 0; i < sample.size(); ++i) sample[i] = 0.3f * std::sin(2 * 3.141592653589793 * 220 * i / 44100);
    assert(lt_render(&options, &output, &error) == 0);
    assert(output.length == 66150 && output.sample_rate == 44100);
    double peak = 0; for (size_t i = 0; i < output.length; ++i) peak = std::max(peak, double(std::abs(output.samples[i])));
    assert(peak > 1e-5);
    assert(std::distance(std::filesystem::directory_iterator(cache_dir), std::filesystem::directory_iterator{}) == 1);
    double tail_energy = 0;
    for (size_t i = output.length - 4410; i < output.length; ++i) tail_energy += output.samples[i] * output.samples[i];
    assert(tail_energy > 1e-6); // the extra second must have WORLD coverage
    std::vector<float> baseline(output.samples, output.samples + output.length);
    lt_pcm_free(&output);
    lt_cache_clear_memory(); // Read the serialized analysis on the next call.
    assert(lt_render(&options, &output, &error) == 0);
    assert(output.length == baseline.size());
    for (size_t i = 0; i < output.length; ++i) assert(output.samples[i] == baseline[i]);
    lt_pcm_free(&output);
    const auto cache_file = std::filesystem::directory_iterator(cache_dir)->path();
    { std::ofstream corrupt(cache_file, std::ios::binary | std::ios::trunc); corrupt << "bad"; }
    lt_cache_clear_memory();
    assert(lt_render(&options, &output, &error) == 0);
    for (size_t i = 0; i < output.length; ++i) assert(output.samples[i] == baseline[i]);
    lt_pcm_free(&output);
    std::vector<std::thread> workers;
    for (int worker = 0; worker < 4; ++worker) workers.emplace_back([&] {
        lt_pcm concurrent{}; concurrent.struct_size = sizeof(concurrent);
        lt_error detail{}; detail.struct_size = sizeof(detail);
        assert(lt_render(&options, &concurrent, &detail) == 0);
        assert(concurrent.length == baseline.size());
        for (size_t i = 0; i < concurrent.length; ++i) assert(concurrent.samples[i] == baseline[i]);
        lt_pcm_free(&concurrent);
    });
    for (auto &worker : workers) worker.join();
    auto differs = [&](lt_render_options changed) {
        lt_pcm variant{}; variant.struct_size = sizeof(variant);
        lt_error detail{}; detail.struct_size = sizeof(detail);
        assert(lt_render(&changed, &variant, &detail) == 0);
        assert(variant.length == baseline.size());
        double sum = 0;
        for (size_t i = 0; i < variant.length; ++i) sum += std::abs(variant.samples[i] - baseline[i]);
        lt_pcm_free(&variant);
        assert(sum > 0.01);
    };
    auto variant = options;
    variant.gender = 50; differs(variant);
    variant = options; variant.pitch_cents = 100; differs(variant);
    variant = options; variant.breathiness = 50; differs(variant);
    variant = options; variant.high_frequency_cutoff = 3000; differs(variant);
    variant = options; variant.loop_start_ms = 70; variant.loop_end_ms = 350; variant.tail_start_ms = 450; differs(variant);
    // Vowel formant warp: neutral ratios are bit-identical, moved ratios differ,
    // out-of-range ratios are rejected.
    variant = options; variant.has_vowel_formant = 1; variant.f1_ratio = 1.0; variant.f2_ratio = 1.0;
    assert(lt_render(&variant, &output, &error) == 0);
    assert(output.length == baseline.size());
    for (size_t i = 0; i < output.length; ++i) assert(output.samples[i] == baseline[i]);
    lt_pcm_free(&output);
    variant.f1_ratio = 0.7; variant.f2_ratio = 1.4; differs(variant);
    variant = options; variant.has_vowel_formant = 1; variant.f1_ratio = 1.0; variant.f2_ratio = 1.0;
    variant.f3_ratio = 0.68; differs(variant);
    variant = options; variant.has_vowel_formant = 1; variant.f1_ratio = 1.0; variant.f2_ratio = 1.0;
    variant.nasality = 0.85; differs(variant);
    variant = options; variant.has_vowel_formant = 1; variant.f1_ratio = 1.0; variant.f2_ratio = 1.0;
    variant.singers_formant = 0.75; differs(variant);
    variant.f1_ratio = 3.0;
    assert(lt_render(&variant, &output, &error) == 1 && error.code == 1);
    lt_formant_analysis analysis{}; analysis.struct_size = sizeof(analysis);
    assert(lt_analyze_formants(&options, &analysis, &error) == 0);
    assert(analysis.f1_hz > 0 && analysis.f1_hz < analysis.f2_hz && analysis.f2_hz < analysis.f3_hz);
    assert(analysis.envelope_len == LT_FORMANT_ENVELOPE_POINTS && analysis.envelope_max_hz > 0);
    variant = options; variant.f0_method = 2;
    const int dio_status = lt_render(&variant, &output, &error);
    // This pure sine is a pathological DIO fixture for WORLD synthesis. The ABI
    // must surface the non-finite output as a diagnostic instead of returning silence.
    assert(dio_status == 4 && error.code == 4);
    assert(std::string(error.message).find("non-finite") != std::string::npos);
    lt_pcm_free(&output);
    sample[100] += 0.001f;
    assert(lt_render(&options, &output, &error) == 0);
    lt_pcm_free(&output);
    assert(std::distance(std::filesystem::directory_iterator(cache_dir), std::filesystem::directory_iterator{}) == 3);
    std::vector<float> harmonic_sample(22050, 0.0f);
    for (size_t i = 0; i < harmonic_sample.size(); ++i) {
        const double t = double(i) / 44100.0;
        double s = 0.0;
        for (int k = 1; k <= 12; ++k) {
            s += (0.25 / k) * std::sin(2 * 3.141592653589793 * 220.0 * k * t + k * 0.37);
        }
        harmonic_sample[i] = static_cast<float>(s);
    }
    auto harmonic_opts = options;
    harmonic_opts.input = harmonic_sample.data();
    harmonic_opts.input_length = harmonic_sample.size();
    assert(lt_analyze_formants(&harmonic_opts, &analysis, &error) == 0);
    assert(analysis.f0_hz > 200 && analysis.f0_hz < 240);
    lt_pitch_point curve[] = {{0, 0}, {1500, 700}};
    options.pitch_points = curve; options.pitch_point_count = 2;
    options.gender = 50; options.breathiness = 30; options.high_frequency_cutoff = 4000;
    options.loop_start_ms = 70; options.loop_end_ms = 350; options.tail_start_ms = 450;
    assert(lt_render(&options, &output, &error) == 0);
    assert(output.length == baseline.size());
    double difference = 0;
    for (size_t i = 0; i < output.length; ++i) difference += std::abs(output.samples[i] - baseline[i]);
    assert(difference > 1.0);
    lt_pcm_free(&output);
    options.cutoff_ms = -5; options.offset_ms = 1000;
    assert(lt_render(&options, &output, &error) == 3 && error.code == 3);
    std::filesystem::remove_all(cache_dir);
}
