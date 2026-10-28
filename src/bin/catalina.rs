//! Standalone UTAU-compatible executable for Catalina NSF HiFi-GAN resampler.

use hound::{SampleFormat, WavReader};
use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};

fn read_wav_samples(path: &Path) -> Result<(Vec<f32>, u32), Box<dyn Error>> {
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        SampleFormat::Float => reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?,
        SampleFormat::Int => reader
            .samples::<i32>()
            .map(|s| s.map(|v| v as f32 / 2.0_f32.powi(spec.bits_per_sample as i32 - 1)))
            .collect::<Result<Vec<_>, _>>()?,
    };

    let mono_samples = if spec.channels > 1 {
        samples
            .chunks(spec.channels as usize)
            .map(|chunk| chunk.iter().copied().sum::<f32>() / chunk.len() as f32)
            .collect()
    } else {
        samples
    };

    Ok((mono_samples, spec.sample_rate))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() < 3 {
        eprintln!("Uso: catalina input.wav output.wav pitch [velocity flags offset duration consonant cutoff volume modulation tempo pitch_bends]");
        std::process::exit(1);
    }

    let input_path = PathBuf::from(&args[0]);
    let output_path = PathBuf::from(&args[1]);
    let pitch_name = args[2].clone();

    let velocity = args.get(3).and_then(|v| v.parse::<f64>().ok()).unwrap_or(100.0);
    let flags = args.get(4).cloned().unwrap_or_else(|| "g0".to_string());
    let offset_ms = args.get(5).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let duration_ms = args.get(6).and_then(|v| v.parse::<f64>().ok()).unwrap_or(500.0);
    let consonant_ms = args.get(7).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let cutoff_ms = args.get(8).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let volume = args.get(9).and_then(|v| v.parse::<f64>().ok()).unwrap_or(100.0);
    let modulation = args.get(10).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let tempo = args
        .get(11)
        .map(|s| s.trim_start_matches('!'))
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(120.0);
    let pitch_bend_str = args.get(12).cloned().unwrap_or_default();

    let (samples, sample_rate) = read_wav_samples(&input_path)
        .map_err(|e| format!("Falha ao ler {}: {e}", input_path.display()))?;

    let res_args = kamafeu::drivers::ResamplerArgs {
        input_wav: input_path,
        output_wav: output_path,
        pitch_name,
        pitch_freq: 0.0,
        velocity,
        flags,
        offset_ms,
        duration_ms,
        source_consonant_ms: consonant_ms,
        consonant_ms,
        cutoff_ms,
        volume,
        modulation,
        normalization: None,
        tempo,
        pitch_bend_str,
        pitch_points: Vec::new(),
        loop_start_ms: None,
        loop_end_ms: None,
        tail_start_ms: None,
        vowel_formant: None,
    };

    kamafeu::dsp::catalina::render_catalina_note(&samples, sample_rate, &res_args, None)
        .map_err(|e| format!("Renderização Catalina falhou: {e}"))?;

    Ok(())
}
