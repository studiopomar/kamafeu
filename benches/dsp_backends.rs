//! Small dependency-free benchmark for the optional native DSP kernels.
//! Run with `cargo bench --bench dsp_backends`.

use kamafeu::dsp::native::{CAbiDspBackend, DspBackend, RustDspBackend};
use std::time::{Duration, Instant};

fn measure<F: FnMut()>(mut operation: F) -> Duration {
    let warmup = Instant::now();
    while warmup.elapsed() < Duration::from_millis(100) {
        operation();
    }
    let start = Instant::now();
    let mut iterations = 0u64;
    while start.elapsed() < Duration::from_millis(500) {
        operation();
        iterations += 1;
    }
    start.elapsed().div_f64(iterations.max(1) as f64)
}

fn main() {
    let source: Vec<f32> = (0..96_000)
        .map(|index| ((index as f32) * 0.013).sin() * 0.2)
        .collect();
    let rust = RustDspBackend;
    let native = CAbiDspBackend;

    let rust_mix = measure(|| {
        let mut destination = vec![0.0; source.len()];
        rust.mix_stereo(&mut destination, &source, 0.8, -0.15)
            .unwrap();
        std::hint::black_box(destination);
    });
    let native_mix = measure(|| {
        let mut destination = vec![0.0; source.len()];
        native
            .mix_stereo(&mut destination, &source, 0.8, -0.15)
            .unwrap();
        std::hint::black_box(destination);
    });

    let mono_source = &source[..48_000];
    let rust_overlap = measure(|| {
        let mut destination = vec![0.0; 64_000];
        rust.overlap_add(&mut destination, mono_source, 4_000, 0.75)
            .unwrap();
        std::hint::black_box(destination);
    });
    let native_overlap = measure(|| {
        let mut destination = vec![0.0; 64_000];
        native
            .overlap_add(&mut destination, mono_source, 4_000, 0.75)
            .unwrap();
        std::hint::black_box(destination);
    });

    println!(
        "backend={} abi={} capabilities=0x{:x}",
        native.name(),
        native.abi_version(),
        native.capabilities()
    );
    println!("mix_stereo rust={rust_mix:?} c_abi={native_mix:?}");
    println!("overlap_add rust={rust_overlap:?} c_abi={native_overlap:?}");
}
