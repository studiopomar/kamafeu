use super::CopaibaToolkitApp;
use crate::audio::AudioPlayer;

impl CopaibaToolkitApp {
    pub fn load_wav_file(&self, filename: &str) -> Option<(Vec<f32>, u32)> {
        self.current_dir.as_ref().and_then(|directory| {
            crate::renderer::TrackRenderer::load_wav_samples(directory.join(filename)).ok()
        })
    }

    pub fn load_selected_wav_samples(&mut self) {
        if self.native_project.is_some() {
            self.load_native_selected_wav_samples();
            return;
        }
        let target_wav = self.selected_entry().map(|e| e.wav_filename.clone());
        if let (Some(ref dir), Some(ref wav_name)) = (&self.current_dir, &target_wav) {
            if let Ok(waveform) =
                crate::renderer::TrackRenderer::load_wav_samples(dir.join(wav_name))
            {
                let wav_path = dir.join(wav_name);
                self.venus_analysis =
                    crate::dsp::venus_analysis::load_or_analyze(&wav_path, &waveform.0, waveform.1)
                        .ok();
                self.venus_analysis_path = self
                    .venus_analysis
                    .as_ref()
                    .map(|_| crate::dsp::venus_analysis::sidecar_path(&wav_path));
                self.loaded_waveform = Some(waveform);
                self.loaded_wav_filename = Some(wav_name.clone());
            } else {
                self.loaded_waveform = None;
                self.loaded_wav_filename = None;
                self.venus_analysis = None;
                self.venus_analysis_path = None;
            }
        } else {
            self.loaded_waveform = None;
            self.loaded_wav_filename = None;
            self.venus_analysis = None;
            self.venus_analysis_path = None;
        }

        self.previous_waveform = self
            .selected_entry_index
            .and_then(|idx| idx.checked_sub(1))
            .and_then(|idx| self.config.entries.get(idx))
            .and_then(|entry| {
                self.load_wav_file(&entry.wav_filename)
                    .map(|(s, sr)| (entry.wav_filename.clone(), s, sr))
            });
        self.next_waveform = self
            .selected_entry_index
            .and_then(|idx| self.config.entries.get(idx + 1))
            .and_then(|entry| {
                self.load_wav_file(&entry.wav_filename)
                    .map(|(s, sr)| (entry.wav_filename.clone(), s, sr))
            });
    }

    pub(super) fn load_native_selected_wav_samples(&mut self) {
        let sample = self.native_project.as_ref().and_then(|project| {
            let layer_id = self.native_selected_layer.as_deref()?;
            let unit_id = self.native_selected_unit.as_deref()?;
            let unit = project.unit(layer_id, unit_id)?;
            let (_, file) = project
                .layers
                .iter()
                .find(|(layer, _)| layer.id == layer_id)?;
            let asset = file.assets.iter().find(|asset| asset.id == unit.asset_id)?;
            Some(project.root.join(&asset.path))
        });
        if let Some(path) = sample {
            match crate::renderer::TrackRenderer::load_wav_samples(&path) {
                Ok(waveform) => {
                    self.loaded_wav_filename = Some(path.to_string_lossy().into_owned());
                    self.loaded_waveform = Some(waveform);
                }
                Err(error) => {
                    self.loaded_wav_filename = None;
                    self.loaded_waveform = None;
                    self.status_message = format!("Falha ao carregar gravação nativa: {error}");
                }
            }
        } else {
            self.loaded_wav_filename = None;
            self.loaded_waveform = None;
        }
    }

    pub fn load_native_comparison_waveforms(&mut self) -> Result<(), String> {
        let project = self
            .native_project
            .as_ref()
            .ok_or("Nenhum banco KFV 2 aberto")?;
        let layer_id = self
            .native_selected_layer
            .as_deref()
            .ok_or("Nenhuma camada selecionada")?;
        let asset_a = self
            .native_selected_asset
            .as_deref()
            .ok_or("Selecione a gravação A")?;
        let asset_b = self.native_compare_asset.as_deref();
        if asset_b == Some(asset_a) {
            return Err("A e B precisam ser gravações diferentes".into());
        }
        let waveform_a = project.load_asset_audio(layer_id, asset_a)?;
        let waveform_b = asset_b
            .map(|asset_id| project.load_asset_audio(layer_id, asset_id))
            .transpose()?;
        self.native_comparison_a = Some(waveform_a);
        self.native_comparison_b = waveform_b;
        Ok(())
    }

    pub fn preview_native_comparison_take(
        &self,
        audio_player: &mut AudioPlayer,
        take: char,
    ) -> Result<(), String> {
        let waveform = match take.to_ascii_uppercase() {
            'A' => self.native_comparison_a.as_ref(),
            'B' => self.native_comparison_b.as_ref(),
            _ => return Err("Selecione a tomada A ou B".into()),
        }
        .ok_or("Waveform da tomada ainda não carregada")?;
        audio_player.play_samples(waveform.0.clone(), waveform.1);
        Ok(())
    }
}
