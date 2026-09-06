use crate::dsp::envelope::UtauEnvelope;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct WavtoolArgs {
    pub output_wav: PathBuf,
    pub input_rendered_wav: PathBuf,
    pub skip_over_ms: f64,
    pub duration_ms: f64,
    pub envelope: UtauEnvelope,
    pub overlap_ms: f64,
    pub phoneme_envelope: [(f64, f64); 5],
    pub sample_time_zero_ms: f64,
}

pub trait WavtoolDriver: Send + Sync {
    fn name(&self) -> &str;
    fn process_note(
        &self,
        note_samples: &mut [f32],
        sample_rate: u32,
        args: &WavtoolArgs,
        cancel: Option<&AtomicBool>,
    ) -> Result<(), String>;
}

fn load_wavtool_output(
    note_samples: &mut [f32],
    expected_sample_rate: u32,
    output_wav: &Path,
) -> Result<(), String> {
    if !output_wav.is_file() {
        return Err("o wavtool não criou o WAV de saída".to_string());
    }

    let (mut output, output_sample_rate) =
        crate::renderer::TrackRenderer::load_wav_samples(output_wav)?;
    if output.is_empty() {
        return Err("o wavtool criou um WAV vazio".to_string());
    }
    if output_sample_rate != expected_sample_rate {
        output = crate::renderer::TrackRenderer::convert_sample_rate(
            &output,
            output_sample_rate,
            expected_sample_rate,
        );
    }

    note_samples.fill(0.0);
    let copied = note_samples.len().min(output.len());
    note_samples[..copied].copy_from_slice(&output[..copied]);
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnownWavtool {
    WavtoolYawu,
    SillySeams,
    WavtoolPl,
    VocalUtauWavTools,
    Organum,
    WavToolCS,
    Kladtool,
    WavtoolRs,
}

impl KnownWavtool {
    pub const ALL: [Self; 8] = [
        Self::WavtoolYawu,
        Self::SillySeams,
        Self::WavtoolPl,
        Self::VocalUtauWavTools,
        Self::Organum,
        Self::WavToolCS,
        Self::Kladtool,
        Self::WavtoolRs,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::WavtoolYawu => "wavtool-yawu (m13253/wavtool-yawu)",
            Self::SillySeams => "SillySeams (MLo7Ghinsan/SillySeams)",
            Self::WavtoolPl => "wavtool-pl (yuanchao/wavtool-pl)",
            Self::VocalUtauWavTools => "VocalUtau.WavTools (scskarsper/VocalUtau.WavTools)",
            Self::Organum => "Organum (KakouLabs/Organum)",
            Self::WavToolCS => "WavTool-CS (OpenSynth/WavTool-CS)",
            Self::Kladtool => "kladtool (adlez27/kladtool)",
            Self::WavtoolRs => "wavtool-rs (SHIACKOWORKS/wavtool-rs)",
        }
    }

    pub const fn executable_names(self) -> &'static [&'static str] {
        match self {
            Self::WavtoolYawu => &["wavtool-yawu", "wavtool-yawu.exe"],
            Self::SillySeams => &[
                "sillyseams",
                "sillyseams.exe",
                "SillySeams",
                "SillySeams.exe",
            ],
            Self::WavtoolPl => &["wavtool-pl", "wavtool-pl.exe", "wavtool.pl"],
            Self::VocalUtauWavTools => &[
                "wavtool2",
                "wavtool2.exe",
                "wavtool4v",
                "wavtool4v.exe",
                "vocalutau-wavtool",
                "vocalutau-wavtool.exe",
            ],
            Self::Organum => &[
                "organum-wavtool",
                "organum-wavtool.exe",
                "organum",
                "organum.exe",
            ],
            Self::WavToolCS => &[
                "wavtool-cs",
                "wavtool-cs.exe",
                "WavTool-CS",
                "WavTool-CS.exe",
                "wavtool-csharp.exe",
            ],
            Self::Kladtool => &["kladtool", "kladtool.exe", "KladTool", "KladTool.exe"],
            Self::WavtoolRs => &["wavtool-rs", "wavtool-rs.exe"],
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|profile| profile.label() == label)
    }

    pub fn default_path(self) -> PathBuf {
        PathBuf::from("./wavtools").join(self.executable_names()[0])
    }

    pub fn find_executable(self) -> Option<PathBuf> {
        static CACHE: std::sync::LazyLock<
            std::sync::Mutex<std::collections::HashMap<KnownWavtool, Option<PathBuf>>>,
        > = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

        if let Ok(guard) = CACHE.lock() {
            if let Some(cached) = guard.get(&self) {
                return cached.clone();
            }
        }

        let result = self.search_executable_uncached();

        if let Ok(mut guard) = CACHE.lock() {
            guard.insert(self, result.clone());
        }

        result
    }

    fn search_executable_uncached(self) -> Option<PathBuf> {
        let mut roots = vec![
            PathBuf::from("./wavtools"),
            PathBuf::from("./resamplers"),
            PathBuf::from("."),
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/bin"),
        ];

        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(exe_dir) = current_exe.parent() {
                roots.push(exe_dir.join("wavtools"));
                roots.push(exe_dir.join("resamplers"));
                roots.push(exe_dir.to_path_buf());
                if let Some(p1) = exe_dir.parent() {
                    roots.push(p1.join("wavtools"));
                    roots.push(p1.join("resamplers"));
                    if let Some(p2) = p1.parent() {
                        roots.push(p2.join("wavtools"));
                        roots.push(p2.join("resamplers"));
                        if let Some(p3) = p2.parent() {
                            roots.push(p3.join("wavtools"));
                            roots.push(p3.join("resamplers"));
                        }
                    }
                }
            }
        }

        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            roots.push(home.join("Documents/kamafeu/wavtools"));
            roots.push(home.join("Documents/kamafeu/resamplers"));
            roots.push(home.join("Documents/kamafeu"));
            roots.push(home.join("Downloads/wavtools"));
            roots.push(home.join("Downloads/resamplers"));
            roots.push(home.join("Downloads"));
            roots.push(home.join(".local/bin"));
            roots.push(home.join("Library/Application Support/OpenUTAU/Wavtools"));
            roots.push(home.join("Library/Application Support/OpenUtau/Wavtools"));
            roots.push(home.join("Library/Application Support/OpenUTAU/Resamplers"));
            roots.push(home.join("Library/Application Support/OpenUtau/Resamplers"));
            roots.push(home.join(".wine/drive_c/Program Files (x86)/UTAU/wavtools"));
            roots.push(home.join(".wine/drive_c/Program Files (x86)/UTAU"));
            roots.push(home.join(".wine/drive_c/Program Files/UTAU/wavtools"));
            roots.push(home.join(".wine/drive_c/Program Files/UTAU"));
            roots.push(home.join(".wine/drive_c/UTAU/wavtools"));
            roots.push(home.join(".wine/drive_c/UTAU"));
        }

        if let Some(path_env) = std::env::var_os("PATH") {
            for p in std::env::split_paths(&path_env) {
                if !roots.contains(&p) {
                    roots.push(p);
                }
            }
        }

        for root in roots {
            for executable in self.executable_names() {
                let candidate = root.join(executable);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }

        None
    }
}

pub struct NativeWavtoolDriver;

impl WavtoolDriver for NativeWavtoolDriver {
    fn name(&self) -> &str {
        "Native Rust (Crossfader)"
    }

    fn process_note(
        &self,
        note_samples: &mut [f32],
        sample_rate: u32,
        args: &WavtoolArgs,
        _cancel: Option<&AtomicBool>,
    ) -> Result<(), String> {
        if note_samples.is_empty() {
            return Ok(());
        }

        // Apply the per-note amplitude envelope. The track mixer performs the
        // complementary crossfade against the preceding phone.
        UtauEnvelope::apply_points(
            note_samples,
            sample_rate,
            args.sample_time_zero_ms,
            &args.phoneme_envelope,
        );
        Ok(())
    }
}

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
}

impl WavtoolDriver for WavtoolYawuDriver {
    fn name(&self) -> &str {
        "wavtool-yawu (m13253/wavtool-yawu)"
    }

    fn process_note(
        &self,
        note_samples: &mut [f32],
        sample_rate: u32,
        args: &WavtoolArgs,
        cancel: Option<&AtomicBool>,
    ) -> Result<(), String> {
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

        // The bundled yawu launcher is intentionally a pass-through wrapper;
        // still read its output so replacing it with the real engine works.
        load_wavtool_output(note_samples, sample_rate, &args.output_wav)?;

        UtauEnvelope::apply_points(
            note_samples,
            sample_rate,
            args.sample_time_zero_ms,
            &args.phoneme_envelope,
        );
        Ok(())
    }
}

pub struct ExternalWavtoolDriver {
    pub executable_path: PathBuf,
    display_name: String,
}

impl ExternalWavtoolDriver {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        let display_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("External Wavtool")
            .to_string();
        Self {
            executable_path: path,
            display_name,
        }
    }

    pub fn for_known(profile: KnownWavtool, configured_path: Option<PathBuf>) -> Self {
        let executable_path = configured_path
            .filter(|path| path.is_file())
            .or_else(|| profile.find_executable())
            .unwrap_or_else(|| profile.default_path());
        Self {
            executable_path,
            display_name: profile.label().to_string(),
        }
    }
}

impl WavtoolDriver for ExternalWavtoolDriver {
    fn name(&self) -> &str {
        &self.display_name
    }

    fn process_note(
        &self,
        note_samples: &mut [f32],
        sample_rate: u32,
        args: &WavtoolArgs,
        cancel: Option<&AtomicBool>,
    ) -> Result<(), String> {
        let resolved_exe = if self.executable_path.is_file() {
            Some(self.executable_path.clone())
        } else {
            KnownWavtool::from_label(&self.display_name).and_then(|p| p.find_executable())
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

        // External engines already applied the UTAU envelope.  Their WAV must
        // become the phone audio before the renderer mixes it into the track.
        load_wavtool_output(note_samples, sample_rate, &args.output_wav)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_wavtools_roundtrip_their_labels() {
        for profile in KnownWavtool::ALL {
            assert_eq!(KnownWavtool::from_label(profile.label()), Some(profile));
            assert!(!profile.executable_names().is_empty());
        }
    }

    #[test]
    fn wavtool_output_replaces_the_phone_audio() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("wavtool.wav");
        crate::renderer::TrackRenderer::save_wav_samples(&output, &[0.25; 4], 44_100).unwrap();
        let mut phone = [0.0; 4];

        load_wavtool_output(&mut phone, 44_100, &output).unwrap();

        assert!(phone.iter().all(|sample| (sample - 0.25).abs() < 1e-3));
    }
}
