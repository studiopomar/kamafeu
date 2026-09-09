use crate::drivers::ExternalResamplerDriver;
use crate::drivers::ExternalWavtoolDriver;
use crate::drivers::KnownResampler;
use crate::drivers::KnownWavtool;
use crate::drivers::MacResDriver;
use crate::drivers::NativeResamplerDriver;
use crate::drivers::NativeSolaResamplerDriver;
use crate::drivers::NativeWorldResamplerDriver;
use crate::drivers::NativeWavtoolDriver;
use crate::drivers::ResamplerDriver;
use crate::drivers::WavtoolDriver;
use crate::drivers::WavtoolYawuDriver;
use crate::gui::KamafeuStudioApp;
use std::path::PathBuf;

impl KamafeuStudioApp {
    pub(super) fn persist_config(&mut self) {
        if let Err(error) = self.config.save() {
            self.transport_state.status_message = error;
        }
    }

    pub(super) fn create_resampler_driver(&self) -> Box<dyn ResamplerDriver> {
        #[cfg(target_os = "android")]
        {
            return Box::new(NativeSolaResamplerDriver {
                mode: crate::dsp::SolaStretchMode::Hybrid,
            });
        }

        #[cfg(not(target_os = "android"))]
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

            if self.selected_resampler.contains("Phase Vocoder")
                || self.selected_resampler.contains("SOLA Híbrido")
            {
                return Box::new(NativeSolaResamplerDriver {
                    mode: crate::dsp::SolaStretchMode::Hybrid,
                });
            }

            if self.selected_resampler.contains("SOLA") || self.selected_resampler.contains("WSOLA")
            {
                let mode = if self.selected_resampler.contains("Loop") {
                    crate::dsp::SolaStretchMode::Loop
                } else if self.selected_resampler.contains("Spline") {
                    crate::dsp::SolaStretchMode::Spline
                } else {
                    crate::dsp::SolaStretchMode::Stretch
                };
                return Box::new(NativeSolaResamplerDriver { mode });
            }

            if self.selected_resampler.contains("Venus")
                || self.selected_resampler.contains("venus")
                || self.selected_resampler.contains("WORLD")
                || self.selected_resampler.contains("world")
            {
                return Box::new(NativeWorldResamplerDriver);
            }

            if self.selected_resampler.contains("TD-PSOLA")
                || self.selected_resampler.contains("PSOLA")
            {
                return Box::new(NativeResamplerDriver);
            }

            if self.selected_resampler.contains("Native")
                || self.selected_resampler.contains("Nativo")
            {
                return Box::new(NativeSolaResamplerDriver::default());
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

    pub(super) fn create_wavtool_driver(&self) -> Box<dyn WavtoolDriver> {
        #[cfg(target_os = "android")]
        {
            return Box::new(NativeWavtoolDriver);
        }

        #[cfg(not(target_os = "android"))]
        {
            if self.selected_wavtool.contains("Native") || self.selected_wavtool.contains("Nativo")
            {
                return Box::new(NativeWavtoolDriver);
            }

            if let Some(profile) = KnownWavtool::from_label(&self.selected_wavtool) {
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
