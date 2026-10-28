use crate::drivers::KnownResampler;
use eframe::egui::{self, Color32, RichText};
use std::path::{Path, PathBuf};

const HIFIGAN_MODEL_FILE: &str = "pc_nsf_hifigan_44.1k_hop512_128bin_2025.02.onnx";
const HIFIGAN_MODEL_URL: &str = "https://github.com/openvpi/vocoders/releases/download/pc-nsf-hifigan-44.1k-hop512-128bin-2025.02/pc_nsf_hifigan_44.1k_hop512_128bin_2025.02.oudep";
const HIFISAMPLER_DOWNLOADS_URL: &str = "https://github.com/openhachimi/hifisampler/releases";
const HIFISAMPLER_RS_URL: &str = "https://github.com/Slidingwall/hifisampler-rs/releases";

fn selected_profile(label: &str) -> Option<KnownResampler> {
    KnownResampler::ALL
        .into_iter()
        .find(|profile| label == profile.label() || label.starts_with(profile.label()))
        .or_else(|| {
            if label.eq_ignore_ascii_case("catalina") {
                Some(KnownResampler::Catalina)
            } else {
                None
            }
        })
}

fn download_url(profile: KnownResampler) -> &'static str {
    match profile {
        KnownResampler::Catalina => "https://github.com/studiopomar/kamafeu/releases",
        KnownResampler::MacRes => "https://github.com/titinko/macres/releases",
        KnownResampler::Organum => "https://github.com/KakouLabs/Organum/releases",
        KnownResampler::StraycatRs => "https://github.com/UtaUtaUtau/straycat-rs/releases",
        KnownResampler::HifisamplerRs => HIFISAMPLER_DOWNLOADS_URL,
        KnownResampler::World4Utau => "https://github.com/xrdavies/world4utau/releases",
        KnownResampler::Tips | KnownResampler::Moresampler => {
            "https://github.com/openutau/OpenUtau/wiki/Resamplers-and-Wavtools"
        }
        KnownResampler::OpenMoresampler => "https://github.com/dorayakito/OpenMoresampler/releases",
    }
}

fn is_hifigan_engine(profile: KnownResampler) -> bool {
    matches!(
        profile,
        KnownResampler::Catalina | KnownResampler::HifisamplerRs
    )
}

fn hifigan_model_exists(dir: &Path) -> bool {
    [HIFIGAN_MODEL_FILE, "pc-nsf-hifigan.onnx", "model.onnx"]
        .iter()
        .any(|name| dir.join(name).is_file())
}

fn hnsep_model_exists(dir: &Path) -> bool {
    dir.join("hnsep_model.onnx").is_file()
}

fn engine_dir(profile: KnownResampler, custom_path: Option<&Path>) -> PathBuf {
    custom_path
        .filter(|path| path.is_file())
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .or_else(|| {
            profile
                .find_executable()
                .and_then(|path| path.parent().map(Path::to_path_buf))
        })
        .unwrap_or_else(|| profile.recommended_engine_dir())
}

fn reveal_directory(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)?;
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("explorer").arg(path).spawn();
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let result = std::process::Command::new("xdg-open").arg(path).spawn();
    result.map(|_| ())
}

pub(super) fn draw(
    ui: &mut egui::Ui,
    selected_resampler: &str,
    custom_resampler_path: Option<&Path>,
) {
    let Some(profile) = selected_profile(selected_resampler) else {
        return;
    };

    let executable = custom_resampler_path
        .filter(|path| path.is_file())
        .map(Path::to_path_buf)
        .or_else(|| profile.find_executable());
    let folder = engine_dir(profile, custom_resampler_path);

    ui.add_space(4.0);
    ui.group(|ui| {
        if let Some(path) = executable.as_ref() {
            ui.label(
                RichText::new(format!("✓ Resampler encontrado: {}", path.display()))
                    .size(9.5)
                    .color(Color32::from_rgb(105, 205, 150)),
            );
        } else {
            ui.label(
                RichText::new("Resampler externo ausente; este perfil não renderizará até ser instalado.")
                    .size(9.5)
                    .color(ui.visuals().warn_fg_color),
            );
            ui.label(
                RichText::new(format!(
                    "Baixe {} e coloque o executável nesta pasta:",
                    profile.recommended_executable_name()
                ))
                .size(9.0),
            );
            ui.monospace(folder.display().to_string())
                .on_hover_text(folder.display().to_string());
            ui.horizontal_wrapped(|ui| {
                ui.hyperlink_to("Downloads / instruções do engine", download_url(profile));
                if ui.small_button("Abrir pasta destino").clicked() {
                    if let Err(error) = reveal_directory(&folder) {
                        eprintln!("Não foi possível abrir a pasta de resamplers: {error}");
                    }
                }
            });
            if matches!(profile, KnownResampler::Moresampler | KnownResampler::OpenMoresampler)
                && cfg!(any(target_os = "macos", target_os = "linux"))
            {
                ui.label(
                    RichText::new("No macOS/Linux, o executável Windows também requer Wine.")
                        .size(9.0)
                        .color(ui.visuals().weak_text_color()),
                );
            }
        }

        if is_hifigan_engine(profile) {
            let model_dir = folder.join("model");
            if hifigan_model_exists(&model_dir) {
                ui.label(
                    RichText::new(format!("✓ Modelo PC-NSF-HiFiGAN encontrado em {}", model_dir.display()))
                        .size(9.0)
                        .color(Color32::from_rgb(105, 205, 150)),
                );
            } else {
                ui.separator();
                ui.label(
                    RichText::new("Modelo PC-NSF-HiFiGAN ausente. Baixe o pacote ONNX e copie o arquivo .onnx para:")
                        .size(9.0)
                        .color(ui.visuals().warn_fg_color),
                );
                ui.monospace(model_dir.display().to_string())
                    .on_hover_text(model_dir.display().to_string());
                ui.horizontal_wrapped(|ui| {
                    ui.hyperlink_to("Baixar modelo PC-NSF-HiFiGAN (.oudep)", HIFIGAN_MODEL_URL);
                    if ui.small_button("Abrir pasta do modelo").clicked() {
                        if let Err(error) = reveal_directory(&model_dir) {
                            eprintln!("Não foi possível abrir a pasta do modelo: {error}");
                        }
                    }
                });
                ui.label(
                    RichText::new(format!(
                        "No pacote baixado, renomeie .oudep para .zip, extraia e coloque {HIFIGAN_MODEL_FILE} nessa pasta."
                    ))
                    .size(8.5)
                    .color(ui.visuals().weak_text_color()),
                );
                ui.hyperlink_to(
                    "Licença do modelo: CC BY-NC-SA 4.0",
                    "https://creativecommons.org/licenses/by-nc-sa/4.0/",
                );
            }

            if profile == KnownResampler::HifisamplerRs {
                if hnsep_model_exists(&model_dir) {
                    ui.label(
                        RichText::new("✓ Modelo HN-SEP encontrado")
                            .size(9.0)
                            .color(Color32::from_rgb(105, 205, 150)),
                    );
                } else {
                    ui.label(
                        RichText::new("O Hifisampler também requer hnsep_model.onnx na pasta model/.")
                            .size(8.5)
                            .color(ui.visuals().warn_fg_color),
                    );
                    ui.hyperlink_to("Downloads e instruções dos modelos Hifisampler", HIFISAMPLER_DOWNLOADS_URL);
                }
                ui.label(
                    RichText::new("O pacote Hifisampler inclui cliente e servidor. Preserve a estrutura extraída e inicie start.bat/servidor conforme o upstream; Hifisampler-RS v1.1 é somente um backend opcional.")
                        .size(8.5)
                        .color(ui.visuals().weak_text_color()),
                );
                ui.hyperlink_to("Backend opcional Hifisampler-RS", HIFISAMPLER_RS_URL);
            }
        }
    });
}
