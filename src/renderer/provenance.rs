//! Deterministic metadata for explaining and reproducing a render.

use super::timing::PhonemeTimingDiagnostic;
use super::RenderOptions;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const PROVENANCE_SCHEMA: u32 = 1;
pub const RENDER_PIPELINE_VERSION: &str = "kamafeu-render-1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderArtifact {
    pub kind: String,
    pub path: String,
    pub content_fingerprint: String,
}

/// Identity of an extension that participated in a render. The manifest
/// fingerprint makes a same-version replacement observable without relying on
/// filesystem timestamps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderExtension {
    pub id: String,
    pub version: String,
    pub origin: String,
    pub manifest_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimingBoundaryDiagnostic {
    pub phone: String,
    pub head_rms: f32,
    pub tail_rms: f32,
    pub window_ms: f32,
}

/// Parses the stable, non-executable boundary record emitted by the renderer.
/// Keeping this parser here lets CLI and other hosts capture the same record
/// without depending on localized human-readable log text.
pub fn parse_timing_boundary_log(message: &str) -> Option<TimingBoundaryDiagnostic> {
    let payload = message
        .trim_start()
        .strip_prefix("[Timing] boundary-rms ")?;
    let phone_start = payload.strip_prefix("phone='")?;
    let (phone, rest) = phone_start.split_once("' head=")?;
    let (head, rest) = rest.split_once(" tail=")?;
    let (tail, window) = rest.split_once(" window_ms=")?;
    Some(TimingBoundaryDiagnostic {
        phone: phone.to_string(),
        head_rms: head.parse().ok()?,
        tail_rms: tail.parse().ok()?,
        window_ms: window.parse().ok()?,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderProvenance {
    pub schema: u32,
    pub pipeline_version: String,
    pub project_fingerprint: String,
    pub voicebank_fingerprint: String,
    pub resampler: String,
    pub wavtool: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub options: RenderOptions,
    /// Flags inherited by the rendered voice profile. Per-phone flags remain
    /// in the render log because they can vary across a phrase.
    #[serde(default)]
    pub effective_flags: String,
    /// Extension identities participating in the render, if any.
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub extension_details: Vec<RenderExtension>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub cache_hit: bool,
    #[serde(default)]
    pub cache_hits: usize,
    #[serde(default)]
    pub cache_misses: usize,
    /// Files that influenced the render, identified by content rather than
    /// timestamp or path alone. This remains optional for old sidecars.
    #[serde(default)]
    pub artifacts: Vec<RenderArtifact>,
    /// Optional per-phoneme timing records captured by vocal renderers.
    /// Defaults to empty for compatibility with older sidecars.
    #[serde(default)]
    pub timing_diagnostics: Vec<PhonemeTimingDiagnostic>,
    /// RMS observations at the beginning and end of each rendered phoneme.
    #[serde(default)]
    pub timing_boundary_diagnostics: Vec<TimingBoundaryDiagnostic>,
    /// Human-readable, non-executable command descriptions emitted by the
    /// render pipeline. Older sidecars deserialize this as an empty list.
    #[serde(default)]
    pub commands: Vec<String>,
}

impl RenderProvenance {
    pub fn new(
        project_fingerprint: impl Into<String>,
        voicebank_fingerprint: impl Into<String>,
        resampler: impl Into<String>,
        wavtool: impl Into<String>,
        sample_rate: u32,
        channels: u16,
        options: RenderOptions,
    ) -> Self {
        Self {
            schema: PROVENANCE_SCHEMA,
            pipeline_version: RENDER_PIPELINE_VERSION.to_string(),
            project_fingerprint: project_fingerprint.into(),
            voicebank_fingerprint: voicebank_fingerprint.into(),
            resampler: resampler.into(),
            wavtool: wavtool.into(),
            sample_rate,
            channels,
            effective_flags: options.flags.clone(),
            options,
            extensions: Vec::new(),
            extension_details: Vec::new(),
            warnings: Vec::new(),
            cache_hit: false,
            cache_hits: 0,
            cache_misses: 0,
            artifacts: Vec::new(),
            timing_diagnostics: Vec::new(),
            timing_boundary_diagnostics: Vec::new(),
            commands: Vec::new(),
        }
    }

    pub fn with_timing_diagnostics(
        mut self,
        diagnostics: impl IntoIterator<Item = PhonemeTimingDiagnostic>,
    ) -> Self {
        self.timing_diagnostics = diagnostics.into_iter().collect();
        self.timing_diagnostics.sort_by(|left, right| {
            left.position_ms
                .total_cmp(&right.position_ms)
                .then_with(|| left.duration_ms.total_cmp(&right.duration_ms))
                .then_with(|| left.index.cmp(&right.index))
                .then_with(|| left.oto_preutter_ms.total_cmp(&right.oto_preutter_ms))
                .then_with(|| {
                    serde_json::to_string(left)
                        .expect("timing diagnostic is serializable")
                        .cmp(
                            &serde_json::to_string(right)
                                .expect("timing diagnostic is serializable"),
                        )
                })
        });
        self
    }

    pub fn with_timing_boundary_diagnostics(
        mut self,
        diagnostics: impl IntoIterator<Item = TimingBoundaryDiagnostic>,
    ) -> Self {
        self.timing_boundary_diagnostics = diagnostics.into_iter().collect();
        self.timing_boundary_diagnostics.sort_by(|left, right| {
            left.phone
                .cmp(&right.phone)
                .then_with(|| left.head_rms.total_cmp(&right.head_rms))
                .then_with(|| left.tail_rms.total_cmp(&right.tail_rms))
        });
        self
    }

    pub fn timing_boundary_warnings(&self) -> Vec<String> {
        let mut warnings = self
            .timing_boundary_diagnostics
            .iter()
            .filter_map(|boundary| {
                if !boundary.head_rms.is_finite() || !boundary.tail_rms.is_finite() {
                    Some(format!(
                        "[Timing] RMS não finito na fronteira do fonema '{}'",
                        boundary.phone
                    ))
                } else if boundary.head_rms <= f32::EPSILON && boundary.tail_rms <= f32::EPSILON {
                    Some(format!(
                        "[Timing] início e fim silenciosos no fonema '{}'",
                        boundary.phone
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        warnings.sort();
        warnings.dedup();
        warnings
    }

    pub fn with_commands(mut self, commands: impl IntoIterator<Item = String>) -> Self {
        self.commands = commands.into_iter().collect();
        self.commands.sort();
        self.commands.dedup();
        self
    }

    pub fn with_cache_stats(mut self, hits: usize, misses: usize) -> Self {
        self.cache_hits = hits;
        self.cache_misses = misses;
        self.cache_hit = hits > 0 && misses == 0;
        self
    }

    pub fn with_extensions<I, S>(mut self, extensions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.extensions = extensions.into_iter().map(Into::into).collect();
        self.extensions.sort();
        self.extensions.dedup();
        self
    }

    pub fn with_extension_details<I>(mut self, extensions: I) -> Self
    where
        I: IntoIterator<Item = RenderExtension>,
    {
        self.extension_details = extensions.into_iter().collect();
        self.extension_details.sort_by(|left, right| {
            left.id
                .cmp(&right.id)
                .then_with(|| left.version.cmp(&right.version))
                .then_with(|| left.origin.cmp(&right.origin))
                .then_with(|| left.manifest_fingerprint.cmp(&right.manifest_fingerprint))
        });
        self.extension_details.dedup();
        self
    }

    pub fn with_artifact_file(
        self,
        kind: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> Result<Self, String> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|error| {
            format!("não foi possível ler artefato {}: {error}", path.display())
        })?;
        Ok(self.with_artifact_bytes(kind, path.to_string_lossy(), &bytes))
    }

    pub fn with_artifact_bytes(
        mut self,
        kind: impl Into<String>,
        path: impl Into<String>,
        bytes: &[u8],
    ) -> Self {
        self.artifacts.push(RenderArtifact {
            kind: kind.into(),
            path: path.into(),
            content_fingerprint: content_fingerprint(bytes),
        });
        self.artifacts.sort_by(|left, right| {
            left.kind
                .cmp(&right.kind)
                .then_with(|| left.path.cmp(&right.path))
        });
        self
    }

    /// Stable, compact identity for logs, cache diagnostics and sidecars.
    pub fn fingerprint(&self) -> String {
        let encoded = serde_json::to_vec(self).expect("render provenance is serializable");
        // FNV-1a is deliberately used instead of Rust's DefaultHasher: the
        // latter is an implementation detail and is not a cross-process
        // identity suitable for render manifests or persistent caches.
        let mut hash = 0xcbf29ce484222325u64;
        for byte in encoded {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{hash:016x}")
    }

    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Returns a stable content identity suitable for cache keys and manifests.
/// FNV-1a is intentionally small and deterministic across processes; it is
/// not intended as a cryptographic signature.
pub fn content_fingerprint(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_fingerprint_changes_when_render_inputs_change() {
        let base = RenderProvenance::new(
            "project-a",
            "voice-a",
            "VENUS",
            "Andromeda",
            44_100,
            2,
            RenderOptions::default(),
        );
        let mut changed = base.clone();
        changed.sample_rate = 48_000;
        assert_ne!(base.fingerprint(), changed.fingerprint());
    }

    #[test]
    fn provenance_roundtrips_as_human_readable_json() {
        let mut options = RenderOptions::default();
        options.flags = "P86g-5".to_string();
        let value = RenderProvenance::new("p", "v", "resampler", "wavtool", 44_100, 2, options)
            .with_extensions(["org.example.engine", "org.example.engine"]);
        let json = value.to_pretty_json().unwrap();
        assert!(json.contains("project_fingerprint"));
        assert_eq!(value.effective_flags, "P86g-5");
        assert_eq!(value.extensions, ["org.example.engine"]);
        assert_eq!(
            serde_json::from_str::<RenderProvenance>(&json).unwrap(),
            value
        );
    }

    #[test]
    fn extension_details_are_sorted_and_part_of_the_fingerprint() {
        let base = RenderProvenance::new(
            "p",
            "v",
            "resampler",
            "wavtool",
            44_100,
            2,
            RenderOptions::default(),
        );
        let detail = RenderExtension {
            id: "org.example.engine".into(),
            version: "1.2.0".into(),
            origin: "/plugins/engine".into(),
            manifest_fingerprint: "abc".into(),
        };
        let changed = base
            .clone()
            .with_extension_details([detail.clone(), detail.clone()]);
        assert_eq!(changed.extension_details, [detail]);
        assert_ne!(base.fingerprint(), changed.fingerprint());
    }

    #[test]
    fn provenance_roundtrips_timing_diagnostics() {
        let diagnostic = PhonemeTimingDiagnostic {
            index: 0,
            position_ms: 0.0,
            duration_ms: 250.0,
            oto_preutter_ms: 30.0,
            oto_overlap_ms: 10.0,
            preutter_delta_ms: 0.0,
            overlap_delta_ms: 2.0,
            adjacent: false,
            preutter_ms: 30.0,
            overlap_ms: 12.0,
            leading_ms: 30.0,
            skip_over_ms: 0.0,
            tail_intrude_ms: 0.0,
            tail_overlap_ms: 0.0,
        };
        let provenance = RenderProvenance::new(
            "p",
            "v",
            "resampler",
            "wavtool",
            44_100,
            2,
            RenderOptions::default(),
        )
        .with_timing_diagnostics([diagnostic])
        .with_timing_boundary_diagnostics([TimingBoundaryDiagnostic {
            phone: "k a".to_string(),
            head_rms: 0.125,
            tail_rms: 0.25,
            window_ms: 10.0,
        }])
        .with_commands([
            "wavtool z".to_string(),
            "resampler a".to_string(),
            "resampler a".to_string(),
        ]);
        let parsed: RenderProvenance =
            serde_json::from_str(&provenance.to_pretty_json().expect("provenance JSON"))
                .expect("timing diagnostics sidecar");
        assert_eq!(parsed.timing_diagnostics, vec![diagnostic]);
        assert_eq!(parsed.timing_boundary_diagnostics.len(), 1);
        assert_eq!(parsed.timing_boundary_diagnostics[0].phone, "k a");
        assert_eq!(parsed.commands, ["resampler a", "wavtool z"]);

        let mut legacy = serde_json::to_value(&provenance).expect("legacy provenance JSON");
        legacy
            .as_object_mut()
            .expect("provenance object")
            .remove("timing_diagnostics");
        legacy
            .as_object_mut()
            .expect("provenance object")
            .remove("timing_boundary_diagnostics");
        legacy
            .as_object_mut()
            .expect("provenance object")
            .remove("extension_details");
        let legacy: RenderProvenance =
            serde_json::from_value(legacy).expect("legacy sidecar compatibility");
        assert!(legacy.timing_diagnostics.is_empty());
        assert!(legacy.timing_boundary_diagnostics.is_empty());
        assert!(legacy.extension_details.is_empty());

        let mut second = diagnostic;
        second.index = 1;
        let ordered = RenderProvenance::new(
            "p",
            "v",
            "resampler",
            "wavtool",
            44_100,
            2,
            RenderOptions::default(),
        )
        .with_timing_diagnostics([diagnostic, second]);
        let reversed = RenderProvenance::new(
            "p",
            "v",
            "resampler",
            "wavtool",
            44_100,
            2,
            RenderOptions::default(),
        )
        .with_timing_diagnostics([second, diagnostic]);
        assert_eq!(ordered.fingerprint(), reversed.fingerprint());
    }

    #[test]
    fn parses_boundary_logs_with_compound_aliases() {
        let parsed = parse_timing_boundary_log(
            "  [Timing] boundary-rms phone='k a' head=0.125000 tail=0.250000 window_ms=10.0",
        )
        .expect("boundary diagnostic");
        assert_eq!(parsed.phone, "k a");
        assert_eq!(parsed.head_rms, 0.125);
        assert_eq!(parsed.tail_rms, 0.25);
        assert_eq!(parsed.window_ms, 10.0);
    }

    #[test]
    fn boundary_warnings_only_flag_nonfinite_or_doubly_silent_edges() {
        let provenance = RenderProvenance::new(
            "p",
            "v",
            "resampler",
            "wavtool",
            44_100,
            2,
            RenderOptions::default(),
        )
        .with_timing_boundary_diagnostics([
            TimingBoundaryDiagnostic {
                phone: "quiet".into(),
                head_rms: 0.0,
                tail_rms: 0.0,
                window_ms: 10.0,
            },
            TimingBoundaryDiagnostic {
                phone: "finite".into(),
                head_rms: 0.1,
                tail_rms: 0.0,
                window_ms: 10.0,
            },
            TimingBoundaryDiagnostic {
                phone: "broken".into(),
                head_rms: f32::NAN,
                tail_rms: 0.1,
                window_ms: 10.0,
            },
        ]);
        let warnings = provenance.timing_boundary_warnings();
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|warning| warning.contains("quiet")));
        assert!(warnings.iter().any(|warning| warning.contains("broken")));
    }

    #[test]
    fn artifact_hash_changes_when_file_content_changes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("oto.ini");
        std::fs::write(&path, b"k a=sample.wav,0,0,0,0,0").unwrap();
        let first = RenderProvenance::new(
            "p",
            "v",
            "resampler",
            "wavtool",
            44_100,
            1,
            RenderOptions::default(),
        )
        .with_artifact_file("oto", &path)
        .unwrap();
        std::fs::write(&path, b"k a=changed.wav,0,0,0,0,0").unwrap();
        let second = RenderProvenance::new(
            "p",
            "v",
            "resampler",
            "wavtool",
            44_100,
            1,
            RenderOptions::default(),
        )
        .with_artifact_file("oto", &path)
        .unwrap();
        assert_ne!(
            first.artifacts[0].content_fingerprint,
            second.artifacts[0].content_fingerprint
        );
        assert_ne!(first.fingerprint(), second.fingerprint());
    }
}
