//! Native Catalina PC-NSF-HiFi-GAN UTAU resampler.

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use ndarray::{Array2, Array3};
use ort::{session::Session, value::TensorRef};
use rustfft::{num_complex::Complex32, FftPlanner};
use std::{env, error::Error, path::Path};

const SR: usize = 44_100;
const HOP: usize = 512;
const FFT: usize = 2048;
const MELS: usize = 128;

fn read_mono(path: &Path) -> Result<Vec<f32>, Box<dyn Error>> {
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();
    let mut samples = match spec.sample_format {
        SampleFormat::Float => reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?,
        SampleFormat::Int => reader
            .samples::<i32>()
            .map(|s| s.map(|v| v as f32 / 2.0_f32.powi(spec.bits_per_sample as i32 - 1)))
            .collect::<Result<Vec<_>, _>>()?,
    };
    if spec.channels > 1 {
        samples = samples
            .chunks(spec.channels as usize)
            .map(|c| c.iter().copied().sum::<f32>() / c.len() as f32)
            .collect();
    }
    if spec.sample_rate != SR as u32 {
        let target = (samples.len() as f64 * SR as f64 / spec.sample_rate as f64) as usize;
        samples = (0..target)
            .map(|i| {
                let p = i as f64 * samples.len().saturating_sub(1) as f64 / target.max(1) as f64;
                let a = p.floor() as usize;
                let f = (p - a as f64) as f32;
                samples[a.min(samples.len().saturating_sub(1))] * (1.0 - f)
                    + samples[(a + 1).min(samples.len().saturating_sub(1))] * f
            })
            .collect();
    }
    Ok(samples)
}

fn hz_from_note(note: &str) -> f32 {
    let mut chars = note.trim().trim_start_matches('!').chars();
    let base = match chars.next().unwrap_or('C').to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => 0,
    };
    let mut semitone = base;
    if let Some(acc) = chars.next() {
        if acc == '#' || acc == 's' {
            semitone += 1;
        }
        if acc == 'b' {
            semitone -= 1;
        }
    }
    let octave = chars.as_str().parse::<i32>().unwrap_or(4);
    440.0 * 2.0_f32.powf(((octave * 12 + semitone) as f32 - 69.0) / 12.0)
}

fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}
fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0)
}

fn reflect_index(index: isize, length: usize) -> usize {
    if length <= 1 {
        return 0;
    }
    let period = 2 * length as isize - 2;
    let mut value = index.rem_euclid(period);
    if value >= length as isize {
        value = period - value;
    }
    value as usize
}

fn features(samples: &[f32], f0: f32) -> (Array3<f32>, Array2<f32>) {
    // This matches OpenVPI's STFT frontend: center=false with reflection
    // padding around every frame. Zero padding changes the mel distribution
    // enough to make the neural vocoder emit hiss and unstable energy.
    let left_pad = (FFT - HOP) / 2;
    let right_pad = (FFT - HOP).div_ceil(2);
    let frames = ((samples.len() + left_pad + right_pad - FFT) / HOP + 1).max(1);
    let mut mel = Array3::<f32>::zeros((1, frames, MELS));
    let mut pitch = Array2::<f32>::zeros((1, frames));
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT);
    let min_mel = hz_to_mel(40.0);
    let max_mel = hz_to_mel(16_000.0);
    let mel_edges_hz: Vec<f32> = (0..MELS + 2)
        .map(|i| mel_to_hz(min_mel + (max_mel - min_mel) * i as f32 / (MELS + 1) as f32))
        .collect();
    let bins: Vec<usize> = mel_edges_hz
        .iter()
        .map(|hz| ((FFT + 1) as f32 * *hz / SR as f32).floor() as usize)
        .collect();
    let window: Vec<f32> = (0..FFT)
        .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (FFT - 1) as f32).cos())
        .collect();
    for frame in 0..frames {
        let mut spectrum = vec![Complex32::new(0.0, 0.0); FFT];
        let frame_start = frame as isize * HOP as isize - left_pad as isize;
        for i in 0..FFT {
            let source_index = reflect_index(frame_start + i as isize, samples.len());
            spectrum[i].re = samples.get(source_index).copied().unwrap_or(0.0) * window[i];
        }
        fft.process(&mut spectrum);
        // The official frontend feeds magnitude, not power, into the mel
        // filterbank, then applies natural-log compression (log E).
        let magnitude: Vec<f32> = spectrum[..FFT / 2 + 1].iter().map(|v| v.norm()).collect();
        for m in 0..MELS {
            let left = bins[m].min(magnitude.len() - 1);
            let center = bins[m + 1].max(left + 1).min(magnitude.len() - 1);
            let right = bins[m + 2].max(center + 1).min(magnitude.len());
            let mut energy = 0.0;
            // librosa's Slaney normalization used by OpenVPI: each triangular
            // filter is normalized by its bandwidth, not by the sum of its
            // sampled weights.
            let band_norm = 2.0 / (mel_edges_hz[m + 2] - mel_edges_hz[m]).max(1.0);
            for (k, value) in magnitude.iter().enumerate().take(center).skip(left) {
                let weight = (k - left) as f32 / (center - left) as f32;
                energy += *value * weight * band_norm;
            }
            for (k, value) in magnitude.iter().enumerate().take(right).skip(center) {
                let weight = (right - k) as f32 / (right - center) as f32;
                energy += *value * weight * band_norm;
            }
            mel[[0, frame, m]] = energy.max(1e-5).ln();
        }
        pitch[[0, frame]] = f0;
    }
    (mel, pitch)
}

fn write_wav(path: &Path, samples: &[f32]) -> Result<(), Box<dyn Error>> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: SR as u32,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)?;
    for sample in samples {
        writer.write_sample((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)?;
    }
    writer.finalize()?;
    Ok(())
}

fn protect_output(mut samples: Vec<f32>) -> Vec<f32> {
    if samples.is_empty() {
        return samples;
    }
    // ONNX vocoders are not required to emit a normalized waveform. Remove
    // DC first, then keep headroom for the wavtool/mixer instead of hard
    // clipping the output at the PCM conversion boundary.
    let mean = samples.iter().copied().sum::<f32>() / samples.len() as f32;
    for sample in &mut samples {
        *sample -= mean;
    }
    let peak = samples
        .iter()
        .fold(0.0_f32, |max, value| max.max(value.abs()));
    if peak > 0.89 {
        let gain = 0.89 / peak;
        for sample in &mut samples {
            *sample *= gain;
        }
    }
    samples
        .into_iter()
        .map(|sample| (sample * 1.15).tanh() / 1.15_f32.tanh())
        .collect()
}

fn fit_duration(samples: &[f32], duration_ms: Option<f64>) -> Vec<f32> {
    let Some(duration_ms) = duration_ms.filter(|value| value.is_finite() && *value > 1.0) else {
        return samples.to_vec();
    };
    let target = ((duration_ms / 1000.0) * SR as f64).round() as usize;
    if target == 0 || samples.len() == target {
        return samples.to_vec();
    }
    if samples.is_empty() {
        return vec![0.0; target];
    }
    (0..target)
        .map(|index| {
            let position = index as f64 * (samples.len() - 1) as f64 / (target - 1).max(1) as f64;
            let left = position.floor() as usize;
            let fraction = (position - left as f64) as f32;
            samples[left] * (1.0 - fraction) + samples[(left + 1).min(samples.len() - 1)] * fraction
        })
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() < 3 {
        return Err("uso: catalina input.wav output.wav nota [parâmetros UTAU...]".into());
    }
    let input = Path::new(&args[0]);
    let output = Path::new(&args[1]);
    let samples = read_mono(input)?;
    let (mel, f0) = features(&samples, hz_from_note(&args[2]));
    let model = resolve_model_path()?;
    let mut session = Session::builder()?
        .with_intra_threads(2)?
        .commit_from_file(model)?;
    let outputs = session.run(ort::inputs!["mel" => TensorRef::from_array_view(&mel)?, "f0" => TensorRef::from_array_view(&f0)?])?;
    let waveform = outputs["waveform"].try_extract_array::<f32>()?;
    // Classic UTAU arguments place the requested duration at index 6.
    // Matching it is essential when Catalina is used by progressive playback.
    let duration_ms = args.get(6).and_then(|value| value.parse::<f64>().ok());
    let fitted = fit_duration(&waveform.iter().copied().collect::<Vec<_>>(), duration_ms);
    write_wav(output, &protect_output(fitted))?;
    Ok(())
}

fn resolve_model_path() -> Result<String, Box<dyn Error>> {
    if let Ok(configured) = env::var("CATALINA_MODEL") {
        let path = Path::new(&configured);
        if path.is_file() {
            return Ok(configured);
        }
        return Err(format!("CATALINA_MODEL não existe: {}", path.display()).into());
    }

    let relative =
        Path::new("resamplers/catalina/model/pc_nsf_hifigan_44.1k_hop512_128bin_2025.02.onnx");
    let mut candidates = vec![relative.to_path_buf()];
    if let Ok(exe) = env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            // Installed layout: resamplers/catalina/catalina + model/.
            candidates.push(exe_dir.join("model/pc_nsf_hifigan_44.1k_hop512_128bin_2025.02.onnx"));
            // Development layout: target/{debug,release}/catalina.
            if let Some(root_dir) = exe_dir
                .parent()
                .and_then(Path::parent)
                .and_then(Path::parent)
            {
                candidates.push(root_dir.join(relative));
            }
        }
    }
    candidates.extend([
        Path::new("resamplers/catalina/model/pc-nsf-hifigan.onnx").to_path_buf(),
        Path::new("resamplers/catalina/model/model.onnx").to_path_buf(),
    ]);
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
        .ok_or_else(|| "Modelo NSF HiFi-GAN do Catalina não encontrado".into())
}
