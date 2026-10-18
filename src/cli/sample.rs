use std::path::PathBuf;

pub(super) fn generate(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    kamafeu::terminal_log::out_header("Sample voicebank", path.display());
    std::fs::create_dir_all(&path)?;

    let char_txt = "name=Kamafeu Sample Synth\nauthor=Kamafeu Team\n";
    std::fs::write(path.join("character.txt"), char_txt)?;

    let vowels = [
        ("ka", 261.63),
        ("ki", 293.66),
        ("ku", 329.63),
        ("ke", 349.23),
        ("ko", 392.00),
    ];

    let sample_rate = 44100;
    let mut oto_lines = Vec::new();

    for &(name, freq) in &vowels {
        let filename = format!("{}.wav", name);
        let wav_path = path.join(&filename);

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: sample_rate as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(&wav_path, spec)?;
        let duration_sec = 1.0;
        let total_samples = (sample_rate as f64 * duration_sec) as usize;

        for i in 0..total_samples {
            let t = i as f64 / sample_rate as f64;
            let sample_val = 0.6 * (2.0 * std::f64::consts::PI * freq * t).sin()
                + 0.3 * (2.0 * std::f64::consts::PI * freq * 2.0 * t).sin()
                + 0.1 * (2.0 * std::f64::consts::PI * freq * 3.0 * t).sin();

            let sample_i16 = (sample_val * i16::MAX as f64) as i16;
            writer.write_sample(sample_i16)?;
        }
        writer.finalize()?;

        oto_lines.push(format!("{}=,10,50,-100,30,15", filename));
    }

    std::fs::write(path.join("oto.ini"), oto_lines.join("\n"))?;

    kamafeu::terminal_log::out_status("OK", "Result", "sample voicebank created");
    kamafeu::terminal_log::out_kv("Files", "character.txt, oto.ini, ka.wav..ko.wav");
    Ok(())
}
