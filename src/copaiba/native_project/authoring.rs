use super::CopaibaNativeProject;
use crate::voicebank::native::{
    Asset, Context, Quality, RenderTuning, SourceRegion, Unit, UnitTiming,
};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub(super) enum UnitAuthoringEdit {
    SourceRegion {
        layer_id: String,
        unit_id: String,
        before: SourceRegion,
        after: SourceRegion,
    },
    Reviewed {
        layer_id: String,
        unit_id: String,
        before: bool,
        after: bool,
    },
}

impl CopaibaNativeProject {
    pub fn set_source_region(
        &mut self,
        layer_id: &str,
        unit_id: &str,
        source: SourceRegion,
    ) -> Result<(), String> {
        let layer_index = self
            .layers
            .iter()
            .position(|(layer, _)| layer.id == layer_id)
            .ok_or_else(|| format!("unknown layer: {layer_id}"))?;
        let unit_index = self.layers[layer_index]
            .1
            .units
            .iter()
            .position(|unit| unit.id == unit_id)
            .ok_or_else(|| format!("unknown unit: {unit_id}"))?;
        let profile = self
            .profiles
            .iter()
            .find(|profile| profile.id == self.manifest.defaults.phonetic_profile_id)
            .ok_or_else(|| "default phonetic profile is missing".to_string())?;
        let before = self.layers[layer_index].1.units[unit_index].source.clone();
        if before == source {
            return Ok(());
        }
        self.layers[layer_index].1.units[unit_index].source = source.clone();
        if let Err(error) = self.layers[layer_index].1.validate(layer_id, profile) {
            self.layers[layer_index].1.units[unit_index].source = before;
            return Err(error);
        }
        // The pre-edit value is read from the previous command when edits are
        // coalesced, otherwise use the value captured before replacement.
        let coalesced_to_original = if let Some(UnitAuthoringEdit::SourceRegion {
            before: previous_before,
            after,
            ..
        }) = self.undo.last_mut().filter(
            |edit| matches!(edit, UnitAuthoringEdit::SourceRegion { layer_id: last_layer, unit_id: last_unit, .. } if last_layer == layer_id && last_unit == unit_id),
        ) {
            *after = source.clone();
            Some(*previous_before == source)
        } else {
            None
        };
        if coalesced_to_original == Some(true) {
            self.undo.pop();
        } else if coalesced_to_original.is_none() {
            self.push_edit(UnitAuthoringEdit::SourceRegion {
                layer_id: layer_id.to_string(),
                unit_id: unit_id.to_string(),
                before,
                after: source,
            });
        }
        self.redo.clear();
        Ok(())
    }

    /// Mark a unit as explicitly reviewed by the voicebank author. The state
    /// is persisted in the units JSON and participates in selector tie breaks.
    pub fn set_unit_reviewed(
        &mut self,
        layer_id: &str,
        unit_id: &str,
        reviewed: bool,
    ) -> Result<(), String> {
        let unit = self
            .unit(layer_id, unit_id)
            .ok_or_else(|| format!("unknown unit: {unit_id}"))?;
        let before = unit.quality.reviewed;
        if before == reviewed {
            return Ok(());
        }
        let coalesced_to_original = if let Some(UnitAuthoringEdit::Reviewed {
            before: previous_before,
            after,
            ..
        }) = self.undo.last_mut().filter(
            |edit| matches!(edit, UnitAuthoringEdit::Reviewed { layer_id: last_layer, unit_id: last_unit, .. } if last_layer == layer_id && last_unit == unit_id),
        ) {
            *after = reviewed;
            Some(*previous_before == reviewed)
        } else {
            None
        };
        if coalesced_to_original == Some(true) {
            self.undo.pop();
        } else if coalesced_to_original.is_none() {
            self.push_edit(UnitAuthoringEdit::Reviewed {
                layer_id: layer_id.to_string(),
                unit_id: unit_id.to_string(),
                before,
                after: reviewed,
            });
        }
        let unit = self
            .layers
            .iter_mut()
            .find(|(layer, _)| layer.id == layer_id)
            .and_then(|(_, file)| file.units.iter_mut().find(|unit| unit.id == unit_id))
            .ok_or_else(|| format!("unknown unit: {unit_id}"))?;
        unit.quality.reviewed = reviewed;
        self.redo.clear();
        Ok(())
    }

    fn push_edit(&mut self, edit: UnitAuthoringEdit) {
        self.undo.push(edit);
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
    }

    pub fn add_unit_from_asset(
        &mut self,
        layer_id: &str,
        asset_id: &str,
        unit_id: &str,
        kind: &str,
        phones: Vec<String>,
    ) -> Result<(), String> {
        if unit_id.is_empty()
            || !unit_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err("unit id must contain only letters, digits, '-' or '_'".into());
        }
        let (_, file) = self
            .layers
            .iter_mut()
            .find(|(layer, _)| layer.id == layer_id)
            .ok_or_else(|| format!("unknown layer: {layer_id}"))?;
        if file.units.iter().any(|unit| unit.id == unit_id) {
            return Err(format!("unit id already exists: {unit_id}"));
        }
        let asset = file
            .assets
            .iter()
            .find(|asset| asset.id == asset_id)
            .ok_or_else(|| format!("unknown asset: {asset_id}"))?;
        if phones.is_empty() {
            return Err("unit must have at least one phone".into());
        }
        file.units.push(Unit {
            id: unit_id.to_string(),
            asset_id: asset_id.to_string(),
            kind: kind.to_string(),
            phones,
            left_context: Context {
                kind: "any".into(),
                phone: None,
            },
            right_context: Context {
                kind: "any".into(),
                phone: None,
            },
            legacy_aliases: Vec::new(),
            source: SourceRegion {
                start_frame: 0,
                end_frame: asset.frame_count,
                anchor_frame: 0,
                fixed_end_frame: 0,
                sustain: None,
                release_start_frame: None,
            },
            timing: UnitTiming {
                min_target_ms: 60.0,
                preferred_crossfade_ms: 20.0,
                max_crossfade_ms: 40.0,
            },
            render: RenderTuning::default(),
            quality: Quality::default(),
        });
        let profile = self
            .profiles
            .iter()
            .find(|profile| profile.id == self.manifest.defaults.phonetic_profile_id)
            .ok_or_else(|| "default phonetic profile is missing".to_string())?;
        if let Err(error) = file.validate(layer_id, profile) {
            file.units.pop();
            return Err(error);
        }
        Ok(())
    }

    pub fn undo(&mut self) -> Result<bool, String> {
        let Some(edit) = self.undo.last().cloned() else {
            return Ok(false);
        };
        match &edit {
            UnitAuthoringEdit::SourceRegion {
                layer_id,
                unit_id,
                before,
                ..
            } => self.set_source_untracked(layer_id, unit_id, before.clone())?,
            UnitAuthoringEdit::Reviewed {
                layer_id,
                unit_id,
                before,
                ..
            } => self.set_reviewed_untracked(layer_id, unit_id, *before)?,
        }
        self.undo.pop();
        self.redo.push(edit);
        Ok(true)
    }

    pub fn redo(&mut self) -> Result<bool, String> {
        let Some(edit) = self.redo.last().cloned() else {
            return Ok(false);
        };
        match &edit {
            UnitAuthoringEdit::SourceRegion {
                layer_id,
                unit_id,
                after,
                ..
            } => self.set_source_untracked(layer_id, unit_id, after.clone())?,
            UnitAuthoringEdit::Reviewed {
                layer_id,
                unit_id,
                after,
                ..
            } => self.set_reviewed_untracked(layer_id, unit_id, *after)?,
        }
        self.redo.pop();
        self.undo.push(edit);
        Ok(true)
    }

    fn set_source_untracked(
        &mut self,
        layer_id: &str,
        unit_id: &str,
        source: SourceRegion,
    ) -> Result<(), String> {
        let (layer, file) = self
            .layers
            .iter_mut()
            .find(|(layer, _)| layer.id == layer_id)
            .ok_or_else(|| format!("unknown layer: {layer_id}"))?;
        let profile = self
            .profiles
            .iter()
            .find(|profile| profile.id == self.manifest.defaults.phonetic_profile_id)
            .ok_or_else(|| "default phonetic profile is missing".to_string())?;
        let unit_index = file
            .units
            .iter_mut()
            .position(|unit| unit.id == unit_id)
            .ok_or_else(|| format!("unknown unit: {unit_id}"))?;
        let previous = file.units[unit_index].source.clone();
        file.units[unit_index].source = source;
        if let Err(error) = file.validate(&layer.id, profile) {
            file.units[unit_index].source = previous;
            return Err(error);
        }
        Ok(())
    }

    fn set_reviewed_untracked(
        &mut self,
        layer_id: &str,
        unit_id: &str,
        reviewed: bool,
    ) -> Result<(), String> {
        let unit = self
            .layers
            .iter_mut()
            .find(|(layer, _)| layer.id == layer_id)
            .and_then(|(_, file)| file.units.iter_mut().find(|unit| unit.id == unit_id))
            .ok_or_else(|| format!("unknown unit: {unit_id}"))?;
        unit.quality.reviewed = reviewed;
        Ok(())
    }

    /// Copy an imported recording into the authoring tree without touching
    /// the original file. Only mono WAV sources are accepted by KFV 2 v1.
    pub fn import_recording(
        &mut self,
        layer_id: &str,
        source: &Path,
        asset_id: &str,
    ) -> Result<String, String> {
        if asset_id.is_empty()
            || !asset_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
            || asset_id.contains('/')
        {
            return Err("asset id must be a simple path-safe name".into());
        }
        let layer_index = self
            .layers
            .iter()
            .position(|(layer, _)| layer.id == layer_id)
            .ok_or_else(|| format!("unknown layer: {layer_id}"))?;
        let reader =
            hound::WavReader::open(source).map_err(|e| format!("invalid source WAV: {e}"))?;
        let spec = reader.spec();
        let frame_count = u64::from(reader.duration());
        if spec.channels != 1 || spec.sample_rate == 0 || frame_count == 0 {
            return Err("KFV 2 imports require a non-empty mono WAV".into());
        }
        if self.layers[layer_index]
            .1
            .assets
            .iter()
            .any(|asset| asset.id == asset_id)
        {
            return Err(format!("asset id already exists: {asset_id}"));
        }
        let relative_path = format!("audio/imports/{asset_id}.wav");
        let destination = self.root.join(&relative_path);
        if fs::symlink_metadata(&destination).is_ok() {
            return Err(format!(
                "destination already exists: {}",
                destination.display()
            ));
        }
        fs::create_dir_all(destination.parent().ok_or("asset path has no parent")?)
            .map_err(|e| format!("cannot create audio import folder: {e}"))?;
        let canonical_root = fs::canonicalize(&self.root)
            .map_err(|e| format!("cannot resolve voicebank root: {e}"))?;
        let canonical_parent = fs::canonicalize(destination.parent().unwrap())
            .map_err(|e| format!("cannot resolve audio import folder: {e}"))?;
        if !canonical_parent.starts_with(canonical_root) {
            return Err("audio import folder resolves outside the voicebank".into());
        }
        let source_path =
            fs::canonicalize(source).map_err(|e| format!("cannot resolve source WAV: {e}"))?;
        let temporary = tempfile::NamedTempFile::new_in(destination.parent().unwrap())
            .map_err(|e| format!("cannot create import staging file: {e}"))?;
        fs::copy(&source_path, temporary.path())
            .map_err(|e| format!("cannot copy source WAV: {e}"))?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|e| format!("cannot sync imported WAV: {e}"))?;
        temporary
            .persist(&destination)
            .map_err(|e| format!("cannot install imported WAV: {}", e.error))?;
        self.layers[layer_index].1.assets.push(Asset {
            id: asset_id.to_string(),
            path: relative_path.clone(),
            sample_rate_hz: spec.sample_rate,
            channels: spec.channels,
            frame_count,
        });
        Ok(relative_path)
    }
}
