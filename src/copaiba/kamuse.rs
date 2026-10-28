//! Guided authoring operations for the native `cantor.toml` format.
use crate::voicebank::cantor::{Ajustes, Cantor, CantorBank, Gravacao, Tom};
use image::GenericImageView;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct KamuseProject {
    pub root: PathBuf,
    pub cantor: Cantor,
    _workspace: Option<std::sync::Arc<tempfile::TempDir>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingPrompt {
    pub onset: String,
    pub vowel: String,
    pub coda: String,
}

impl RecordingPrompt {
    pub fn label(&self) -> String {
        format!("{}{}{}", self.onset, self.vowel, self.coda)
    }
}

impl KamuseProject {
    pub fn set_engine(&mut self, engine: &str) -> Result<(), String> {
        if engine != "tainah" {
            return Err("bancos cantor.toml usam exclusivamente a engine Tainah".into());
        }
        let previous = std::mem::replace(&mut self.cantor.engine, engine.to_string());
        if let Err(error) = self.save() {
            self.cantor.engine = previous;
            return Err(error);
        }
        Ok(())
    }

    pub fn set_adjustments(&mut self, gain_db: f32, transition_ms: f32) -> Result<(), String> {
        let previous = self.cantor.ajustes.clone();
        self.cantor.ajustes.ganho_db = gain_db;
        self.cantor.ajustes.transicao_ms = transition_ms;
        if let Err(error) = self.save() {
            self.cantor.ajustes = previous;
            return Err(error);
        }
        Ok(())
    }

    pub fn import_presentation_asset(&mut self, source: &Path, avatar: bool) -> Result<(), String> {
        if !source.is_file() {
            return Err("selecione um arquivo de imagem existente".into());
        }
        let decoded = image::open(source).map_err(|e| format!("imagem inválida: {e}"))?;
        if avatar && decoded.dimensions() != (200, 200) {
            return Err("o avatar deve medir exatamente 200×200 pixels".into());
        }
        if !avatar && !decoded.pixels().any(|(_, _, pixel)| pixel.0[3] < 255) {
            return Err("o retrato do piano roll precisa conter transparência".into());
        }
        let name = if avatar { "avatar" } else { "portrait" };
        let target = self.root.join("assets").join(format!("{name}.png"));
        std::fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::copy(source, &target).map_err(|e| format!("falha ao copiar imagem: {e}"))?;
        let previous = if avatar {
            self.cantor.imagem.replace(format!("assets/{name}.png"))
        } else {
            self.cantor.portrait.replace(format!("assets/{name}.png"))
        };
        if let Err(error) = self.save() {
            if avatar {
                self.cantor.imagem = previous;
            } else {
                self.cantor.portrait = previous;
            }
            let _ = std::fs::remove_file(target);
            return Err(error);
        }
        Ok(())
    }

    pub fn create(parent: &Path, id: &str, name: &str, language: &str) -> Result<Self, String> {
        if !parent.is_dir() {
            return Err("selecione uma pasta existente para criar o cantor".into());
        }
        let id = id.trim();
        let name = name.trim();
        if id.is_empty() || name.is_empty() || language.trim().is_empty() {
            return Err("id, nome e idioma são obrigatórios".into());
        }
        let folder = id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        if folder.is_empty() {
            return Err("o id não produz um nome de pasta seguro".into());
        }
        let root = parent.join(folder);
        if root.exists() {
            return Err(format!("a pasta já existe: {}", root.display()));
        }
        let stage = tempfile::Builder::new()
            .prefix(".kamuse-new-")
            .tempdir_in(parent)
            .map_err(|e| format!("falha ao criar área temporária: {e}"))?;
        std::fs::create_dir_all(stage.path().join("src")).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(stage.path().join("assets")).map_err(|e| e.to_string())?;
        let project = Self {
            root: stage.path().to_path_buf(),
            cantor: Cantor {
                versao: 1,
                id: id.into(),
                nome: name.into(),
                idiomas: vec![language.into()],
                engine: "tainah".into(),
                ajustes: Ajustes {
                    ganho_db: 0.0,
                    transicao_ms: 20.0,
                },
                tons: Vec::new(),
                imagem: None,
                portrait: None,
            },
            _workspace: None,
        };
        project.save()?;
        std::fs::rename(stage.path(), &root)
            .map_err(|e| format!("falha ao publicar banco novo: {e}"))?;
        let _ = stage.keep();
        Self::load(&root)
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let bank = CantorBank::load_path(path)?;
        Ok(Self {
            root: bank.root,
            cantor: bank.cantor,
            _workspace: bank._workspace,
        })
    }

    pub fn save(&self) -> Result<(), String> {
        self.cantor.validate(&self.root)?;
        let text = toml::to_string_pretty(&self.cantor)
            .map_err(|e| format!("falha ao serializar cantor.toml: {e}"))?;
        let target = self.root.join("cantor.toml");
        let parent = target.parent().ok_or("cantor.toml sem pasta pai")?;
        let mut tmp = tempfile::NamedTempFile::new_in(parent)
            .map_err(|e| format!("falha ao criar arquivo temporário: {e}"))?;
        use std::io::Write;
        tmp.write_all(text.as_bytes())
            .map_err(|e| format!("falha ao salvar cantor.toml: {e}"))?;
        tmp.as_file()
            .sync_all()
            .map_err(|e| format!("falha ao sincronizar cantor.toml: {e}"))?;
        tmp.persist(target)
            .map_err(|e| format!("falha ao publicar cantor.toml: {}", e.error))?;
        Ok(())
    }

    pub fn add_tone(&mut self, midi: u8) -> Result<String, String> {
        if midi > 127 {
            return Err("tom MIDI fora do intervalo 0–127".into());
        }
        if self.cantor.tons.iter().any(|tone| tone.midi == midi) {
            return Err(format!("já existe um tom MIDI {midi} neste banco"));
        }
        let id = format!("midi-{midi}");
        self.cantor.tons.push(Tom {
            id: id.clone(),
            midi,
            gravacoes: Vec::new(),
        });
        if let Err(error) = self.save() {
            self.cantor.tons.pop();
            return Err(error);
        }
        Ok(id)
    }

    pub fn set_tone_midi(&mut self, tone_id: &str, midi: u8) -> Result<(), String> {
        if midi > 127 {
            return Err("tom MIDI fora do intervalo 0–127".into());
        }
        if self
            .cantor
            .tons
            .iter()
            .any(|tone| tone.id != tone_id && tone.midi == midi)
        {
            return Err(format!("já existe outro tom MIDI {midi} neste banco"));
        }
        let tone = self
            .cantor
            .tons
            .iter_mut()
            .find(|tone| tone.id == tone_id)
            .ok_or("tom não encontrado")?;
        let previous = tone.midi;
        tone.midi = midi;
        if let Err(error) = self.save() {
            if let Some(tone) = self.cantor.tons.iter_mut().find(|tone| tone.id == tone_id) {
                tone.midi = previous;
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn delete_tone(&mut self, tone_id: &str) -> Result<(), String> {
        let index = self
            .cantor
            .tons
            .iter()
            .position(|tone| tone.id == tone_id)
            .ok_or("tom não encontrado")?;
        let removed = self.cantor.tons.remove(index);
        if let Err(error) = self.save() {
            self.cantor.tons.insert(index, removed);
            return Err(error);
        }
        Ok(())
    }

    pub fn set_recording_phones(
        &mut self,
        tone_id: &str,
        recording_id: &str,
        phones: &[String],
    ) -> Result<(), String> {
        let (tone_index, recording_index) = self.recording_indices(tone_id, recording_id)?;
        let previous = self.cantor.tons[tone_index].gravacoes[recording_index]
            .fonemas
            .clone();
        if phones.len() + 1
            != self.cantor.tons[tone_index].gravacoes[recording_index]
                .marcacoes_ms
                .len()
        {
            return Err(
                "mantenha a mesma quantidade de fonemas para preservar as marcações editadas"
                    .into(),
            );
        }
        self.cantor.tons[tone_index].gravacoes[recording_index].fonemas =
            phones.iter().map(|phone| phone.trim().to_owned()).collect();
        if let Err(error) = self.save() {
            self.cantor.tons[tone_index].gravacoes[recording_index].fonemas = previous;
            return Err(error);
        }
        Ok(())
    }

    pub fn rename_recording(
        &mut self,
        tone_id: &str,
        recording_id: &str,
        new_id: &str,
    ) -> Result<(), String> {
        let (tone_index, recording_index) = self.recording_indices(tone_id, recording_id)?;
        let previous = self.cantor.tons[tone_index].gravacoes[recording_index]
            .id
            .clone();
        self.cantor.tons[tone_index].gravacoes[recording_index].id = new_id.trim().to_owned();
        if let Err(error) = self.save() {
            self.cantor.tons[tone_index].gravacoes[recording_index].id = previous;
            return Err(error);
        }
        Ok(())
    }

    pub fn duplicate_recording(
        &mut self,
        tone_id: &str,
        recording_id: &str,
    ) -> Result<String, String> {
        let (tone_index, recording_index) = self.recording_indices(tone_id, recording_id)?;
        let mut duplicate = self.cantor.tons[tone_index].gravacoes[recording_index].clone();
        let base = format!("{}-copy", duplicate.id);
        let mut id = base.clone();
        let mut suffix = 2;
        while self.cantor.tons[tone_index]
            .gravacoes
            .iter()
            .any(|recording| recording.id == id)
        {
            id = format!("{base}-{suffix}");
            suffix += 1;
        }
        duplicate.id = id.clone();
        self.cantor.tons[tone_index]
            .gravacoes
            .insert(recording_index + 1, duplicate);
        if let Err(error) = self.save() {
            self.cantor.tons[tone_index]
                .gravacoes
                .remove(recording_index + 1);
            return Err(error);
        }
        Ok(id)
    }

    pub fn delete_recording(&mut self, tone_id: &str, recording_id: &str) -> Result<(), String> {
        let (tone_index, recording_index) = self.recording_indices(tone_id, recording_id)?;
        let removed = self.cantor.tons[tone_index]
            .gravacoes
            .remove(recording_index);
        if let Err(error) = self.save() {
            self.cantor.tons[tone_index]
                .gravacoes
                .insert(recording_index, removed);
            return Err(error);
        }
        Ok(())
    }

    fn recording_indices(
        &self,
        tone_id: &str,
        recording_id: &str,
    ) -> Result<(usize, usize), String> {
        let tone_index = self
            .cantor
            .tons
            .iter()
            .position(|tone| tone.id == tone_id)
            .ok_or("tom não encontrado")?;
        let recording_index = self.cantor.tons[tone_index]
            .gravacoes
            .iter()
            .position(|recording| recording.id == recording_id)
            .ok_or("gravação não encontrada")?;
        Ok((tone_index, recording_index))
    }

    /// Copies source audio into `src/` and creates editable, evenly spaced
    /// provisional marks. The chosen original is never modified.
    pub fn import_recording(
        &mut self,
        source: &Path,
        tone_midi: u8,
        phones: &[String],
    ) -> Result<String, String> {
        if phones.len() < 2 || phones.iter().any(|p| p.trim().is_empty()) {
            return Err("informe ao menos dois fonemas não vazios".into());
        }
        if tone_midi > 127 {
            return Err("tom MIDI fora do intervalo 0–127".into());
        }
        let info = crate::audio::probe_audio_file(source)
            .ok_or_else(|| format!("WAV/MP3 inválido ou não suportado: {}", source.display()))?;
        let ext = source
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "wav" | "mp3") {
            return Err("KAMUSE aceita gravações WAV e MP3".into());
        }
        let tone_id = format!("midi-{tone_midi}");
        let tone_index = if let Some(i) = self.cantor.tons.iter().position(|t| t.midi == tone_midi)
        {
            i
        } else {
            self.cantor.tons.push(Tom {
                id: tone_id.clone(),
                midi: tone_midi,
                gravacoes: Vec::new(),
            });
            self.cantor.tons.len() - 1
        };
        let base = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("gravacao")
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>();
        let mut recording_id = base.trim_matches('-').to_string();
        if recording_id.is_empty() {
            recording_id = "gravacao".into();
        }
        let mut suffix = 2;
        let original_id = recording_id.clone();
        while self.cantor.tons[tone_index]
            .gravacoes
            .iter()
            .any(|r| r.id == recording_id)
        {
            recording_id = format!("{original_id}-{suffix}");
            suffix += 1;
        }
        let filename = format!("{recording_id}.{ext}");
        let rel = Path::new("src").join(filename);
        let target = self.root.join(&rel);
        let same_file = std::fs::canonicalize(source)
            .ok()
            .zip(std::fs::canonicalize(&target).ok())
            .is_some_and(|(a, b)| a == b);
        if target.exists() && !same_file {
            return Err(format!(
                "o caminho de gravação já existe: {}",
                target.display()
            ));
        }
        if !same_file {
            std::fs::copy(source, &target)
                .map_err(|e| format!("falha ao copiar áudio sem alterar o original: {e}"))?;
        }
        let step = info.duration_ms / phones.len() as f64;
        let marks = (0..=phones.len())
            .map(|i| {
                if i == phones.len() {
                    info.duration_ms
                } else {
                    step * i as f64
                }
            })
            .collect();
        self.cantor.tons[tone_index].gravacoes.push(Gravacao {
            id: recording_id.clone(),
            arquivo: rel.to_string_lossy().replace('\\', "/"),
            fonemas: phones.iter().map(|p| p.trim().into()).collect(),
            marcacoes_ms: marks,
        });
        if let Err(error) = self.save() {
            self.cantor.tons[tone_index].gravacoes.pop();
            if !same_file {
                let _ = std::fs::remove_file(target);
            }
            return Err(error);
        }
        Ok(recording_id)
    }

    pub fn set_marks(
        &mut self,
        tone_id: &str,
        recording_id: &str,
        marks: Vec<f64>,
    ) -> Result<(), String> {
        let tone_index = self
            .cantor
            .tons
            .iter()
            .position(|tone| tone.id == tone_id)
            .ok_or("tom não encontrado")?;
        let recording_index = self.cantor.tons[tone_index]
            .gravacoes
            .iter()
            .position(|recording| recording.id == recording_id)
            .ok_or("gravação não encontrada")?;
        let old = self.cantor.tons[tone_index].gravacoes[recording_index]
            .marcacoes_ms
            .clone();
        self.update_marks_in_memory(tone_id, recording_id, marks)?;
        if let Err(error) = self.save() {
            self.cantor.tons[tone_index].gravacoes[recording_index].marcacoes_ms = old;
            return Err(error);
        }
        Ok(())
    }

    pub fn update_marks_in_memory(
        &mut self,
        tone_id: &str,
        recording_id: &str,
        marks: Vec<f64>,
    ) -> Result<(), String> {
        let tone_index = self
            .cantor
            .tons
            .iter()
            .position(|t| t.id == tone_id)
            .ok_or("tom não encontrado")?;
        let recording_index = self.cantor.tons[tone_index]
            .gravacoes
            .iter()
            .position(|r| r.id == recording_id)
            .ok_or("gravação não encontrada")?;
        self.cantor.tons[tone_index].gravacoes[recording_index].marcacoes_ms = marks;
        Ok(())
    }

    pub fn recording_prompts(language: &str) -> Result<Vec<RecordingPrompt>, String> {
        let (onsets, vowels, codas): (&[&str], &[&str], &[&str]) = match language {
            "pt-BR" => (
                &[
                    "b", "p", "t", "d", "k", "g", "f", "v", "s", "z", "sh", "zh", "m", "n", "nh",
                    "l", "lh", "r", "rr", "ch", "dj", "y", "w",
                ],
                &[
                    "a", "e", "eh", "i", "o", "oh", "u", "an", "en", "in", "on", "un",
                ],
                &["m", "n", "nh", "l", "r", "rr", "s", "z"],
            ),
            "en-US" => (
                &[
                    "b", "p", "t", "d", "k", "g", "f", "v", "s", "z", "sh", "zh", "m", "n", "l",
                    "r", "w", "y", "ch", "j",
                ],
                &[
                    "ae", "eh", "ih", "iy", "aa", "ao", "uh", "uw", "ah", "er", "ay", "aw", "oy",
                    "ow",
                ],
                &["m", "n", "ng", "l", "r", "s", "z", "t", "d"],
            ),
            "ja-JP" => (
                &[
                    "k", "g", "s", "z", "t", "d", "n", "h", "b", "p", "m", "y", "r", "w",
                ],
                &["a", "i", "u", "e", "o"],
                &["k", "s", "t", "n", "h", "m", "r"],
            ),
            other => return Err(format!("perfil de gravação não disponível: {other}")),
        };
        let count = onsets.len() * vowels.len() * codas.len();
        if count > 10_000 {
            return Err("perfil produz mais de 10 mil prompts; reduza o inventário".into());
        }
        Ok(onsets
            .iter()
            .flat_map(|onset| {
                vowels.iter().flat_map(move |vowel| {
                    codas.iter().map(move |coda| RecordingPrompt {
                        onset: (*onset).into(),
                        vowel: (*vowel).into(),
                        coda: (*coda).into(),
                    })
                })
            })
            .collect())
    }
}
