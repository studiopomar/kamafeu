#include "tainah.h"
#include "tainah_timing.h"
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
inline double sqr(double v) { return v * v; }

std::mutex &cache_mutex() {
    static auto *mtx = new std::mutex();
    return *mtx;
}
// Upstream WORLD's randn state is process-global. CheapTrick, D4C and
// Synthesis reseed and consume it, so overlapping renders can change PCM.
std::recursive_mutex &world_mutex() {
    static auto *mtx = new std::recursive_mutex();
    return *mtx;
}
std::unordered_map<uint64_t, std::shared_ptr<const lessAudioModel>> &analysis_cache() {
    static auto *cache = new std::unordered_map<uint64_t, std::shared_ptr<const lessAudioModel>>();
    return *cache;
}

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
    if (const char *tmpdir = std::getenv("TMPDIR")) return fs::path(tmpdir) / "kamafeu" / "tainah-analysis";
#endif
    std::error_code ec;
    auto temp = fs::temp_directory_path(ec);
    if (!ec && !temp.empty()) return temp / "kamafeu" / "tainah-analysis";
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

/* oto.ini region shared by render and analysis. */
bool source_region(const lt_render_options *o, size_t &start, size_t &end) {
    start = std::min<size_t>(o->input_length, static_cast<size_t>(std::max(0.0, o->offset_ms) * o->sample_rate / 1000.0));
    end = o->cutoff_ms > 0 ? o->input_length - std::min<uint64_t>(o->input_length, static_cast<uint64_t>(o->cutoff_ms * o->sample_rate / 1000.0)) :
        o->cutoff_ms < 0 ? std::min<uint64_t>(o->input_length, start + static_cast<uint64_t>(-o->cutoff_ms * o->sample_rate / 1000.0)) : o->input_length;
    return end > start && end - start >= 64;
}

/* Returns the WORLD model for the complete recording from the memory/disk cache or
   analyses it. Harvesting the full WAV preserves context around oto.ini cuts and lets
   all aliases sharing the recording reuse the analysis. */
std::shared_ptr<const lessAudioModel> obtain_analysis(const lt_render_options *o, lt_error *error, int32_t &status) {
    status = 0;
    uint64_t key = 14695981039346656037ull;
    key = hash_bytes(key, "tainah-rt/2.3.0-world", std::strlen("tainah-rt/2.3.0-world"));
    key = hash_bytes(key, o->input, o->input_length * sizeof(float));
    key = hash_bytes(key, &o->sample_rate, sizeof(o->sample_rate));
    key = hash_bytes(key, &o->frame_period_ms, sizeof(double));
    key = hash_bytes(key, &o->f0_floor, sizeof(double));
    key = hash_bytes(key, &o->f0_ceiling, sizeof(double));
    key = hash_bytes(key, &o->f0_method, sizeof(o->f0_method));
    std::shared_ptr<const lessAudioModel> cached;
    {
        std::lock_guard<std::mutex> lock(cache_mutex());
        auto &cache = analysis_cache();
        auto it = cache.find(key);
        if (it != cache.end()) cached = it->second;
    }
    if (!cached) cached = load_analysis(key, o->sample_rate, o->frame_period_ms);
    if (!cached) {
        std::vector<double> input(o->input, o->input + o->input_length);
        lessConfigure config;
        config.audio_model_frame_period = o->frame_period_ms;
        config.f0_dio_floor = o->f0_floor;
        config.f0_harvest_floor = o->f0_floor;
        config.f0_ceiling = o->f0_ceiling;
        config.f0_mode = o->f0_method == 2 ? lessConfigure::F0_MODE::F0_MODE_DIO : lessConfigure::F0_MODE::F0_MODE_HARVEST;
        config.f0_cheap_trick_floor = o->f0_floor;
        if (cancelled(o)) { fail(error, 2, "render cancelled"); status = 2; return {}; }
        lessAudioModel model;
        {
            std::lock_guard<std::recursive_mutex> world_lock(world_mutex());
            AudioModel analysis(input.data(), static_cast<int>(input.size()), static_cast<int>(o->sample_rate), config);
            model = analysis.GetAudioModel();
        }
        if (model.f0.size() < 2 || model.spectrogram.size() != model.f0.size() ||
            model.aperiodicity.size() != model.f0.size() || model.w_length <= 0) {
            fail(error, 3, "WORLD analysis returned incomplete coverage"); status = 3; return {};
        }
        cached = std::make_shared<lessAudioModel>(std::move(model));
        if (cancelled(o)) { fail(error, 2, "render cancelled"); status = 2; return {}; }
        save_analysis(key, *cached);
    }
    {
        std::lock_guard<std::mutex> lock(cache_mutex());
        auto &cache = analysis_cache();
        if (cache.size() >= 256 && cache.find(key) == cache.end()) {
            cache.erase(cache.begin());
        }
        cache[key] = cached;
    }
    return cached;
}

struct FormantEstimate {
    double f0 = 0.0, f1 = 0.0, f2 = 0.0, f3 = 0.0;
    std::vector<double> smoothed_db; /* one value per WORLD bin */
    bool ok = false;
};

/* Mean log envelope of the voiced frames in [start_ms, end_ms) followed by
   peak picking. CheapTrick envelopes are already harmonic-free, so a light
   ~80 Hz moving average is enough to stabilise the peaks. */
FormantEstimate estimate_formants(const lessAudioModel &m, double start_ms, double end_ms) {
    FormantEstimate est;
    const int bins = m.w_length;
    if (bins < 16 || m.f0.empty() || m.frame_period <= 0) return est;
    const double hz_per_bin = double(m.fs) / m.fft_size;
    const size_t frames = m.f0.size();
    size_t first = static_cast<size_t>(std::clamp(start_ms / m.frame_period, 0.0, double(frames - 1)));
    size_t last = static_cast<size_t>(std::clamp(end_ms / m.frame_period, 0.0, double(frames)));
    if (last <= first + 1) { first = 0; last = frames; }
    std::vector<double> acc(bins, 0.0);
    std::vector<double> f0s;
    size_t count = 0;
    for (int pass = 0; pass < 2 && count == 0; ++pass) {
        for (size_t i = first; i < last; ++i) {
            if (pass == 0 && !(m.f0[i] > 0.0)) continue;
            for (int b = 0; b < bins; ++b) acc[b] += 10.0 * std::log10(std::max(m.spectrogram[i][b], 1e-20));
            if (m.f0[i] > 0.0) f0s.push_back(m.f0[i]);
            ++count;
        }
    }
    if (!count) return est;
    for (double &v : acc) v /= double(count);
    if (!f0s.empty()) {
        const size_t mid = f0s.size() / 2;
        std::nth_element(f0s.begin(), f0s.begin() + mid, f0s.end());
        est.f0 = f0s[mid];
    }
    const int hw = std::max(1, static_cast<int>(80.0 / hz_per_bin));
    std::vector<double> prefix(bins + 1, 0.0);
    for (int b = 0; b < bins; ++b) prefix[b + 1] = prefix[b] + acc[b];
    est.smoothed_db.resize(bins);
    for (int b = 0; b < bins; ++b) {
        const int lo = std::max(0, b - hw), hi = std::min(bins - 1, b + hw);
        est.smoothed_db[b] = (prefix[hi + 1] - prefix[lo]) / double(hi - lo + 1);
    }
    const auto &s = est.smoothed_db;
    auto refine = [&](int b) {
        if (b <= 0 || b >= bins - 1) return b * hz_per_bin;
        const double a = s[b - 1], c = s[b], d = s[b + 1];
        const double denom = a - 2.0 * c + d;
        const double delta = std::abs(denom) > 1e-12 ? std::clamp(0.5 * (a - d) / denom, -0.5, 0.5) : 0.0;
        return (b + delta) * hz_per_bin;
    };
    auto find_peak = [&](double lo_hz, double hi_hz) -> double {
        const int lo = std::clamp(static_cast<int>(lo_hz / hz_per_bin), 1, bins - 2);
        const int hi = std::clamp(static_cast<int>(hi_hz / hz_per_bin), 1, bins - 2);
        if (hi <= lo) return 0.0;
        int best = -1;
        for (int b = lo; b <= hi; ++b)
            if (s[b] >= s[b - 1] && s[b] > s[b + 1] && (best < 0 || s[b] > s[best])) best = b;
        if (best < 0) {
            /* Merged formants (back vowels): use the strongest shoulder. */
            double best_curv = -1e300;
            for (int b = lo; b <= hi; ++b) {
                const double curv = -(s[b - 1] - 2.0 * s[b] + s[b + 1]);
                if (curv > best_curv) { best_curv = curv; best = b; }
            }
        }
        return best < 0 ? 0.0 : refine(best);
    };
    const double nyquist = m.fs / 2.0;
    est.f1 = find_peak(200.0, 1100.0);
    if (est.f1 <= 0.0) est.f1 = 500.0;
    est.f2 = find_peak(std::max(est.f1 + 250.0, 600.0), std::min(3200.0, nyquist * 0.8));
    if (est.f2 <= 0.0) est.f2 = std::max(est.f1 + 700.0, 1500.0);
    est.f3 = find_peak(est.f2 + 250.0, std::min(4500.0, nyquist * 0.9));
    if (est.f3 <= 0.0) est.f3 = std::min(est.f2 + 1000.0, nyquist * 0.9);
    est.ok = true;
    return est;
}

double piecewise(double x, const double *xs, const double *ys, int n) {
    if (x <= xs[0]) return ys[0];
    for (int i = 1; i < n; ++i) {
        if (x <= xs[i]) {
            const double span = xs[i] - xs[i - 1];
            const double t = span > 1e-9 ? (x - xs[i - 1]) / span : 0.0;
            return ys[i - 1] + t * (ys[i] - ys[i - 1]);
        }
    }
    return ys[n - 1];
}
}

extern "C" {
uint32_t lt_abi_version(void) { return LT_ABI_VERSION; }
uint32_t lt_render_options_size(void) { return static_cast<uint32_t>(sizeof(lt_render_options)); }
const char *lt_version(void) { return "tainah-rt/2.3.0-world"; }
void lt_pcm_free(lt_pcm *pcm) {
    if (!pcm || pcm->struct_size < sizeof(lt_pcm)) return;
    delete[] pcm->samples;
    pcm->samples = nullptr; pcm->length = 0; pcm->sample_rate = 0;
}
void lt_cache_clear_memory(void) {
    std::lock_guard<std::mutex> lock(cache_mutex());
    analysis_cache().clear();
}

int32_t lt_analyze_formants(const lt_render_options *o, lt_formant_analysis *out, lt_error *error) {
    if (!out || out->struct_size < sizeof(lt_formant_analysis) || !error || error->struct_size < sizeof(lt_error)) return 1;
    out->f0_hz = 0.0; out->f1_hz = 0.0; out->f2_hz = 0.0; out->f3_hz = 0.0;
    out->envelope_len = 0; out->envelope_max_hz = 0.0;
    std::fill(out->envelope_db, out->envelope_db + LT_FORMANT_ENVELOPE_POINTS, -120.0);
    fail(error, 0, "");
    if (!o || o->struct_size < offsetof(lt_render_options, formant_shift) || !o->input || o->input_length < 64 ||
        o->input_length > INT32_MAX || o->sample_rate < 8000 || o->sample_rate > 192000 ||
        !std::isfinite(o->offset_ms) || !std::isfinite(o->consonant_ms) || !std::isfinite(o->cutoff_ms) ||
        !std::isfinite(o->frame_period_ms) || o->frame_period_ms < 1 || o->frame_period_ms > 20 ||
        !std::isfinite(o->f0_floor) || !std::isfinite(o->f0_ceiling) || o->f0_floor < 20 ||
        o->f0_ceiling <= o->f0_floor || (o->f0_method != 1 && o->f0_method != 2)) {
        fail(error, 1, "invalid analysis arguments"); return 1;
    }
    size_t start = 0, end = 0;
    if (!source_region(o, start, end)) { fail(error, 3, "oto.ini source region has insufficient coverage"); return 3; }
    int32_t status = 0;
    std::shared_ptr<const lessAudioModel> cached = obtain_analysis(o, error, status);
    if (!cached) return status ? status : 3;
    const double source_duration = (end - start) * 1000.0 / o->sample_rate;
    const double vowel_start = start * 1000.0 / o->sample_rate + std::clamp(o->consonant_ms, 0.0, source_duration);
    const double vowel_end = start * 1000.0 / o->sample_rate + source_duration;
    FormantEstimate est = estimate_formants(*cached, vowel_start, vowel_end);
    if (!est.ok) { fail(error, 3, "unable to estimate formants from vowel region"); return 3; }
    out->f0_hz = est.f0;
    out->f1_hz = est.f1;
    out->f2_hz = est.f2;
    out->f3_hz = est.f3;
    out->envelope_len = LT_FORMANT_ENVELOPE_POINTS;
    const double nyquist = o->sample_rate / 2.0;
    out->envelope_max_hz = std::min(5000.0, nyquist);
    const int bins = cached->w_length;
    const double hz_per_bin = double(cached->fs) / cached->fft_size;
    for (uint32_t p = 0; p < LT_FORMANT_ENVELOPE_POINTS; ++p) {
        const double target_hz = (p / double(LT_FORMANT_ENVELOPE_POINTS - 1)) * out->envelope_max_hz;
        const double b = std::clamp(target_hz / hz_per_bin, 0.0, double(bins - 1));
        const int lo = static_cast<int>(b);
        const int hi = std::min(lo + 1, bins - 1);
        const double frac = b - lo;
        out->envelope_db[p] = est.smoothed_db[lo] * (1.0 - frac) + est.smoothed_db[hi] * frac;
    }
    return 0;
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
    const bool has_target = o && o->struct_size >=
        offsetof(lt_render_options, has_target_consonant) + sizeof(o->has_target_consonant) &&
        o->has_target_consonant != 0;
    const double target_consonant = has_target ? o->target_consonant_ms : 0.0;
    const bool has_vowel_fields = o && o->struct_size >=
        offsetof(lt_render_options, has_vowel_formant) + sizeof(o->has_vowel_formant);
    const bool vowel_requested = has_vowel_fields && o->has_vowel_formant != 0;
    const double f1_ratio = vowel_requested ? o->f1_ratio : 1.0;
    const double f2_ratio = vowel_requested ? o->f2_ratio : 1.0;
    const bool has_extended_vowel = vowel_requested && o->struct_size >=
        offsetof(lt_render_options, singers_formant) + sizeof(o->singers_formant);
    const double raw_f3_ratio = has_extended_vowel ? o->f3_ratio : 1.0;
    const double f3_ratio = (raw_f3_ratio == 0.0) ? 1.0 : raw_f3_ratio;
    const double nasality = has_extended_vowel ? o->nasality : 0.0;
    const double singers_formant = has_extended_vowel ? o->singers_formant : 0.0;
    if (vowel_requested && (!std::isfinite(f1_ratio) || !std::isfinite(f2_ratio) ||
        !std::isfinite(f3_ratio) || !std::isfinite(nasality) || !std::isfinite(singers_formant) ||
        f1_ratio < 0.5 || f1_ratio > 2.0 || f2_ratio < 0.5 || f2_ratio > 2.0 ||
        f3_ratio < 0.5 || f3_ratio > 2.0 || nasality < 0.0 || nasality > 1.0 ||
        singers_formant < -1.0 || singers_formant > 1.0)) {
        fail(error, 1, "invalid vowel formant ratios"); return 1;
    }
    const bool vowel_active = vowel_requested &&
        (std::abs(f1_ratio - 1.0) > 1e-6 || std::abs(f2_ratio - 1.0) > 1e-6 ||
         std::abs(f3_ratio - 1.0) > 1e-6 || nasality > 1e-6 || std::abs(singers_formant) > 1e-6);

    if (!o || o->struct_size < base_options_size || !o->input || o->input_length < 64 ||
        o->input_length > INT32_MAX || o->sample_rate < 8000 || o->sample_rate > 192000 ||
        !std::isfinite(o->duration_ms) || o->duration_ms <= 0 || o->duration_ms > 600000 ||
        !std::isfinite(o->offset_ms) || !std::isfinite(o->consonant_ms) || !std::isfinite(o->cutoff_ms) ||
        !std::isfinite(o->pitch_hz) || o->pitch_hz <= 0 || !std::isfinite(o->gender) ||
        !std::isfinite(o->pitch_cents) || !std::isfinite(o->breathiness) ||
        !std::isfinite(target_consonant) || target_consonant < 0 ||
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
        std::lock_guard<std::recursive_mutex> world_lock(world_mutex());
        const auto output_length = static_cast<uint64_t>(std::llround(o->duration_ms * o->sample_rate / 1000.0));
        if (!output_length || output_length > INT32_MAX) { fail(error, 1, "requested duration out of range"); return 1; }
        size_t start = 0, end = 0;
        if (!source_region(o, start, end)) { fail(error, 3, "oto.ini source region has insufficient coverage"); return 3; }
        const size_t source_length = end - start;
        int32_t analysis_status = 0;
        std::shared_ptr<const lessAudioModel> cached = obtain_analysis(o, error, analysis_status);
        if (!cached) return analysis_status ? analysis_status : 3;

        lessAudioModel render;
        render.frame_period = cached->frame_period;
        render.fs = cached->fs;
        render.w_length = cached->w_length;
        render.fft_size = cached->fft_size;
        const double formant_ratio = std::pow(2.0, formant_shift / 400.0);
        const size_t frames = static_cast<size_t>(std::ceil(o->duration_ms / render.frame_period)) + 2;
        render.f0.reserve(frames); render.spectrogram.reserve(frames); render.aperiodicity.reserve(frames);
        const double source_duration = source_length * 1000.0 / o->sample_rate;

        const int bins = render.w_length;
        const double nyquist = o->sample_rate / 2.0;

        std::vector<int> bin_source_left;
        std::vector<int> bin_source_right;
        std::vector<double> bin_source_fraction;
        std::vector<double> bin_tilt(bins);
        std::vector<double> bin_high_cut(bins);
        std::vector<double> vowel_eq_gain;
        FormantEstimate est;
        double t1 = 0.0, t2 = 0.0, t3 = 0.0, t4 = 0.0;
        double vowel_transition_start_ms = 0.0;
        double vowel_transition_duration_ms = 80.0;

        for (int bin = 0; bin < bins; ++bin) {
            const double hz = double(bin) * o->sample_rate / render.fft_size;
            bin_tilt[bin] = std::pow(std::max(1.0, hz / 1000.0), -std::clamp(o->gender, -100.0, 100.0) / 200.0);
            bin_high_cut[bin] = o->high_frequency_cutoff > 0 && hz > o->high_frequency_cutoff ?
                std::exp(-6.0 * (hz - o->high_frequency_cutoff) / o->sample_rate) : 1.0;
        }

        if (!vowel_active) {
            bin_source_left.resize(bins);
            bin_source_right.resize(bins);
            bin_source_fraction.resize(bins);
            for (int bin = 0; bin < bins; ++bin) {
                const double source_bin = std::clamp(bin / formant_ratio, 0.0, double(bins - 1));
                const int left = static_cast<int>(source_bin);
                bin_source_left[bin] = left;
                bin_source_right[bin] = std::min(left + 1, bins - 1);
                bin_source_fraction[bin] = source_bin - left;
            }
        } else {
            const double vowel_start = start * 1000.0 / o->sample_rate + std::clamp(o->consonant_ms, 0.0, source_duration);
            const double vowel_end = start * 1000.0 / o->sample_rate + source_duration;
            est = estimate_formants(*cached, vowel_start, vowel_end);
            if (!est.ok) {
                est.f1 = 500.0; est.f2 = 1500.0; est.f3 = 2500.0;
            }
            t3 = std::clamp(est.f3 * f3_ratio, std::max(est.f2 * f2_ratio + 250.0, 1500.0), nyquist * 0.9);
            t1 = std::clamp(est.f1 * f1_ratio, 150.0, std::min(t3 - 450.0, 1150.0));
            t2 = std::clamp(est.f2 * f2_ratio, std::max(t1 + 250.0, 550.0), std::min(t3 - 200.0, 3400.0));
            t4 = std::clamp(est.f3 + 1200.0, t3 + 300.0, nyquist * 0.95);

            vowel_eq_gain.resize(bins, 1.0);
            const double rhotic_amount = (f3_ratio < 0.88) ? std::clamp((0.88 - f3_ratio) / 0.38, 0.0, 1.0) : 0.0;
            for (int bin = 0; bin < bins; ++bin) {
                const double base_hz = double(bin) * o->sample_rate / render.fft_size;
                double db = 0.0;
                if (nasality > 1e-6) {
                    /* Acoustic nasality: low nasal pole ~270 Hz, anti-formant notch ~1.38*F1, F1 damping */
                    db += 3.5 * nasality * std::exp(-0.5 * sqr((base_hz - 270.0) / 75.0));
                    db -= 6.0 * nasality * std::exp(-0.5 * sqr((base_hz - (t1 * 1.38)) / 110.0));
                    db -= 3.0 * nasality * std::exp(-0.5 * sqr((base_hz - t1) / 90.0));
                    db -= 2.0 * nasality * std::clamp((base_hz - 2600.0) / 1800.0, 0.0, 1.0);
                }
                if (rhotic_amount > 1e-6) {
                    /* American English retroflex/rhotic R: merged F2-F3 boost + sublingual notch above F3 */
                    db += 3.8 * rhotic_amount * std::exp(-0.5 * sqr((base_hz - t3) / 160.0));
                    db -= 4.5 * rhotic_amount * std::exp(-0.5 * sqr((base_hz - (t3 + 550.0)) / 260.0));
                }
                if (std::abs(singers_formant) > 1e-6) {
                    /* Singer's Formant (~3 kHz ring / projection) */
                    db += 6.5 * singers_formant * std::exp(-0.5 * sqr((base_hz - 3000.0) / 420.0));
                }
                vowel_eq_gain[bin] = std::pow(10.0, db / 10.0);
            }

            const double consonant_boundary_ms = has_target ? target_consonant : std::max(0.0, o->consonant_ms);
            const double early_lead_ms = std::min(consonant_boundary_ms * 0.65, 55.0);
            vowel_transition_start_ms = std::max(0.0, consonant_boundary_ms - early_lead_ms);
            vowel_transition_duration_ms = std::max(70.0, early_lead_ms + 55.0);
        }

        const double breath = std::clamp(o->breathiness / 100.0, 0.0, 0.95);

        for (size_t i = 0; i < frames; ++i) {
            if (cancelled(o)) { fail(error, 2, "render cancelled"); return 2; }
            const double time = std::min(o->duration_ms, i * render.frame_period);
            const double mapped = tainah::source_time(o, time, source_duration);
            const double pos = std::clamp((start * 1000.0 / o->sample_rate + mapped) / render.frame_period, 0.0, double(cached->f0.size() - 1));
            const size_t left = static_cast<size_t>(pos);
            const size_t right = std::min(left + 1, cached->f0.size() - 1);
            const double f = pos - left;
            // Match lessampler: interpolation must not voice an unvoiced
            // consonant merely because the following frame has an F0.
            const double source_f0 = cached->f0[left];
            const double target_f0 = o->pitch_hz * std::pow(2.0, (pitch_cents(o, time) + o->pitch_cents) / 1200.0);
            render.f0.push_back(source_f0 > 0 ? std::clamp(target_f0, o->f0_floor, o->f0_ceiling) : 0.0);
            render.spectrogram.emplace_back(render.w_length);
            render.aperiodicity.emplace_back(render.w_length);

            if (!vowel_active) {
                for (int bin = 0; bin < bins; ++bin) {
                    const int source_left = bin_source_left[bin];
                    const int source_right = bin_source_right[bin];
                    const double source_fraction = bin_source_fraction[bin];
                    const double spectrum_left = cached->spectrogram[left][source_left] * (1.0 - source_fraction) +
                        cached->spectrogram[left][source_right] * source_fraction;
                    const double spectrum_right = cached->spectrogram[right][source_left] * (1.0 - source_fraction) +
                        cached->spectrogram[right][source_right] * source_fraction;
                    const double spectrum = spectrum_left * (1.0 - f) + spectrum_right * f;
                    render.spectrogram.back()[bin] = std::max(0.0, spectrum * bin_tilt[bin] * bin_high_cut[bin]);
                    const double ap = cached->aperiodicity[f > 0.5 ? right : left][bin];
                    render.aperiodicity.back()[bin] = std::clamp(ap + breath * (1.0 - ap), 0.0, 1.0);
                }
            } else {
                double vowel_weight = 1.0;
                if (time <= vowel_transition_start_ms) {
                    vowel_weight = 0.0;
                } else if (time < vowel_transition_start_ms + vowel_transition_duration_ms) {
                    const double u = (time - vowel_transition_start_ms) / vowel_transition_duration_ms;
                    vowel_weight = 0.5 * (1.0 - std::cos(3.14159265358979323846 * u));
                }

                const double current_f1 = (1.0 - vowel_weight) * est.f1 + vowel_weight * t1;
                const double current_f2 = (1.0 - vowel_weight) * est.f2 + vowel_weight * t2;
                const double current_f3 = (1.0 - vowel_weight) * est.f3 + vowel_weight * t3;

                const double src_anchors[6] = {0.0, est.f1, est.f2, est.f3, t4, nyquist};
                const double dst_anchors[6] = {0.0, current_f1, current_f2, current_f3, t4, nyquist};

                for (int bin = 0; bin < bins; ++bin) {
                    const double base_hz = double(bin) * o->sample_rate / render.fft_size;
                    const double warped_hz = piecewise(base_hz, dst_anchors, src_anchors, 6);
                    const double source_bin = std::clamp((warped_hz / formant_ratio) * (render.fft_size / double(o->sample_rate)), 0.0, double(bins - 1));
                    const int source_left = static_cast<int>(source_bin);
                    const int source_right = std::min(source_left + 1, bins - 1);
                    const double source_fraction = source_bin - source_left;
                    const double spectrum_left = cached->spectrogram[left][source_left] * (1.0 - source_fraction) +
                        cached->spectrogram[left][source_right] * source_fraction;
                    const double spectrum_right = cached->spectrogram[right][source_left] * (1.0 - source_fraction) +
                        cached->spectrogram[right][source_right] * source_fraction;
                    const double spectrum = spectrum_left * (1.0 - f) + spectrum_right * f;
                    const double eq_gain = (1.0 - vowel_weight) + vowel_weight * vowel_eq_gain[bin];
                    render.spectrogram.back()[bin] = std::max(0.0, spectrum * eq_gain * bin_tilt[bin] * bin_high_cut[bin]);
                    const double ap = cached->aperiodicity[f > 0.5 ? right : left][bin];
                    render.aperiodicity.back()[bin] = std::clamp(ap + breath * (1.0 - ap), 0.0, 1.0);
                }
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
