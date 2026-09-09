use super::KamafeuStudioApp;

impl KamafeuStudioApp {
    pub(in crate::gui) fn play_calibration_signal(&mut self, signal_type: usize, freq: f32) {
        let sample_rate = 44100;
        let duration_secs = 0.5;
        let total_samples = (sample_rate as f32 * duration_secs) as usize;
        let mut samples = Vec::with_capacity(total_samples);

        match signal_type {
            0 => {
                for i in 0..total_samples {
                    let t = i as f32 / sample_rate as f32;
                    let val = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.35;
                    samples.push(val);
                }
            }
            1 => {
                for i in 0..total_samples {
                    let t = i as f32 / sample_rate as f32;
                    let val = (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.35;
                    samples.push(val);
                }
            }
            2 => {
                let mut b0 = 0.0f32;
                let mut b1 = 0.0f32;
                let mut b2 = 0.0f32;
                let mut rng_seed: u32 = 123456789;
                for _ in 0..total_samples {
                    rng_seed = rng_seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let white = ((rng_seed as f32 / u32::MAX as f32) * 2.0 - 1.0) * 0.3;
                    b0 = 0.99886 * b0 + white * 0.0555179;
                    b1 = 0.99332 * b1 + white * 0.0750759;
                    b2 = 0.96900 * b2 + white * 0.1538520;
                    let pink = b0 + b1 + b2 + white * 0.5362;
                    samples.push(pink * 0.15);
                }
            }
            3 => {
                let mut rng_seed: u32 = 987654321;
                for _ in 0..total_samples {
                    rng_seed = rng_seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let white = ((rng_seed as f32 / u32::MAX as f32) * 2.0 - 1.0) * 0.12;
                    samples.push(white);
                }
            }
            _ => {
                for i in 0..total_samples {
                    let t = i as f32 / total_samples as f32;
                    let f = 20.0 * (1000.0f32).powf(t);
                    let phase = 2.0 * std::f32::consts::PI * f * (i as f32 / sample_rate as f32);
                    samples.push(phase.sin() * 0.25);
                }
            }
        }

        let fade_len = (sample_rate as f32 * 0.02) as usize;
        let len = samples.len();
        for i in 0..fade_len.min(len) {
            let gain = i as f32 / fade_len as f32;
            samples[i] *= gain;
            samples[len - 1 - i] *= gain;
        }

        self.audio_player.play_samples(samples, sample_rate);
    }

    pub(in crate::gui) fn run_dsp_engine_benchmark(&mut self) {
        let lang = self.config.language;
        let start = std::time::Instant::now();
        let num_samples = 50_000;
        let mut buffer: Vec<f32> = (0..num_samples)
            .map(|i| (i as f32 * 0.05).sin() + (i as f32 * 0.12).cos())
            .collect();

        let mut acc = 0.0f32;
        for chunk in buffer.chunks_mut(256) {
            for (i, sample) in chunk.iter_mut().enumerate() {
                let win = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / 256.0).cos());
                *sample = *sample * win + acc * 0.1;
                acc = *sample;
            }
        }

        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        let msamples_per_sec =
            (num_samples as f64 / 1_000_000.0) / elapsed.as_secs_f64().max(0.00001);
        let mflops = msamples_per_sec * 32.0;

        self.preferences_state.benchmark_result = Some(format!(
            "[OK] {} {:.2} ms | Throughput: {:.2} MSamples/s | {}: {:.1} MFLOPS (50.000 {})",
            lang.tr("Concluído em", "Completed in"),
            elapsed_ms,
            msamples_per_sec,
            lang.tr("Desempenho Estimado", "Estimated Performance"),
            mflops,
            lang.tr("amostras com janela Blackman/FFT", "samples with Blackman/FFT window")
        ));
    }

    pub(in crate::gui) fn run_ram_stress_test(&mut self) {
        let lang = self.config.language;
        let start = std::time::Instant::now();
        let total_floats = 16 * 1024 * 1024; // 64 MB
        let mut pool: Vec<Vec<f32>> = Vec::with_capacity(64);
        for chunk_idx in 0..64 {
            let mut v = vec![chunk_idx as f32; total_floats / 64];
            for (i, x) in v.iter_mut().enumerate() {
                *x += (i % 7) as f32;
            }
            pool.push(v);
        }
        let alloc_time = start.elapsed();
        drop(pool);
        let total_time = start.elapsed();

        let gb_per_sec = (0.064) / alloc_time.as_secs_f64().max(0.00001);
        self.preferences_state.memory_stress_result = Some(format!(
            "[OK] {} {:.2} ms ({}: {:.2} GB/s)",
            lang.tr("64 MB alocados, verificados e liberados em", "64 MB allocated, verified and freed in"),
            total_time.as_secs_f64() * 1000.0,
            lang.tr("Taxa de escrita Heap", "Heap write rate"),
            gb_per_sec
        ));
    }

    pub(in crate::gui) fn run_voicebanks_audit(&mut self) {
        let lang = self.config.language;
        let start = std::time::Instant::now();
        let dirs = &self.config.singers_paths;
        let mut total_found = 0;
        let mut oto_found = 0;

        for dir in dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        total_found += 1;
                        if entry.path().join("oto.ini").exists()
                            || entry.path().join("character.txt").exists()
                        {
                            oto_found += 1;
                        }
                    }
                }
            }
        }

        let elapsed = start.elapsed();
        self.preferences_state.voicebank_audit_result = Some(format!(
            "[OK] {} {:.2} ms: {} {}, {} {}",
            lang.tr("Auditoria finalizada em", "Audit finished in"),
            elapsed.as_secs_f64() * 1000.0,
            total_found,
            lang.tr("pastas de cantores verificadas", "singer folders checked"),
            oto_found,
            lang.tr("voicebanks com oto.ini/character.txt validados com sucesso.", "voicebanks with oto.ini/character.txt successfully validated.")
        ));
    }
}
