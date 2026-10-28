#ifndef MOTOR_TAINAH_H
#define MOTOR_TAINAH_H
#include <stddef.h>
#include <stdint.h>
#if defined(_WIN32)
# if defined(MOTOR_TAINAH_BUILD)
#  define LT_API __declspec(dllexport)
# else
#  define LT_API __declspec(dllimport)
# endif
#else
# define LT_API __attribute__((visibility("default")))
#endif
#ifdef __cplusplus
extern "C" {
#endif
#define LT_ABI_VERSION 2u
typedef struct lt_error {
    uint32_t struct_size;
    int32_t code;
    char message[256];
} lt_error;
typedef struct lt_pitch_point {
    double time_ms;
    double cents;
    uint32_t shape; /* 0 smooth, 1 linear, 2 ease-in, 3 ease-out, 4 Hermite */
} lt_pitch_point;
typedef int32_t (*lt_cancel_callback)(void *user_data);
typedef struct lt_render_options {
    uint32_t struct_size;
    uint32_t sample_rate;
    const float *input;
    uint64_t input_length;
    double duration_ms;
    double offset_ms;
    double consonant_ms;
    double cutoff_ms;
    double pitch_hz;
    const lt_pitch_point *pitch_points;
    uint32_t pitch_point_count;
    double gender;
    double pitch_cents;
    double breathiness;
    double high_frequency_cutoff;
    double frame_period_ms;
    double f0_floor;
    double f0_ceiling;
    uint32_t f0_method; /* 1 Harvest, 2 DIO */
    double loop_start_ms;
    double loop_end_ms;
    double tail_start_ms;
    lt_cancel_callback cancel;
    void *cancel_user_data;
    /* Optional ABI v2 extension: formant frequency-axis shift, -100 to +100. */
    double formant_shift;
    /* Optional ABI v2 extension: fixed consonant adjustment, -100 to +100; default 0. */
    double consonant_adjustment;
    /* Optional ABI v2 extension: consonant duration on the output timeline. */
    double target_consonant_ms;
    uint32_t has_target_consonant;
} lt_render_options;
typedef struct lt_pcm {
    uint32_t struct_size;
    float *samples;
    uint64_t length;
    uint32_t sample_rate;
} lt_pcm;
LT_API uint32_t lt_abi_version(void);
LT_API const char *lt_version(void);
LT_API int32_t lt_render(const lt_render_options *options, lt_pcm *output, lt_error *error);
LT_API void lt_pcm_free(lt_pcm *pcm);
LT_API void lt_cache_clear_memory(void);
#ifdef __cplusplus
}
#endif
#endif
