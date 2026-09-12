use super::KamafeuStudioApp;
use eframe::egui::{self, RichText};

struct Package {
    name: &'static str,
    kind: &'static str,
    author: &'static str,
    license: &'static str,
    url: &'static str,
    windows_only: bool,
}

fn install_straycat(target_os: usize) -> Result<String, String> {
    let platform = match target_os {
        0 => "windows",
        1 => "macos x64",
        2 => "macos arm64",
        _ => "linux",
    };
    let api = "https://api.github.com/repos/UtaUtaUtau/straycat-rs/releases/tags/v1.1.0";
    let response = std::process::Command::new("curl")
        .args(["-fsSL", "-H", "Accept: application/vnd.github+json", api])
        .output()
        .map_err(|e| format!("curl indisponível: {e}"))?;
    if !response.status.success() {
        return Err("não foi possível consultar a release do GitHub".into());
    }
    let json: serde_json::Value = serde_json::from_slice(&response.stdout)
        .map_err(|e| format!("resposta inválida do GitHub: {e}"))?;
    let assets = json["assets"].as_array().ok_or("release sem assets")?;
    let asset = assets.iter().find(|asset| {
        let name = asset["name"].as_str().unwrap_or("").to_ascii_lowercase();
        name.contains(platform) || (target_os == 0 && name.contains("windows"))
    }).ok_or_else(|| format!("nenhum binário compatível com {platform}"))?;
    let name = asset["name"].as_str().ok_or("asset sem nome")?;
    let url = asset["browser_download_url"].as_str().ok_or("asset sem URL")?;
    let root = std::env::current_exe().map_err(|e| e.to_string())?
        .parent().ok_or("pasta do executável indisponível")?.join("packages").join("resamplers").join("straycat-rs");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let destination = root.join(name);
    let status = std::process::Command::new("curl").args(["-fL", "--retry", "3", "-o"]).arg(&destination).arg(url).status().map_err(|e| e.to_string())?;
    if !status.success() { return Err("falha ao baixar o binário".into()); }
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let mut p = std::fs::metadata(&destination).map_err(|e| e.to_string())?.permissions(); p.set_mode(0o755); std::fs::set_permissions(&destination, p).map_err(|e| e.to_string())?; }
    Ok(destination.display().to_string())
}

const PACKAGES: &[Package] = &[
    Package { name: "straycat-rs", kind: "Resampler", author: "UtaUtaUtau", license: "Ver licença", url: "https://github.com/UtaUtaUtau/straycat-rs/releases/tag/v1.1.0", windows_only: false },
    Package { name: "macres", kind: "Resampler", author: "titinko", license: "Consulte LICENSE", url: "https://github.com/titinko/macres/releases/tag/v0.2.1", windows_only: false },
    Package { name: "world4utau", kind: "Resampler", author: "xrdavies", license: "Consulte o repositório", url: "https://github.com/xrdavies/world4utau", windows_only: false },
    Package { name: "ESPER-Utau", kind: "Resampler", author: "CdrSonan", license: "Consulte LICENSE", url: "https://github.com/CdrSonan/ESPER-Utau/releases/tag/v2.5.0", windows_only: false },
    Package { name: "Organum", kind: "Resampler", author: "KakouLabs", license: "Consulte LICENSE", url: "https://github.com/KakouLabs/Organum/releases/tag/v0.0.8", windows_only: false },
    Package { name: "SpaceWorld", kind: "Resampler", author: "LovelyA72", license: "Consulte LICENSE", url: "https://github.com/LovelyA72/SpaceWorld/releases/tag/1.1.0", windows_only: true },
    Package { name: "ChopSampler", kind: "Resampler", author: "MLo7Ghinsan", license: "Consulte LICENSE", url: "https://github.com/MLo7Ghinsan/ChopSampler/releases/tag/1.2", windows_only: false },
    Package { name: "kuresampler", kind: "Resampler", author: "oatsu-gh", license: "Consulte LICENSE", url: "https://github.com/oatsu-gh/kuresampler/releases/tag/v0.0.1", windows_only: false },
    Package { name: "axis", kind: "Resampler", author: "cyntheria", license: "Consulte o repositório", url: "https://github.com/cyntheria/axis", windows_only: false },
    Package { name: "NeoWorld", kind: "Resampler", author: "LovelyA72", license: "Consulte LICENSE", url: "https://github.com/LovelyA72/NeoWorld/releases/tag/msvc-0.1", windows_only: true },
    Package { name: "SillyBeams", kind: "Wavtool", author: "MLo7Ghinsan", license: "Consulte LICENSE", url: "https://github.com/MLo7Ghinsan/SillySeams/releases/tag/1.0.1", windows_only: false },
    Package { name: "wavtool-yawu", kind: "Wavtool", author: "m13253", license: "Consulte LICENSE", url: "https://github.com/m13253/wavtool-yawu", windows_only: false },
    Package { name: "wavtool-pl", kind: "Wavtool", author: "yuanchao", license: "Consulte LICENSE", url: "https://github.com/yuanchao/wavtool-pl", windows_only: false },
    Package { name: "WavTool-CS", kind: "Wavtool", author: "OpenSynth", license: "Consulte LICENSE", url: "https://github.com/OpenSynth/WavTool-CS", windows_only: false },
    Package { name: "kladtool", kind: "Wavtool", author: "adlez27", license: "Consulte LICENSE", url: "https://github.com/adlez27/kladtool", windows_only: false },
    Package { name: "wavtool-rs", kind: "Wavtool", author: "SHIACKOWORKS", license: "Consulte LICENSE", url: "https://github.com/SHIACKOWORKS/wavtool-rs", windows_only: false },
];

impl KamafeuStudioApp {
    pub(crate) fn show_packages_window(&mut self, ctx: &egui::Context) {
        if !self.packages_window_open {
            return;
        }
        let mut open = true;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("kamafeu_packages_viewport"),
            egui::ViewportBuilder::default()
                .with_title("Packages - Kamafeu Studio")
                .with_inner_size([1080.0, 700.0])
                .with_min_inner_size([760.0, 480.0]),
            |ctx, _| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_packages_tab(ui);
                });
                if ctx.input(|i| i.viewport().close_requested()) {
                    open = false;
                }
            },
        );
        self.packages_window_open = open;
    }

    pub(super) fn render_packages_tab(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        ui.heading(lang.tr("Pacotes adicionais", "Additional packages"));
        ui.label(lang.tr(
            "Baixe resamplers e wavtools oficiais. O botão abre a página de release para escolher o executável correto do seu sistema.",
            "Download official resamplers and wavtools. The button opens the release page so you can choose the executable for your system.",
        ));
        ui.horizontal(|ui| {
            ui.label(lang.tr("Sistema operacional:", "Operating system:"));
            let options = ["Windows", "macOS Intel", "macOS Apple Silicon", "Linux"];
            egui::ComboBox::from_id_salt("packages_target_os")
                .selected_text(options[self.packages_target_os.min(options.len() - 1)])
                .show_ui(ui, |ui| {
                    for (idx, label) in options.iter().enumerate() {
                        ui.selectable_value(&mut self.packages_target_os, idx, *label);
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label(lang.tr("Buscar:", "Search:"));
            ui.add_sized([260.0, 22.0], egui::TextEdit::singleline(&mut self.packages_search));
            if ui.button(lang.tr("Atualizar lista", "Refresh list")).clicked() {
                self.packages_search.clear();
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().max_height(560.0).show(ui, |ui| {
            for (kind, title) in [("Resampler", lang.tr("RESAMPLERS", "RESAMPLERS")), ("Wavtool", lang.tr("WAVTOOLS", "WAVTOOLS"))] {
                ui.add_space(8.0);
                ui.label(RichText::new(title).strong().color(egui::Color32::from_rgb(0, 255, 157)));
                egui::Grid::new(("packages_table", kind))
                .num_columns(6)
                .striped(true)
                .min_col_width(90.0)
                .spacing([16.0, 8.0])
                .show(ui, |ui| {
                    for (heading, width) in [
                        (lang.tr("ID", "ID"), 170.0),
                        (lang.tr("Tipo", "Type"), 90.0),
                        (lang.tr("Desenvolvedor", "Developer"), 130.0),
                        (lang.tr("Licença", "License"), 130.0),
                        (lang.tr("Compatibilidade", "Compatibility"), 180.0),
                        (lang.tr("Ação", "Action"), 90.0),
                    ] {
                        ui.add_sized([width, 22.0], egui::Label::new(RichText::new(heading).strong()));
                    }
                    ui.end_row();

                    for package in PACKAGES.iter().filter(|package| package.kind == kind && (self.packages_search.trim().is_empty() || package.name.to_ascii_lowercase().contains(&self.packages_search.to_ascii_lowercase()) || package.author.to_ascii_lowercase().contains(&self.packages_search.to_ascii_lowercase()))) {
                        ui.add_sized([170.0, 24.0], egui::Label::new(RichText::new(package.name).strong()));
                        ui.add_sized([90.0, 24.0], egui::Label::new(package.kind));
                        ui.add_sized([130.0, 24.0], egui::Label::new(package.author));
                        ui.add_sized([130.0, 24.0], egui::Label::new(package.license));
                        let compatibility = if package.windows_only {
                            "Windows; Wine no macOS/Linux"
                        } else {
                            "Consulte os assets"
                        };
                        ui.add_sized([180.0, 24.0], egui::Label::new(RichText::new(compatibility).italics()));
                        if ui.button(lang.tr("Instalar", "Install")).clicked() {
                            if package.name == "straycat-rs" {
                                match install_straycat(self.packages_target_os) {
                                    Ok(path) => ui.ctx().copy_text(format!("Instalado: {path}")),
                                    Err(error) => ui.ctx().copy_text(format!("Erro: {error}")),
                                }
                            }
                        // Until a release publishes a uniquely identifiable
                        // asset for this platform, open the official release
                        // page as a safe guided installer rather than guessing
                        // and installing an incompatible binary.
                        #[cfg(target_os = "macos")]
                        let _ = std::process::Command::new("open").arg(package.url).spawn();
                        #[cfg(target_os = "windows")]
                        let _ = std::process::Command::new("cmd").args(["/C", "start", "", package.url]).spawn();
                        #[cfg(all(unix, not(target_os = "macos")))]
                        let _ = std::process::Command::new("xdg-open").arg(package.url).spawn();
                        }
                        ui.end_row();
                    }
                });
            }
        });
        ui.separator();
        let install_root = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.join("packages")));
        ui.horizontal_wrapped(|ui| {
            ui.label(lang.tr("Pasta de instalação:", "Install location:"));
            ui.monospace(install_root.as_deref().map(|p| p.display().to_string()).unwrap_or_else(|| "indisponível".into()));
            ui.label(RichText::new(lang.tr("Pronto", "Ready")).color(egui::Color32::from_rgb(0, 255, 157)));
        });
    }
}
