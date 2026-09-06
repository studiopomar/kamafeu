use crate::oto::OtoParser;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

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
}
