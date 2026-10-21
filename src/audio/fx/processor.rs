use super::equalizer::Equalizer;
use super::FxRackConfig;
use crate::dsp::oversampling::Oversampler4x;

/// Hysteresis between the gate opening and closing thresholds.
const GATE_HYSTERESIS_DB: f32 = 4.0;
/// Time the gate stays open after the signal falls below the close threshold.
const GATE_HOLD_MS: f32 = 25.0;
/// Release of the gate peak detector; bridges zero crossings of low notes.
const GATE_DETECTOR_RELEASE_MS: f32 = 12.0;
/// Phase offset between the left and right chorus LFOs (quadrature).
const CHORUS_STEREO_PHASE: f32 = std::f32::consts::FRAC_PI_2;
/// Signals below this magnitude are flushed to zero inside feedback loops,
/// avoiding subnormal arithmetic (up to 100x slower) in decaying tails.
const DENORMAL_FLOOR: f32 = 1.0e-20;

#[inline]
fn db_to_gain(db: f32) -> f32 {
    (db * (std::f32::consts::LN_10 / 20.0)).exp()
}

/// One-pole smoothing coefficient for a time constant in milliseconds.
#[inline]
fn time_coefficient(time_ms: f32, sample_rate: f32) -> f32 {
    (-1.0 / (time_ms * 0.001 * sample_rate).max(1.0)).exp()
}

#[inline]
fn flush_denormal(value: f32) -> f32 {
    if value.abs() < DENORMAL_FLOOR {
        0.0
    } else {
        value
    }
}

/// Catmull-Rom interpolation between `y1` and `y2` (`t` in `[0, 1)`).
#[inline]
fn hermite(y0: f32, y1: f32, y2: f32, y3: f32, t: f32) -> f32 {
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
    ((c3 * t + c2) * t + c1) * t + y1
}

/// Static gain computer of a soft-knee compressor, in dB (always <= 0).
#[inline]
fn compressor_gain_db(level_db: f32, threshold_db: f32, slope: f32, knee_db: f32) -> f32 {
    let over = level_db - threshold_db;
    if knee_db > 0.0 && over.abs() < knee_db * 0.5 {
        let x = over + knee_db * 0.5;
        -slope * x * x / (2.0 * knee_db)
    } else if over > 0.0 {
        -slope * over
    } else {
        0.0
    }
}

/// Integer delay line used for the dry path of the oversampled saturator.
#[derive(Clone)]
struct DelayLine {
    buffer: Vec<f32>,
    position: usize,
}

impl DelayLine {
    fn new(delay: usize) -> Self {
        Self {
            buffer: vec![0.0; delay.max(1)],
            position: 0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.position];
        self.buffer[self.position] = input;
        self.position = (self.position + 1) % self.buffer.len();
        output
    }
}

/// State for audio DSP processing
pub struct FxRackProcessor {
    pub config: FxRackConfig,
    sample_rate: u32,
    // 31-band biquads
    equalizer: Equalizer,
    // Compressor gain (dB, <= 0) after the smooth branching detector.
    comp_gain_db: f32,
    gate_gain: f32,
    gate_envelope: f32,
    gate_open: bool,
    gate_hold: u32,
    limiter_gain: f32,
    // Delay buffer
    delay_buf_l: Vec<f32>,
    delay_buf_r: Vec<f32>,
    delay_pos: usize,
    chorus_buf_l: Vec<f32>,
    chorus_buf_r: Vec<f32>,
    chorus_pos: usize,
    chorus_phase: f32,
    auto_pan_phase: f32,
    // 4x oversampled saturation and its latency-matched dry path.
    saturation_oversamplers: [Oversampler4x; 2],
    saturation_dry: [DelayLine; 2],
    // Reverb pre-delay, comb & allpass buffers
    reverb_predelay: Vec<f32>,
    reverb_predelay_pos: usize,
    reverb_combs_l: Vec<(Vec<f32>, usize, f32)>,
    reverb_combs_r: Vec<(Vec<f32>, usize, f32)>,
    reverb_allpass_l: Vec<(Vec<f32>, usize)>,
    reverb_allpass_r: Vec<(Vec<f32>, usize)>,
}

impl FxRackProcessor {
    pub fn new(config: FxRackConfig, sample_rate: u32) -> Self {
        let sr = sample_rate.max(8000) as f32;
        let max_delay = (sr * 1.5) as usize;

        let equalizer = Equalizer::new(config.eq.gains, sr);

        // Standard Freeverb comb tuning (scaled to sample rate)
        let comb_tunings_l = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
        let comb_tunings_r = [1139, 1211, 1300, 1379, 1445, 1514, 1580, 1640];
        let allpass_tunings_l = [556, 441, 341, 225];
        let allpass_tunings_r = [579, 464, 364, 248];
        let scale = sr / 44100.0;

        let combs_l = comb_tunings_l
            .iter()
            .map(|&t| {
                (
                    vec![0.0f32; ((t as f32) * scale).round() as usize],
                    0,
                    0.0f32,
                )
            })
            .collect();
        let combs_r = comb_tunings_r
            .iter()
            .map(|&t| {
                (
                    vec![0.0f32; ((t as f32) * scale).round() as usize],
                    0,
                    0.0f32,
                )
            })
            .collect();

        let ap_l = allpass_tunings_l
            .iter()
            .map(|&t| (vec![0.0f32; ((t as f32) * scale).round() as usize], 0))
            .collect();
        let ap_r = allpass_tunings_r
            .iter()
            .map(|&t| (vec![0.0f32; ((t as f32) * scale).round() as usize], 0))
            .collect();

        Self {
            config,
            sample_rate,
            equalizer,
            comp_gain_db: 0.0,
            gate_gain: 0.0,
            gate_envelope: 0.0,
            gate_open: false,
            gate_hold: 0,
            limiter_gain: 1.0,
            delay_buf_l: vec![0.0f32; max_delay],
            delay_buf_r: vec![0.0f32; max_delay],
            delay_pos: 0,
            chorus_buf_l: vec![0.0; (sr * 0.06) as usize],
            chorus_buf_r: vec![0.0; (sr * 0.06) as usize],
            chorus_pos: 0,
            chorus_phase: 0.0,
            auto_pan_phase: 0.0,
            saturation_oversamplers: [Oversampler4x::new(), Oversampler4x::new()],
            saturation_dry: [
                DelayLine::new(Oversampler4x::LATENCY),
                DelayLine::new(Oversampler4x::LATENCY),
            ],
            reverb_predelay: vec![0.0; (sr * 0.2) as usize + 1],
            reverb_predelay_pos: 0,
            reverb_combs_l: combs_l,
            reverb_combs_r: combs_r,
            reverb_allpass_l: ap_l,
            reverb_allpass_r: ap_r,
        }
    }

    /// Clear every delay line, envelope and filter state, as if the
    /// processor had just been created. Use between independent renders.
    pub fn reset(&mut self) {
        *self = Self::new(self.config.clone(), self.sample_rate);
    }

    /// Processing latency in frames introduced by the current configuration.
    /// Only the oversampled saturation delays the signal.
    pub fn latency_frames(&self) -> usize {
        if self.config.master_enabled && self.config.saturation.enabled {
            Oversampler4x::LATENCY
        } else {
            0
        }
    }

    /// Process a complete interleaved stereo render in place, compensating the
    /// processing latency so the output stays aligned with the input.
    pub fn process_render(&mut self, samples: &mut Vec<f32>, sample_rate: u32) {
        let latency = self.latency_frames();
        if latency == 0 {
            self.process_interleaved(samples, sample_rate);
            return;
        }
        let original_len = samples.len();
        samples.resize(original_len + latency * 2, 0.0);
        self.process_interleaved(samples, sample_rate);
        samples.drain(..latency * 2);
        samples.truncate(original_len);
    }

    /// Process interleaved stereo audio samples in-place.
    ///
    /// State persists across calls, so a render may be streamed in chunks.
    /// When saturation is enabled the output is delayed by
    /// [`FxRackProcessor::latency_frames`]; [`FxRackProcessor::process_render`]
    /// compensates it for complete buffers.
    pub fn process_interleaved(&mut self, samples: &mut [f32], sample_rate: u32) {
        if !self.config.master_enabled {
            return;
        }

        let sr = sample_rate.max(8000) as f32;

        // 1. 31-Band Graphic Equalizer
        if self.config.eq.enabled {
            self.equalizer.process(samples, &self.config.eq.gains);
        }

        if self.config.gate.enabled {
            self.apply_gate(samples, sr);
        }

        // 2. Dynamic Compressor
        if self.config.compressor.enabled {
            self.apply_compressor(samples, sr);
        }

        // 3. Stereo Delay
        if self.config.delay.enabled {
            self.apply_delay(samples, sr);
        }

        // 4. Algorithmic Reverb
        if self.config.reverb.enabled {
            self.apply_reverb(samples, sr);
        }

        // 5. Soft saturation adds controlled harmonics without hard clipping.
        if self.config.saturation.enabled {
            self.apply_saturation(samples);
        }

        if self.config.chorus.enabled {
            self.apply_chorus(samples, sr);
        }
        if self.config.auto_pan.enabled {
            self.apply_auto_pan(samples, sr);
        }

        // 6. Transparent safety ceiling after every gain-producing effect.
        if self.config.limiter.enabled {
            self.apply_limiter(samples, sr);
        }
    }

    /// `tanh` waveshaper evaluated at 4x the sample rate, so the harmonics it
    /// creates above Nyquist are filtered instead of folding back as aliasing.
    /// The dry signal is delayed by the oversampler latency before mixing.
    fn apply_saturation(&mut self, samples: &mut [f32]) {
        let drive = db_to_gain(self.config.saturation.drive_db.clamp(0.0, 30.0));
        let mix = self.config.saturation.mix.clamp(0.0, 1.0);
        let normalizer = drive.tanh().max(f32::EPSILON);
        let shape = |value: f32| (value * drive).tanh() / normalizer;
        for frame in samples.chunks_exact_mut(2) {
            for (channel, sample) in frame.iter_mut().enumerate() {
                let wet = self.saturation_oversamplers[channel].process(*sample, shape);
                let dry = self.saturation_dry[channel].process(*sample);
                *sample = dry * (1.0 - mix) + wet * mix;
            }
        }
    }

    /// Stereo-linked peak limiter with instant attack and exponential
    /// release. Unlike clipping, the gain rides over whole waveform cycles,
    /// so sustained overs are attenuated without adding distortion.
    fn apply_limiter(&mut self, samples: &mut [f32], sr: f32) {
        let ceiling = db_to_gain(self.config.limiter.ceiling_db.clamp(-24.0, 0.0));
        let release = time_coefficient(self.config.limiter.release_ms.clamp(1.0, 2_000.0), sr);
        for frame in samples.chunks_exact_mut(2) {
            let peak = frame[0].abs().max(frame[1].abs());
            let required = if peak > ceiling { ceiling / peak } else { 1.0 };
            let recovered = 1.0 - (1.0 - self.limiter_gain) * release;
            self.limiter_gain = required.min(recovered);
            // The clamp only catches rounding; the gain already respects it.
            frame[0] = (frame[0] * self.limiter_gain).clamp(-ceiling, ceiling);
            frame[1] = (frame[1] * self.limiter_gain).clamp(-ceiling, ceiling);
        }
    }

    /// Stereo chorus: the right channel LFO runs 90 degrees ahead of the left
    /// one for a wide image, and the modulated delay is read with cubic
    /// Hermite interpolation (linear interpolation dulled the highs and added
    /// modulation noise).
    fn apply_chorus(&mut self, samples: &mut [f32], sr: f32) {
        let len = self.chorus_buf_l.len();
        if len < 8 {
            return;
        }
        let max_delay = (len - 4) as f32;
        let base = (sr * 0.020).min(max_delay * 0.5);
        let depth =
            (self.config.chorus.depth_ms.clamp(0.1, 25.0) * sr / 1_000.0).min(max_delay - base);
        let mix = self.config.chorus.mix.clamp(0.0, 1.0);
        let phase_step = std::f32::consts::TAU * self.config.chorus.rate_hz.clamp(0.05, 8.0) / sr;
        let read = |buffer: &[f32], position: usize, delay: f32| -> f32 {
            let index = position as f32 + len as f32 - delay;
            let whole = index.floor();
            let fraction = index - whole;
            let i1 = whole as usize % len;
            let i0 = (i1 + len - 1) % len;
            let i2 = (i1 + 1) % len;
            let i3 = (i1 + 2) % len;
            hermite(buffer[i0], buffer[i1], buffer[i2], buffer[i3], fraction)
        };
        for frame in samples.chunks_exact_mut(2) {
            let delay_l = base + (self.chorus_phase.sin() * 0.5 + 0.5) * depth;
            let delay_r =
                base + ((self.chorus_phase + CHORUS_STEREO_PHASE).sin() * 0.5 + 0.5) * depth;
            let wet_l = read(&self.chorus_buf_l, self.chorus_pos, delay_l);
            let wet_r = read(&self.chorus_buf_r, self.chorus_pos, delay_r);
            self.chorus_buf_l[self.chorus_pos] = frame[0];
            self.chorus_buf_r[self.chorus_pos] = frame[1];
            self.chorus_pos = (self.chorus_pos + 1) % len;
            self.chorus_phase = (self.chorus_phase + phase_step) % std::f32::consts::TAU;
            frame[0] = frame[0] * (1.0 - mix) + wet_l * mix;
            frame[1] = frame[1] * (1.0 - mix) + wet_r * mix;
        }
    }

    /// Constant-power auto-pan normalised to unity at the centre. The old law
    /// lost 3 dB whenever the effect was enabled.
    fn apply_auto_pan(&mut self, samples: &mut [f32], sr: f32) {
        let depth = self.config.auto_pan.depth.clamp(0.0, 1.0);
        let step = std::f32::consts::TAU * self.config.auto_pan.rate_hz.clamp(0.05, 8.0) / sr;
        for frame in samples.chunks_exact_mut(2) {
            let pan = self.auto_pan_phase.sin() * depth;
            frame[0] *= (1.0 - pan).sqrt();
            frame[1] *= (1.0 + pan).sqrt();
            self.auto_pan_phase = (self.auto_pan_phase + step) % std::f32::consts::TAU;
        }
    }

    /// Feed-forward compressor with a stereo-linked peak detector, a soft
    /// knee and smooth branching attack/release applied to the gain in dB
    /// (Giannoulis, Massberg & Reiss, JAES 2012). The output is no longer
    /// hard-clipped at 0 dBFS; ceilings belong to the limiter.
    fn apply_compressor(&mut self, samples: &mut [f32], sr: f32) {
        let settings = &self.config.compressor;
        let threshold_db = settings.threshold_db.clamp(-60.0, 0.0);
        let ratio = settings.ratio.clamp(1.0, 100.0);
        let knee_db = settings.knee_db.clamp(0.0, 24.0);
        let makeup_db = settings.makeup_gain_db.clamp(-24.0, 24.0);
        let slope = 1.0 - 1.0 / ratio;
        let attack = time_coefficient(settings.attack_ms.clamp(0.05, 500.0), sr);
        let release = time_coefficient(settings.release_ms.clamp(1.0, 5_000.0), sr);
        // Below the knee the static curve is 0 dB, so the logarithm can be
        // skipped for quiet passages.
        let knee_start = db_to_gain(threshold_db - knee_db * 0.5);

        for frame in samples.chunks_exact_mut(2) {
            let peak = frame[0].abs().max(frame[1].abs());
            let target_db = if peak > knee_start {
                compressor_gain_db(20.0 * peak.log10(), threshold_db, slope, knee_db)
            } else {
                0.0
            };
            let coefficient = if target_db < self.comp_gain_db {
                attack
            } else {
                release
            };
            self.comp_gain_db = target_db + (self.comp_gain_db - target_db) * coefficient;
            let gain = db_to_gain(self.comp_gain_db + makeup_db);
            frame[0] *= gain;
            frame[1] *= gain;
        }
    }

    /// Noise gate with a peak-hold detector, open/close hysteresis and a hold
    /// time, so signals hovering around the threshold (or the zero crossings
    /// of low notes) no longer make the gain chatter.
    fn apply_gate(&mut self, samples: &mut [f32], sr: f32) {
        let open_threshold = db_to_gain(self.config.gate.threshold_db.clamp(-80.0, 0.0));
        let close_threshold = open_threshold * db_to_gain(-GATE_HYSTERESIS_DB);
        let attack = time_coefficient(self.config.gate.attack_ms.clamp(0.1, 100.0), sr);
        let release = time_coefficient(self.config.gate.release_ms.clamp(5.0, 2_000.0), sr);
        let detector_release = time_coefficient(GATE_DETECTOR_RELEASE_MS, sr);
        let hold_frames = (GATE_HOLD_MS * 0.001 * sr) as u32;
        for frame in samples.chunks_exact_mut(2) {
            let peak = frame[0].abs().max(frame[1].abs());
            self.gate_envelope = peak.max(self.gate_envelope * detector_release);
            if self.gate_envelope >= open_threshold
                || (self.gate_open && self.gate_envelope >= close_threshold)
            {
                self.gate_open = true;
                self.gate_hold = hold_frames;
            } else if self.gate_hold > 0 {
                self.gate_hold -= 1;
            } else {
                self.gate_open = false;
            }
            let target = if self.gate_open { 1.0 } else { 0.0 };
            let coefficient = if target > self.gate_gain {
                attack
            } else {
                release
            };
            self.gate_gain = target + (self.gate_gain - target) * coefficient;
            frame[0] *= self.gate_gain;
            frame[1] *= self.gate_gain;
        }
    }

    fn apply_delay(&mut self, samples: &mut [f32], sr: f32) {
        let delay_samples = (self.config.delay.time_ms * 0.001 * sr).round() as usize;
        let buf_len = self.delay_buf_l.len();
        if delay_samples == 0 || delay_samples >= buf_len {
            return;
        }

        let fb = self.config.delay.feedback.clamp(0.0, 0.95);
        let wet = self.config.delay.wet_level.clamp(0.0, 1.0);
        let dry = 1.0 - wet * 0.5;

        for chunk in samples.chunks_exact_mut(2) {
            let in_l = chunk[0];
            let in_r = chunk[1];

            let read_pos = (self.delay_pos + buf_len - delay_samples) % buf_len;
            let delayed_l = self.delay_buf_l[read_pos];
            let delayed_r = self.delay_buf_r[read_pos];

            let (next_del_l, next_del_r) = if self.config.delay.ping_pong {
                (in_l + delayed_r * fb, in_r + delayed_l * fb)
            } else {
                (in_l + delayed_l * fb, in_r + delayed_r * fb)
            };

            self.delay_buf_l[self.delay_pos] = flush_denormal(next_del_l);
            self.delay_buf_r[self.delay_pos] = flush_denormal(next_del_r);
            self.delay_pos = (self.delay_pos + 1) % buf_len;

            chunk[0] = in_l * dry + delayed_l * wet;
            chunk[1] = in_r * dry + delayed_r * wet;
        }
    }

    /// Freeverb with an optional pre-delay before the tank. Comb and allpass
    /// states are flushed to zero once they decay below audibility, so long
    /// tails do not fall into slow subnormal arithmetic.
    fn apply_reverb(&mut self, samples: &mut [f32], sr: f32) {
        let feedback = 0.7 + self.config.reverb.room_size.clamp(0.0, 1.0) * 0.28;
        let damp = self.config.reverb.damping.clamp(0.0, 1.0) * 0.4;
        let wet = self.config.reverb.wet_level * 0.5;
        let dry = self.config.reverb.dry_level;
        let width = self.config.reverb.width.clamp(0.0, 1.0);
        let predelay_len = self.reverb_predelay.len();
        let predelay = ((self.config.reverb.pre_delay_ms.clamp(0.0, 200.0) * 0.001 * sr).round()
            as usize)
            .min(predelay_len - 1);

        for chunk in samples.chunks_exact_mut(2) {
            let mut input = (chunk[0] + chunk[1]) * 0.015;
            if predelay > 0 {
                let read = (self.reverb_predelay_pos + predelay_len - predelay) % predelay_len;
                let delayed = self.reverb_predelay[read];
                self.reverb_predelay[self.reverb_predelay_pos] = input;
                self.reverb_predelay_pos = (self.reverb_predelay_pos + 1) % predelay_len;
                input = delayed;
            }

            // Process Combs L
            let mut out_l = 0.0f32;
            for (buf, pos, filter_store) in &mut self.reverb_combs_l {
                let blen = buf.len();
                let output = buf[*pos];
                *filter_store = flush_denormal(output * (1.0 - damp) + (*filter_store) * damp);
                buf[*pos] = flush_denormal(input + (*filter_store) * feedback);
                *pos = (*pos + 1) % blen;
                out_l += output;
            }

            // Process Combs R
            let mut out_r = 0.0f32;
            for (buf, pos, filter_store) in &mut self.reverb_combs_r {
                let blen = buf.len();
                let output = buf[*pos];
                *filter_store = flush_denormal(output * (1.0 - damp) + (*filter_store) * damp);
                buf[*pos] = flush_denormal(input + (*filter_store) * feedback);
                *pos = (*pos + 1) % blen;
                out_r += output;
            }

            // Allpass filters L
            for (buf, pos) in &mut self.reverb_allpass_l {
                let blen = buf.len();
                let buf_out = buf[*pos];
                let ap_in = out_l;
                let ap_out = -ap_in + buf_out;
                buf[*pos] = flush_denormal(ap_in + buf_out * 0.5);
                *pos = (*pos + 1) % blen;
                out_l = ap_out;
            }

            // Allpass filters R
            for (buf, pos) in &mut self.reverb_allpass_r {
                let blen = buf.len();
                let buf_out = buf[*pos];
                let ap_in = out_r;
                let ap_out = -ap_in + buf_out;
                buf[*pos] = flush_denormal(ap_in + buf_out * 0.5);
                *pos = (*pos + 1) % blen;
                out_r = ap_out;
            }

            let wet_1 = wet * (1.0 + width);
            let wet_2 = wet * (1.0 - width);

            chunk[0] = chunk[0] * dry + out_l * wet_1 + out_r * wet_2;
            chunk[1] = chunk[1] * dry + out_r * wet_1 + out_l * wet_2;
        }
    }
}
