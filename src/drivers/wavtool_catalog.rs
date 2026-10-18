use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnownWavtool {
    Galapagos,
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
    pub const ALL: [Self; 9] = [
        Self::Galapagos,
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
            Self::Galapagos => "Galapagos Convergence (nativo)",
            Self::WavtoolYawu => "Kamafeu Simple / yawu externo",
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
            Self::Galapagos => &[
                "galapagos",
                "galapagos.exe",
                "openutau-wavtool",
                "openutau-wavtool.exe",
                "worldline",
                "worldline.exe",
                "worldline-wavtool",
                "worldline-wavtool.exe",
            ],
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
