#include "kamafeu_dsp.h"

#include <math.h>

uint32_t kamafeu_dsp_abi_version(void) {
    return KAMAFEU_DSP_ABI_VERSION;
}

KamafeuDspStatus kamafeu_dsp_info(KamafeuDspInfo *info) {
    if (info == NULL) {
        return KAMAFEU_DSP_INVALID_ARGUMENT;
    }
    info->abi_version = KAMAFEU_DSP_ABI_VERSION;
    info->capabilities = KAMAFEU_DSP_CAP_MIX | KAMAFEU_DSP_CAP_OVERLAP_ADD;
#if defined(__SSE2__) || defined(__ARM_NEON) || defined(__ARM_NEON__)
    info->capabilities |= KAMAFEU_DSP_CAP_SIMD;
    info->simd_flags = 1u;
#else
    info->simd_flags = 0u;
#endif
    info->backend_name = "kamafeu-c-abi";
    return KAMAFEU_DSP_OK;
}

KamafeuDspStatus kamafeu_mix_stereo_f32(
    float *destination,
    size_t destination_frames,
    const float *source,
    size_t source_frames,
    float gain,
    float pan) {
    if ((destination == NULL && destination_frames != 0) ||
        (source == NULL && source_frames != 0) ||
        destination_frames < source_frames ||
        !isfinite(gain) || !isfinite(pan) || pan < -1.0f || pan > 1.0f) {
        return KAMAFEU_DSP_INVALID_ARGUMENT;
    }

    const float left = gain * (1.0f - pan);
    const float right = gain * (1.0f + pan);
    for (size_t frame = 0; frame < source_frames; ++frame) {
        destination[frame * 2] += source[frame * 2] * left;
        destination[frame * 2 + 1] += source[frame * 2 + 1] * right;
    }
    return KAMAFEU_DSP_OK;
}

KamafeuDspStatus kamafeu_overlap_add_f32(
    float *destination,
    const float *source,
    size_t destination_frames,
    size_t source_frames,
    size_t offset,
    float gain) {
    if ((destination == NULL && destination_frames != 0) ||
        (source == NULL && source_frames != 0) ||
        offset > destination_frames ||
        source_frames > destination_frames - offset || !isfinite(gain)) {
        return KAMAFEU_DSP_INVALID_ARGUMENT;
    }
    for (size_t frame = 0; frame < source_frames; ++frame) {
        destination[offset + frame] += source[frame] * gain;
    }
    return KAMAFEU_DSP_OK;
}
