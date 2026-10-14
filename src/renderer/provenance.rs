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

/// Complete, deterministic alias-resolution record for one rendered phone.
/// This is also the schema used by the CLI diagnostics, so the terminal and
/// the saved manifest describe the same resolution decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhonemeResolutionDiagnostic {
    #[serde(default)]
    pub part_index: usize,
    /// Implementation status at the time this phone was resolved.
    #[serde(default)]
    pub phonemizer_implementation: String,
    pub note_index: usize,
    pub lyric: String,
    pub pitch: String,
    #[serde(default)]
    pub requested_timbre: Option<String>,
    #[serde(default)]
    pub resolved_alias: Option<String>,
    #[serde(default)]
    pub resolved_timbre: Option<String>,
    #[serde(default)]
    pub oto_wav: Option<String>,
    #[serde(default)]
    pub oto_preutter_ms: Option<f64>,
    #[serde(default)]
    pub oto_overlap_ms: Option<f64>,
    pub position_ms: f64,
    pub duration_ms: f64,
    pub flags: String,
}

/// Structured boundary record for the external resampler/wavtool handoff.
/// The renderer emits this exact value to the terminal and the CLI stores it
/// in the provenance sidecar, so diagnostics cannot diverge between them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhonemeRenderDiagnostic {
    pub phone_index: usize,
    pub lyric: String,
    pub resampler: String,
    pub wavtool: String,
    pub input_wav: String,
    pub output_wav: String,
    pub command: String,
    pub pitch: String,
    pub pitch_frequency: f64,
    pub flags: String,
    pub offset_ms: f64,
    pub requested_duration_ms: f64,
    pub resampler_duration_ms: f64,
    pub source_consonant_ms: f64,
    pub consonant_ms: f64,
    pub cutoff_ms: f64,
    pub skip_over_ms: f64,
    pub overlap_ms: f64,
    pub input_sample_rate: u32,
    pub output_sample_rate: u32,
    pub resampler_output_sample_rate: u32,
    pub sample_rate_converted: bool,
    pub resampler_peak: f32,
    pub resampler_rms: f32,
    pub resampler_clipped_samples: usize,
    pub wavtool_peak: f32,
    pub wavtool_rms: f32,
    pub wavtool_clipped_samples: usize,
    pub envelope_points: [(f64, f64); 5],
    pub crossfade_ms: f64,
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
    /// Original project identity, kept separate from the content fingerprint.
    #[serde(default)]
    pub project_path: String,
    #[serde(default)]
    pub project_format: String,
    #[serde(default = "default_import_phoneme_policy_label")]
    pub import_phoneme_policy: String,
    #[serde(default)]
    pub voicebank_requested: Option<String>,
    #[serde(default)]
    pub voicebank_effective: String,
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
    #[serde(default)]
    pub phoneme_resolutions: Vec<PhonemeResolutionDiagnostic>,
    #[serde(default)]
    pub phoneme_renders: Vec<PhonemeRenderDiagnostic>,
    /// Human-readable, non-executable command descriptions emitted by the
    /// render pipeline. Older sidecars deserialize this as an empty list.
    #[serde(default)]
    pub commands: Vec<String>,
}

fn default_import_phoneme_policy_label() -> String {
    "preserve".to_string()
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
            project_path: String::new(),
            project_format: String::new(),
            import_phoneme_policy: "preserve".to_string(),
            voicebank_requested: None,
            voicebank_effective: String::new(),
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
            phoneme_resolutions: Vec::new(),
            phoneme_renders: Vec::new(),
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

    pub fn with_phoneme_resolutions(
        mut self,
        diagnostics: impl IntoIterator<Item = PhonemeResolutionDiagnostic>,
    ) -> Self {
        self.phoneme_resolutions = diagnostics.into_iter().collect();
        self.phoneme_resolutions.sort_by(|a, b| {
            a.part_index
                .cmp(&b.part_index)
                .then_with(|| a.position_ms.total_cmp(&b.position_ms))
                .then_with(|| a.note_index.cmp(&b.note_index))
                .then_with(|| a.lyric.cmp(&b.lyric))
        });
        self
    }

    pub fn with_phoneme_renders(
        mut self,
        diagnostics: impl IntoIterator<Item = PhonemeRenderDiagnostic>,
    ) -> Self {
        self.phoneme_renders = diagnostics.into_iter().collect();
        self.phoneme_renders.sort_by(|left, right| {
            left.phone_index
                .cmp(&right.phone_index)
                .then_with(|| left.lyric.cmp(&right.lyric))
        });
        self
    }

    pub fn with_import_context(
        mut self,
        project_path: impl Into<String>,
        project_format: impl Into<String>,
        policy: impl Into<String>,
        voicebank_requested: Option<String>,
        voicebank_effective: impl Into<String>,
    ) -> Self {
        self.project_path = project_path.into();
        self.project_format = project_format.into();
        self.import_phoneme_policy = policy.into();
        self.voicebank_requested = voicebank_requested;
        self.voicebank_effective = voicebank_effective.into();
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
            .with_extensions(["org.example.engine", "org.example.engine"])
            .with_import_context(
                "song.ustx",
                "ustx",
                "preserve",
                Some("/voicebanks/singer".to_string()),
                "/voicebanks/singer",
            );
        let json = value.to_pretty_json().unwrap();
        assert!(json.contains("project_fingerprint"));
        assert!(json.contains("project_format"));
        assert_eq!(value.project_format, "ustx");
        assert_eq!(value.import_phoneme_policy, "preserve");
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
            auto_preutter_ms: 30.0,
            auto_overlap_ms: 10.0,
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
        for field in [
            "project_path",
            "project_format",
            "import_phoneme_policy",
            "voicebank_requested",
            "voicebank_effective",
            "phoneme_resolutions",
        ] {
            legacy
                .as_object_mut()
                .expect("provenance object")
                .remove(field);
        }
        let legacy: RenderProvenance =
            serde_json::from_value(legacy).expect("legacy sidecar compatibility");
        assert!(legacy.timing_diagnostics.is_empty());
        assert!(legacy.timing_boundary_diagnostics.is_empty());
        assert!(legacy.extension_details.is_empty());
        assert_eq!(legacy.project_path, "");
        assert_eq!(legacy.project_format, "");
        assert_eq!(legacy.import_phoneme_policy, "preserve");
        assert!(legacy.voicebank_requested.is_none());
        assert!(legacy.phoneme_resolutions.is_empty());

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
    fn phoneme_resolution_order_includes_part_identity() {
        let base = RenderProvenance::new(
            "p",
            "v",
            "resampler",
            "wavtool",
            44_100,
            1,
            RenderOptions::default(),
        );
        let make = |part_index: usize, position_ms: f64, lyric: &str| PhonemeResolutionDiagnostic {
            part_index,
            phonemizer_implementation: "Passthrough".to_string(),
            note_index: 0,
            lyric: lyric.to_string(),
            pitch: "C4".to_string(),
            requested_timbre: None,
            resolved_alias: Some(lyric.to_string()),
            resolved_timbre: None,
            oto_wav: None,
            oto_preutter_ms: None,
            oto_overlap_ms: None,
            position_ms,
            duration_ms: 100.0,
            flags: String::new(),
        };
        let ordered = base
            .clone()
            .with_phoneme_resolutions([make(0, 500.0, "part0"), make(1, 0.0, "part1")]);
        let reversed =
            base.with_phoneme_resolutions([make(1, 0.0, "part1"), make(0, 500.0, "part0")]);
        assert_eq!(ordered.phoneme_resolutions, reversed.phoneme_resolutions);
        assert_eq!(ordered.phoneme_resolutions[0].part_index, 0);
        assert_eq!(ordered.phoneme_resolutions[1].part_index, 1);
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
