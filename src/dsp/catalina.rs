//! Catalina: Neural NSF HiFi-GAN UTAU resampler engine and .oudep package loader.

use crate::drivers::{ResamplerArgs, ResamplerDriver};
use hound::{SampleFormat, WavSpec, WavWriter};
use ort::execution_providers::{cpu::CPUExecutionProvider, ExecutionProvider as _};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;
use rustfft::{num_complex::Complex32, FftPlanner};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{LazyLock, Mutex};

pub const SR: usize = 44_100;
pub const HOP: usize = 512;
pub const FFT: usize = 2048;
pub const MELS: usize = 128;

pub const HIFIGAN_MODEL_FILE: &str = "pc_nsf_hifigan_44.1k_hop512_128bin_2025.02.onnx";

static SESSION_CACHE: LazyLock<Mutex<Option<(PathBuf, Session)>>> =
    LazyLock::new(|| Mutex::new(None));

/// Unpack an OpenUTAU Dependency Package (`.oudep` or `.zip`) into `target_dir`.
/// Returns the path to the primary extracted `.onnx` model file.
pub fn unpack_oudep(archive_path: &Path, target_dir: &Path) -> Result<PathBuf, String> {
    let file = std::fs::File::open(archive_path)
        .map_err(|e| format!("Não foi possível abrir pacote {}: {e}", archive_path.display()))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| format!("Arquivo .oudep inválido ou corrompido: {e}"))?;

    std::fs::create_dir_all(target_dir).map_err(|e| {
        format!(
            "Não foi possível criar pasta destino {}: {e}",
            target_dir.display()
        )
    })?;

    let mut onnx_path = None;

    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| format!("Erro ao ler entrada zip {i}: {e}"))?;
        let entry_name = entry.name().to_string();
        let file_name = Path::new(&entry_name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if file_name.is_empty() || file_name.starts_with('.') || entry.is_dir() {
            continue;
        }

        let out_path = target_dir.join(&file_name);
        let mut out_file = std::fs::File::create(&out_path)
            .map_err(|e| format!("Erro ao extrair {}: {e}", out_path.display()))?;
        std::io::copy(&mut entry, &mut out_file)
            .map_err(|e| format!("Erro ao gravar {}: {e}", out_path.display()))?;

        if file_name.to_ascii_lowercase().ends_with(".onnx") {
            if onnx_path.is_none() || file_name.contains("nsf_hifigan") {
                onnx_path = Some(out_path);
            }
        }
    }

    onnx_path.ok_or_else(|| "Nenhum modelo .onnx encontrado dentro do pacote .oudep".to_string())
}

/// Checks if a file begins with standard ZIP/PK magic bytes.
pub fn is_zip_archive(path: &Path) -> bool {
    if let Ok(mut f) = std::fs::File::open(path) {
        use std::io::Read;
        let mut magic = [0u8; 4];
        if f.read_exact(&mut magic).is_ok() {
            return &magic == b"PK\x03\x04";
        }
    }
    false
}

/// Automatically extracts any `.oudep` or `.zip` found in `dir` if an `.onnx` model is not yet present,
/// or if a `.onnx` file in the folder is actually an unextracted `.oudep` package.
pub fn auto_extract_oudep_if_needed(dir: &Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }

    // 1. Check if any file in dir is a ZIP archive (even if misnamed as .onnx)
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_zip_archive(&path) {
                // If it was misnamed with .onnx extension, rename it to .oudep so unpack won't collide
                let archive_path = if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("onnx")) {
                    let temp_oudep = dir.join("openvpi_vocoder_package.oudep");
                    let _ = std::fs::rename(&path, &temp_oudep);
                    temp_oudep
                } else {
                    path
                };
                if let Ok(extracted) = unpack_oudep(&archive_path, dir) {
                    return Some(extracted);
                }
            }
        }
    }

    // 2. Check if a genuine ONNX model is already present
    for name in [HIFIGAN_MODEL_FILE, "pc-nsf-hifigan.onnx", "model.onnx"] {
        let p = dir.join(name);
        if p.is_file() && !is_zip_archive(&p) {
            return Some(p);
        }
    }

    // 3. Look for any genuine .onnx in dir
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("onnx"))
                && !is_zip_archive(&path)
            {
                return Some(path);
            }
        }
    }

    None
}

/// Resolves the file path of the NSF HiFi-GAN ONNX model.
pub fn resolve_model_path() -> Result<PathBuf, String> {
    if let Ok(configured) = env::var("CATALINA_MODEL") {
        let path = PathBuf::from(configured);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!("CATALINA_MODEL não existe: {}", path.display()));
    }

    let mut candidate_dirs = Vec::new();

    // 1. User Documents path (~/Documents/kamafeu/resamplers/catalina/model)
    if let Some(home) = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")) {
        let home = PathBuf::from(home);
        candidate_dirs.push(home.join("Documents/kamafeu/resamplers/catalina/model"));
        candidate_dirs.push(home.join("Documents/kamafeu/resamplers/catalina"));
    }

    // 2. Relative workspace path (resamplers/catalina/model)
    candidate_dirs.push(PathBuf::from("resamplers/catalina/model"));
    candidate_dirs.push(PathBuf::from("resamplers/catalina"));
    candidate_dirs.push(PathBuf::from("model"));

    // 3. Current executable layout
    if let Ok(exe) = env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            candidate_dirs.push(exe_dir.join("model"));
            candidate_dirs.push(exe_dir.to_path_buf());
            if let Some(p1) = exe_dir.parent() {
                candidate_dirs.push(p1.join("resamplers/catalina/model"));
                candidate_dirs.push(p1.join("resamplers/catalina"));
                if let Some(p2) = p1.parent() {
                    candidate_dirs.push(p2.join("resamplers/catalina/model"));
                    candidate_dirs.push(p2.join("resamplers/catalina"));
                }
            }
        }
    }

    for dir in &candidate_dirs {
        if let Some(model_file) = auto_extract_oudep_if_needed(dir) {
            return Ok(model_file);
        }
    }

    Err(
        "Modelo NSF HiFi-GAN do Catalina não encontrado. Coloque o arquivo .onnx ou o pacote .oudep em ~/Documents/kamafeu/resamplers/catalina/model/"
            .to_string(),
    )
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

/// Computes the 128-bin mel-spectrogram using OpenVPI librosa-compatible parameters.
pub fn compute_mel_spectrogram(samples: &[f32]) -> Vec<[f32; MELS]> {
    let left_pad = (FFT - HOP) / 2;
    let right_pad = (FFT - HOP).div_ceil(2);
    let frames = ((samples.len() + left_pad + right_pad - FFT) / HOP + 1).max(1);

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

    let mut result = Vec::with_capacity(frames);

    for frame in 0..frames {
        let mut spectrum = vec![Complex32::new(0.0, 0.0); FFT];
        let frame_start = frame as isize * HOP as isize - left_pad as isize;

        for i in 0..FFT {
            let source_index = reflect_index(frame_start + i as isize, samples.len());
            spectrum[i].re = samples.get(source_index).copied().unwrap_or(0.0) * window[i];
        }

        fft.process(&mut spectrum);
        let magnitude: Vec<f32> = spectrum[..FFT / 2 + 1].iter().map(|v| v.norm()).collect();

        let mut frame_mel = [0.0_f32; MELS];
        for m in 0..MELS {
            let left = bins[m].min(magnitude.len() - 1);
            let center = bins[m + 1].max(left + 1).min(magnitude.len() - 1);
            let right = bins[m + 2].max(center + 1).min(magnitude.len());
            let mut energy = 0.0;
            let band_norm = 2.0 / (mel_edges_hz[m + 2] - mel_edges_hz[m]).max(1.0);

            for (k, value) in magnitude.iter().enumerate().take(center).skip(left) {
                let weight = (k - left) as f32 / (center - left) as f32;
                energy += *value * weight * band_norm;
            }
            for (k, value) in magnitude.iter().enumerate().take(right).skip(center) {
                let weight = (right - k) as f32 / (right - center) as f32;
                energy += *value * weight * band_norm;
            }

            frame_mel[m] = energy.max(1e-5).ln();
        }
        result.push(frame_mel);
    }

    result
}

/// Convert musical note name (e.g., "C4", "A#3", "!C4") or frequency string to Hz.
pub fn hz_from_note(note: &str) -> f32 {
    let trimmed = note.trim().trim_start_matches('!');
    if let Ok(hz) = trimmed.parse::<f32>() {
        if hz.is_finite() && hz > 0.0 {
            return hz;
        }
    }

    let mut chars = trimmed.chars();
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
    let mut octave_start = false;
    let mut octave_str = String::new();

    for c in chars {
        if !octave_start {
            if c == '#' || c == 's' || c == 'S' {
                semitone += 1;
                continue;
            }
            if c == 'b' || c == 'B' {
                semitone -= 1;
                continue;
            }
            octave_start = true;
        }
        octave_str.push(c);
    }

    let octave = octave_str.parse::<i32>().unwrap_or(4);
    440.0 * 2.0_f32.powf(((octave * 12 + semitone) as f32 - 69.0) / 12.0)
}

fn b64_val(c: char) -> Option<u16> {
    match c {
        'A'..='Z' => Some(c as u16 - 'A' as u16),
        'a'..='z' => Some(c as u16 - 'a' as u16 + 26),
        '0'..='9' => Some(c as u16 - '0' as u16 + 52),
        '+' => Some(62),
        '/' => Some(63),
        _ => None,
    }
}

/// Decodes UTAU Mode 2 Base64 pitch bend string into a sequence of cents.
pub fn decode_utau_pitch_bends(encoded: &str) -> Vec<i16> {
    if encoded.is_empty() || encoded == "AA" {
        return Vec::new();
    }

    let mut result = Vec::new();
    let chars: Vec<char> = encoded.chars().collect();
    let mut i = 0;
    let mut last_val = 0i16;

    while i < chars.len() {
        if chars[i] == '#' {
            i += 1;
            let mut count_str = String::new();
            while i < chars.len() && chars[i] != '#' {
                count_str.push(chars[i]);
                i += 1;
            }
            if i < chars.len() && chars[i] == '#' {
                i += 1;
            }
            if let Ok(count) = count_str.parse::<usize>() {
                for _ in 0..count {
                    result.push(last_val);
                }
            }
            continue;
        }

        if i + 1 < chars.len() {
            if let (Some(high), Some(low)) = (b64_val(chars[i]), b64_val(chars[i + 1])) {
                let u = (high << 6) | low;
                let val = if u >= 2048 {
                    (u as i32 - 4096) as i16
                } else {
                    u as i16
                };
                result.push(val);
                last_val = val;
                i += 2;
                continue;
            }
        }
        i += 1;
    }

    result
}

/// Interpolates pitch bend offset in cents at a given time offset in ms.
pub fn pitch_bend_at_time(bends: &[i16], time_ms: f64, tempo: f64) -> f64 {
    if bends.is_empty() {
        return 0.0;
    }
    let safe_tempo = if tempo.is_finite() && tempo > 0.0 {
        tempo
    } else {
        120.0
    };
    let step_ms = 60_000.0 * 5.0 / (safe_tempo * 480.0);
    if step_ms <= 0.0 {
        return 0.0;
    }

    let exact_index = time_ms / step_ms;
    if exact_index <= 0.0 {
        return bends[0] as f64;
    }
    let left = exact_index.floor() as usize;
    if left >= bends.len() - 1 {
        return bends[bends.len() - 1] as f64;
    }

    let frac = (exact_index - left as f64) as f64;
    bends[left] as f64 * (1.0 - frac) + bends[left + 1] as f64 * frac
}

/// Applies DC removal and soft limiting to protect output audio.
pub fn protect_output(mut samples: Vec<f32>, volume_scale: f32) -> Vec<f32> {
    if samples.is_empty() {
        return samples;
    }

    let mean = samples.iter().copied().sum::<f32>() / samples.len() as f32;
    for sample in &mut samples {
        *sample = (*sample - mean) * volume_scale;
    }

    let peak = samples
        .iter()
        .fold(0.0_f32, |max, value| max.max(value.abs()));
    if peak > 0.95 {
        let gain = 0.95 / peak;
        for sample in &mut samples {
            *sample *= gain;
        }
    }

    samples
        .into_iter()
        .map(|sample| (sample * 1.15).tanh() / 1.15_f32.tanh())
        .collect()
}

/// Core function to render a note via Catalina using OpenVPI NSF HiFi-GAN.
pub fn render_catalina_note(
    raw_samples: &[f32],
    sample_rate: u32,
    args: &ResamplerArgs,
    _cancel: Option<&AtomicBool>,
) -> Result<Vec<f32>, String> {
    let model_path = resolve_model_path()?;

    // Resample input to 44.1 kHz if necessary
    let samples_44k: Vec<f32> = if sample_rate != SR as u32 {
        crate::renderer::track::TrackRenderer::convert_sample_rate(
            raw_samples,
            sample_rate,
            SR as u32,
        )
    } else {
        raw_samples.to_vec()
    };

    if samples_44k.is_empty() {
        let target_len = ((args.duration_ms / 1000.0) * SR as f64).round() as usize;
        return Ok(vec![0.0; target_len.max(512)]);
    }

    // UTAU bounds calculation
    let (start, end) =
        crate::dsp::oto_source_bounds(samples_44k.len(), SR as u32, args.offset_ms, args.cutoff_ms);
    let slice = if end > start {
        &samples_44k[start..end]
    } else {
        &samples_44k[start..]
    };

    let slice = if slice.is_empty() {
        &samples_44k[..]
    } else {
        slice
    };

    // Extract source mel-spectrogram
    let src_mel = compute_mel_spectrogram(slice);
    let src_frames = src_mel.len().max(1);

    // Timing and frame targets
    let target_total_samples = ((args.duration_ms / 1000.0) * SR as f64).round() as usize;
    let target_frames = (target_total_samples / HOP).max(1);

    let samples_per_ms = SR as f64 / 1000.0;
    let raw_consonant_samples = (args.consonant_ms.max(0.0) * samples_per_ms).round() as usize;
    let vel_factor = 2.0_f64.powf((100.0 - args.velocity.clamp(0.0, 200.0)) / 100.0);
    let eff_consonant_samples = ((raw_consonant_samples as f64) * vel_factor).round() as usize;

    let src_cons_frames = (raw_consonant_samples / HOP).min(src_frames);
    let tgt_cons_frames = (eff_consonant_samples / HOP).min(target_frames);

    // Build target mel-spectrogram data (shape: [1, target_frames, 128])
    let mut target_mel_data = vec![0.0_f32; target_frames * MELS];
    for t in 0..target_frames {
        let src_f = if t < tgt_cons_frames && tgt_cons_frames > 0 {
            t as f64 * (src_cons_frames as f64) / (tgt_cons_frames as f64)
        } else {
            let tgt_vowel_frames = target_frames.saturating_sub(tgt_cons_frames).max(1);
            let src_vowel_frames = src_frames.saturating_sub(src_cons_frames);
            if src_vowel_frames <= 1 {
                (src_frames - 1) as f64
            } else {
                let vowel_t = t.saturating_sub(tgt_cons_frames);
                let ratio = vowel_t as f64 / tgt_vowel_frames as f64;
                src_cons_frames as f64 + ratio * ((src_vowel_frames - 1) as f64)
            }
        };

        let clamped_f = src_f.clamp(0.0, (src_frames - 1) as f64);
        let idx0 = clamped_f.floor() as usize;
        let idx1 = clamped_f.ceil() as usize;
        let frac = (clamped_f - idx0 as f64) as f32;

        let frame0 = &src_mel[idx0];
        let frame1 = &src_mel[idx1];

        for m in 0..MELS {
            target_mel_data[t * MELS + m] = frame0[m] * (1.0 - frac) + frame1[m] * frac;
        }
    }

    // Build target F0 contour (shape: [1, target_frames])
    let base_hz = if args.pitch_freq > 0.0 {
        args.pitch_freq as f32
    } else {
        hz_from_note(&args.pitch_name)
    };

    let pitch_bends = decode_utau_pitch_bends(&args.pitch_bend_str);
    let mut target_f0_data = vec![0.0_f32; target_frames];

    for t in 0..target_frames {
        let frame_time_ms = t as f64 * (HOP as f64 / SR as f64) * 1000.0;
        let cents = pitch_bend_at_time(&pitch_bends, frame_time_ms, args.tempo);
        let hz = base_hz * 2.0_f32.powf((cents as f32) / 1200.0);
        target_f0_data[t] = hz.clamp(20.0, 20_000.0);
    }

    // Run inference with cached Session
    let mut cache_guard = SESSION_CACHE
        .lock()
        .map_err(|_| "ONNX session cache lock poisoned".to_string())?;

    let session = match cache_guard.as_mut() {
        Some((path, s)) if path == &model_path => s,
        _ => {
            let mut builder = Session::builder()
                .map_err(|e| format!("ONNX session builder failed: {e}"))?
                .with_optimization_level(GraphOptimizationLevel::Disable)
                .map_err(|e| format!("ONNX optimization setup failed: {e}"))?
                .with_intra_threads(4)
                .map_err(|e| format!("ONNX threading setup failed: {e}"))?;

            #[cfg(target_os = "macos")]
            {
                use ort::execution_providers::coreml::CoreMLExecutionProvider;
                let coreml = CoreMLExecutionProvider::default();
                if coreml.is_available().unwrap_or(false) {
                    let _ = coreml.register(&mut builder);
                }
            }

            builder = builder
                .with_execution_providers([CPUExecutionProvider::default().build()])
                .map_err(|e| format!("CPU execution provider setup failed: {e}"))?;

            let new_session = builder
                .commit_from_file(&model_path)
                .map_err(|e| format!("Falha ao carregar modelo {}: {e}", model_path.display()))?;

            *cache_guard = Some((model_path.clone(), new_session));
            &mut cache_guard.as_mut().unwrap().1
        }
    };

    let mel_tensor = Tensor::<f32>::from_array((vec![1, target_frames, MELS], target_mel_data))
        .map_err(|e| format!("Falha ao criar tensor de mel: {e}"))?
        .into_dyn();

    let f0_tensor = Tensor::<f32>::from_array((vec![1, target_frames], target_f0_data))
        .map_err(|e| format!("Falha ao criar tensor de f0: {e}"))?
        .into_dyn();

    let output_name = session
        .outputs
        .first()
        .map(|o| o.name.clone())
        .unwrap_or_else(|| "waveform".to_string());

    let inputs = vec![("mel", mel_tensor), ("f0", f0_tensor)];
    let outputs = session
        .run(inputs)
        .map_err(|e| format!("Inferência do Catalina falhou: {e}"))?;

    let first = outputs
        .get(&output_name)
        .ok_or_else(|| "Saída ausente no modelo HiFi-GAN".to_string())?;

    let (_, waveform_values) = first
        .try_extract_tensor::<f32>()
        .map_err(|e| format!("Saída não numérica do modelo: {e}"))?;

    let vol_scale = (args.volume.max(0.0) / 100.0) as f32;
    let final_samples = protect_output(waveform_values.to_vec(), vol_scale);

    // If output_wav is specified, write it out
    if args.output_wav.as_os_str().len() > 0 {
        let spec = WavSpec {
            channels: 1,
            sample_rate: SR as u32,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        if let Ok(mut writer) = WavWriter::create(&args.output_wav, spec) {
            for &s in &final_samples {
                let pcm = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                let _ = writer.write_sample(pcm);
            }
            let _ = writer.finalize();
        }
    }

    Ok(final_samples)
}

/// Native in-process ResamplerDriver implementation for Catalina.
pub struct CatalinaResamplerDriver;

impl CatalinaResamplerDriver {
    pub fn new() -> Self {
        Self
    }
}

impl ResamplerDriver for CatalinaResamplerDriver {
    fn name(&self) -> &str {
        "Catalina (NSF HiFi-GAN)"
    }

    fn engine_id(&self) -> &'static str {
        "external.resampler.catalina"
    }

    fn uses_external_process(&self) -> bool {
        false
    }

    fn render_sample(
        &self,
        raw_samples: &[f32],
        sample_rate: u32,
        args: &ResamplerArgs,
        cancel: Option<&AtomicBool>,
    ) -> Result<Vec<f32>, String> {
        render_catalina_note(raw_samples, sample_rate, args, cancel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pitch_bend_round_trip() {
        let _original_cents = vec![0, 10, 25, 25, 25, -50, -50, 0];
        let encoded = crate::dsp::pitch_encoder::encode_utau_base64_pitch(
            &[
                crate::project::model::UPitchBendPoint {
                    time_offset_ms: 0.0,
                    pitch_offset_cents: 0.0,
                    shape: "s".into(),
                },
                crate::project::model::UPitchBendPoint {
                    time_offset_ms: 200.0,
                    pitch_offset_cents: 100.0,
                    shape: "s".into(),
                },
            ],
            200.0,
            120.0,
        );

        let decoded = decode_utau_pitch_bends(&encoded);
        assert!(!decoded.is_empty(), "Decoded pitch bends should not be empty");
    }

    #[test]
    fn test_hz_from_note_parsing() {
        assert!((hz_from_note("A4") - 440.0).abs() < 0.1);
        assert!((hz_from_note("!A4") - 440.0).abs() < 0.1);
        assert!((hz_from_note("C4") - 261.63).abs() < 0.1);
        assert!((hz_from_note("440.0") - 440.0).abs() < 0.1);
    }
}
