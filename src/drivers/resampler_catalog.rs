use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnownResampler {
    Catalina,
    MacRes,
    Organum,
    StraycatRs,
    HifisamplerRs,
    World4Utau,
    Tips,
    Moresampler,
    OpenMoresampler,
}

static CACHE: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<KnownResampler, Option<PathBuf>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

impl KnownResampler {
    pub const ALL: [Self; 9] = [
        Self::Catalina,
        Self::MacRes,
        Self::Organum,
        Self::StraycatRs,
        Self::HifisamplerRs,
        Self::World4Utau,
        Self::Tips,
        Self::Moresampler,
        Self::OpenMoresampler,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Catalina => "Catalina (NSF HiFi-GAN)",
            Self::MacRes => "macres (titinko/macres)",
            Self::Organum => "Organum (KakouLabs/Organum)",
            Self::StraycatRs => "straycat-rs (UtaUtaUtau)",
            Self::HifisamplerRs => "Hifisampler (Slidingwall/hifisampler-rs)",
            Self::World4Utau => "World4UTAU (xrdavies/world4utau)",
            Self::Tips => "TIPS (TIPS.exe)",
            Self::Moresampler => "moresampler (moresampler.exe)",
            Self::OpenMoresampler => "OpenMoresampler (openmoresampler.exe)",
        }
    }

    pub const fn engine_id(self) -> &'static str {
        match self {
            Self::Catalina => "external.resampler.catalina",
            Self::MacRes => "external.resampler.macres",
            Self::Organum => "external.resampler.organum",
            Self::StraycatRs => "external.resampler.straycat-rs",
            Self::HifisamplerRs => "external.resampler.hifisampler-rs",
            Self::World4Utau => "external.resampler.world4utau",
            Self::Tips => "external.resampler.tips",
            Self::Moresampler => "external.resampler.moresampler",
            Self::OpenMoresampler => "external.resampler.openmoresampler",
        }
    }

    pub const fn executable_names(self) -> &'static [&'static str] {
        match self {
            Self::Catalina => &[
                "catalina",
                "catalina.exe",
                "hifisampler",
                "hifisampler.exe",
                "hifiserver-rust",
                "hifiserver-rust.exe",
                "hifisampler-rs",
                "hifisampler-rs.exe",
            ],
            Self::MacRes => &["macres", "macres.exe"],
            Self::Organum => &["organum-resampler", "organum-resampler.exe"],
            Self::StraycatRs => &["straycat-rs", "straycat-rs.exe"],
            Self::HifisamplerRs => &[
                "hifisampler",
                "hifisampler.exe",
                "hifiserver-rust",
                "hifiserver-rust.exe",
                "hifisampler-rs",
                "hifisampler-rs.exe",
            ],
            Self::World4Utau => &["world4utau", "world4utau.exe"],
            Self::Tips => &["TIPS.exe", "tips.exe", "TIPS", "tips"],
            Self::Moresampler => &["moresampler.exe", "moresampler"],
            Self::OpenMoresampler => &[
                "openmoresampler.exe",
                "openmoresampler",
                "moresampler.exe",
                "moresampler",
            ],
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|profile| profile.label() == label)
    }

    pub fn default_path(self) -> PathBuf {
        PathBuf::from("./resamplers").join(self.executable_names()[0])
    }

    pub fn clear_cache() {
        if let Ok(mut guard) = CACHE.lock() {
            guard.clear();
        }
    }

    pub fn find_executable(self) -> Option<PathBuf> {
        if let Ok(guard) = CACHE.lock() {
            if let Some(Some(cached)) = guard.get(&self) {
                // A resampler can be installed while the application is
                // running. Never keep a stale path alive after it disappears.
                if cached.is_file() {
                    return Some(cached.clone());
                }
            }
        }

        // Do not short-circuit a cached `None`: a newly copied/downloaded
        // resampler must become selectable without restarting Kamafeu.
        let result = self.search_executable_uncached();

        if let Ok(mut guard) = CACHE.lock() {
            guard.insert(self, result.clone());
        }

        result
    }

    fn search_executable_uncached(self) -> Option<PathBuf> {
        let mut roots = vec![
            PathBuf::from("./resamplers"),
            PathBuf::from("."),
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/bin"),
        ];

        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(exe_dir) = current_exe.parent() {
                roots.push(exe_dir.join("resamplers"));
                roots.push(exe_dir.to_path_buf());
                if let Some(p1) = exe_dir.parent() {
                    roots.push(p1.join("resamplers"));
                    if let Some(p2) = p1.parent() {
                        roots.push(p2.join("resamplers"));
                        if let Some(p3) = p2.parent() {
                            roots.push(p3.join("resamplers"));
                        }
                    }
                }
            }
        }

        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            roots.push(home.join("Documents/kamafeu/resamplers"));
            roots.push(home.join("Documents/kamafeu"));
            roots.push(home.join("Downloads/resamplers"));
            roots.push(home.join("Downloads"));
            roots.push(home.join(".local/bin"));
            roots.push(home.join("Library/Application Support/OpenUTAU/Resamplers"));
            roots.push(home.join("Library/Application Support/OpenUtau/Resamplers"));
            roots.push(home.join(".wine/drive_c/Program Files (x86)/UTAU/resamplers"));
            roots.push(home.join(".wine/drive_c/Program Files (x86)/UTAU"));
            roots.push(home.join(".wine/drive_c/Program Files/UTAU/resamplers"));
            roots.push(home.join(".wine/drive_c/Program Files/UTAU"));
            roots.push(home.join(".wine/drive_c/UTAU/resamplers"));
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
