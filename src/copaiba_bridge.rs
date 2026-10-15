use crate::oto::OtoEntry;
use crate::oto::OtoParser;
use encoding_rs::SHIFT_JIS;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "caminho sem diretório pai",
        )
    })?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    use std::io::Write;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OtoSignature(u64);

pub fn oto_signature(root: &Path) -> std::io::Result<OtoSignature> {
    let mut paths = Vec::new();
    collect_oto_files(root, &mut paths)?;
    paths.sort();

    let mut hasher = DefaultHasher::new();
    for path in paths {
        path.hash(&mut hasher);
        let metadata = fs::metadata(&path)?;
        metadata.len().hash(&mut hasher);
        metadata.modified()?.hash(&mut hasher);
    }
    Ok(OtoSignature(hasher.finish()))
}

pub fn oto_path_for_alias(root: &Path, alias: &str) -> std::io::Result<PathBuf> {
    let mut paths = Vec::new();
    collect_oto_files(root, &mut paths)?;
    paths.sort();

    let target = alias.trim();
    for path in &paths {
        let entries = OtoParser::parse_file(path)?;
        if entries
            .keys()
            .any(|candidate| candidate.trim().eq_ignore_ascii_case(target))
        {
            return Ok(path.clone());
        }
    }

    paths.into_iter().next().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("nenhum oto.ini encontrado em {}", root.display()),
        )
    })
}

/// Rewrites only one OTO record, preserving comments, malformed lines,
/// ordering, line endings and the source encoding of the file.
pub fn update_oto_entry(
    oto_path: &Path,
    original_alias: &str,
    updated: &OtoEntry,
) -> Result<(), String> {
    if updated.alias.trim().is_empty() {
        return Err("o alias não pode ficar vazio".to_string());
    }
    let values = [
        updated.offset,
        updated.consonant,
        updated.cutoff,
        updated.preutterance,
        updated.overlap,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err("os parâmetros do oto.ini precisam ser finitos".to_string());
    }

    let bytes = fs::read(oto_path).map_err(|error| error.to_string())?;
    let utf8 = std::str::from_utf8(&bytes).is_ok();
    let content = if utf8 {
        String::from_utf8(bytes.clone()).map_err(|error| error.to_string())?
    } else {
        SHIFT_JIS.decode(&bytes).0.into_owned()
    };
    let target = original_alias.trim();
    let mut found = false;
    let mut replacement_count = 0;
    let mut output = String::with_capacity(content.len() + 32);

    for raw_line in content.split_inclusive('\n') {
        let (line, newline) = raw_line
            .strip_suffix('\n')
            .map(|line| (line.strip_suffix('\r').unwrap_or(line), "\r\n"))
            .unwrap_or((raw_line, ""));
        let trimmed = line.trim();
        let is_comment = trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#');
        let alias = (!is_comment)
            .then(|| {
                trimmed
                    .split_once('=')
                    .map(|(_, values)| values.split(',').next().unwrap_or(""))
            })
            .flatten()
            .map(str::trim)
            .filter(|alias| !alias.is_empty())
            .unwrap_or_else(|| {
                (!is_comment)
                    .then(|| {
                        trimmed.split_once('=').map(|(wav, _)| {
                            Path::new(wav.trim())
                                .file_stem()
                                .and_then(|stem| stem.to_str())
                                .unwrap_or("")
                        })
                    })
                    .flatten()
                    .unwrap_or("")
            });

        if !is_comment
            && alias.eq_ignore_ascii_case(updated.alias.trim())
            && !alias.eq_ignore_ascii_case(target)
        {
            return Err(format!(
                "o alias '{}' já existe em {}",
                updated.alias,
                oto_path.display()
            ));
        }

        if !is_comment && alias.eq_ignore_ascii_case(target) && !found {
            let rendered = format!(
                "{}={},{},{},{},{},{}",
                updated.wav_filename,
                updated.alias,
                updated.offset,
                updated.consonant,
                updated.cutoff,
                updated.preutterance,
                updated.overlap
            );
            output.push_str(&rendered);
            output.push_str(newline);
            found = true;
            replacement_count += 1;
        } else {
            output.push_str(line);
            output.push_str(newline);
        }
    }

    if !found || replacement_count != 1 {
        return Err(format!(
            "alias '{}' não encontrado em {}",
            target,
            oto_path.display()
        ));
    }
    let encoded = if utf8 {
        output.into_bytes()
    } else {
        let (encoded, _, had_errors) = SHIFT_JIS.encode(&output);
        if had_errors {
            return Err("o alias contém caracteres incompatíveis com Shift-JIS".to_string());
        }
        encoded.into_owned()
    };
    atomic_write(oto_path, &encoded).map_err(|error| error.to_string())
}

#[cfg(not(target_os = "android"))]
pub fn launch_editor(root: &Path, alias: &str) -> Result<(), String> {
    use std::process::Command;

    let oto_path = oto_path_for_alias(root, alias).map_err(|error| error.to_string())?;
    let mut command = if let Some(path) = std::env::var_os("KAMAFEU_COPAIBA_NEO") {
        Command::new(path)
    } else {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("copaiba-neo");
        let executable_name = if cfg!(target_os = "windows") {
            "copaiba_neo.exe"
        } else {
            "copaiba_neo"
        };
        let executable = ["release", "debug"]
            .into_iter()
            .map(|profile| workspace.join("target").join(profile).join(executable_name))
            .find(|path| path.is_file());

        if let Some(executable) = executable {
            Command::new(executable)
        } else {
            let manifest = workspace.join("Cargo.toml");
            if !manifest.is_file() {
                return Err("Copaiba NEO não encontrado. Clone-o em copaiba-neo/ ou defina KAMAFEU_COPAIBA_NEO.".to_string());
            }
            let mut command = Command::new("cargo");
            command.args(["run", "--manifest-path"]);
            command.arg(manifest);
            command.arg("--");
            command
        }
    };

    command
        .args(["--oto"])
        .arg(oto_path)
        .args(["--alias", alias])
        .spawn()
        .map_err(|error| format!("não foi possível abrir o Copaiba NEO: {error}"))?;
    Ok(())
}

fn collect_oto_files(root: &Path, paths: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            collect_oto_files(&path, paths)?;
        } else if path
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("oto.ini"))
        {
            paths.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_subfolder_oto_that_owns_an_alias() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("oto.ini"), "a.wav=a,0,0,0,0,0\n").unwrap();
        let subdir = root.path().join("C4");
        fs::create_dir(&subdir).unwrap();
        let sub_oto = subdir.join("oto.ini");
        fs::write(&sub_oto, "ka.wav=ka,0,0,0,0,0\n").unwrap();

        assert_eq!(oto_path_for_alias(root.path(), "KA").unwrap(), sub_oto);
    }

    #[test]
    fn signature_changes_when_an_oto_is_saved() {
        let root = tempfile::tempdir().unwrap();
        let oto = root.path().join("oto.ini");
        fs::write(&oto, "a.wav=a,0,0,0,0,0\n").unwrap();
        let initial = oto_signature(root.path()).unwrap();

        fs::write(&oto, "a.wav=a,10,0,0,0,0\n").unwrap();

        assert_ne!(initial, oto_signature(root.path()).unwrap());
    }

    #[test]
    fn updates_only_the_requested_entry_and_preserves_comments_and_lines() {
        let root = tempfile::tempdir().unwrap();
        let oto = root.path().join("oto.ini");
        fs::write(
            &oto,
            "; preserve\r\nka.wav=ka,0,50,0,20,10\r\ninvalid line\r\nki.wav=ki,1,2,3,4,5\r\n",
        )
        .unwrap();
        let updated = OtoEntry::new(
            "ka.wav".into(),
            "KA novo".into(),
            10.0,
            60.0,
            -20.0,
            25.0,
            12.0,
        );
        update_oto_entry(&oto, "KA", &updated).unwrap();
        let result = fs::read_to_string(&oto).unwrap();
        assert!(result.contains("; preserve\r\n"));
        assert!(result.contains("KA novo,10,60,-20,25,12\r\n"));
        assert!(result.contains("invalid line\r\n"));
        assert!(result.contains("ki.wav=ki,1,2,3,4,5\r\n"));
    }

    #[test]
    fn rejects_alias_collisions_and_non_finite_values() {
        let root = tempfile::tempdir().unwrap();
        let oto = root.path().join("oto.ini");
        fs::write(&oto, "ka.wav=ka,0,1,0,1,0\nki.wav=ki,0,1,0,1,0\n").unwrap();
        let collision = OtoEntry::new("ka.wav".into(), "ki".into(), 0.0, 1.0, 0.0, 1.0, 0.0);
        assert!(update_oto_entry(&oto, "ka", &collision).is_err());
        let invalid = OtoEntry::new("ka.wav".into(), "ka".into(), f64::NAN, 1.0, 0.0, 1.0, 0.0);
        assert!(update_oto_entry(&oto, "ka", &invalid).is_err());
    }
}
