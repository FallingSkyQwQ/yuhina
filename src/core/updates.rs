use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::http;
use super::{Result, YuhinaError};

const RELEASES_URL: &str = "https://api.github.com/repos/FallingSkyQwQ/yuhina/releases/latest";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAsset {
    pub name: String,
    pub size: u64,
    pub url: String,
    pub digest: Option<String>,
    pub installable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub tag: String,
    pub notes: String,
    pub page_url: String,
    pub published_at: Option<String>,
    pub asset: Option<UpdateAsset>,
}

pub fn normalize_tag(tag: &str) -> String {
    tag.trim().trim_start_matches(['v', 'V']).to_string()
}

fn parse_version(value: &str) -> Option<(Vec<u64>, Vec<String>)> {
    let normalized = normalize_tag(value);
    let normalized = normalized.split('+').next().unwrap_or(&normalized);
    if normalized.is_empty() {
        return None;
    }
    let (core, pre) = match normalized.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (normalized, None),
    };
    let core: Option<Vec<u64>> = core
        .split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect();
    let core = core?;
    let pre = pre
        .map(|pre| pre.split('.').map(str::to_string).collect())
        .unwrap_or_default();
    Some((core, pre))
}

pub fn compare_versions(left: &str, right: &str) -> i32 {
    let Some((a_core, a_pre)) = parse_version(left) else {
        return 0;
    };
    let Some((b_core, b_pre)) = parse_version(right) else {
        return 0;
    };
    let length = a_core.len().max(b_core.len());
    for index in 0..length {
        let a = a_core.get(index).copied().unwrap_or(0);
        let b = b_core.get(index).copied().unwrap_or(0);
        if a != b {
            return if a < b { -1 } else { 1 };
        }
    }
    match (a_pre.is_empty(), b_pre.is_empty()) {
        (true, true) => 0,
        (true, false) => 1,
        (false, true) => -1,
        (false, false) => {
            let length = a_pre.len().max(b_pre.len());
            for index in 0..length {
                match (a_pre.get(index), b_pre.get(index)) {
                    (None, Some(_)) => return -1,
                    (Some(_), None) => return 1,
                    (Some(a), Some(b)) => {
                        let a_num = a.parse::<u64>();
                        let b_num = b.parse::<u64>();
                        match (a_num, b_num) {
                            (Ok(a), Ok(b)) if a != b => return if a < b { -1 } else { 1 },
                            (Ok(_), Err(_)) => return -1,
                            (Err(_), Ok(_)) => return 1,
                            (Ok(_), Ok(_)) => {}
                            (Err(_), Err(_)) if a != b => return if a < b { -1 } else { 1 },
                            _ => {}
                        }
                    }
                    (None, None) => break,
                }
            }
            0
        }
    }
}

fn platform_suffix() -> (&'static str, &'static str) {
    let os = if cfg!(target_os = "windows") {
        "win"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        _ => "x64",
    };
    (os, arch)
}

fn score_asset(name: &str) -> i32 {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".blockmap") || lower.ends_with(".yml") || lower.ends_with(".yaml") {
        return 0;
    }
    let (os, arch) = platform_suffix();
    let mut score = 0;
    if cfg!(target_os = "windows") {
        if lower.ends_with(".exe") {
            score += if lower.contains("setup") { 100 } else { 90 };
        } else if lower.ends_with(".zip") {
            score += 30;
        } else {
            return 0;
        }
    } else if cfg!(target_os = "macos") {
        if lower.ends_with(".dmg") {
            score += 100;
        } else if lower.ends_with(".zip") {
            score += 30;
        } else {
            return 0;
        }
    } else if lower.ends_with(".appimage") {
        score += 100;
    } else if lower.ends_with(".deb") {
        score += 80;
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tar.xz") {
        score += 40;
    } else {
        return 0;
    }
    if lower.contains(os) {
        score += 10;
    }
    let arch_tokens: &[&str] = match arch {
        "arm64" => &["arm64", "aarch64"],
        _ => &["x64", "x86_64", "amd64"],
    };
    let has_any_arch = ["x64", "x86_64", "amd64", "arm64", "aarch64", "ia32", "x86"]
        .iter()
        .any(|token| lower.contains(token));
    if has_any_arch && !arch_tokens.iter().any(|token| lower.contains(token)) {
        return 0;
    }
    score + if has_any_arch { 20 } else { 5 }
}

pub fn check(current_version: &str) -> Result<Option<UpdateInfo>> {
    let response = http::request(
        "GET",
        RELEASES_URL,
        &[
            ("Accept", "application/vnd.github+json"),
            ("X-GitHub-Api-Version", "2022-11-28"),
        ],
        None,
    )?;
    if response.status == 404 {
        return Ok(None);
    }
    if !response.is_success() {
        return Err(YuhinaError::new(
            super::ErrorCode::Http,
            format!("The update check failed ({}).", response.status),
        ));
    }
    let value: Value = response.json()?;
    if value["draft"].as_bool().unwrap_or(false) || value["prerelease"].as_bool().unwrap_or(false) {
        return Ok(None);
    }
    let tag = value["tag_name"].as_str().unwrap_or_default().to_string();
    let version = normalize_tag(&tag);
    if version.is_empty() || compare_versions(&version, current_version) <= 0 {
        return Ok(None);
    }

    let mut best: Option<(i32, UpdateAsset)> = None;
    if let Some(assets) = value["assets"].as_array() {
        for asset in assets {
            let name = asset["name"].as_str().unwrap_or_default();
            let score = score_asset(name);
            if score == 0 {
                continue;
            }
            let digest = asset["digest"].as_str().map(str::to_string);
            let installable = cfg!(target_os = "windows")
                && name.to_ascii_lowercase().ends_with(".exe")
                && digest.is_some();
            let candidate = UpdateAsset {
                name: name.to_string(),
                size: asset["size"].as_u64().unwrap_or(0),
                url: asset["browser_download_url"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                digest,
                installable,
            };
            if best
                .as_ref()
                .map(|(best_score, _)| score > *best_score)
                .unwrap_or(true)
            {
                best = Some((score, candidate));
            }
        }
    }

    Ok(Some(UpdateInfo {
        version,
        tag,
        notes: value["body"].as_str().unwrap_or_default().to_string(),
        page_url: value["html_url"].as_str().unwrap_or_default().to_string(),
        published_at: value["published_at"].as_str().map(str::to_string),
        asset: best.map(|(_, asset)| asset),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_ordering() {
        assert!(compare_versions("0.2.0", "0.1.9") > 0);
        assert!(compare_versions("1.0.0", "1.0.0") == 0);
        assert!(compare_versions("1.0.0-beta.1", "1.0.0") < 0);
        assert!(compare_versions("v1.2.3", "1.2.2") > 0);
    }
}
