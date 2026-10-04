//! Native DiffSinger ONNX runtime primitives.
//!
//! This module intentionally owns model loading and contract validation. The
//! project renderer can use it without knowing whether a bank is UTAU or
//! neural, and malformed models fail before an audio render starts.

use crate::oto::DiffSingerConfig;
#[cfg(not(target_arch = "wasm32"))]
use ndarray::{Array1, Array2, Array3};
#[cfg(test)]
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use ort::session::Session;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSingerModelContract {
    pub acoustic_inputs: Vec<String>,
    pub acoustic_outputs: Vec<String>,
    pub vocoder_inputs: Vec<String>,
    pub vocoder_outputs: Vec<String>,
}

#[cfg(not(target_arch = "wasm32"))]
pub struct DiffSingerRuntime {
    acoustic: Session,
    vocoder: Session,
    contract: DiffSingerModelContract,
    config_root: std::path::PathBuf,
    config_speakers: Vec<String>,
    config_hidden_size: Option<usize>,
}

#[cfg(not(target_arch = "wasm32"))]
impl DiffSingerRuntime {
    pub fn load(config: &DiffSingerConfig) -> Result<Self, String> {
        let acoustic = Session::builder()
            .map_err(|error| format!("falha ao criar sessão acústica: {error}"))?
            .commit_from_file(&config.acoustic)
            .map_err(|error| {
                format!(
                    "falha ao carregar acoustic.onnx ({}): {error}",
                    config.acoustic.display()
                )
            })?;
        let vocoder = Session::builder()
            .map_err(|error| format!("falha ao criar sessão do vocoder: {error}"))?
            .commit_from_file(&config.vocoder)
            .map_err(|error| {
                format!(
                    "falha ao carregar vocoder ONNX ({}): {error}",
                    config.vocoder.display()
                )
            })?;

        let contract = DiffSingerModelContract {
            acoustic_inputs: acoustic
                .inputs()
                .into_iter()
                .map(|input| input.name().to_string())
                .collect(),
            acoustic_outputs: acoustic
                .outputs()
                .into_iter()
                .map(|output| output.name().to_string())
                .collect(),
            vocoder_inputs: vocoder
                .inputs()
                .into_iter()
                .map(|input| input.name().to_string())
                .collect(),
            vocoder_outputs: vocoder
                .outputs()
                .into_iter()
                .map(|output| output.name().to_string())
                .collect(),
        };

        validate_contract(&contract)?;
        Ok(Self {
            acoustic,
            vocoder,
            contract,
            config_root: config.root.clone(),
            config_speakers: config.speakers.clone(),
            config_hidden_size: config.hidden_size,
        })
    }

    pub fn contract(&self) -> &DiffSingerModelContract {
        &self.contract
    }

    pub fn acoustic(&self) -> &Session {
        &self.acoustic
    }

    pub fn vocoder(&self) -> &Session {
        &self.vocoder
    }

    /// Runs the standard DiffSinger acoustic + vocoder pair.
    /// `tokens` and `durations` have one item per phoneme; `f0` is one value
    /// per acoustic frame. The common OpenUtau model contract uses [1, N]
    /// tensors for the first two and [1, frames] for F0.
    pub fn render_phrase(
        &mut self,
        tokens: &[i64],
        durations: &[i64],
        f0: &[f32],
        language_ids: Option<&[i64]>,
    ) -> Result<Vec<f32>, String> {
        if tokens.is_empty() || tokens.len() != durations.len() || f0.is_empty() {
            return Err("frase DiffSinger vazia ou com dimensões incompatíveis".into());
        }
        if let Some(language_ids) = language_ids {
            if language_ids.len() != tokens.len() {
                return Err("IDs de idioma DiffSinger incompatíveis com os tokens".into());
            }
        }
        let tokens = Array2::from_shape_vec((1, tokens.len()), tokens.to_vec())
            .map_err(|error| format!("tokens DiffSinger inválidos: {error}"))?;
        let durations = Array2::from_shape_vec((1, durations.len()), durations.to_vec())
            .map_err(|error| format!("durações DiffSinger inválidas: {error}"))?;
        let f0 = Array2::from_shape_vec((1, f0.len()), f0.to_vec())
            .map_err(|error| format!("F0 DiffSinger inválido: {error}"))?;
        let speedup = Array1::from_vec(vec![1i64]);
        let steps = Array1::from_vec(vec![1i64]);
        let depth = Array1::from_vec(vec![1.0f32]);
        let language_values = language_ids
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| vec![0; tokens.len()]);
        let languages = Array2::from_shape_vec((1, language_values.len()), language_values)
            .map_err(|error| format!("IDs de idioma DiffSinger inválidos: {error}"))?;
        let frame_default = Array2::from_elem((1, f0.len()), 0.0f32);
        let frame_one = Array2::from_elem((1, f0.len()), 1.0f32);
        let speaker_embed = if self
            .contract
            .acoustic_inputs
            .iter()
            .any(|name| name == "spk_embed")
        {
            let hidden_size = self.config_hidden_size.ok_or_else(|| {
                "acoustic.onnx exige spk_embed, mas hiddenSize não foi definido em dsconfig.yaml".to_string()
            })?;
            let speaker = self.config_speakers.first().ok_or_else(|| {
                "acoustic.onnx exige spk_embed, mas nenhum speaker foi definido".to_string()
            })?;
            let path = self.config_root.join(format!("{speaker}.emb"));
            let bytes = std::fs::read(&path)
                .map_err(|error| format!("não foi possível ler {}: {error}", path.display()))?;
            if bytes.len() != hidden_size * std::mem::size_of::<f32>() {
                return Err(format!(
                    "embedding {} possui {} bytes; esperado {}",
                    path.display(),
                    bytes.len(),
                    hidden_size * std::mem::size_of::<f32>()
                ));
            }
            let values = bytes
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect::<Vec<_>>();
            Some(Array3::from_shape_fn(
                (1, f0.len(), hidden_size),
                |(_, _, h)| values[h],
            ))
        } else {
            None
        };
        let mut acoustic_inputs: Vec<(&str, ort::session::SessionInputValue<'_>)> = vec![
            (
                "tokens",
                ort::value::TensorRef::from_array_view(&tokens)
                    .map_err(|e| e.to_string())?
                    .into(),
            ),
            (
                "durations",
                ort::value::TensorRef::from_array_view(&durations)
                    .map_err(|e| e.to_string())?
                    .into(),
            ),
            (
                "f0",
                ort::value::TensorRef::from_array_view(&f0)
                    .map_err(|e| e.to_string())?
                    .into(),
            ),
        ];
        if self
            .contract
            .acoustic_inputs
            .iter()
            .any(|name| name == "speedup")
        {
            acoustic_inputs.push((
                "speedup",
                ort::value::TensorRef::from_array_view(&speedup)
                    .map_err(|e| e.to_string())?
                    .into(),
            ));
        }
        if self
            .contract
            .acoustic_inputs
            .iter()
            .any(|name| name == "steps")
        {
            acoustic_inputs.push((
                "steps",
                ort::value::TensorRef::from_array_view(&steps)
                    .map_err(|e| e.to_string())?
                    .into(),
            ));
        }
        if self
            .contract
            .acoustic_inputs
            .iter()
            .any(|name| name == "depth")
        {
            acoustic_inputs.push((
                "depth",
                ort::value::TensorRef::from_array_view(&depth)
                    .map_err(|e| e.to_string())?
                    .into(),
            ));
        }
        if self
            .contract
            .acoustic_inputs
            .iter()
            .any(|name| name == "languages")
        {
            acoustic_inputs.push((
                "languages",
                ort::value::TensorRef::from_array_view(&languages)
                    .map_err(|e| e.to_string())?
                    .into(),
            ));
        }
        for name in ["gender", "energy", "breathiness", "voicing", "tension"] {
            if self
                .contract
                .acoustic_inputs
                .iter()
                .any(|input| input == name)
            {
                acoustic_inputs.push((
                    name,
                    ort::value::TensorRef::from_array_view(&frame_default)
                        .map_err(|e| e.to_string())?
                        .into(),
                ));
            }
        }
        if let Some(speaker_embed) = speaker_embed.as_ref() {
            acoustic_inputs.push((
                "spk_embed",
                ort::value::TensorRef::from_array_view(speaker_embed)
                    .map_err(|e| e.to_string())?
                    .into(),
            ));
        }
        if self
            .contract
            .acoustic_inputs
            .iter()
            .any(|name| name == "velocity")
        {
            acoustic_inputs.push((
                "velocity",
                ort::value::TensorRef::from_array_view(&frame_one)
                    .map_err(|e| e.to_string())?
                    .into(),
            ));
        }
        let outputs = self
            .acoustic
            .run(acoustic_inputs)
            .map_err(|error| format!("falha na inferência acústica DiffSinger: {error}"))?;
        let mel = outputs[0]
            .try_extract_array::<f32>()
            .map_err(|error| format!("saída mel inválida: {error}"))?;
        let mel_shape = mel.shape();
        if mel_shape.len() != 3 {
            return Err(format!(
                "saída mel esperada em 3 dimensões, recebida em {}",
                mel_shape.len()
            ));
        }
        if mel_shape[0] != 1 || mel_shape[1] != f0.len() {
            return Err(format!(
                "dimensões do mel incompatíveis: recebido [{}, {}, {}], esperado [1, {}, mel_bins]",
                mel_shape[0],
                mel_shape[1],
                mel_shape[2],
                f0.len()
            ));
        }
        let mel_vec: Vec<f32> = mel.iter().copied().collect();
        let mel = Array3::from_shape_vec((mel_shape[0], mel_shape[1], mel_shape[2]), mel_vec)
            .map_err(|error| format!("não foi possível preparar o mel: {error}"))?;
        let mut vocoder_inputs: Vec<(&str, ort::session::SessionInputValue<'_>)> = vec![(
            "mel",
            ort::value::TensorRef::from_array_view(&mel)
                .map_err(|e| e.to_string())?
                .into(),
        )];
        if self.contract.vocoder_inputs.iter().any(|name| name == "f0") {
            vocoder_inputs.push((
                "f0",
                ort::value::TensorRef::from_array_view(&f0)
                    .map_err(|e| e.to_string())?
                    .into(),
            ));
        }
        let vocoder_outputs = self
            .vocoder
            .run(vocoder_inputs)
            .map_err(|error| format!("falha na inferência do vocoder DiffSinger: {error}"))?;
        let waveform = vocoder_outputs[0]
            .try_extract_array::<f32>()
            .map_err(|error| format!("saída do vocoder inválida: {error}"))?;
        if waveform.ndim() != 2 || waveform.shape()[0] != 1 || waveform.shape()[1] == 0 {
            return Err(format!(
                "saída do vocoder incompatível: esperado [1, samples] com samples > 0, recebido {:?}",
                waveform.shape()
            ));
        }
        Ok(waveform.iter().copied().collect())
    }
}

#[cfg(target_arch = "wasm32")]
pub struct DiffSingerRuntime;

#[cfg(target_arch = "wasm32")]
impl DiffSingerRuntime {
    pub fn load(_config: &DiffSingerConfig) -> Result<Self, String> {
        Err("DiffSinger ONNX não está disponível no alvo WASM".to_string())
    }
}

fn validate_contract(contract: &DiffSingerModelContract) -> Result<(), String> {
    for required in ["tokens", "durations", "f0"] {
        if !contract.acoustic_inputs.iter().any(|name| name == required) {
            return Err(format!(
                "acoustic.onnx não possui a entrada DiffSinger obrigatória '{required}'"
            ));
        }
    }
    if contract.acoustic_outputs.is_empty() {
        return Err("acoustic.onnx não possui saída".to_string());
    }
    let supported_acoustic = [
        "tokens",
        "durations",
        "f0",
        "speedup",
        "steps",
        "depth",
        "languages",
        "gender",
        "energy",
        "breathiness",
        "voicing",
        "tension",
        "velocity",
        "spk_embed",
    ];
    if let Some(input) = contract
        .acoustic_inputs
        .iter()
        .find(|input| !supported_acoustic.contains(&input.as_str()))
    {
        return Err(format!(
            "acoustic.onnx exige a entrada '{input}', que ainda precisa de um recurso adicional do banco DiffSinger"
        ));
    }
    for required in ["mel"] {
        if !contract.vocoder_inputs.iter().any(|name| name == required) {
            return Err(format!(
                "vocoder não possui a entrada DiffSinger obrigatória '{required}'"
            ));
        }
    }
    if contract.vocoder_outputs.is_empty() {
        return Err("vocoder DiffSinger não possui saída".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_standard_contract() {
        let contract = DiffSingerModelContract {
            acoustic_inputs: vec!["tokens".into(), "durations".into(), "f0".into()],
            acoustic_outputs: vec!["mel".into()],
            vocoder_inputs: vec!["mel".into(), "f0".into()],
            vocoder_outputs: vec!["waveform".into()],
        };
        assert!(validate_contract(&contract).is_ok());
    }

    #[test]
    fn rejects_missing_f0() {
        let contract = DiffSingerModelContract {
            acoustic_inputs: vec!["tokens".into(), "durations".into()],
            acoustic_outputs: vec!["mel".into()],
            vocoder_inputs: vec!["mel".into(), "f0".into()],
            vocoder_outputs: vec!["waveform".into()],
        };
        assert!(validate_contract(&contract).is_err());
    }

    #[test]
    fn rejects_inputs_that_need_external_bank_data() {
        let contract = DiffSingerModelContract {
            acoustic_inputs: vec![
                "tokens".into(),
                "durations".into(),
                "f0".into(),
                "mystery".into(),
            ],
            acoustic_outputs: vec!["mel".into()],
            vocoder_inputs: vec!["mel".into()],
            vocoder_outputs: vec!["waveform".into()],
        };
        let error = validate_contract(&contract).unwrap_err();
        assert!(error.contains("mystery"));
    }

    #[allow(dead_code)]
    fn _path_is_used(path: &Path) -> bool {
        path.exists()
    }
}
