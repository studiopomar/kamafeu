//! GitHub release checks. Download/install is deliberately user-confirmed.

use serde::Deserialize;

pub const GITHUB_REPOSITORY: &str = "studiopomar/kamafeu";

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub html_url: String,
    pub body: Option<String>,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub draft: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Available {
        version: String,
        url: String,
        notes: String,
    },
    UpToDate,
}

pub fn check_latest_release() -> Result<GitHubRelease, String> {
    #[cfg(target_arch = "wasm32")]
    return Err("verificação de atualização indisponível no navegador".to_string());

    #[cfg(not(target_arch = "wasm32"))]
    {
        let url = format!("https://api.github.com/repos/{GITHUB_REPOSITORY}/releases?per_page=20");
        let releases = ureq::get(&url)
            .set("Accept", "application/vnd.github+json")
            .set("User-Agent", "Kamafeu-Studio-Update-Checker")
            .call()
            .map_err(|error| format!("não foi possível consultar o GitHub: {error}"))?
            .into_json::<Vec<GitHubRelease>>()
            .map_err(|error| format!("resposta de atualização inválida: {error}"))?;
        releases
            .into_iter()
            .find(|release| !release.draft)
            .ok_or_else(|| "nenhuma release publicada encontrada".to_string())
    }
}

fn normalized_version(value: &str) -> Vec<u32> {
    value
        .trim_start_matches(['v', 'V'])
        .split(|character: char| !character.is_ascii_digit())
        .filter_map(|part| (!part.is_empty()).then(|| part.parse().ok()).flatten())
        .collect()
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    let mut left = normalized_version(latest);
    let mut right = normalized_version(current);
    left.resize(3, 0);
    right.resize(3, 0);
    left > right
}

pub fn classify_release(release: GitHubRelease, current: &str) -> UpdateStatus {
    if is_newer(&release.tag_name, current) {
        UpdateStatus::Available {
            version: release.tag_name,
            url: release.html_url,
            notes: release.body.unwrap_or_default(),
        }
    } else {
        UpdateStatus::UpToDate
    }
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn compares_release_versions_without_the_v_prefix() {
        assert!(is_newer("v1.2.0", "1.1.9"));
        assert!(!is_newer("v1.1.9", "1.2.0"));
        assert!(!is_newer("v1.2.0-rc.1", "1.2.0"));
    }
}
