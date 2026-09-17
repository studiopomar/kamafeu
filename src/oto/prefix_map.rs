use encoding_rs::SHIFT_JIS;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct PrefixMap {
    /// Maps pitch name (e.g. "C4", "F#4") to (prefix, suffix) tuple
    map: HashMap<String, (String, String)>,
    colors: std::collections::BTreeMap<String, HashMap<String, (String, String)>>,
    selected_color: String,
}

impl PrefixMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            ..Default::default()
        }
    }

    pub fn colors(&self) -> impl Iterator<Item = &str> {
        self.colors.keys().map(String::as_str)
    }

    pub fn selected_color(&self) -> &str {
        &self.selected_color
    }

    pub fn select_color(&mut self, color: &str) -> bool {
        if let Some(map) = self.colors.get(color) {
            self.map = map.clone();
            self.selected_color = color.to_string();
            true
        } else {
            false
        }
    }

    pub fn with_voicecolors(mut self, yaml: &str) -> Self {
        let parsed = Self::parse_yaml_str(yaml);
        self.colors = parsed.colors;
        self.colors.insert(String::new(), self.map.clone());
        self
    }

    pub fn insert(&mut self, pitch_name: String, prefix: String, suffix: String) {
        self.map.insert(pitch_name, (prefix, suffix));
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn get_prefix_suffix(&self, pitch_name: &str) -> Option<(&str, &str)> {
        self.map
            .get(pitch_name)
            .map(|(p, s)| (p.as_str(), s.as_str()))
    }

    pub fn mapped_pitches(&self) -> impl Iterator<Item = &str> {
        self.map.keys().map(|k| k.as_str())
    }

    /// Retrieve the prefixed/suffixed alias for a given lyric and pitch name.
    /// E.g. lyric "ka" and pitch "C4" -> "ka_C4" or "C4_ka" depending on prefix/suffix map.
    pub fn get_alias(&self, lyric: &str, pitch_name: &str) -> String {
        if let Some((prefix, suffix)) = self.map.get(pitch_name) {
            format!("{}{}{}", prefix, lyric, suffix)
        } else {
            lyric.to_string()
        }
    }

    pub fn parse_str(content: &str) -> Self {
        let content = content.strip_prefix('\u{feff}').unwrap_or(content);
        let mut pmap = PrefixMap::new();

        fn clean_token(s: &str) -> String {
            let trimmed = s.trim();
            if trimmed == "\"\"" || trimmed == "''" {
                return String::new();
            }
            if (trimmed.starts_with('"') && trimmed.ends_with('"'))
                || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
            {
                trimmed[1..trimmed.len() - 1].to_string()
            } else {
                s.to_string()
            }
        }

        for line in content.lines() {
            let line = line.trim_end(); // Don't trim_start because prefix map lines shouldn't start with whitespace anyway, but empty tabs matter!
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            // Some old prefix.maps might use spaces, but standard UTAU uses tabs.
            // Let's first try tab separation. If there's no tab, fallback to space splitting.
            if line.contains('\t') {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    let pitch_name = parts[0].trim().to_string();
                    let prefix = clean_token(parts[1]);
                    let suffix = if parts.len() >= 3 {
                        clean_token(parts[2])
                    } else {
                        String::new()
                    };
                    if !pitch_name.is_empty() {
                        pmap.insert(pitch_name, prefix, suffix);
                    }
                }
            } else {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let pitch_name = parts[0].trim().to_string();
                    let prefix = clean_token(parts[1]);
                    let suffix = if parts.len() >= 3 {
                        clean_token(parts[2])
                    } else {
                        String::new()
                    };
                    if !pitch_name.is_empty() {
                        pmap.insert(pitch_name, prefix, suffix);
                    }
                }
            }
        }
        pmap
    }

    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let bytes = fs::read(path)?;
        let (text, _, _) = SHIFT_JIS.decode(&bytes);
        Ok(Self::parse_str(&text))
    }

    pub fn parse_yaml_str(content: &str) -> Self {
        let content = content.strip_prefix('\u{feff}').unwrap_or(content);
        let mut pmap = PrefixMap::new();
        if let Ok(val) = yaml_serde::from_str::<yaml_serde::Value>(content) {
            if let Some(subbanks) = val.get("subbanks").and_then(|v| v.as_sequence()) {
                for sub in subbanks {
                    let color = sub
                        .get("color")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let mut color_map = PrefixMap::new();
                    let prefix = sub
                        .get("prefix")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let suffix = sub
                        .get("suffix")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if sub
                        .get("tone_ranges")
                        .and_then(|v| v.as_sequence())
                        .is_none_or(|ranges| ranges.is_empty())
                    {
                        for midi in 0..=127 {
                            color_map.insert(
                                crate::dsp::pitch::midi_to_note_name(midi),
                                prefix.clone(),
                                suffix.clone(),
                            );
                        }
                    }
                    if let Some(ranges) = sub.get("tone_ranges").and_then(|v| v.as_sequence()) {
                        for range in ranges {
                            if let Some(range_str) = range.as_str() {
                                let parts: Vec<&str> = range_str.split('-').collect();
                                if parts.len() == 2 {
                                    if let (Some(min_midi), Some(max_midi)) = (
                                        crate::dsp::pitch::note_name_to_midi(parts[0]),
                                        crate::dsp::pitch::note_name_to_midi(parts[1]),
                                    ) {
                                        for m in min_midi..=max_midi {
                                            color_map.insert(
                                                crate::dsp::pitch::midi_to_note_name(m),
                                                prefix.clone(),
                                                suffix.clone(),
                                            );
                                        }
                                    }
                                } else if parts.len() == 1 {
                                    color_map.insert(
                                        parts[0].trim().to_string(),
                                        prefix.clone(),
                                        suffix.clone(),
                                    );
                                }
                            }
                        }
                    }
                    pmap.colors.entry(color).or_default().extend(color_map.map);
                }
            }
        }
        let initial = if pmap.colors.contains_key("") {
            String::new()
        } else {
            pmap.colors.keys().next().cloned().unwrap_or_default()
        };
        pmap.select_color(&initial);
        pmap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voicecolors_keep_separate_pitch_maps() {
        let mut map = PrefixMap::parse_yaml_str("subbanks:\n  - color: ''\n    suffix: _normal\n    tone_ranges: [C4-D4]\n  - color: Soft\n    suffix: _soft\n    tone_ranges: [C4-D4]\n");
        assert_eq!(map.get_alias("a", "C4"), "a_normal");
        assert!(map.select_color("Soft"));
        assert_eq!(map.get_alias("a", "C4"), "a_soft");
        assert!(!map.select_color("Missing"));
        assert_eq!(map.get_alias("a", "C4"), "a_soft");
        assert!(map.select_color(""));
        assert_eq!(map.get_alias("a", "D4"), "a_normal");
    }

    #[test]
    fn test_prefix_map_alias() {
        let sample = "C4  C4_ \n F4  \"\"  _F4\n";
        let map = PrefixMap::parse_str(sample);
        assert_eq!(map.get_alias("ka", "C4"), "C4_ka");
        assert_eq!(map.get_alias("ka", "A4"), "ka");
    }
}
