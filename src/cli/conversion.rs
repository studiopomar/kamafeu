use super::*;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ConversionFingerprint {
    bpm: f64,
    time_signature_numerator: u8,
    time_signature_denominator: u8,
    track_count: usize,
    voice_part_count: usize,
    wave_part_count: usize,
    marker_count: usize,
    section_count: usize,
    extension_count: usize,
    pub(super) note_count: usize,
    duration_ms: f64,
    pitch_bend_point_count: usize,
    dynamics_curve_point_count: usize,
    phoneme_override_count: usize,
    non_default_expression_count: usize,
    pub(super) note_content_signature: u64,
}

impl ConversionFingerprint {
    pub(super) fn from_project(project: &UProject) -> Self {
        let mut fingerprint = Self {
            bpm: project.bpm,
            time_signature_numerator: project.time_signature_numerator,
            time_signature_denominator: project.time_signature_denominator,
            track_count: project.tracks.len(),
            voice_part_count: project.parts.len(),
            wave_part_count: project.wave_parts.len(),
            marker_count: project.markers.len(),
            section_count: project.sections.len(),
            extension_count: project.extensions.len(),
            note_count: 0,
            duration_ms: 0.0,
            pitch_bend_point_count: 0,
            dynamics_curve_point_count: 0,
            phoneme_override_count: 0,
            non_default_expression_count: 0,
            note_content_signature: 0xcbf2_9ce4_8422_2325,
        };
        for part in &project.parts {
            fingerprint.hash_u64(part.track_index as u64);
            fingerprint.hash_f64(part.position_ms);
            for note in &part.notes {
                fingerprint.note_count += 1;
                fingerprint.duration_ms = fingerprint
                    .duration_ms
                    .max(part.position_ms + note.position_ms + note.duration_ms.max(0.0));
                fingerprint.pitch_bend_point_count += note.pitch_bend.points.len();
                fingerprint.dynamics_curve_point_count += note.expressions.dynamics_curve.len();
                fingerprint.phoneme_override_count += note.phoneme_overrides.len();
                if note.expressions != kamafeu::project::UExpressions::default() {
                    fingerprint.non_default_expression_count += 1;
                }
                fingerprint.hash_text(&note.lyric);
                fingerprint.hash_text(&note.pitch);
                fingerprint.hash_f64(note.position_ms);
                fingerprint.hash_f64(note.duration_ms);
                for phoneme_override in &note.phoneme_overrides {
                    fingerprint
                        .hash_text(&serde_json::to_string(phoneme_override).unwrap_or_default());
                }
            }
        }
        for wave in &project.wave_parts {
            fingerprint.duration_ms = fingerprint
                .duration_ms
                .max(wave.position_ms + wave.duration_ms.max(0.0));
        }
        fingerprint
    }

    fn hash_u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.note_content_signature ^= u64::from(byte);
            self.note_content_signature =
                self.note_content_signature.wrapping_mul(0x1000_0000_01b3);
        }
    }

    fn hash_f64(&mut self, value: f64) {
        self.hash_u64(value.to_bits());
    }

    fn hash_text(&mut self, value: &str) {
        self.hash_u64(value.len() as u64);
        for byte in value.as_bytes() {
            self.note_content_signature ^= u64::from(*byte);
            self.note_content_signature =
                self.note_content_signature.wrapping_mul(0x1000_0000_01b3);
        }
    }

    fn json(&self) -> Value {
        json!({
            "bpm": self.bpm,
            "time_signature": format!("{}/{}", self.time_signature_numerator, self.time_signature_denominator),
            "track_count": self.track_count,
            "voice_part_count": self.voice_part_count,
            "wave_part_count": self.wave_part_count,
            "marker_count": self.marker_count,
            "section_count": self.section_count,
            "extension_count": self.extension_count,
            "note_count": self.note_count,
            "duration_ms": self.duration_ms,
            "pitch_bend_point_count": self.pitch_bend_point_count,
            "dynamics_curve_point_count": self.dynamics_curve_point_count,
            "phoneme_override_count": self.phoneme_override_count,
            "non_default_expression_count": self.non_default_expression_count,
            "note_content_signature": format!("{:016x}", self.note_content_signature),
        })
    }
}

pub(super) fn convert_project(
    input: &PathBuf,
    output: &PathBuf,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut source = load_project(input)?;
    source.normalize();
    let before = ConversionFingerprint::from_project(&source);
    save_project(&source, output)?;

    // Reopen through the selected adapter. A write succeeding is not enough:
    // interchange formats can intentionally omit data the native project has.
    let destination = load_project(output)?;
    let after = ConversionFingerprint::from_project(&destination);
    let mut warnings = Vec::new();
    compare_conversion_feature(
        &mut warnings,
        "tempo (BPM)",
        before.bpm,
        after.bpm,
        |left, right| (left - right).abs() > 0.001,
    );
    let before_meter = format!(
        "{}/{}",
        before.time_signature_numerator, before.time_signature_denominator
    );
    let after_meter = format!(
        "{}/{}",
        after.time_signature_numerator, after.time_signature_denominator
    );
    compare_conversion_feature(
        &mut warnings,
        "compasso",
        before_meter.as_str(),
        after_meter.as_str(),
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "faixas",
        before.track_count,
        after.track_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "partes vocais",
        before.voice_part_count,
        after.voice_part_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "partes de áudio",
        before.wave_part_count,
        after.wave_part_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "marcadores de projeto",
        before.marker_count,
        after.marker_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "seções de projeto",
        before.section_count,
        after.section_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "notas",
        before.note_count,
        after.note_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "conteúdo de letra, altura ou tempo das notas (assinatura)",
        before.note_content_signature,
        after.note_content_signature,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "duração do projeto (ms)",
        before.duration_ms,
        after.duration_ms,
        |left, right| (left - right).abs() > 1.0,
    );
    compare_conversion_feature(
        &mut warnings,
        "pontos de pitch bend",
        before.pitch_bend_point_count,
        after.pitch_bend_point_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "pontos de curva de dinâmica",
        before.dynamics_curve_point_count,
        after.dynamics_curve_point_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "overrides persistidos por fonema",
        before.phoneme_override_count,
        after.phoneme_override_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "notas com expressões não padrão",
        before.non_default_expression_count,
        after.non_default_expression_count,
        |left, right| left != right,
    );
    compare_conversion_feature(
        &mut warnings,
        "extensões associadas ao projeto",
        before.extension_count,
        after.extension_count,
        |left, right| left != right,
    );

    Ok(json!({
        "input": input,
        "output": output,
        "source": before.json(),
        "destination": after.json(),
        "warnings": warnings,
        "tracked_features_preserved": warnings.is_empty(),
    }))
}

fn compare_conversion_feature<T: std::fmt::Display + Copy>(
    warnings: &mut Vec<String>,
    label: &str,
    source: T,
    destination: T,
    differs: impl FnOnce(T, T) -> bool,
) {
    if differs(source, destination) {
        warnings.push(format!(
            "{label}: origem {source}, destino {destination}; revise este recurso após a conversão"
        ));
    }
}

pub(super) fn project_validation_json(
    path: &PathBuf,
    extensions_path: Option<&PathBuf>,
) -> Result<Value, Box<dyn std::error::Error>> {
    let project = load_project(path)?;
    let report = project.diagnostic_report();
    let extension_diagnostics = extensions_path
        .map(|catalog_path| {
            let discovery = discover(catalog_path);
            let mut diagnostics = diagnose_project_extensions(&project.extensions, &discovery);
            diagnostics.extend(discovery.diagnostics);
            diagnostics
        })
        .unwrap_or_default();
    Ok(json!({
        "path": path,
        "valid": report.is_valid(),
        "track_count": report.track_count,
        "voice_part_count": report.voice_part_count,
        "wave_part_count": report.wave_part_count,
        "note_count": report.note_count,
        "error_count": report.error_count(),
        "warning_count": report.warning_count(),
        "vocal_health_score": report.vocal_health_score(),
        "extension_diagnostics": extension_diagnostics,
        "extension_diagnostic_records": extensions_path.map(|catalog_path| {
            let discovery = discover(catalog_path);
            let mut records = diagnose_project_extensions_structured(&project.extensions, &discovery)
                .into_iter()
                .map(|record| serde_json::to_value(record).unwrap_or_else(|_| serde_json::json!({})))
                .collect::<Vec<_>>();
            records.extend(discovery.diagnostic_records.into_iter().map(|diagnostic| {
                serde_json::json!({
                    "extension_id": serde_json::Value::Null,
                    "kind": "discovery",
                    "detail": diagnostic.reason,
                    "manifest_path": diagnostic.manifest_path,
                })
            }));
            serde_json::to_value(records).unwrap_or_else(|_| serde_json::json!([]))
        }).unwrap_or_else(|| serde_json::json!([])),
        "issues": report.issues.iter().map(|issue| json!({
            "severity": format!("{:?}", issue.severity).to_ascii_lowercase(),
            "kind": format!("{:?}", issue.kind),
            "location": issue.location,
            "detail": issue.detail,
            "suggestion": issue.suggestion(),
        })).collect::<Vec<_>>(),
    }))
}
