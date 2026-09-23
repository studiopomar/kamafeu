use crate::drivers::ExternalResamplerDriver;
use crate::drivers::ExternalWavtoolDriver;
use crate::drivers::GalapagosWavtoolDriver;
use crate::drivers::KnownResampler;
use crate::drivers::KnownWavtool;
use crate::drivers::MacResDriver;
use crate::drivers::NativeVenusResamplerDriver;
use crate::drivers::NativeWavtoolDriver;
use crate::drivers::ResamplerDriver;
use crate::drivers::WavtoolDriver;
use crate::drivers::WavtoolYawuDriver;
use crate::gui::KamafeuStudioApp;
use std::path::PathBuf;

impl KamafeuStudioApp {
    pub(super) fn persist_config(&mut self) {
        crate::renderer::resampler_cache::set_persistent_cache_dir_override(
            self.config.memory.custom_cache_dir.clone(),
        );
        crate::renderer::resampler_cache::set_cache_limits(
            self.config.memory.max_ram_cache_mb,
            self.config.memory.max_disk_cache_mb,
        );
        self.config.layout.show_arrangement_view = self.piano_roll_state.show_arrangement_view;
        self.config.layout.show_parameters_drawer = self.piano_roll_state.show_parameters_drawer;
        self.config.layout.show_phoneme_ruler = self.piano_roll_state.show_phoneme_ruler;
        self.config.layout.show_inspector = self.piano_roll_state.show_inspector;
        self.config.layout.is_maximized = self.piano_roll_state.is_maximized;
        self.config.layout.px_per_ms = self.piano_roll_state.px_per_ms;
        self.config.layout.row_height = self.piano_roll_state.row_height;
        self.config.dsp.render_threads = self.render_threads;
        self.config.dsp.default_resampler = self.selected_resampler.clone();
        self.config.dsp.default_wavtool = self.selected_wavtool.clone();
        self.config.dsp.custom_resampler_path = self.custom_resampler_path.clone();
        self.config.dsp.custom_wavtool_path = self.custom_wavtool_path.clone();
        self.config.vocal_render = self.vocal_mode_params.clone();
        if let Some(voicebank) = &self.voicebank {
            self.config
                .voicebank_render_profiles
                .insert(voicebank.root_path.clone(), self.vocal_mode_params.clone());
            self.config.voicebank_engine_profiles.insert(
                voicebank.root_path.clone(),
                crate::config::EngineProfile {
                    resampler: Some(self.selected_resampler.clone()),
                    wavtool: Some(self.selected_wavtool.clone()),
                },
            );
        }
        if let Some(project_path) = &self.current_project_path {
            self.config
                .project_render_profiles
                .insert(project_path.clone(), self.vocal_mode_params.clone());
            self.config.project_engine_profiles.insert(
                project_path.clone(),
                crate::config::EngineProfile {
                    resampler: Some(self.selected_resampler.clone()),
                    wavtool: Some(self.selected_wavtool.clone()),
                },
            );
        }
        if let Err(error) = self.config.save() {
            self.transport_state.status_message = error;
        }
    }

    pub(super) fn create_resampler_driver(&self) -> Box<dyn ResamplerDriver> {
        #[cfg(any(target_os = "android", target_arch = "wasm32"))]
        {
            return Box::new(NativeVenusResamplerDriver::default());
        }

        #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
        {
            if self.selected_resampler.contains("straycat") {
                let profile = KnownResampler::StraycatRs;
                return Box::new(ExternalResamplerDriver::for_known(
                    profile,
                    self.custom_resampler_path.clone(),
                ));
            }

            if self.selected_resampler.contains("Hifisampler")
                || self.selected_resampler.contains("hifisampler")
            {
                let profile = KnownResampler::HifisamplerRs;
                return Box::new(ExternalResamplerDriver::for_known(
                    profile,
                    self.custom_resampler_path.clone(),
                ));
            }

            if self.selected_resampler.contains("Venus")
                || self.selected_resampler.contains("venus")
                || self.selected_resampler.contains("VENUS")
                || self.selected_resampler.contains("WORLD")
                || self.selected_resampler.contains("world")
                || self.selected_resampler.contains("Native")
                || self.selected_resampler.contains("Nativo")
            {
                let preserve_formants = self.config.dsp.formant_preservation_mode != "Desativado";
                let f0_method = match self.config.dsp.f0_detection_method.as_str() {
                    "pyIN (Probabilístico)" => crate::dsp::F0TrackerMethod::Pyin,
                    "Harvest/DIO (Espectral)" => crate::dsp::F0TrackerMethod::World,
                    _ => crate::dsp::F0TrackerMethod::Yin,
                };
                return Box::new(NativeVenusResamplerDriver::new(
                    self.config.dsp.oversampling_factor,
                    preserve_formants,
                    f64::from(self.config.dsp.f0_min_hz),
                    f64::from(self.config.dsp.f0_max_hz),
                    f0_method,
                ));
            }

            if let Some(profile) = KnownResampler::from_label(&self.selected_resampler) {
                if profile == KnownResampler::MacRes {
                    let path = self
                        .custom_resampler_path
                        .clone()
                        .filter(|path| path.is_file())
                        .or_else(|| profile.find_executable())
                        .unwrap_or_else(|| profile.default_path());
                    return Box::new(MacResDriver::new(path));
                }
                return Box::new(ExternalResamplerDriver::for_known(
                    profile,
                    self.custom_resampler_path.clone(),
                ));
            }

            let path = self
                .custom_resampler_path
                .clone()
                .unwrap_or_else(|| PathBuf::from(&self.selected_resampler));
            Box::new(ExternalResamplerDriver::new(path))
        }
    }

    pub(super) fn restore_render_profile_for_active_context(&mut self) {
        let profile = self
            .current_project_path
            .as_ref()
            .and_then(|path| self.config.project_render_profiles.get(path))
            .or_else(|| {
                self.voicebank.as_ref().and_then(|voicebank| {
                    self.config
                        .voicebank_render_profiles
                        .get(&voicebank.root_path)
                })
            })
            .cloned()
            .unwrap_or_else(|| self.config.vocal_render.clone());
        self.vocal_mode_params = profile;
        let engine_profile = self
            .current_project_path
            .as_ref()
            .and_then(|path| self.config.project_engine_profiles.get(path))
            .or_else(|| {
                self.voicebank.as_ref().and_then(|voicebank| {
                    self.config
                        .voicebank_engine_profiles
                        .get(&voicebank.root_path)
                })
            });
        if let Some(engine_profile) = engine_profile {
            if let Some(resampler) = &engine_profile.resampler {
                self.selected_resampler = resampler.clone();
            }
            if let Some(wavtool) = &engine_profile.wavtool {
                self.selected_wavtool = wavtool.clone();
            }
        }
    }

    pub(super) fn create_wavtool_driver(&self) -> Box<dyn WavtoolDriver> {
        #[cfg(any(target_os = "android", target_arch = "wasm32"))]
        {
            return Box::new(GalapagosWavtoolDriver);
        }

        #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
        {
            if self.selected_wavtool.contains("Galapagos")
                || self.selected_wavtool.contains("galapagos")
                || self.selected_wavtool.contains("OpenUtau")
                || self.selected_wavtool.contains("openutau")
                || self.selected_wavtool.contains("Worldline")
                || self.selected_wavtool.contains("worldline")
            {
                if let Some(ref path) = self.custom_wavtool_path {
                    if path.is_file() {
                        return Box::new(ExternalWavtoolDriver::new(path.clone()));
                    }
                }
                return Box::new(GalapagosWavtoolDriver);
            }

            if self.selected_wavtool.contains("Andromeda")
                || self.selected_wavtool.contains("andromeda")
                || self.selected_wavtool.contains("Alternativo")
                || self.selected_wavtool.contains("alternativo")
            {
                return Box::new(NativeWavtoolDriver);
            }

            if self.selected_wavtool.contains("Native") || self.selected_wavtool.contains("Nativo")
            {
                return Box::new(GalapagosWavtoolDriver);
            }

            if let Some(profile) = KnownWavtool::from_label(&self.selected_wavtool) {
                if profile == KnownWavtool::Galapagos {
                    if let Some(path) = self
                        .custom_wavtool_path
                        .clone()
                        .filter(|p| p.is_file())
                        .or_else(|| profile.find_executable())
                    {
                        return Box::new(ExternalWavtoolDriver::new(path));
                    }
                    return Box::new(GalapagosWavtoolDriver);
                }
                if profile == KnownWavtool::WavtoolYawu {
                    let path = self
                        .custom_wavtool_path
                        .clone()
                        .filter(|p| p.is_file())
                        .or_else(|| profile.find_executable())
                        .unwrap_or_else(|| profile.default_path());
                    return Box::new(WavtoolYawuDriver::new(path));
                }
                return Box::new(ExternalWavtoolDriver::for_known(
                    profile,
                    self.custom_wavtool_path.clone(),
                ));
            }

            let path = self
                .custom_wavtool_path
                .clone()
                .unwrap_or_else(|| PathBuf::from(&self.selected_wavtool));
            Box::new(ExternalWavtoolDriver::new(path))
        }
    }
}
