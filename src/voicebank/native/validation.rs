use super::{Manifest, PhoneticProfile, UnitFile};
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 200
}
pub fn safe_relative_path(value: &str) -> bool {
    if value.is_empty() || value.contains('\\') || value.contains('\0') {
        return false;
    }
    let path = Path::new(value);
    !path.is_absolute() && path.components().all(|c| matches!(c, Component::Normal(_)))
}
fn version_major(value: &str) -> Option<u64> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let major = parts[0].parse().ok()?;
    parts[1].parse::<u64>().ok()?;
    parts[2].parse::<u64>().ok()?;
    Some(major)
}
fn valid_language_tag(value: &str) -> bool {
    let mut parts = value.split('-');
    let Some(primary) = parts.next() else {
        return false;
    };
    (2..=8).contains(&primary.len())
        && primary.bytes().all(|b| b.is_ascii_alphabetic())
        && parts.all(|part| {
            (1..=8).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_alphanumeric())
        })
}

impl Manifest {
    pub fn parse_json(content: &str) -> Result<Self, String> {
        let value: serde_json::Value =
            serde_json::from_str(content).map_err(|e| format!("manifest JSON: {e}"))?;
        if value.get("format").and_then(|v| v.as_str()) != Some("kamafeu.voicebank") {
            return Err("manifest format must be kamafeu.voicebank".into());
        }
        let manifest: Self =
            serde_json::from_value(value).map_err(|e| format!("manifest fields: {e}"))?;
        manifest.validate()?;
        Ok(manifest)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.format != "kamafeu.voicebank" || version_major(&self.schema_version) != Some(2) {
            return Err("KFV 2 schema_version 2.x is required".into());
        }
        if [
            self.voice_id.as_str(),
            self.bank_id.as_str(),
            self.bank_version.as_str(),
            self.display_name.as_str(),
            self.publisher.as_str(),
        ]
        .iter()
        .any(|v| !valid_id(v))
        {
            return Err("manifest contains an empty or oversized identity field".into());
        }
        if version_major(&self.bank_version).is_none() {
            return Err("bank_version must use numeric SemVer (major.minor.patch)".into());
        }
        if self.package_revision == 0
            || self.engine.id != "kamafeu.concat"
            || self.engine.api_major != 1
        {
            return Err("unsupported package revision or native engine API".into());
        }
        const FEATURES: &[&str] = &["explicit-anchor", "sustain-region", "finite-units"];
        if self
            .engine
            .required_features
            .iter()
            .any(|feature| !FEATURES.contains(&feature.as_str()))
        {
            return Err("manifest requires an unsupported native engine feature".into());
        }
        if self.languages.is_empty()
            || self.phonetic_profiles.is_empty()
            || self.styles.is_empty()
            || self.layers.is_empty()
        {
            return Err("manifest must declare language, profile, style, and layer".into());
        }
        if self
            .languages
            .iter()
            .any(|language| !valid_language_tag(language))
            || self
                .phonetic_profiles
                .iter()
                .any(|profile| !valid_language_tag(&profile.language))
        {
            return Err("invalid BCP 47 language tag".into());
        }
        let mut ids = HashSet::new();
        for (id, path) in self.phonetic_profiles.iter().map(|x| (&x.id, &x.path)) {
            if !valid_id(id) || !ids.insert(id) || !safe_relative_path(path) {
                return Err(format!(
                    "invalid or duplicate phonetic profile reference: {id}"
                ));
            }
        }
        let language_tags: HashSet<_> = self.languages.iter().map(String::as_str).collect();
        if self
            .phonetic_profiles
            .iter()
            .any(|p| !language_tags.contains(p.language.as_str()))
        {
            return Err("phonetic profile language is not declared by the voicebank".into());
        }
        let profile_ids: HashSet<_> = self
            .phonetic_profiles
            .iter()
            .map(|x| x.id.as_str())
            .collect();
        let mut style_ids = HashSet::new();
        for style in &self.styles {
            if !valid_id(&style.id) || !style_ids.insert(style.id.as_str()) {
                return Err(format!("invalid or duplicate style id: {}", style.id));
            }
        }
        let mut layer_ids = HashSet::new();
        for layer in &self.layers {
            if !valid_id(&layer.id)
                || !layer_ids.insert(layer.id.as_str())
                || !style_ids.contains(layer.style_id.as_str())
                || layer.recorded_midi > 127
                || !safe_relative_path(&layer.units_path)
            {
                return Err(format!("invalid layer: {}", layer.id));
            }
        }
        if !profile_ids.contains(self.defaults.phonetic_profile_id.as_str())
            || !style_ids.contains(self.defaults.style_id.as_str())
        {
            return Err("manifest defaults reference an unknown profile or style".into());
        }
        if self.defaults.layer_policy != "nearest-recorded"
            || self.defaults.missing_unit_policy != "error"
        {
            return Err("unsupported layer or missing-unit policy".into());
        }
        for path in self
            .presentation
            .iter()
            .flat_map(|p| {
                [p.avatar.as_ref(), p.portrait.as_ref()]
                    .into_iter()
                    .flatten()
            })
            .chain(self.license.iter().map(|x| &x.path))
        {
            if !safe_relative_path(path) {
                return Err(format!("unsafe manifest path: {path}"));
            }
        }
        Ok(())
    }
    pub fn resolve(&self, root: &Path, relative: &str) -> Result<PathBuf, String> {
        if !safe_relative_path(relative) {
            return Err(format!("unsafe relative path: {relative}"));
        }
        Ok(root.join(relative))
    }

    /// Resolve an existing referenced file and reject symlinks that leave the
    /// bank root. Lexical path validation alone does not stop `link/file`.
    pub fn resolve_existing(&self, root: &Path, relative: &str) -> Result<PathBuf, String> {
        let root = std::fs::canonicalize(root)
            .map_err(|error| format!("cannot resolve voicebank root: {error}"))?;
        let path = self.resolve(&root, relative)?;
        let canonical = std::fs::canonicalize(&path)
            .map_err(|error| format!("referenced file is missing '{}': {error}", relative))?;
        if !canonical.starts_with(&root) || !canonical.is_file() {
            return Err(format!(
                "referenced file escapes the voicebank root: {relative}"
            ));
        }
        Ok(canonical)
    }
}

impl PhoneticProfile {
    pub fn validate(&self) -> Result<(), String> {
        if self.format != "kamafeu.phonetics"
            || version_major(&self.schema_version) != Some(1)
            || !valid_id(&self.id)
            || !valid_language_tag(&self.language)
        {
            return Err("invalid phonetic profile header".into());
        }
        let mut ids = HashSet::new();
        for symbol in &self.symbols {
            if !valid_id(&symbol.id) || !ids.insert(symbol.id.as_str()) {
                return Err(format!("invalid or duplicate phone symbol: {}", symbol.id));
            }
        }
        if !["ask-for-pronunciation", "error"].contains(&self.unknown_word_policy.as_str()) {
            return Err("unsupported unknown_word_policy".into());
        }
        for (word, pronunciations) in &self.lexicon {
            if word.trim().is_empty() || pronunciations.is_empty() {
                return Err(format!("empty lexicon entry: {word}"));
            }
            for p in pronunciations {
                if p.phones.is_empty()
                    || p.stress_nuclei.iter().any(|i| *i >= p.phones.len())
                    || p.phones.iter().any(|phone| !ids.contains(phone.as_str()))
                {
                    return Err(format!("invalid pronunciation for {word}"));
                }
            }
        }
        Ok(())
    }
}

impl UnitFile {
    pub fn validate(&self, expected_layer: &str, profile: &PhoneticProfile) -> Result<(), String> {
        if self.format != "kamafeu.units"
            || version_major(&self.schema_version) != Some(1)
            || self.layer_id != expected_layer
        {
            return Err("unit file format, version, or layer id mismatch".into());
        }
        let phones: HashSet<_> = profile.symbols.iter().map(|p| p.id.as_str()).collect();
        let mut assets = HashSet::new();
        for a in &self.assets {
            if !valid_id(&a.id)
                || !assets.insert(a.id.as_str())
                || !safe_relative_path(&a.path)
                || a.sample_rate_hz == 0
                || a.channels != 1
                || a.frame_count == 0
            {
                return Err(format!("invalid audio asset: {}", a.id));
            }
        }
        let mut units = HashSet::new();
        for u in &self.units {
            if !valid_id(&u.id)
                || !units.insert(u.id.as_str())
                || !assets.contains(u.asset_id.as_str())
                || u.phones.is_empty()
                || u.phones.iter().any(|p| !phones.contains(p.as_str()))
            {
                return Err(format!("invalid unit reference or phones: {}", u.id));
            }
            let asset = self
                .assets
                .iter()
                .find(|a| a.id == u.asset_id)
                .ok_or_else(|| format!("unit '{}' refers to missing asset", u.id))?;
            if !["CV", "VC", "V", "VV", "CC", "breath", "release"].contains(&u.kind.as_str()) {
                return Err(format!("unsupported unit kind in {}", u.id));
            }
            for context in [&u.left_context, &u.right_context] {
                match context.kind.as_str() {
                    "boundary" | "any" if context.phone.is_none() => {}
                    "phone"
                        if context
                            .phone
                            .as_ref()
                            .is_some_and(|p| phones.contains(p.as_str())) => {}
                    _ => return Err(format!("invalid phonetic context in {}", u.id)),
                }
            }
            let s = &u.source;
            if !(s.start_frame < s.end_frame
                && s.end_frame <= asset.frame_count
                && s.start_frame <= s.anchor_frame
                && s.anchor_frame < s.end_frame
                && s.anchor_frame <= s.fixed_end_frame
                && s.fixed_end_frame <= s.end_frame)
            {
                return Err(format!("invalid frame bounds in unit {}", u.id));
            }
            if let Some(r) = &s.sustain {
                if !(s.fixed_end_frame <= r.start_frame
                    && r.start_frame < r.end_frame
                    && r.end_frame <= s.end_frame)
                {
                    return Err(format!("invalid sustain region in unit {}", u.id));
                }
            }
            if !["finite", "loop", "stretch"].contains(&u.render.sustain_mode.as_str())
                || (u.render.sustain_mode != "finite" && s.sustain.is_none())
            {
                return Err(format!("invalid sustain mode in {}", u.id));
            }
            if let Some(r) = s.release_start_frame {
                if r < s.start_frame
                    || r >= s.end_frame
                    || s.sustain
                        .as_ref()
                        .is_some_and(|sustain| r < sustain.end_frame)
                {
                    return Err(format!("invalid release region in unit {}", u.id));
                }
            }
            if !u.timing.min_target_ms.is_finite()
                || u.timing.min_target_ms <= 0.0
                || !u.timing.preferred_crossfade_ms.is_finite()
                || !u.timing.max_crossfade_ms.is_finite()
                || u.timing.preferred_crossfade_ms < 0.0
                || u.timing.max_crossfade_ms < u.timing.preferred_crossfade_ms
                || !u.render.gain_db.is_finite()
                || !u.render.formant_shift_cents.is_finite()
            {
                return Err(format!("invalid timing or DSP values in unit {}", u.id));
            }
        }
        Ok(())
    }
}
