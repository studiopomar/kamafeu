#include "tainah.h"
#include "AudioModel/AudioModel.h"
#include "AudioModel/Synthesis/Synthesis.h"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <exception>
#include <filesystem>
#include <fstream>
#include <memory>
#include <mutex>
#include <new>
#include <unordered_map>
#include <vector>
#include <thread>

namespace {
void fail(lt_error *e, int code, const char *message) {
    if (!e || e->struct_size < sizeof(lt_error)) return;
    e->code = code;
    std::snprintf(e->message, sizeof(e->message), "%s", message);
}
bool cancelled(const lt_render_options *o) { return o->cancel && o->cancel(o->cancel_user_data) != 0; }
uint64_t hash_bytes(uint64_t hash, const void *data, size_t length) {
    const auto *bytes = static_cast<const unsigned char *>(data);
    for (size_t i = 0; i < length; ++i) { hash ^= bytes[i]; hash *= 1099511628211ull; }
    return hash;
}
std::mutex cache_mutex;
// Upstream WORLD's randn state is process-global. CheapTrick, D4C and
// Synthesis reseed and consume it, so overlapping renders can change PCM.
std::mutex world_mutex;
std::unordered_map<uint64_t, std::shared_ptr<const lessAudioModel>> analysis_cache;
namespace fs = std::filesystem;
fs::path analysis_cache_dir() {
    if (const char *custom = std::getenv("TAINAH_ANALYSIS_CACHE_DIR")) return fs::path(custom);
#if defined(_WIN32)
    if (const char *local = std::getenv("LOCALAPPDATA")) return fs::path(local) / "Kamafeu" / "tainah-analysis";
#else
    if (const char *xdg = std::getenv("XDG_CACHE_HOME")) return fs::path(xdg) / "kamafeu" / "tainah-analysis";
    if (const char *home = std::getenv("HOME")) {
# if defined(__APPLE__)
        return fs::path(home) / "Library" / "Caches" / "Kamafeu" / "tainah-analysis";
# else
        return fs::path(home) / ".cache" / "kamafeu" / "tainah-analysis";
# endif
    }
#endif
    return {};
}
fs::path analysis_cache_file(uint64_t key) {
    char name[40];
    std::snprintf(name, sizeof(name), "v2-%016llx.bin", static_cast<unsigned long long>(key));
    return analysis_cache_dir() / name;
}
template<class T> bool read_value(std::ifstream &stream, T &value) {
    return bool(stream.read(reinterpret_cast<char *>(&value), sizeof(T)));
}
template<class T> void write_value(std::ofstream &stream, const T &value) {
    stream.write(reinterpret_cast<const char *>(&value), sizeof(T));
}
std::shared_ptr<const lessAudioModel> load_analysis(uint64_t key, uint32_t sample_rate, double frame_period) {
    try {
        auto directory = analysis_cache_dir();
        if (directory.empty()) return {};
        std::ifstream stream(analysis_cache_file(key), std::ios::binary);
        if (!stream) return {};
        uint64_t magic = 0, stored_key = 0, frames = 0;
        uint32_t rate = 0, fft = 0, bins = 0;
        double period = 0;
        if (!read_value(stream, magic) || !read_value(stream, stored_key) || !read_value(stream, rate) ||
            !read_value(stream, period) || !read_value(stream, fft) || !read_value(stream, bins) ||
            !read_value(stream, frames) || magic != 0x3257414e4941544cull || stored_key != key ||
            rate != sample_rate || period != frame_period || frames < 2 || frames > 100000 ||
            fft < 64 || fft > 16384 || bins != fft / 2 + 1 ||
            frames * uint64_t(bins) * sizeof(double) * 2 > 256ull * 1024 * 1024) return {};
        auto model = std::make_shared<lessAudioModel>();
        model->fs = static_cast<int>(rate); model->frame_period = period;
        model->fft_size = static_cast<int>(fft); model->w_length = static_cast<int>(bins);
        model->f0.resize(frames);
        if (!stream.read(reinterpret_cast<char *>(model->f0.data()), frames * sizeof(double))) return {};
        model->spectrogram.resize(frames, std::vector<double>(bins));
        model->aperiodicity.resize(frames, std::vector<double>(bins));
        for (auto &row : model->spectrogram)
            if (!stream.read(reinterpret_cast<char *>(row.data()), bins * sizeof(double))) return {};
        for (auto &row : model->aperiodicity)
            if (!stream.read(reinterpret_cast<char *>(row.data()), bins * sizeof(double))) return {};
        if (!std::all_of(model->f0.begin(), model->f0.end(), [](double x) { return std::isfinite(x) && x >= 0; })) return {};
        for (const auto &matrix : {&model->spectrogram, &model->aperiodicity})
            for (const auto &row : *matrix)
                if (!std::all_of(row.begin(), row.end(), [](double x) { return std::isfinite(x) && x >= 0; })) return {};
        return model;
    } catch (...) { return {}; }
}
void save_analysis(uint64_t key, const lessAudioModel &model) {
    try {
        const uint64_t frames = model.f0.size();
        const uint32_t bins = static_cast<uint32_t>(model.w_length);
        if (frames < 2 || frames > 100000 || bins == 0 ||
            frames * uint64_t(bins) * sizeof(double) * 2 > 256ull * 1024 * 1024) return;
        auto directory = analysis_cache_dir();
        if (directory.empty()) return;
        fs::create_directories(directory);
        const auto destination = analysis_cache_file(key);
        const auto temporary = destination.string() + "." + std::to_string(std::hash<std::thread::id>{}(std::this_thread::get_id())) + ".tmp";
        std::ofstream stream(temporary, std::ios::binary | std::ios::trunc);
        const uint64_t magic = 0x3257414e4941544cull;
        const uint32_t rate = static_cast<uint32_t>(model.fs), fft = static_cast<uint32_t>(model.fft_size);
        write_value(stream, magic); write_value(stream, key); write_value(stream, rate);
        write_value(stream, model.frame_period); write_value(stream, fft); write_value(stream, bins); write_value(stream, frames);
        stream.write(reinterpret_cast<const char *>(model.f0.data()), frames * sizeof(double));
        for (const auto &row : model.spectrogram) stream.write(reinterpret_cast<const char *>(row.data()), bins * sizeof(double));
        for (const auto &row : model.aperiodicity) stream.write(reinterpret_cast<const char *>(row.data()), bins * sizeof(double));
        stream.close();
        if (!stream) { fs::remove(temporary); return; }
        fs::remove(destination);
        fs::rename(temporary, destination);
    } catch (...) { /* Cache I/O never changes render semantics. */ }
}

double pitch_cents(const lt_render_options *o, double time) {
    if (!o->pitch_point_count) return 0.0;
    const auto *p = o->pitch_points;
    if (time <= p[0].time_ms) return p[0].cents;
    for (uint32_t i = 1; i < o->pitch_point_count; ++i) {
        if (time <= p[i].time_ms) {
            const double f = (time - p[i-1].time_ms) / (p[i].time_ms - p[i-1].time_ms);
            const double delta = p[i].cents - p[i-1].cents;
            double factor = f;
            switch (p[i-1].shape) {
            case 1: break;
            case 2: factor = 1.0 - std::cos(f * 1.5707963267948966); break;
            case 3: factor = std::sin(f * 1.5707963267948966); break;
            case 4: {
                const double dt = p[i].time_ms - p[i-1].time_ms;
                const double slope = delta / dt;
                double m1 = delta, m2 = delta;
                if (i >= 2) {
                    const double prev_dt = p[i-1].time_ms - p[i-2].time_ms;
                    m1 = (dt * (p[i-1].cents - p[i-2].cents) / prev_dt + prev_dt * slope) / (prev_dt + dt) * dt;
                }
                if (i + 1 < o->pitch_point_count) {
                    const double next_dt = p[i+1].time_ms - p[i].time_ms;
                    m2 = (next_dt * slope + dt * (p[i+1].cents - p[i].cents) / next_dt) / (dt + next_dt) * dt;
                }
                if (std::abs(delta) < 1e-4) { m1 = 0; m2 = 0; }
                else {
                    if (m1 * delta < 0) m1 = 0;
                    if (m2 * delta < 0) m2 = 0;
                    m1 = std::clamp(m1, -3.0 * std::abs(delta), 3.0 * std::abs(delta));
                    m2 = std::clamp(m2, -3.0 * std::abs(delta), 3.0 * std::abs(delta));
                }
                const double f2 = f * f, f3 = f2 * f;
                return (2*f3 - 3*f2 + 1)*p[i-1].cents + (f3 - 2*f2 + f)*m1 +
                    (-2*f3 + 3*f2)*p[i].cents + (f3 - f2)*m2;
            }
            default: factor = 0.5 - 0.5 * std::cos(f * 3.141592653589793); break;
            }
            return p[i-1].cents + factor * delta;
        }
    }
    return p[o->pitch_point_count-1].cents;
}
double source_time(const lt_render_options *o, double time, double source_duration) {
    const double marked_begin = o->loop_start_ms - o->offset_ms;
    const double marked_end = o->loop_end_ms - o->offset_ms;
    const double tail = o->tail_start_ms - o->offset_ms;
    const bool valid_loop = std::isfinite(marked_begin) && std::isfinite(marked_end) &&
        marked_begin >= 0 && marked_end > marked_begin && marked_end <= source_duration;
    const bool has_consonant_adjustment = o->struct_size >=
        offsetof(lt_render_options, consonant_adjustment) + sizeof(o->consonant_adjustment);
    const double consonant_ratio = has_consonant_adjustment ? 1.0 + o->consonant_adjustment / 100.0 : 1.0;
    const double begin = valid_loop ? marked_begin :
        std::clamp(o->consonant_ms * consonant_ratio, 0.0, source_duration);
    const double end = valid_loop ? marked_end :
        (std::isfinite(tail) && tail > begin && tail <= source_duration ? tail : source_duration);
    if (end <= begin) return std::clamp(time * source_duration / o->duration_ms, 0.0, source_duration);
    const double tail_begin = std::isfinite(tail) && tail >= end && tail <= source_duration ? tail : source_duration;
    const double attack = begin;
    const double release = source_duration - tail_begin;
    const double scale = std::min(1.0, o->duration_ms / std::max(attack + release, 1e-9));
    const double target_attack = attack * scale;
    const double target_tail = o->duration_ms - release * scale;
    if (time < target_attack) return std::clamp(time / scale, 0.0, source_duration);
    if (time >= target_tail) return std::clamp(tail_begin + (time - target_tail) / scale, 0.0, source_duration);
    return std::clamp(begin + (time - target_attack) * (end - begin) /
        std::max(target_tail - target_attack, 1e-9), 0.0, source_duration);
}
}
extern "C" {
uint32_t lt_abi_version(void) { return LT_ABI_VERSION; }
const char *lt_version(void) { return "tainah-rt/2.1.0-world"; }
void lt_pcm_free(lt_pcm *pcm) {
    if (!pcm || pcm->struct_size < sizeof(lt_pcm)) return;
    delete[] pcm->samples;
    pcm->samples = nullptr; pcm->length = 0; pcm->sample_rate = 0;
}
void lt_cache_clear_memory(void) {
    std::lock_guard<std::mutex> lock(cache_mutex);
    analysis_cache.clear();
}
int32_t lt_render(const lt_render_options *o, lt_pcm *out, lt_error *error) {
    if (!out || out->struct_size < sizeof(lt_pcm) || !error || error->struct_size < sizeof(lt_error)) return 1;
    out->samples = nullptr; out->length = 0; out->sample_rate = 0;
    fail(error, 0, "");
    constexpr uint32_t base_options_size = static_cast<uint32_t>(offsetof(lt_render_options, formant_shift));
    const bool has_formant_shift = o && o->struct_size >= offsetof(lt_render_options, formant_shift) + sizeof(o->formant_shift);
    const double formant_shift = has_formant_shift ? o->formant_shift : 0.0;
    const bool has_consonant_adjustment = o && o->struct_size >=
        offsetof(lt_render_options, consonant_adjustment) + sizeof(o->consonant_adjustment);
    const double consonant_adjustment = has_consonant_adjustment ? o->consonant_adjustment : 0.0;
    if (!o || o->struct_size < base_options_size || !o->input || o->input_length < 64 ||
        o->input_length > INT32_MAX || o->sample_rate < 8000 || o->sample_rate > 192000 ||
        !std::isfinite(o->duration_ms) || o->duration_ms <= 0 || o->duration_ms > 600000 ||
        !std::isfinite(o->offset_ms) || !std::isfinite(o->consonant_ms) || !std::isfinite(o->cutoff_ms) ||
        !std::isfinite(o->pitch_hz) || o->pitch_hz <= 0 || !std::isfinite(o->gender) ||
        !std::isfinite(o->pitch_cents) || !std::isfinite(o->breathiness) ||
        !std::isfinite(formant_shift) ||
        formant_shift < -100.0 || formant_shift > 100.0 ||
        !std::isfinite(consonant_adjustment) ||
        consonant_adjustment < -100.0 || consonant_adjustment > 100.0 ||
        !std::isfinite(o->high_frequency_cutoff) || !std::isfinite(o->frame_period_ms) ||
        o->frame_period_ms < 1 || o->frame_period_ms > 20 || !std::isfinite(o->f0_floor) ||
        !std::isfinite(o->f0_ceiling) || o->f0_floor < 20 || o->f0_ceiling <= o->f0_floor ||
        (o->f0_method != 1 && o->f0_method != 2) ||
        o->pitch_point_count > 100000 || (o->pitch_point_count && !o->pitch_points) ||
        !std::all_of(o->input, o->input + o->input_length, [](float v) { return std::isfinite(v); })) {
        fail(error, 1, "invalid render arguments"); return 1;
    }
    for (uint32_t i = 0; i < o->pitch_point_count; ++i) {
        if (!std::isfinite(o->pitch_points[i].time_ms) || !std::isfinite(o->pitch_points[i].cents) ||
            (i && o->pitch_points[i].time_ms <= o->pitch_points[i-1].time_ms)) {
            fail(error, 1, "invalid pitch curve"); return 1;
        }
    }
    if (cancelled(o)) { fail(error, 2, "render cancelled"); return 2; }
    try {
        std::lock_guard<std::mutex> world_lock(world_mutex);
        const auto output_length = static_cast<uint64_t>(std::llround(o->duration_ms * o->sample_rate / 1000.0));
        if (!output_length || output_length > INT32_MAX) { fail(error, 1, "requested duration out of range"); return 1; }
        const size_t start = std::min<size_t>(o->input_length, static_cast<size_t>(std::max(0.0, o->offset_ms) * o->sample_rate / 1000.0));
        const size_t end = o->cutoff_ms > 0 ? o->input_length - std::min<uint64_t>(o->input_length, static_cast<uint64_t>(o->cutoff_ms * o->sample_rate / 1000.0)) :
            o->cutoff_ms < 0 ? std::min<uint64_t>(o->input_length, start + static_cast<uint64_t>(-o->cutoff_ms * o->sample_rate / 1000.0)) : o->input_length;
        if (end <= start || end - start < 64) { fail(error, 3, "oto.ini source region has insufficient coverage"); return 3; }
        const float *source = o->input + start;
        const size_t source_length = end - start;
        uint64_t key = 14695981039346656037ull;
        key = hash_bytes(key, lt_version(), std::strlen(lt_version()));
        key = hash_bytes(key, source, source_length * sizeof(float));
        key = hash_bytes(key, &o->sample_rate, sizeof(o->sample_rate));
        key = hash_bytes(key, &o->frame_period_ms, sizeof(double));
        key = hash_bytes(key, &o->f0_floor, sizeof(double));
        key = hash_bytes(key, &o->f0_ceiling, sizeof(double));
        key = hash_bytes(key, &o->f0_method, sizeof(o->f0_method));
        std::shared_ptr<const lessAudioModel> cached;
        { std::lock_guard<std::mutex> lock(cache_mutex); auto it = analysis_cache.find(key); if (it != analysis_cache.end()) cached = it->second; }
        if (!cached) cached = load_analysis(key, o->sample_rate, o->frame_period_ms);
        if (!cached) {
            std::vector<double> input(source, source + source_length);
            lessConfigure config;
            config.audio_model_frame_period = o->frame_period_ms;
            config.f0_dio_floor = o->f0_floor;
            config.f0_harvest_floor = o->f0_floor;
            config.f0_ceiling = o->f0_ceiling;
            config.f0_mode = o->f0_method == 2 ? lessConfigure::F0_MODE::F0_MODE_DIO : lessConfigure::F0_MODE::F0_MODE_HARVEST;
            config.f0_cheap_trick_floor = o->f0_floor;
            if (cancelled(o)) { fail(error, 2, "render cancelled"); return 2; }
            AudioModel analysis(input.data(), static_cast<int>(input.size()), static_cast<int>(o->sample_rate), config);
            auto model = analysis.GetAudioModel();
            if (model.f0.size() < 2 || model.spectrogram.size() != model.f0.size() ||
                model.aperiodicity.size() != model.f0.size() || model.w_length <= 0) {
                fail(error, 3, "WORLD analysis returned incomplete coverage"); return 3;
            }
            cached = std::make_shared<lessAudioModel>(std::move(model));
            if (cancelled(o)) { fail(error, 2, "render cancelled"); return 2; }
            save_analysis(key, *cached);
        }
        { std::lock_guard<std::mutex> lock(cache_mutex);
          if (analysis_cache.size() >= 16) analysis_cache.clear();
          analysis_cache[key] = cached; }
        lessAudioModel render;
        render.frame_period = cached->frame_period;
        render.fs = cached->fs;
        render.w_length = cached->w_length;
        render.fft_size = cached->fft_size;
        const double formant_ratio = std::pow(2.0, formant_shift / 400.0);
        const size_t frames = static_cast<size_t>(std::ceil(o->duration_ms / render.frame_period)) + 2;
        render.f0.reserve(frames); render.spectrogram.reserve(frames); render.aperiodicity.reserve(frames);
        const double source_duration = source_length * 1000.0 / o->sample_rate;
        for (size_t i = 0; i < frames; ++i) {
            if (cancelled(o)) { fail(error, 2, "render cancelled"); return 2; }
            const double time = std::min(o->duration_ms, i * render.frame_period);
            const double mapped = source_time(o, time, source_duration);
            const double pos = std::clamp(mapped / render.frame_period, 0.0, double(cached->f0.size() - 1));
            const size_t left = static_cast<size_t>(pos);
            const size_t right = std::min(left + 1, cached->f0.size() - 1);
            const double f = pos - left;
            const double source_f0 = cached->f0[left] * (1 - f) + cached->f0[right] * f;
            const double target_f0 = o->pitch_hz * std::pow(2.0, (pitch_cents(o, time) + o->pitch_cents) / 1200.0);
            render.f0.push_back(source_f0 > 0 ? std::clamp(target_f0, o->f0_floor, o->f0_ceiling) : 0.0);
            render.spectrogram.emplace_back(render.w_length);
            render.aperiodicity.emplace_back(render.w_length);
            for (int bin = 0; bin < render.w_length; ++bin) {
                const double hz = double(bin) * o->sample_rate / render.fft_size;
                const double source_bin = std::clamp(bin / formant_ratio, 0.0, double(render.w_length - 1));
                const int source_left = static_cast<int>(source_bin);
                const int source_right = std::min(source_left + 1, render.w_length - 1);
                const double source_fraction = source_bin - source_left;
                const double spectrum_left = cached->spectrogram[left][source_left] * (1 - source_fraction) +
                    cached->spectrogram[left][source_right] * source_fraction;
                const double spectrum_right = cached->spectrogram[right][source_left] * (1 - source_fraction) +
                    cached->spectrogram[right][source_right] * source_fraction;
                const double spectrum = spectrum_left * (1-f) + spectrum_right * f;
                const double tilt = std::pow(std::max(1.0, hz / 1000.0), -std::clamp(o->gender, -100.0, 100.0) / 200.0);
                const double high_cut = o->high_frequency_cutoff > 0 && hz > o->high_frequency_cutoff ?
                    std::exp(-6.0 * (hz - o->high_frequency_cutoff) / o->sample_rate) : 1.0;
                render.spectrogram.back()[bin] = std::max(0.0, spectrum * tilt * high_cut);
                const double ap = cached->aperiodicity[left][bin] * (1-f) + cached->aperiodicity[right][bin] * f;
                const double breath = std::clamp(o->breathiness / 100.0, 0.0, 0.95);
                render.aperiodicity.back()[bin] = std::clamp(ap + breath * (1.0 - ap), 0.0, 1.0);
            }
        }
        if (cancelled(o)) { fail(error, 2, "render cancelled"); return 2; }
        Synthesis synth(std::move(render), static_cast<int>(output_length));
        const double *rendered = synth.GetWavData();
        if (!rendered) { fail(error, 4, "WORLD synthesis returned no PCM"); return 4; }
        auto buffer = std::make_unique<float[]>(static_cast<size_t>(output_length));
        double peak = 0.0;
        for (uint64_t i = 0; i < output_length; ++i) {
            if (!std::isfinite(rendered[i])) { fail(error, 4, "WORLD synthesis returned non-finite PCM"); return 4; }
            buffer[i] = static_cast<float>(rendered[i]);
            peak = std::max(peak, std::abs(rendered[i]));
        }
        if (peak <= 1e-5) { fail(error, 5, "WORLD synthesis returned silence"); return 5; }
        if (cancelled(o)) { fail(error, 2, "render cancelled"); return 2; }
        out->samples = buffer.release(); out->length = output_length; out->sample_rate = o->sample_rate;
        return 0;
    } catch (const std::exception &e) { fail(error, 6, e.what()); return 6; }
    catch (...) { fail(error, 6, "unknown WORLD error"); return 6; }
}
}
