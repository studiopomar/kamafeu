use super::equalizer::Equalizer;
use super::FxRackConfig;

/// State for audio DSP processing
pub struct FxRackProcessor {
    pub config: FxRackConfig,
    // 31-band biquads
    equalizer: Equalizer,
    // Compressor internal state
    comp_envelope: f32,
    // Delay buffer
    delay_buf_l: Vec<f32>,
    delay_buf_r: Vec<f32>,
    delay_pos: usize,
    // Reverb comb & allpass buffers
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
            equalizer,
            comp_envelope: 0.0,
            delay_buf_l: vec![0.0f32; max_delay],
            delay_buf_r: vec![0.0f32; max_delay],
            delay_pos: 0,
            reverb_combs_l: combs_l,
            reverb_combs_r: combs_r,
            reverb_allpass_l: ap_l,
            reverb_allpass_r: ap_r,
        }
    }

    /// Process interleaved stereo audio samples in-place
    pub fn process_interleaved(&mut self, samples: &mut [f32], sample_rate: u32) {
        if !self.config.master_enabled {
            return;
        }

        let sr = sample_rate.max(8000) as f32;

        // 1. 31-Band Graphic Equalizer
        if self.config.eq.enabled {
            self.equalizer.process(samples, &self.config.eq.gains);
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
            self.apply_reverb(samples);
        }
    }

    fn apply_compressor(&mut self, samples: &mut [f32], sr: f32) {
        let thresh_lin = 10.0f32.powf(self.config.compressor.threshold_db / 20.0);
        let ratio = self.config.compressor.ratio.max(1.0);
        let makeup = 10.0f32.powf(self.config.compressor.makeup_gain_db / 20.0);

        let attack_coeff = (-1.0 / (self.config.compressor.attack_ms * 0.001 * sr).max(1.0)).exp();
        let release_coeff =
            (-1.0 / (self.config.compressor.release_ms * 0.001 * sr).max(1.0)).exp();

        for chunk in samples.chunks_exact_mut(2) {
            let peak = chunk[0].abs().max(chunk[1].abs());

            let coeff = if peak > self.comp_envelope {
                attack_coeff
            } else {
                release_coeff
            };
            self.comp_envelope = self.comp_envelope * coeff + peak * (1.0 - coeff);

            let gain = if self.comp_envelope > thresh_lin {
                let over_db = 20.0 * (self.comp_envelope / thresh_lin).log10();
                let compressed_over_db = over_db / ratio;
                let reduction_db = compressed_over_db - over_db;
                10.0f32.powf(reduction_db / 20.0)
            } else {
                1.0
            };

            chunk[0] = (chunk[0] * gain * makeup).clamp(-1.0, 1.0);
            chunk[1] = (chunk[1] * gain * makeup).clamp(-1.0, 1.0);
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

            self.delay_buf_l[self.delay_pos] = next_del_l;
            self.delay_buf_r[self.delay_pos] = next_del_r;
            self.delay_pos = (self.delay_pos + 1) % buf_len;

            chunk[0] = in_l * dry + delayed_l * wet;
            chunk[1] = in_r * dry + delayed_r * wet;
        }
    }

    fn apply_reverb(&mut self, samples: &mut [f32]) {
        let feedback = 0.7 + self.config.reverb.room_size * 0.28;
        let damp = self.config.reverb.damping * 0.4;
        let wet = self.config.reverb.wet_level * 0.5;
        let dry = self.config.reverb.dry_level;
        let width = self.config.reverb.width;

        for chunk in samples.chunks_exact_mut(2) {
            let input = (chunk[0] + chunk[1]) * 0.015;

            // Process Combs L
            let mut out_l = 0.0f32;
            for (buf, pos, filter_store) in &mut self.reverb_combs_l {
                let blen = buf.len();
                let output = buf[*pos];
                *filter_store = output * (1.0 - damp) + (*filter_store) * damp;
                buf[*pos] = input + (*filter_store) * feedback;
                *pos = (*pos + 1) % blen;
                out_l += output;
            }

            // Process Combs R
            let mut out_r = 0.0f32;
            for (buf, pos, filter_store) in &mut self.reverb_combs_r {
                let blen = buf.len();
                let output = buf[*pos];
                *filter_store = output * (1.0 - damp) + (*filter_store) * damp;
                buf[*pos] = input + (*filter_store) * feedback;
                *pos = (*pos + 1) % blen;
                out_r += output;
            }

            // Allpass filters L
            for (buf, pos) in &mut self.reverb_allpass_l {
                let blen = buf.len();
                let buf_out = buf[*pos];
                let ap_in = out_l;
                let ap_out = -ap_in + buf_out;
                buf[*pos] = ap_in + buf_out * 0.5;
                *pos = (*pos + 1) % blen;
                out_l = ap_out;
            }

            // Allpass filters R
            for (buf, pos) in &mut self.reverb_allpass_r {
                let blen = buf.len();
                let buf_out = buf[*pos];
                let ap_in = out_r;
                let ap_out = -ap_in + buf_out;
                buf[*pos] = ap_in + buf_out * 0.5;
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
