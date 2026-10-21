use super::CopaibaToolkitApp;
use crate::audio::AudioPlayer;

impl CopaibaToolkitApp {
    pub fn preview_native_unit(&mut self, audio_player: &mut AudioPlayer) -> Result<(), String> {
        self.preview_native_unit_backend(
            audio_player,
            crate::voicebank::render::PreviewBackend::Sola,
        )
    }

    pub fn preview_native_unit_backend(
        &mut self,
        audio_player: &mut AudioPlayer,
        backend: crate::voicebank::render::PreviewBackend,
    ) -> Result<(), String> {
        let project = self
            .native_project
            .as_ref()
            .ok_or("Nenhum banco KFV 2 aberto")?;
        let layer_id = self
            .native_selected_layer
            .as_deref()
            .ok_or("Nenhuma camada selecionada")?;
        let unit_id = self
            .native_selected_unit
            .as_deref()
            .ok_or("Nenhuma unidade selecionada")?;
        let (layer, _) = project
            .layers
            .iter()
            .find(|(layer, _)| layer.id == layer_id)
            .ok_or("Camada selecionada não existe")?;
        let unit = project
            .unit(layer_id, unit_id)
            .ok_or("Unidade selecionada não existe")?;
        let event = crate::voicebank::planner::PhoneEvent {
            phones: unit.phones.clone(),
            kind: unit.kind.clone(),
            position_ms: 0.0,
            duration_ms: unit.timing.min_target_ms,
            midi: layer.recorded_midi,
            pitch_points: Vec::new(),
            expressions: crate::project::model::UExpressions::default(),
            flags: String::new(),
            unit_override_id: Some(unit.id.clone()),
            layer_override_id: Some(layer.id.clone()),
        };
        let bank = project.native_bank_snapshot();
        let phrase = crate::voicebank::planner::PhraseRequest {
            style_id: layer.style_id.clone(),
            events: vec![event],
        };
        let plan = crate::voicebank::planner::plan_phrase(&bank, &phrase)?;
        let rendered = match backend {
            crate::voicebank::render::PreviewBackend::Sola => {
                crate::voicebank::render::render_phrase_sola(&bank, &plan, 48_000)?
            }
            crate::voicebank::render::PreviewBackend::World => {
                crate::voicebank::render::render_phrase_world(&bank, &plan, 48_000)?
            }
        };
        if rendered.samples.is_empty() {
            return Err(format!(
                "{} não produziu áudio para a unidade selecionada",
                rendered.engine_id
            ));
        }
        audio_player.play_samples(rendered.samples, rendered.sample_rate_hz);
        let fallback = rendered
            .units
            .iter()
            .flat_map(|unit| unit.fallback_reasons.iter())
            .next()
            .cloned();
        self.status_message = format!(
            "Preview {} · {}{}",
            rendered.engine_id,
            unit.id,
            fallback
                .map(|reason| format!(" · fallback: {reason}"))
                .unwrap_or_default()
        );
        Ok(())
    }

    pub fn preview_native_phrase(&mut self, audio_player: &mut AudioPlayer) -> Result<(), String> {
        self.preview_native_phrase_backend(
            audio_player,
            crate::voicebank::render::PreviewBackend::Sola,
        )
    }

    pub fn preview_native_phrase_backend(
        &mut self,
        audio_player: &mut AudioPlayer,
        backend: crate::voicebank::render::PreviewBackend,
    ) -> Result<(), String> {
        let project = self
            .native_project
            .as_ref()
            .ok_or("Nenhum banco KFV 2 aberto")?;
        let layer_id = self
            .native_selected_layer
            .as_deref()
            .ok_or("Nenhuma camada selecionada")?;
        let plan = project.plan_direct_phone_phrase(layer_id, &self.native_test_phones)?;
        let bank = project.native_bank_snapshot();
        let rendered = match backend {
            crate::voicebank::render::PreviewBackend::Sola => {
                crate::voicebank::render::render_phrase_sola(&bank, &plan, 48_000)?
            }
            crate::voicebank::render::PreviewBackend::World => {
                crate::voicebank::render::render_phrase_world(&bank, &plan, 48_000)?
            }
        };
        if rendered.samples.is_empty() {
            return Err(format!(
                "{} não produziu áudio para a frase de teste",
                rendered.engine_id
            ));
        }
        audio_player.play_samples(rendered.samples, rendered.sample_rate_hz);
        let fallback = rendered
            .units
            .iter()
            .flat_map(|unit| unit.fallback_reasons.iter())
            .next()
            .cloned();
        self.status_message = format!(
            "Preview {} · {} eventos{}",
            rendered.engine_id,
            plan.units.len(),
            fallback
                .map(|reason| format!(" · fallback: {reason}"))
                .unwrap_or_default()
        );
        Ok(())
    }
}
