use std::collections::HashMap;
use std::fs;
use std::path::Path;
use encoding_rs::SHIFT_JIS;

#[derive(Debug, Clone, Default)]
pub struct PrefixMap {
    /// Maps pitch name (e.g. "C4", "F#4") to (prefix, suffix) tuple
    map: HashMap<String, (String, String)>,
}

impl PrefixMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, pitch_name: String, prefix: String, suffix: String) {
        self.map.insert(pitch_name, (prefix, suffix));
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
        let mut pmap = PrefixMap::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let pitch_name = parts[0].to_string();
                let prefix = parts[1].to_string();
                let suffix = if parts.len() >= 3 { parts[2].to_string() } else { String::new() };
                pmap.insert(pitch_name, prefix, suffix);
            }
        }
        pmap
    }

    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let bytes = fs::read(path)?;
        let (text, _, _) = SHIFT_JIS.decode(&bytes);
        Ok(Self::parse_str(&text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_map_alias() {
        let sample = "C4  C4_ \n F4  \"\"  _F4\n";
        let map = PrefixMap::parse_str(sample);
        assert_eq!(map.get_alias("ka", "C4"), "C4_ka");
        assert_eq!(map.get_alias("ka", "A4"), "ka");
    }
}
