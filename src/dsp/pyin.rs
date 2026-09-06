pub struct PitchResult {
    pub pitch_contour: Vec<f32>,
    pub gci_marks: Vec<usize>,
}

pub struct PitchExtractor;

impl PitchExtractor {
    pub fn extract_pitch_and_gci(vowel_slice: &[f32], sample_rate: u32) -> PitchResult {
        if vowel_slice.is_empty() {
            return PitchResult {
                pitch_contour: vec![],
                gci_marks: vec![],
            };
        }

        let min_period = (sample_rate as f64 / 1000.0) as usize; // 1000 Hz
        let max_period = (sample_rate as f64 / 50.0) as usize; // 50 Hz

        let frame_size = max_period * 2;
        let hop_size = max_period / 4;

        let mut pitch_contour = Vec::new();
        let mut gci_marks = Vec::new();

        if vowel_slice.len() < frame_size {
            let p0 = crate::dsp::Resampler::estimate_pitch_period(vowel_slice, sample_rate);
            pitch_contour.push(p0 as f32);
            let mut gci = 0;
            while gci < vowel_slice.len() {
                gci_marks.push(gci);
                gci += p0.max(16);
            }
            return PitchResult {
                pitch_contour,
                gci_marks,
            };
        }

        let mut pos = 0;
        let mut last_gci = 0;

        while pos + frame_size <= vowel_slice.len() {
            let frame = &vowel_slice[pos..pos + frame_size];

            let mut diff = vec![0.0f32; max_period + 1];
            for tau in min_period..=max_period {
                for i in 0..max_period {
                    let d = frame[i] - frame[i + tau];
                    diff[tau] += d * d;
                }
            }

            let mut cmnd = vec![1.0f32; max_period + 1];
            let mut running_sum = 0.0;
            for tau in 1..=max_period {
                running_sum += diff[tau];
                cmnd[tau] = diff[tau] * tau as f32 / running_sum.max(1e-6);
            }

            let threshold = 0.15;
            let mut best_tau = 0;
            for tau in min_period..=max_period {
                if cmnd[tau] < threshold {
                    let mut local_min_tau = tau;
                    let mut min_val = cmnd[tau];
                    for (t, &value) in cmnd.iter().enumerate().take(max_period + 1).skip(tau) {
                        if value < min_val {
                            min_val = value;
                            local_min_tau = t;
                        } else if value > min_val + 0.1 {
                            break;
                        }
                    }
                    best_tau = local_min_tau;
                    break;
                }
            }

            if best_tau == 0 {
                let mut min_val = f32::MAX;
                for (tau, &value) in cmnd
                    .iter()
                    .enumerate()
                    .take(max_period + 1)
                    .skip(min_period)
                {
                    if value < min_val {
                        min_val = value;
                        best_tau = tau;
                    }
                }
            }

            let period = best_tau.max(min_period);
            pitch_contour.push(period as f32);

            let expected_gci = last_gci + period;
            let search_window = period / 4;

            if expected_gci >= vowel_slice.len() {
                break;
            }

            let search_start = expected_gci.saturating_sub(search_window).max(last_gci + 1);
            let search_end = (expected_gci + search_window).min(vowel_slice.len() - 1);

            let mut max_energy = -1.0;
            let mut best_gci = expected_gci;

            for i in search_start..search_end {
                let energy = vowel_slice[i].abs() + (vowel_slice[i] - vowel_slice[i - 1]).abs();
                if energy > max_energy {
                    max_energy = energy;
                    best_gci = i;
                }
            }

            if best_gci > last_gci {
                gci_marks.push(best_gci);
                last_gci = best_gci;
            } else {
                last_gci += period; // force advance
            }

            pos += hop_size;
        }

        let last_p = *pitch_contour.last().unwrap_or(&(min_period as f32)) as usize;
        while last_gci + last_p < vowel_slice.len() {
            last_gci += last_p;
            gci_marks.push(last_gci);
        }

        PitchResult {
            pitch_contour,
            gci_marks,
        }
    }
}
