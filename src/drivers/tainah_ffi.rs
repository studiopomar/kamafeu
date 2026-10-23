//! Versioned Tainah WORLD ABI. The library lives beside the executable
//! in desktop packages; TAINAH_LIBRARY_PATH is useful for local development.
use super::resampler_driver::{parse_utau_flags, ResamplerArgs};
use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

#[repr(C)]
struct Error {
    struct_size: u32,
    code: i32,
    message: [c_char; 256],
}
#[repr(C)]
struct PitchPoint {
    time_ms: f64,
    cents: f64,
    shape: u32,
}
#[repr(C)]
struct Options {
    struct_size: u32,
    sample_rate: u32,
    input: *const f32,
    input_length: u64,
    duration_ms: f64,
    offset_ms: f64,
    consonant_ms: f64,
    cutoff_ms: f64,
    pitch_hz: f64,
    pitch_points: *const PitchPoint,
    pitch_point_count: u32,
    gender: f64,
    pitch_cents: f64,
    breathiness: f64,
    high_frequency_cutoff: f64,
    frame_period_ms: f64,
    f0_floor: f64,
    f0_ceiling: f64,
    f0_method: u32,
    loop_start_ms: f64,
    loop_end_ms: f64,
    tail_start_ms: f64,
    cancel: Option<unsafe extern "C" fn(*mut c_void) -> c_int>,
    cancel_user_data: *mut c_void,
    formant_shift: f64,
    consonant_adjustment: f64,
}
#[repr(C)]
struct Pcm {
    struct_size: u32,
    samples: *mut f32,
    length: u64,
    sample_rate: u32,
}

unsafe extern "C" fn cancelled(context: *mut c_void) -> c_int {
    // The render call is synchronous, so the borrowed flag outlives this callback.
    i32::from(unsafe { &*(context as *const AtomicBool) }.load(Ordering::Relaxed))
}

pub fn library_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("TAINAH_LIBRARY_PATH") {
        return Some(path.into());
    }
    let name = if cfg!(target_os = "windows") {
        "tainah_motor.dll"
    } else if cfg!(target_os = "macos") {
        "libtainah_motor.dylib"
    } else {
        "libtainah_motor.so.1"
    };
    let executable = std::env::current_exe().ok()?;
    let dir = executable.parent()?;
    for candidate in [
        dir.join(name),
        dir.join("../Frameworks").join(name),
        dir.join("../lib").join(name),
        dir.join("../lib/kamafeu").join(name),
        // The local Tainah builder writes its artifact to
        // target/tainah-package, while Cargo executables live in target/debug
        // (or target/debug/deps for tests).
        dir.join("../tainah-package").join(name),
        dir.join("../../tainah-package").join(name),
    ] {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    Some(dir.join(name))
}

pub fn cache_identity() -> String {
    let path = library_path();
    let fingerprint = path
        .as_ref()
        .and_then(|path| std::fs::read(path).ok())
        .map(|bytes| {
            use sha2::{Digest, Sha256};
            format!("{:x}", Sha256::digest(bytes))
        })
        .unwrap_or_else(|| "missing".to_string());
    format!(
        "tainah-motor:abi2:{fingerprint}:{}",
        crate::dsp::world_resampler::cache_identity()
    )
}

fn world_detector_method(configured: &str) -> Result<u32, String> {
    let detector = configured.to_ascii_lowercase();
    if detector.contains("harvest") {
        Ok(1)
    } else if detector.contains("dio") {
        Ok(2)
    } else if detector.contains("yin") {
        // Tainah exposes WORLD Harvest and DIO. The app-wide YIN/pyIN
        // preference is used by analysis tools, so use Harvest for Tainah.
        Ok(1)
    } else {
        Err(format!(
            "WORLD detector '{}' is unsupported by Tainah (use Harvest or DIO)",
            configured
        ))
    }
}

pub fn render(
    input: &[f32],
    sample_rate: u32,
    args: &ResamplerArgs,
    cancel: Option<&AtomicBool>,
) -> Result<Vec<f32>, String> {
    let path = library_path().ok_or("cannot locate executable for Tainah library")?;
    // Loading is local and synchronous. Symbols are used only while `library` is alive.
    let library = unsafe { libloading::Library::new(&path) }
        .map_err(|error| format!("cannot load {}: {error}", path.display()))?;
    unsafe {
        let version: libloading::Symbol<unsafe extern "C" fn() -> u32> = library
            .get(b"lt_abi_version\0")
            .map_err(|e| e.to_string())?;
        if version() != 2 {
            return Err(format!(
                "Tainah library ABI mismatch: expected 2, found {}",
                version()
            ));
        }
        let render: libloading::Symbol<
            unsafe extern "C" fn(*const Options, *mut Pcm, *mut Error) -> i32,
        > = library.get(b"lt_render\0").map_err(|e| e.to_string())?;
        let free: libloading::Symbol<unsafe extern "C" fn(*mut Pcm)> =
            library.get(b"lt_pcm_free\0").map_err(|e| e.to_string())?;
        let mut points: Vec<PitchPoint> = args
            .pitch_points
            .iter()
            .map(|point| PitchPoint {
                time_ms: point.time_offset_ms,
                cents: point.pitch_offset_cents,
                shape: match point.shape.to_ascii_lowercase().as_str() {
                    "l" | "linear" => 1,
                    "i" | "j" | "easein" | "ease-in" | "exponential" => 2,
                    "o" | "r" | "easeout" | "ease-out" | "logarithmic" => 3,
                    "sp" | "spline" | "h" | "hermite" | "c" | "catmull" | "catmull-rom"
                    | "cubic" => 4,
                    _ => 0,
                },
            })
            .collect();
        points.sort_by(|left, right| left.time_ms.total_cmp(&right.time_ms));
        points.dedup_by(|left, right| left.time_ms == right.time_ms);
        let mut gender = 0.0_f64;
        let mut pitch_cents = 0.0_f64;
        let mut breathiness = 0.0_f64;
        let mut high = 0.0_f64;
        let mut formant_shift = 0.0_f64;
        let mut consonant_adjustment = 0.0_f64;
        for flag in parse_utau_flags(&args.flags) {
            let value = flag.num_val.unwrap_or(0.0);
            match flag.key.as_str() {
                "g" => gender = value.clamp(-100.0, 100.0),
                "t" => pitch_cents = value.clamp(-100.0, 100.0) * 10.0,
                "B" => breathiness = value.clamp(0.0, 100.0),
                "H" => high = value.clamp(0.0, 99.0),
                "P" => formant_shift = value.clamp(-100.0, 100.0),
                "C" => consonant_adjustment = value.clamp(-100.0, 100.0),
                _ => {}
            }
        }
        let config = crate::dsp::world_resampler::get_runtime_config();
        let f0_method = world_detector_method(&config.f0_detection_method)?;
        let nyquist = sample_rate as f64 / 2.0;
        let high_hz = if high <= 0.0 || nyquist <= 1000.0 {
            0.0
        } else {
            nyquist * (1000.0 / nyquist).powf(high / 99.0)
        };
        let options = Options {
            struct_size: std::mem::size_of::<Options>() as u32,
            sample_rate,
            input: input.as_ptr(),
            input_length: input.len() as u64,
            duration_ms: args.duration_ms,
            offset_ms: args.offset_ms,
            consonant_ms: args.source_consonant_ms,
            cutoff_ms: args.cutoff_ms,
            pitch_hz: args.pitch_freq,
            pitch_points: points.as_ptr(),
            pitch_point_count: points.len() as u32,
            gender,
            pitch_cents,
            breathiness,
            high_frequency_cutoff: high_hz,
            frame_period_ms: config.frame_period_ms,
            f0_floor: config.f0_floor_hz,
            f0_ceiling: config.f0_ceil_hz,
            f0_method,
            loop_start_ms: args.loop_start_ms.unwrap_or(f64::NAN),
            loop_end_ms: args.loop_end_ms.unwrap_or(f64::NAN),
            tail_start_ms: args.tail_start_ms.unwrap_or(f64::NAN),
            cancel: cancel.map(|_| cancelled as unsafe extern "C" fn(*mut c_void) -> c_int),
            cancel_user_data: cancel.map_or(std::ptr::null_mut(), |flag| {
                flag as *const AtomicBool as *mut c_void
            }),
            formant_shift,
            consonant_adjustment,
        };
        let mut pcm = Pcm {
            struct_size: std::mem::size_of::<Pcm>() as u32,
            samples: std::ptr::null_mut(),
            length: 0,
            sample_rate: 0,
        };
        let mut error = Error {
            struct_size: std::mem::size_of::<Error>() as u32,
            code: 0,
            message: [0; 256],
        };
        let status = render(&options, &mut pcm, &mut error);
        if status != 0 {
            free(&mut pcm);
            return Err(format!(
                "WORLD code {status}: {}",
                CStr::from_ptr(error.message.as_ptr()).to_string_lossy()
            ));
        }
        let expected = (args.duration_ms * sample_rate as f64 / 1000.0).round() as usize;
        if pcm.samples.is_null() || pcm.length != expected as u64 || pcm.sample_rate != sample_rate
        {
            free(&mut pcm);
            return Err("WORLD output has incorrect duration or sample rate".into());
        }
        let samples = std::slice::from_raw_parts(pcm.samples, expected).to_vec();
        free(&mut pcm);
        Ok(samples)
    }
}

#[cfg(test)]
mod tests {
    use super::world_detector_method;

    #[test]
    fn app_wide_pitch_trackers_map_to_world_harvest_for_tainah() {
        assert_eq!(
            world_detector_method("YIN (Pitch Fundamental Adaptativo)"),
            Ok(1)
        );
        assert_eq!(world_detector_method("pyIN (Probabilístico)"), Ok(1));
        assert_eq!(world_detector_method("Harvest/DIO (Espectral)"), Ok(1));
        assert_eq!(world_detector_method("DIO"), Ok(2));
        assert!(world_detector_method("unknown tracker").is_err());
    }
}
