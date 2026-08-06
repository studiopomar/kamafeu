use super::entry::OtoEntry;
use encoding_rs::SHIFT_JIS;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct OtoParser;

impl OtoParser {
    /// Parse `oto.ini` content from a byte slice, handling Shift-JIS and UTF-8 automatically.
    pub fn parse_bytes(bytes: &[u8]) -> HashMap<String, OtoEntry> {
        let (text, _, _) = SHIFT_JIS.decode(bytes);
        Self::parse_str(&text)
    }

    /// Parse `oto.ini` content from a string slice.
    pub fn parse_str(content: &str) -> HashMap<String, OtoEntry> {
        let mut map = HashMap::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() < 2 {
                continue;
            }

            let wav_filename = parts[0].trim().to_string();
            let values_str = parts[1].trim();

            let val_parts: Vec<&str> = values_str.split(',').map(|s| s.trim()).collect();
            if val_parts.is_empty() {
                continue;
            }

            let raw_alias = val_parts[0];
            let alias = if raw_alias.is_empty() {
                Path::new(&wav_filename)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&wav_filename)
                    .to_string()
            } else {
                raw_alias.to_string()
            };

            let parse_num = |idx: usize, default: f64| -> f64 {
                if idx < val_parts.len() {
                    val_parts[idx].parse::<f64>().unwrap_or(default)
                } else {
                    default
                }
            };

            let offset = parse_num(1, 0.0);
            let consonant = parse_num(2, 0.0);
            let cutoff = parse_num(3, 0.0);
            let preutterance = parse_num(4, 0.0);
            let overlap = parse_num(5, 0.0);

            let entry = OtoEntry::new(
                wav_filename,
                alias.clone(),
                offset,
                consonant,
                cutoff,
                preutterance,
                overlap,
            );

            map.insert(alias, entry);
        }

        map
    }

    /// Parse `oto.ini` from a file path.
    pub fn parse_file<P: AsRef<Path>>(
        path: P,
    ) -> Result<HashMap<String, OtoEntry>, std::io::Error> {
        let bytes = fs::read(path)?;
        Ok(Self::parse_bytes(&bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_oto() {
        let sample = r#"
ka.wav=ka,10,50,-100,20,10
ki.wav=,15,60,-110,25,12
"#;
        let map = OtoParser::parse_str(sample);
        assert_eq!(map.len(), 2);

        let ka = map.get("ka").unwrap();
        assert_eq!(ka.wav_filename, "ka.wav");
        assert_eq!(ka.offset, 10.0);
        assert_eq!(ka.consonant, 50.0);
        assert_eq!(ka.cutoff, -100.0);
        assert_eq!(ka.preutterance, 20.0);
        assert_eq!(ka.overlap, 10.0);

        let ki = map.get("ki").unwrap();
        assert_eq!(ki.alias, "ki");
        assert_eq!(ki.wav_filename, "ki.wav");
    }
}
