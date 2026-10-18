use super::{load_wavtool_output, KnownWavtool, WavtoolArgs, WavtoolDriver};
use crate::dsp::envelope::UtauEnvelope;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub struct WavtoolYawuDriver {
    pub executable_path: PathBuf,
}

impl WavtoolYawuDriver {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let p = path.as_ref();
        let final_path = if p.exists() {
            p.to_path_buf()
        } else {
            Self::find_executable().unwrap_or_else(|| p.to_path_buf())
        };
        Self {
            executable_path: final_path,
        }
    }

    pub fn find_executable() -> Option<PathBuf> {
        KnownWavtool::WavtoolYawu.find_executable()
    }

    fn uses_integrated_backend(&self) -> bool {
        if !self.executable_path.is_file() {
            return false;
        }
        std::fs::read_to_string(&self.executable_path)
            .is_ok_and(|contents| contents.contains("Kamafeu Internal Wavtool-Yawu Driver Wrapper"))
    }

    pub(super) fn process_integrated(
        note_samples: &mut [f32],
        sample_rate: u32,
        args: &WavtoolArgs,
    ) {
        UtauEnvelope::apply_points(
            note_samples,
            sample_rate,
            args.sample_time_zero_ms,
            &args.phoneme_envelope,
        );
    }
}

impl WavtoolDriver for WavtoolYawuDriver {
    fn engine_id(&self) -> &'static str {
        if self.uses_integrated_backend() {
            "kamafeu.simple-wavtool"
        } else {
            "org.wavtool.yawu"
        }
    }

    fn phrase_executable(&self) -> Option<PathBuf> {
        if self.uses_integrated_backend() {
            None
        } else {
            Some(self.executable_path.clone())
        }
    }
    fn name(&self) -> &str {
        if self.uses_integrated_backend() {
            "Kamafeu Simple (wrapper de compatibilidade yawu)"
        } else {
            "wavtool-yawu (externo)"
        }
    }

    fn envelope_curve_name(&self) -> &'static str {
        "linear"
    }

    fn consumes_skip_over(&self) -> bool {
        !self.uses_integrated_backend()
    }

    fn process_note(
        &self,
        note_samples: &mut [f32],
        sample_rate: u32,
        args: &WavtoolArgs,
        cancel: Option<&AtomicBool>,
    ) -> Result<(), String> {
        if self.uses_integrated_backend() {
            Self::process_integrated(note_samples, sample_rate, args);
            return Ok(());
        }

        let resolved_exe = if self.executable_path.is_file() {
            Some(self.executable_path.clone())
        } else {
            KnownWavtool::WavtoolYawu.find_executable()
        };
        let final_exe = match resolved_exe {
            Some(p) => p,
            None => {
                return Err(format!(
                    "wavtool não encontrado: {}",
                    self.executable_path.display()
                ));
            }
        };
        {
            if !args.input_rendered_wav.exists() {
                crate::renderer::TrackRenderer::save_wav_samples(
                    &args.input_rendered_wav,
                    note_samples,
                    sample_rate,
                )?;
            }

            let is_exe = final_exe
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("exe"))
                .unwrap_or(false);

            let out_arg = if is_exe {
                crate::drivers::process::to_wine_windows_path(&args.output_wav)
            } else {
                args.output_wav.as_os_str().to_owned()
            };
            let in_arg = if is_exe {
                crate::drivers::process::to_wine_windows_path(&args.input_rendered_wav)
            } else {
                args.input_rendered_wav.as_os_str().to_owned()
            };

            let mut cmd = crate::drivers::process::prepare_command(&final_exe)?;
            cmd.arg(out_arg)
                .arg(in_arg)
                .arg(format!("{:.1}", args.skip_over_ms))
                .arg(format!("{:.1}", args.duration_ms))
                .arg(format!("{:.1}", args.envelope.p1))
                .arg(format!("{:.1}", args.envelope.p2))
                .arg(format!("{:.1}", args.envelope.p3))
                .arg(format!("{:.0}", args.envelope.v1))
                .arg(format!("{:.0}", args.envelope.v2))
                .arg(format!("{:.0}", args.envelope.v3))
                .arg(format!("{:.0}", args.envelope.v4))
                .arg(format!("{:.1}", args.overlap_ms))
                .arg(format!("{:.1}", args.envelope.p4))
                .arg(format!("{:.1}", args.envelope.p5))
                .arg(format!("{:.0}", args.envelope.v5));

            let output = crate::drivers::process::run_with_timeout(
                &mut cmd,
                Duration::from_secs(120),
                cancel,
            )?;
            if !output.status.success() {
                return Err(format!(
                    "wavtool falhou: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
        }

        load_wavtool_output(note_samples, sample_rate, &args.output_wav)?;
        Ok(())
    }
}
