use crate::project::model::{UNote, UPitchBend, UPitchBendPoint};

#[derive(Debug, Clone)]
pub struct PitchReference {
    pub source: String,
    pub frame_period_ms: f64,
    pub f0_hz: Vec<f32>,
    pub voiced_frames: usize,
}

impl PitchReference {
    pub fn analyze_wav(path: &std::path::Path) -> Result<Self, String> {
        let mut reader = hound::WavReader::open(path)
            .map_err(|error| format!("Não foi possível abrir o áudio: {error}"))?;
        let spec = reader.spec();
        if spec.sample_rate == 0 || spec.channels == 0 {
            return Err("O WAV tem taxa de amostragem ou canais inválidos.".into());
        }
        let channels = spec.channels as usize;
        let mut mono = Vec::new();
        match spec.sample_format {
            hound::SampleFormat::Float => {
                let mut accum = 0.0f32;
                for (index, sample) in reader.samples::<f32>().enumerate() {
                    accum += sample.map_err(|error| format!("WAV inválido: {error}"))?;
                    if index % channels == channels - 1 {
                        mono.push(accum / channels as f32);
                        accum = 0.0;
                    }
                }
            }
            hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
                let mut accum = 0.0f32;
                let scale = (1u32 << (spec.bits_per_sample - 1)) as f32;
                for (index, sample) in reader.samples::<i16>().enumerate() {
                    accum +=
                        sample.map_err(|error| format!("WAV inválido: {error}"))? as f32 / scale;
                    if index % channels == channels - 1 {
                        mono.push(accum / channels as f32);
                        accum = 0.0;
                    }
                }
            }
            hound::SampleFormat::Int => {
                let mut accum = 0.0f32;
                let scale = (1u64 << (spec.bits_per_sample.saturating_sub(1) as u32)) as f32;
                for (index, sample) in reader.samples::<i32>().enumerate() {
                    accum +=
                        sample.map_err(|error| format!("WAV inválido: {error}"))? as f32 / scale;
                    if index % channels == channels - 1 {
                        mono.push(accum / channels as f32);
                        accum = 0.0;
                    }
                }
            }
        }
        if mono.len() < spec.sample_rate as usize / 10 {
            return Err("O áudio precisa ter pelo menos 100 ms.".into());
        }
        // Pitch tracking does not need the full playback sample rate. Averaging
        // short blocks keeps the existing WORLD-style autocorrelation bounded
        // for long reference files while retaining the vocal F0 range.
        let decimation = ((spec.sample_rate as f32 / 11_025.0).ceil() as usize).max(1);
        let analysis_samples: Vec<f32> = mono
            .chunks(decimation)
            .map(|block| block.iter().sum::<f32>() / block.len() as f32)
            .collect();
        let analysis_rate = spec.sample_rate / decimation as u32;
        let extractor = crate::dsp::world_analysis::WorldF0Extractor {
            min_f0: 55.0,
            max_f0: 1_000.0,
            frame_period_ms: 10.0,
        };
        let (f0_hz, _voiced) = extractor.extract_f0(&analysis_samples, analysis_rate);
        let voiced_frames = f0_hz
            .iter()
            .filter(|&&f0| f0.is_finite() && f0 > 0.0)
            .count();
        if voiced_frames < 3 {
            return Err("Não encontrei uma sequência vocal vozeada suficiente nesse áudio.".into());
        }
        Ok(Self {
            source: path.display().to_string(),
            frame_period_ms: extractor.frame_period_ms as f64,
            f0_hz,
            voiced_frames,
        })
    }

    pub fn apply_to_notes(&self, notes: &mut [UNote], selected_indices: Option<&[usize]>) -> usize {
        let mut applied = 0;
        for (index, note) in notes.iter_mut().enumerate() {
            if selected_indices.is_some_and(|selected| !selected.contains(&index)) {
                continue;
            }
            let midi = note.midi_key() as f64;
            let frame_start = (note.position_ms / self.frame_period_ms).floor().max(0.0) as usize;
            let frame_end = ((note.position_ms + note.duration_ms) / self.frame_period_ms)
                .ceil()
                .max(0.0) as usize;
            let mut points = Vec::new();
            for frame in (frame_start..frame_end.min(self.f0_hz.len())).step_by(2) {
                let f0 = self.f0_hz[frame];
                if !f0.is_finite() || f0 <= 0.0 {
                    continue;
                }
                let cents = (1200.0 * ((f0 as f64 / 440.0).log2() - (midi - 69.0) / 12.0))
                    .clamp(-1_200.0, 1_200.0);
                let offset = frame as f64 * self.frame_period_ms - note.position_ms;
                if offset <= note.duration_ms {
                    points.push(UPitchBendPoint {
                        time_offset_ms: offset,
                        pitch_offset_cents: cents,
                        shape: "s".into(),
                    });
                }
            }
            if points.len() < 2 {
                continue;
            }
            let simplified =
                crate::dsp::pitch_bend::PitchBendSolver::simplify_pitch_points(&points, 8.0);
            note.pitch_bend = UPitchBend {
                points: simplified,
                ..UPitchBend::default()
            };
            applied += 1;
        }
        applied
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_reference_maps_f0_to_note_pitch_bends() {
        let reference = PitchReference {
            source: "fixture.wav".into(),
            frame_period_ms: 10.0,
            f0_hz: vec![220.0; 80],
            voiced_frames: 80,
        };
        let mut notes = vec![UNote::new("a", "A3", 100.0, 300.0)];
        assert_eq!(reference.apply_to_notes(&mut notes, None), 1);
        assert!(notes[0].pitch_bend.points.len() >= 2);
        assert!(notes[0]
            .pitch_bend
            .points
            .iter()
            .all(|point| point.pitch_offset_cents.abs() < 0.1));
    }

    #[test]
    fn audio_reference_respects_selected_note_indices() {
        let reference = PitchReference {
            source: "fixture.wav".into(),
            frame_period_ms: 10.0,
            f0_hz: vec![220.0; 100],
            voiced_frames: 100,
        };
        let mut notes = vec![
            UNote::new("a", "A3", 0.0, 300.0),
            UNote::new("i", "A3", 400.0, 300.0),
        ];
        assert_eq!(reference.apply_to_notes(&mut notes, Some(&[1])), 1);
        assert!(notes[0].pitch_bend.points.is_empty());
        assert!(!notes[1].pitch_bend.points.is_empty());
    }

    #[test]
    fn wav_analysis_detects_a_stable_synthetic_vocal_tone() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tone.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 11_025,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for index in 0..(11_025 / 2) {
            let sample = (0.7
                * (std::f32::consts::TAU * 220.0 * index as f32 / 11_025.0).sin()
                * i16::MAX as f32) as i16;
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
        let reference = PitchReference::analyze_wav(&path).unwrap();
        assert!(reference.voiced_frames > 3);
        let mut estimates: Vec<f32> = reference
            .f0_hz
            .iter()
            .copied()
            .filter(|f0| *f0 > 0.0)
            .collect();
        estimates.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = estimates[estimates.len() / 2];
        assert!((median - 220.0).abs() < 10.0, "estimated f0: {median}");
    }
}
