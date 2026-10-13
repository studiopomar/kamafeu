//! Optional C ABI DSP kernels.
//!
//! The native backend is deliberately not selected by the renderer yet.  It
//! exists behind a small, allocation-free interface so its output and speed
//! can be compared with the Rust/SIMD implementation before activation.

#[cfg(not(target_arch = "wasm32"))]
mod ffi {
    unsafe extern "C" {
        pub fn kamafeu_dsp_abi_version() -> u32;
        pub fn kamafeu_dsp_info(info: *mut super::KamafeuDspInfo) -> i32;
        pub fn kamafeu_mix_stereo_f32(
            destination: *mut f32,
            destination_frames: usize,
            source: *const f32,
            source_frames: usize,
            gain: f32,
            pan: f32,
        ) -> i32;
        pub fn kamafeu_overlap_add_f32(
            destination: *mut f32,
            source: *const f32,
            destination_frames: usize,
            source_frames: usize,
            offset: usize,
            gain: f32,
        ) -> i32;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct KamafeuDspInfo {
    pub abi_version: u32,
    pub capabilities: u32,
    pub simd_flags: u32,
    pub backend_name: *const std::ffi::c_char,
}

pub trait DspBackend {
    fn name(&self) -> &'static str;
    fn abi_version(&self) -> u32;
    fn capabilities(&self) -> u32;
    fn mix_stereo(
        &self,
        destination: &mut [f32],
        source: &[f32],
        gain: f32,
        pan: f32,
    ) -> Result<(), i32>;
    fn overlap_add(
        &self,
        destination: &mut [f32],
        source: &[f32],
        offset: usize,
        gain: f32,
    ) -> Result<(), i32>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RustDspBackend;

impl DspBackend for RustDspBackend {
    fn name(&self) -> &'static str {
        "rust"
    }
    fn abi_version(&self) -> u32 {
        0
    }
    fn capabilities(&self) -> u32 {
        0
    }
    fn mix_stereo(
        &self,
        destination: &mut [f32],
        source: &[f32],
        gain: f32,
        pan: f32,
    ) -> Result<(), i32> {
        if destination.len() < source.len()
            || source.len() % 2 != 0
            || !gain.is_finite()
            || !(-1.0..=1.0).contains(&pan)
        {
            return Err(1);
        }
        let left = gain * (1.0 - pan);
        let right = gain * (1.0 + pan);
        for (dst, src) in destination[..source.len()]
            .chunks_exact_mut(2)
            .zip(source.chunks_exact(2))
        {
            dst[0] += src[0] * left;
            dst[1] += src[1] * right;
        }
        Ok(())
    }
    fn overlap_add(
        &self,
        destination: &mut [f32],
        source: &[f32],
        offset: usize,
        gain: f32,
    ) -> Result<(), i32> {
        if offset.checked_add(source.len()).is_none()
            || offset + source.len() > destination.len()
            || !gain.is_finite()
        {
            return Err(1);
        }
        for (dst, src) in destination[offset..offset + source.len()]
            .iter_mut()
            .zip(source)
        {
            *dst += *src * gain;
        }
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Default, Clone, Copy)]
pub struct CAbiDspBackend;

#[cfg(not(target_arch = "wasm32"))]
impl DspBackend for CAbiDspBackend {
    fn name(&self) -> &'static str {
        "c-abi"
    }
    fn abi_version(&self) -> u32 {
        unsafe { ffi::kamafeu_dsp_abi_version() }
    }
    fn capabilities(&self) -> u32 {
        let mut info = std::mem::MaybeUninit::<KamafeuDspInfo>::zeroed();
        let status = unsafe { ffi::kamafeu_dsp_info(info.as_mut_ptr()) };
        if status == 0 {
            unsafe { info.assume_init().capabilities }
        } else {
            0
        }
    }
    fn mix_stereo(
        &self,
        destination: &mut [f32],
        source: &[f32],
        gain: f32,
        pan: f32,
    ) -> Result<(), i32> {
        if destination.len() < source.len() || source.len() % 2 != 0 {
            return Err(1);
        }
        let status = unsafe {
            ffi::kamafeu_mix_stereo_f32(
                destination.as_mut_ptr(),
                destination.len() / 2,
                source.as_ptr(),
                source.len() / 2,
                gain,
                pan,
            )
        };
        if status == 0 {
            Ok(())
        } else {
            Err(status)
        }
    }
    fn overlap_add(
        &self,
        destination: &mut [f32],
        source: &[f32],
        offset: usize,
        gain: f32,
    ) -> Result<(), i32> {
        if offset.checked_add(source.len()).is_none() || offset + source.len() > destination.len() {
            return Err(1);
        }
        let status = unsafe {
            ffi::kamafeu_overlap_add_f32(
                destination.as_mut_ptr(),
                source.as_ptr(),
                destination.len(),
                source.len(),
                offset,
                gain,
            )
        };
        if status == 0 {
            Ok(())
        } else {
            Err(status)
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{CAbiDspBackend, DspBackend, RustDspBackend};

    #[test]
    fn c_backend_matches_rust_mix_kernel() {
        let source: Vec<f32> = (0..128).map(|i| (i as f32 - 40.0) * 0.01).collect();
        let mut rust = vec![0.2; source.len()];
        let mut native = rust.clone();
        RustDspBackend
            .mix_stereo(&mut rust, &source, 0.75, -0.2)
            .unwrap();
        CAbiDspBackend
            .mix_stereo(&mut native, &source, 0.75, -0.2)
            .unwrap();
        assert!(rust.iter().zip(native).all(|(a, b)| (*a - b).abs() < 1e-6));
    }

    #[test]
    fn c_backend_matches_rust_overlap_add() {
        let source: Vec<f32> = (0..37).map(|i| i as f32 * 0.03).collect();
        let mut rust = vec![0.1; 64];
        let mut native = rust.clone();
        RustDspBackend
            .overlap_add(&mut rust, &source, 11, 0.5)
            .unwrap();
        CAbiDspBackend
            .overlap_add(&mut native, &source, 11, 0.5)
            .unwrap();
        assert!(rust.iter().zip(native).all(|(a, b)| (*a - b).abs() < 1e-6));
    }

    #[test]
    fn both_backends_reject_invalid_buffer_geometry() {
        let source = vec![1.0f32; 8];
        let mut short_destination = vec![0.0f32; 2];
        assert!(RustDspBackend
            .mix_stereo(&mut short_destination, &source, 1.0, 0.0)
            .is_err());
        assert!(CAbiDspBackend
            .mix_stereo(&mut short_destination, &source, 1.0, 0.0)
            .is_err());

        let mut destination = vec![0.0f32; 8];
        assert!(RustDspBackend
            .overlap_add(&mut destination, &source, 4, 1.0)
            .is_err());
        assert!(CAbiDspBackend
            .overlap_add(&mut destination, &source, 4, 1.0)
            .is_err());
    }

    #[test]
    fn c_backend_reports_stable_capabilities() {
        assert_eq!(CAbiDspBackend.abi_version(), 1);
        assert_ne!(CAbiDspBackend.capabilities() & 1, 0);
        assert_ne!(CAbiDspBackend.capabilities() & 2, 0);
    }
}
