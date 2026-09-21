/// Biquad filter state (Direct Form II Transposed)
#[derive(Clone, Copy, Default)]
pub(super) struct BiquadState {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1_l: f32,
    z2_l: f32,
    z1_r: f32,
    z2_r: f32,
}

impl BiquadState {
    /// Compute peaking EQ filter coefficients
    pub(super) fn set_peaking(&mut self, freq: f32, gain_db: f32, q: f32, sample_rate: f32) {
        if gain_db.abs() < 0.05 {
            self.b0 = 1.0;
            self.b1 = 0.0;
            self.b2 = 0.0;
            self.a1 = 0.0;
            self.a2 = 0.0;
            return;
        }

        let nyquist = sample_rate * 0.499;
        let f0 = freq.clamp(10.0, nyquist);
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * f0 / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;

        let inv_a0 = 1.0 / a0;
        self.b0 = b0 * inv_a0;
        self.b1 = b1 * inv_a0;
        self.b2 = b2 * inv_a0;
        self.a1 = a1 * inv_a0;
        self.a2 = a2 * inv_a0;
    }

    #[inline(always)]
    pub(super) fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        // Direct Form II Transposed
        let out_l = self.b0 * in_l + self.z1_l;
        self.z1_l = self.b1 * in_l - self.a1 * out_l + self.z2_l;
        self.z2_l = self.b2 * in_l - self.a2 * out_l;

        let out_r = self.b0 * in_r + self.z1_r;
        self.z1_r = self.b1 * in_r - self.a1 * out_r + self.z2_r;
        self.z2_r = self.b2 * in_r - self.a2 * out_r;

        (out_l, out_r)
    }
}
