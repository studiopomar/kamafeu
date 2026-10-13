#ifndef KAMAFEU_DSP_H
#define KAMAFEU_DSP_H

#include <stddef.h>
#include <stdint.h>

#define KAMAFEU_DSP_ABI_VERSION 1u
#define KAMAFEU_DSP_CAP_MIX (1u << 0)
#define KAMAFEU_DSP_CAP_OVERLAP_ADD (1u << 1)
#define KAMAFEU_DSP_CAP_SIMD (1u << 2)

#ifdef __cplusplus
extern "C" {
#endif

typedef enum KamafeuDspStatus {
    KAMAFEU_DSP_OK = 0,
    KAMAFEU_DSP_INVALID_ARGUMENT = 1,
    KAMAFEU_DSP_BUFFER_TOO_SMALL = 2,
    KAMAFEU_DSP_NONFINITE_INPUT = 3
} KamafeuDspStatus;

typedef struct KamafeuDspInfo {
    uint32_t abi_version;
    uint32_t capabilities;
    uint32_t simd_flags;
    const char *backend_name;
} KamafeuDspInfo;

uint32_t kamafeu_dsp_abi_version(void);
KamafeuDspStatus kamafeu_dsp_info(KamafeuDspInfo *info);

KamafeuDspStatus kamafeu_mix_stereo_f32(
    float *destination,
    size_t destination_frames,
    const float *source,
    size_t source_frames,
    float gain,
    float pan);

KamafeuDspStatus kamafeu_overlap_add_f32(
    float *destination,
    const float *source,
    size_t destination_frames,
    size_t source_frames,
    size_t offset,
    float gain);

#ifdef __cplusplus
}
#endif

#endif
