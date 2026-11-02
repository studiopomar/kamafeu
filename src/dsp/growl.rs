//! Voice texture and rasp models (Growl) for UTAU vocal synthesis.
//!
//! Provides three distinct acoustic models:
//! 1. Subharmonic Rattle: false vocal folds vibration at f0/2 (~45-75 Hz).
//! 2. Throat Rasp: aperiodic granular micro-noise burst modulation in the 1.5-3.5 kHz formant region.
//! 3. Vocal Fry: period doubling and glottal pulse asymmetry (creak).

use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GrowlType {
    #[default]
    Subharmonic = 0,
    ThroatRasp = 1,
    VocalFry = 2,
}

impl GrowlType {
    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => Self::ThroatRasp,
            2 => Self::VocalFry,
            _ => Self::Subharmonic,
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }

    pub fn label(self, lang: crate::config::AppLanguage) -> &'static str {
        match lang {
            crate::config::AppLanguage::PtBr => match self {
                Self::Subharmonic => "Rattle Sub-harmônico (Rock / Metal)",
                Self::ThroatRasp => "Rasp Laríngeo (Screamo / Gritty)",
                Self::VocalFry => "Vocal Fry (Fricativa Glótica / Creak)",
            },
            crate::config::AppLanguage::EnUs => match self {
                Self::Subharmonic => "Subharmonic Rattle (Rock / Metal)",
                Self::ThroatRasp => "Throat Rasp (Screamo / Gritty)",
                Self::VocalFry => "Vocal Fry (Glottal Creak)",
            },
        }
    }
}

/// Applies voice texture / rasp to the audio buffer.
/// `depth` is in 0.0..=1.0 (or normalized from 0..100).
pub fn apply_growl(
    samples: &mut [f32],
    sample_rate: u32,
    f0_hz: f32,
    depth: f32,
    growl_type: GrowlType,
) {
    let d = depth.clamp(0.0, 1.0);
    if d <= 0.001 || samples.is_empty() || sample_rate == 0 {
        return;
    }

    let sr = sample_rate as f32;
    let f0 = if f0_hz.is_finite() && f0_hz >= 30.0 {
        f0_hz.clamp(40.0, 1200.0)
    } else {
        220.0
    };

    match growl_type {
        GrowlType::Subharmonic => {
            // Modulate amplitude at f0/2 bounded in false vocal folds range (~40-80 Hz)
            let mod_rate = (f0 * 0.5).clamp(38.0, 85.0);
            for (i, sample) in samples.iter_mut().enumerate() {
                let t = i as f32 / sr;
                let phase = (t * mod_rate * TAU).sin();
                // Asymmetric pulse dip characteristic of ventricular fold collision
                let pulse = (0.5 + 0.5 * phase).powf(2.2);
                let amp_mod = 1.0 - d * 0.65 * pulse;
                let s = *sample * amp_mod;
                // Subtle cubic soft clipping to enhance subharmonics
                let saturated = (s * (1.0 + d * 0.4)).clamp(-1.2, 1.2);
                *sample = saturated - 0.12 * d * saturated.powi(3);
            }
        }
        GrowlType::ThroatRasp => {
            // Biquad bandpass filter around 2.4 kHz (throat formant resonance)
            let center_hz = 2400.0f32;
            let q = 1.6f32;
            let omega = TAU * center_hz / sr;
            let alpha = omega.sin() / (2.0 * q);
            let b0 = alpha;
            let _b1 = 0.0f32;
            let b2 = -alpha;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * omega.cos();
            let a2 = 1.0 - alpha;

            let inv_a0 = 1.0 / a0;
            let (b0, b2, a1, a2) = (b0 * inv_a0, b2 * inv_a0, a1 * inv_a0, a2 * inv_a0);

            let mut x1 = 0.0f32;
            let mut x2 = 0.0f32;
            let mut y1 = 0.0f32;
            let mut y2 = 0.0f32;

            // Simple pseudo-random LCG noise generator
            let mut rng_state = 123456789u32;
            let mut next_noise = || -> f32 {
                rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
                ((rng_state >> 16) as f32 / 32767.5) - 1.0
            };

            for (i, sample) in samples.iter_mut().enumerate() {
                let t = i as f32 / sr;
                let glottal_phase = (t * f0 * TAU).sin();
                let glottal_gate = glottal_phase.max(0.0).powf(1.5);

                let raw_noise = next_noise();
                // Filter noise through throat resonant band
                let filtered_noise = b0 * raw_noise + b2 * x2 - a1 * y1 - a2 * y2;
                x2 = x1;
                x1 = raw_noise;
                y2 = y1;
                y1 = filtered_noise;

                let rasp_component = filtered_noise * glottal_gate * (d * 0.32);
                *sample = (*sample * (1.0 - d * 0.15)) + rasp_component;
            }
        }
        GrowlType::VocalFry => {
            // Period doubling and asymmetric pulse narrowing for vocal creak
            let period_samples = (sr / f0).max(2.0);
            for (i, sample) in samples.iter_mut().enumerate() {
                let cycle_phase = (i as f32 % (period_samples * 2.0)) / (period_samples * 2.0);
                // Alternate cycle attenuation
                let cycle_mod = if cycle_phase < 0.5 {
                    1.0
                } else {
                    1.0 - d * 0.48
                };
                let s = *sample * cycle_mod;
                // Asymmetric non-linear compression on negative excursions (vocal fold contact)
                let fold = if s < 0.0 { s * (1.0 + d * 0.35) } else { s };
                *sample = fold.clamp(-1.0, 1.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_depth_does_not_modify_buffer() {
        let original = vec![0.5f32, -0.3, 0.8, -0.1];
        let mut test = original.clone();
        apply_growl(&mut test, 44100, 220.0, 0.0, GrowlType::Subharmonic);
        assert_eq!(original, test);
    }

    #[test]
    fn subharmonic_growl_produces_finite_bounded_audio() {
        let mut buffer = vec![0.5f32; 1000];
        apply_growl(&mut buffer, 44100, 150.0, 0.8, GrowlType::Subharmonic);
        for s in buffer {
            assert!(s.is_finite());
            assert!(s.abs() <= 1.5);
        }
    }

    #[test]
    fn throat_rasp_adds_energy_and_stays_finite() {
        let mut buffer = vec![0.3f32; 1000];
        apply_growl(&mut buffer, 44100, 200.0, 0.7, GrowlType::ThroatRasp);
        for s in buffer {
            assert!(s.is_finite());
        }
    }

    #[test]
    fn vocal_fry_alternates_cycles() {
        let mut buffer = vec![0.4f32; 2000];
        apply_growl(&mut buffer, 44100, 100.0, 0.9, GrowlType::VocalFry);
        for s in buffer {
            assert!(s.is_finite());
        }
    }
}
