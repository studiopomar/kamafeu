//! Audio-to-note transcription adapters used by the editor.

use crate::dsp::pitch::{midi_to_note_name, note_name_to_midi};
use crate::project::model::{UNote, UProject, UTrack, UVoicePart, UWavePart};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct GameNote {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub pitch_midi: f64,
}

/// Runs the optional `mvt midi` adapter and reads GAME's CSV output.
/// The model is selected by its catalog id or local path; inference remains
/// outside the WASM extension sandbox and never downloads a model implicitly
/// from Kamafeu itself.
#[cfg(not(target_arch = "wasm32"))]
pub fn transcribe_with_game_command(
    command: &str,
    audio_path: &Path,
    model: &str,
    language: Option<&str>,
) -> Result<Vec<GameNote>, String> {
    if command.trim().is_empty() {
        return Err("selecione o comando do backend GAME".into());
    }
    if model.trim().is_empty() {
        return Err("informe o ID ou caminho do modelo GAME".into());
    }
    if !audio_path.is_file() {
        return Err(format!(
            "arquivo de áudio não encontrado: {}",
            audio_path.display()
        ));
    }

    let output_dir = tempfile::tempdir()
        .map_err(|error| format!("não foi possível criar a pasta temporária: {error}"))?;
    let mut process = Command::new(command);
    process
        .arg("midi")
        .arg(audio_path)
        .args([
            "--model",
            model,
            "--tempo",
            "120",
            "--formats",
            "csv",
            "--out",
        ])
        .arg(output_dir.path());
    if let Some(language) = language.filter(|language| !language.trim().is_empty()) {
        process.args(["--lang", language]);
    }
    let output = process
        .output()
        .map_err(|error| format!("não foi possível executar '{command}': {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        return Err(format!("o backend GAME terminou com erro: {detail}"));
    }

    let csv_path = find_output_csv(output_dir.path())?;
    parse_game_csv(&std::fs::read_to_string(&csv_path).map_err(|error| error.to_string())?)
}

#[cfg(target_arch = "wasm32")]
pub fn transcribe_with_game_command(
    _command: &str,
    _audio_path: &Path,
    _model: &str,
    _language: Option<&str>,
) -> Result<Vec<GameNote>, String> {
    Err("a transcrição GAME ainda requer o host nativo do Kamafeu".into())
}

fn find_output_csv(directory: &Path) -> Result<std::path::PathBuf, String> {
    for entry in std::fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            if let Ok(path) = find_output_csv(&entry.path()) {
                return Ok(path);
            }
        } else if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "csv")
        {
            return Ok(entry.path());
        }
    }
    Err("o backend GAME não gerou arquivo CSV".into())
}

pub fn parse_game_csv(contents: &str) -> Result<Vec<GameNote>, String> {
    let mut notes = Vec::new();
    for (line_number, line) in contents.lines().enumerate() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with("onset,") {
            continue;
        }
        let columns: Vec<_> = line.split(',').collect();
        if columns.len() < 3 {
            return Err(format!("CSV GAME inválido na linha {}", line_number + 1));
        }
        let start_seconds = columns[0]
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("início inválido na linha {}", line_number + 1))?;
        let end_seconds = columns[1]
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("fim inválido na linha {}", line_number + 1))?;
        let pitch_midi = parse_pitch(columns[2].trim())
            .ok_or_else(|| format!("afinação inválida na linha {}", line_number + 1))?;
        if !start_seconds.is_finite()
            || !end_seconds.is_finite()
            || !pitch_midi.is_finite()
            || start_seconds < 0.0
            || end_seconds <= start_seconds
            || !(0.0..=127.0).contains(&pitch_midi)
        {
            return Err(format!(
                "nota fora dos limites na linha {}",
                line_number + 1
            ));
        }
        notes.push(GameNote {
            start_seconds,
            end_seconds,
            pitch_midi,
        });
    }
    if notes.is_empty() {
        return Err("GAME não encontrou notas no áudio".into());
    }
    notes.sort_by(|left, right| left.start_seconds.total_cmp(&right.start_seconds));
    Ok(notes)
}

fn parse_pitch(value: &str) -> Option<f64> {
    if let Ok(number) = value.parse::<f64>() {
        return Some(number);
    }
    let split_at = value
        .char_indices()
        .skip(1)
        .find(|(_, character)| *character == '+' || *character == '-')
        .map(|(index, _)| index)
        .unwrap_or(value.len());
    let (note, cents) = value.split_at(split_at);
    let midi = f64::from(note_name_to_midi(note)?);
    let cents = if cents.is_empty() {
        0.0
    } else {
        cents.parse::<f64>().ok()?
    };
    Some(midi + cents / 100.0)
}

pub fn project_from_game_notes(
    name: &str,
    audio_path: &Path,
    notes: &[GameNote],
    provisional_lyric: &str,
) -> Result<UProject, String> {
    if notes.is_empty() {
        return Err("não há notas transcritas para criar o projeto".into());
    }
    let audio_name = audio_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("vocal.wav")
        .to_string();
    let rounded_lyric = if provisional_lyric.trim().is_empty() {
        "a"
    } else {
        provisional_lyric.trim()
    };
    let mut voice_notes = Vec::with_capacity(notes.len());
    for note in notes {
        let midi = note.pitch_midi.round().clamp(0.0, 127.0) as u8;
        let mut voice_note = UNote::new(
            rounded_lyric,
            midi_to_note_name(midi),
            note.start_seconds * 1000.0,
            (note.end_seconds - note.start_seconds) * 1000.0,
        );
        voice_note.expressions.pitch_delta = (note.pitch_midi - f64::from(midi)) * 100.0;
        voice_notes.push(voice_note);
    }

    let mut project = UProject::default();
    project.name = name.to_string();
    project.tracks = vec![
        UTrack {
            name: "Vocal de referência".into(),
            singer: "Áudio de referência".into(),
            ..UTrack::default()
        },
        UTrack {
            name: "Transcrição GAME".into(),
            singer: "Default Singer".into(),
            ..UTrack::default()
        },
    ];
    project.parts = vec![UVoicePart {
        name: "Notas estimadas pelo GAME".into(),
        track_index: 1,
        position_ms: 0.0,
        notes: voice_notes,
    }];
    project.wave_parts = vec![UWavePart::new(audio_name, audio_path.to_string_lossy(), 0)];
    Ok(project)
}
