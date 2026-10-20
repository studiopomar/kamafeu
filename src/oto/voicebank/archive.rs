use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::Path;

pub(crate) fn extract_kfv(zip_path: &Path, dest_dir: &Path) -> Result<(), std::io::Error> {
    const MAX_FILES: usize = 10_000;
    const MAX_FILE_SIZE: u64 = 512 * 1024 * 1024;
    const MAX_TOTAL_SIZE: u64 = 4 * 1024 * 1024 * 1024;

    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    if archive.len() > MAX_FILES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "arquivo KFV contém entradas demais",
        ));
    }

    let mut total_size = 0u64;
    let mut actual_total_size = 0u64;
    let mut seen_paths = HashSet::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let entry_name = file
            .name()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
            .into_owned();
        if file
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("links are not allowed in KFV packages: {entry_name}"),
            ));
        }
        if file.size() > MAX_FILE_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("entrada KFV excede o limite de tamanho: {entry_name}"),
            ));
        }
        if file.size() > 10 * 1024 * 1024
            && file.compressed_size() > 0
            && file.size() / file.compressed_size() > 1_000
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("entrada KFV possui compressão suspeita: {entry_name}"),
            ));
        }
        total_size = total_size.checked_add(file.size()).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "tamanho KFV inválido")
        })?;
        if total_size > MAX_TOTAL_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "conteúdo descompactado do KFV excede o limite permitido",
            ));
        }
        if entry_name.contains('\\') || entry_name.contains('\0') {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unsafe path in KFV package",
            ));
        }
        let enclosed = file.enclosed_name().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unsafe path in KFV package: {entry_name}"),
            )
        })?;
        if enclosed.as_os_str().is_empty()
            || enclosed
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unsafe path in KFV package: {entry_name}"),
            ));
        }
        let normalized_name = enclosed.to_string_lossy().into_owned();
        if !seen_paths.insert(normalized_name) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("duplicate path in KFV package: {entry_name}"),
            ));
        }
        let outpath = dest_dir.join(enclosed);

        if entry_name.ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            let copied = std::io::copy(&mut file.by_ref().take(MAX_FILE_SIZE + 1), &mut outfile)?;
            if copied > MAX_FILE_SIZE || copied != file.size() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "tamanho extraído de entrada KFV diverge do declarado",
                ));
            }
            actual_total_size = actual_total_size.checked_add(copied).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "tamanho extraído KFV inválido",
                )
            })?;
            if actual_total_size > MAX_TOTAL_SIZE {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "conteúdo extraído do KFV excede o limite permitido",
                ));
            }
        }
    }
    Ok(())
}
