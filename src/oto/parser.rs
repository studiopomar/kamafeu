use super::entry::OtoEntry;
use encoding_rs::SHIFT_JIS;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

pub struct OtoParser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtoParseDiagnostic {
    pub line: usize,
    pub field: Option<String>,
    pub reason: String,
}

impl OtoParseDiagnostic {
    pub fn summary(&self) -> String {
        self.reason.clone()
    }

    pub fn display(&self) -> String {
        format!("linha {}: {}", self.line, self.reason)
    }
}

impl OtoParser {
    /// Parse an oto.ini while retaining valid lines when another line is
    /// malformed. Real-world UTAU banks often contain a few legacy or empty
    /// records; one bad record must not hide the whole singer.
    pub fn parse_bytes_lossy(bytes: &[u8]) -> (HashMap<String, OtoEntry>, Vec<String>) {
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text.to_owned(),
            Err(_) => SHIFT_JIS.decode(bytes).0.into_owned(),
        };
        Self::parse_str_lossy(text.trim_start_matches('\u{feff}'))
    }

    /// Parse an oto.ini using the same fail-fast rule as OpenUtau: present
    /// empty numeric fields mean zero; malformed or missing fields are errors.
    pub fn parse_bytes(bytes: &[u8]) -> Result<HashMap<String, OtoEntry>, io::Error> {
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text.to_owned(),
            Err(_) => SHIFT_JIS.decode(bytes).0.into_owned(),
        };
        Self::parse_str(text.trim_start_matches('\u{feff}'))
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    /// Parse an oto.ini from text, reporting the exact line and field that
    /// failed instead of silently substituting a default value.
    pub fn parse_str(content: &str) -> Result<HashMap<String, OtoEntry>, String> {
        let mut map = HashMap::new();
        for (line_number, raw_line) in content.lines().enumerate() {
            let line_number = line_number + 1;
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            let (wav_filename, values) = line.split_once('=').ok_or_else(|| {
                format!("linha {line_number}: esperado '<wav>=<alias>,<offset>,...'")
            })?;
            let wav_filename = wav_filename.trim();
            if wav_filename.is_empty() {
                return Err(format!("linha {line_number}: nome do WAV vazio"));
            }
            let fields: Vec<&str> = values.split(',').map(str::trim).collect();
            if fields.len() < 6 {
                return Err(format!(
                    "linha {line_number}: esperado alias + 5 campos numéricos"
                ));
            }
            let raw_alias = fields[0];
            let alias = if raw_alias.is_empty() {
                Path::new(wav_filename)
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or(wav_filename)
                    .to_string()
            } else {
                raw_alias.to_string()
            };
            if alias.trim().is_empty() {
                return Err(format!("linha {line_number}: alias vazio"));
            }
            let parse_num = |field: &str, name: &str| {
                if field.is_empty() {
                    Ok(0.0)
                } else {
                    field.parse::<f64>().map_err(|_| {
                        format!("linha {line_number}: campo {name} inválido: '{field}'")
                    })
                }
            };
            let offset = parse_num(fields[1], "offset")?;
            let consonant = parse_num(fields[2], "consonant")?;
            let cutoff = parse_num(fields[3], "cutoff")?;
            let preutterance = parse_num(fields[4], "preutterance")?;
            let overlap = parse_num(fields[5], "overlap")?;
            if [offset, consonant, cutoff, preutterance, overlap]
                .iter()
                .any(|value| !value.is_finite())
            {
                return Err(format!("linha {line_number}: campo numérico não finito"));
            }
            // OpenUtau keeps the last definition for a repeated alias. This
            // is common in layered/subbank files, so it must not make the
            // entire singer unusable.
            map.insert(
                alias.clone(),
                OtoEntry::new(
                    wav_filename.to_string(),
                    alias,
                    offset,
                    consonant,
                    cutoff,
                    preutterance,
                    overlap,
                ),
            );
        }
        Ok(map)
    }

    pub fn parse_str_lossy(content: &str) -> (HashMap<String, OtoEntry>, Vec<String>) {
        let (entries, diagnostics) = Self::parse_str_lossy_diagnostics(content);
        (
            entries,
            diagnostics
                .iter()
                .map(OtoParseDiagnostic::display)
                .collect(),
        )
    }

    pub fn parse_str_lossy_diagnostics(
        content: &str,
    ) -> (HashMap<String, OtoEntry>, Vec<OtoParseDiagnostic>) {
        let mut entries = HashMap::new();
        let mut diagnostics = Vec::new();
        for (line_number, line) in content.lines().enumerate() {
            if line.trim().is_empty()
                || line.trim_start().starts_with(';')
                || line.trim_start().starts_with('#')
            {
                continue;
            }
            match Self::parse_str(line) {
                Ok(parsed) => entries.extend(parsed),
                Err(error) => diagnostics.push(parse_diagnostic(line_number + 1, &error)),
            }
        }
        (entries, diagnostics)
    }

    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<HashMap<String, OtoEntry>, io::Error> {
        Self::parse_bytes(&fs::read(path)?)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
    }

    pub fn parse_file_lossy<P: AsRef<Path>>(
        path: P,
    ) -> Result<(HashMap<String, OtoEntry>, Vec<String>), io::Error> {
        Ok(Self::parse_bytes_lossy(&fs::read(path)?))
    }

    pub fn parse_file_lossy_diagnostics<P: AsRef<Path>>(
        path: P,
    ) -> Result<(HashMap<String, OtoEntry>, Vec<OtoParseDiagnostic>), io::Error> {
        let bytes = fs::read(path)?;
        let text = match std::str::from_utf8(&bytes) {
            Ok(text) => text.to_owned(),
            Err(_) => SHIFT_JIS.decode(&bytes).0.into_owned(),
        };
        Ok(Self::parse_str_lossy_diagnostics(
            text.trim_start_matches('\u{feff}'),
        ))
    }
}

fn parse_diagnostic(line: usize, error: &str) -> OtoParseDiagnostic {
    let reason = error.strip_prefix("linha 1: ").unwrap_or(error).to_string();
    let field = reason.strip_prefix("campo ").and_then(|rest| {
        rest.split_once(" inválido")
            .map(|(field, _)| field.to_string())
    });
    let summary = field
        .as_ref()
        .map(|field| format!("campo {field} inválido"))
        .unwrap_or_else(|| reason.clone());
    OtoParseDiagnostic {
        line,
        field,
        reason: summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_oto() {
        let map =
            OtoParser::parse_str("ka.wav=ka,10,50,-100,20,10\nki.wav=,15,60,-110,25,12\n").unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map["ka"].offset, 10.0);
        assert_eq!(map["ka"].overlap, 10.0);
        assert_eq!(map["ki"].alias, "ki");
    }

    #[test]
    fn parses_utf8_aliases_without_mojibake() {
        let map = OtoParser::parse_bytes("voz.wav=céu,1,2,3,4,3\n".as_bytes()).unwrap();
        assert!(map.contains_key("céu"));
    }

    #[test]
    fn rejects_invalid_numbers_instead_of_using_zero() {
        let error = OtoParser::parse_str("ka.wav=ka,not-a-number,50,0,20,10").unwrap_err();
        assert!(error.contains("offset"));
    }

    #[test]
    fn empty_numeric_fields_match_explicit_zeroes() {
        let empty = OtoParser::parse_str("sample.wav=,,,,,\n").unwrap();
        let zero = OtoParser::parse_str("sample.wav=,0,0,0,0,0\n").unwrap();
        assert_eq!(empty["sample"].offset, zero["sample"].offset);
        assert_eq!(empty["sample"].consonant, zero["sample"].consonant);
        assert_eq!(empty["sample"].cutoff, zero["sample"].cutoff);
        assert_eq!(empty["sample"].preutterance, zero["sample"].preutterance);
        assert_eq!(empty["sample"].overlap, zero["sample"].overlap);
        assert_eq!(empty["sample"].alias, "sample");
        assert!(OtoParser::parse_str("sample.wav=,0,0,0,0\n").is_err());
    }

    #[test]
    fn rejects_missing_fields_and_keeps_last_duplicate_alias() {
        assert!(OtoParser::parse_str("ka.wav=ka,0,50,0,20").is_err());
        let map = OtoParser::parse_str("a.wav=ka,0,1,0,1,0\nb.wav=ka,5,1,0,1,0").unwrap();
        assert_eq!(map["ka"].wav_filename, "b.wav");
    }

    #[test]
    fn lossy_diagnostics_keep_the_original_line_number() {
        let (_, diagnostics) = OtoParser::parse_str_lossy_diagnostics(
            "; header\n\nka.wav=ka,0,50,0,20,invalid\nki.wav=ki,0,50,0,20,10\n",
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].line, 3);
        assert_eq!(diagnostics[0].field.as_deref(), Some("overlap"));
    }
}
