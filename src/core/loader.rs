//! Forge and NeoForge loader support.
//!
//! Both projects ship an installer jar rather than a ready-made version
//! profile. The installer writes `versions/<id>/<id>.json` plus its libraries
//! into the shared Minecraft directory, so the launcher only has to run it and
//! then record the version id it produced.

use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use super::download::{self, DownloadTask};
use super::instances::{self, LoaderType};
use super::java;
use super::paths::Paths;
use super::settings::JavaSettings;
use super::{ErrorCode, Result, YuhinaError, http};

pub const FORGE_PROMOTIONS_URL: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
pub const FORGE_MAVEN_BASE: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";
pub const NEOFORGE_METADATA_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";
pub const NEOFORGE_MAVEN_BASE: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";

/// Maximum number of NeoForge builds offered in the create dialog.
const NEOFORGE_CHOICES: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoaderVersion {
    pub version: String,
    /// True for Forge's recommended promotion and for the newest stable
    /// NeoForge build.
    pub recommended: bool,
}

impl LoaderVersion {
    fn new(version: impl Into<String>, recommended: bool) -> Self {
        Self {
            version: version.into(),
            recommended,
        }
    }
}

/// Parses `promotions_slim.json` and returns the recommended build first, then
/// the latest build when it differs.
pub fn parse_forge_promotions(body: &str, minecraft_version: &str) -> Result<Vec<LoaderVersion>> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| YuhinaError::new(ErrorCode::Parse, error.to_string()))?;
    let promos = value
        .get("promos")
        .and_then(|value| value.as_object())
        .ok_or_else(|| {
            YuhinaError::new(
                ErrorCode::Parse,
                "The Forge promotion list has no promos object.",
            )
        })?;
    let latest = promos
        .get(&format!("{minecraft_version}-latest"))
        .and_then(|value| value.as_str());
    let recommended = promos
        .get(&format!("{minecraft_version}-recommended"))
        .and_then(|value| value.as_str());

    let mut versions = Vec::new();
    if let Some(version) = recommended {
        versions.push(LoaderVersion::new(version, true));
    }
    if let Some(version) = latest
        && !versions.iter().any(|entry| entry.version == version)
    {
        // When Forge has no recommendation for this game version, the latest
        // build is the sensible default.
        versions.push(LoaderVersion::new(version, recommended.is_none()));
    }

    if versions.is_empty() {
        return Err(YuhinaError::new(
            ErrorCode::NotFound,
            format!("Forge has no build for Minecraft {minecraft_version}."),
        ));
    }
    Ok(versions)
}

/// Extracts the `<version>` entries from NeoForge's Maven metadata.
pub fn parse_neoforge_versions(xml: &str) -> Result<Vec<String>> {
    let mut versions = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<version>") {
        let after = &rest[start + "<version>".len()..];
        let Some(end) = after.find("</version>") else {
            break;
        };
        let value = after[..end].trim();
        if !value.is_empty() && !versions.iter().any(|entry| entry == value) {
            versions.push(value.to_string());
        }
        rest = &after[end + "</version>".len()..];
    }
    if versions.is_empty() {
        return Err(YuhinaError::new(
            ErrorCode::Parse,
            "The NeoForge version list is empty.",
        ));
    }
    Ok(versions)
}

/// NeoForge versions use `<mc minor>.<mc patch>.<build>` for Minecraft 1.20.2
/// and newer. For example 1.21.1 maps to the `21.1` series and 1.21 to `21.0`.
pub fn neoforge_prefix(minecraft_version: &str) -> Option<String> {
    let mut parts = minecraft_version.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    if major != 1 {
        return None;
    }
    let minor = parts.next()?.parse::<u32>().ok()?;
    let patch = parts
        .next()
        .and_then(|part| part.split(['-', '+']).next())
        .and_then(|part| part.parse::<u32>().ok())
        .unwrap_or(0);
    Some(format!("{minor}.{patch}"))
}

fn version_sort_key(value: &str) -> Vec<u64> {
    value
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

/// Stable builds first, then the highest version number.
fn is_stable(value: &str) -> bool {
    !value.contains('-') && !value.to_ascii_lowercase().contains("beta")
}

/// Returns the compatible NeoForge builds, newest stable build first.
pub fn compatible_neoforge_versions<'a>(
    versions: &'a [String],
    minecraft_version: &str,
) -> Vec<&'a str> {
    let Some(prefix) = neoforge_prefix(minecraft_version) else {
        return Vec::new();
    };
    let series = format!("{prefix}.");
    let mut matches: Vec<&str> = versions
        .iter()
        .map(String::as_str)
        .filter(|version| *version == prefix || version.starts_with(&series))
        .collect();
    matches.sort_by(|a, b| {
        is_stable(b)
            .cmp(&is_stable(a))
            .then_with(|| version_sort_key(b).cmp(&version_sort_key(a)))
    });
    matches.dedup();
    matches.truncate(NEOFORGE_CHOICES);
    matches
}

/// Fetches loader builds for the create-instance dialog.
pub fn discover(minecraft_version: &str, loader: LoaderType) -> Result<Vec<LoaderVersion>> {
    match loader {
        LoaderType::Forge => {
            let body = http::get_text(FORGE_PROMOTIONS_URL)?;
            parse_forge_promotions(&body, minecraft_version)
        }
        LoaderType::Neoforge => {
            let body = http::get_text(NEOFORGE_METADATA_URL)?;
            let versions = parse_neoforge_versions(&body)?;
            let compatible = compatible_neoforge_versions(&versions, minecraft_version);
            if compatible.is_empty() {
                return Err(YuhinaError::new(
                    ErrorCode::NotFound,
                    format!("NeoForge has no build for Minecraft {minecraft_version}."),
                ));
            }
            Ok(compatible
                .into_iter()
                .enumerate()
                .map(|(index, version)| LoaderVersion::new(version, index == 0))
                .collect())
        }
        other => Err(YuhinaError::new(
            ErrorCode::Unsupported,
            format!("{} does not use an installer.", other.label()),
        )),
    }
}

fn validate_url(url: &str) -> Result<()> {
    let parsed = url::Url::parse(url)?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(YuhinaError::new(
            ErrorCode::InvalidInput,
            "The installer URL must use http or https.",
        ));
    }
    Ok(())
}

/// Returns the official installer jar URL for a loader build.
pub fn installer_url(
    loader: LoaderType,
    minecraft_version: &str,
    loader_version: &str,
) -> Result<String> {
    super::paths::assert_safe_id(minecraft_version)?;
    super::paths::assert_safe_id(loader_version)?;
    let url = match loader {
        LoaderType::Forge => {
            let combined = format!("{minecraft_version}-{loader_version}");
            format!("{FORGE_MAVEN_BASE}/{combined}/forge-{combined}-installer.jar")
        }
        LoaderType::Neoforge => {
            format!(
                "{NEOFORGE_MAVEN_BASE}/{loader_version}/neoforge-{loader_version}-installer.jar"
            )
        }
        other => {
            return Err(YuhinaError::new(
                ErrorCode::Unsupported,
                format!("{} does not use an installer.", other.label()),
            ));
        }
    };
    validate_url(&url)?;
    Ok(url)
}

/// The command that runs an installer inside the shared Minecraft directory.
/// `--installClient` installs into the current working directory.
pub fn installer_arguments(installer: &Path) -> Vec<String> {
    vec![
        "-jar".to_string(),
        installer.to_string_lossy().to_string(),
        "--installClient".to_string(),
    ]
}

/// Every file id an installer is known to produce, most likely first.
pub fn candidate_version_ids(
    loader: LoaderType,
    minecraft_version: &str,
    loader_version: &str,
) -> Vec<String> {
    let mut ids = match loader {
        LoaderType::Forge => vec![format!("{minecraft_version}-forge-{loader_version}")],
        LoaderType::Neoforge => vec![
            format!("neoforge-{loader_version}"),
            format!("{minecraft_version}-neoforge-{loader_version}"),
            // NeoForge 1.20.1 was published under Forge's legacy scheme.
            format!("{minecraft_version}-forge-{loader_version}"),
        ],
        _ => Vec::new(),
    };
    ids.dedup();
    ids
}

fn existing_version_ids(paths: &Paths) -> HashSet<String> {
    let Ok(entries) = fs::read_dir(paths.versions_dir()) else {
        return HashSet::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
}

/// Finds the version id the installer wrote. Candidate ids are checked first;
/// otherwise the newest version directory for this loader is used.
pub fn detect_installed_version(
    paths: &Paths,
    minecraft_version: &str,
    loader: LoaderType,
    loader_version: &str,
    before: &HashSet<String>,
) -> Result<String> {
    for id in candidate_version_ids(loader, minecraft_version, loader_version) {
        let Ok(path) = paths.version_json(&id) else {
            continue;
        };
        if path.is_file() {
            return Ok(id);
        }
    }

    let needle = loader_version.to_ascii_lowercase();
    let loader_name = match loader {
        LoaderType::Neoforge => "neoforge",
        _ => "forge",
    };
    let mut fallback = None;
    if let Ok(entries) = fs::read_dir(paths.versions_dir()) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                continue;
            }
            let Ok(id) = entry.file_name().into_string() else {
                continue;
            };
            if before.contains(&id) {
                continue;
            }
            let lower = id.to_ascii_lowercase();
            if !lower.contains(loader_name) {
                continue;
            }
            let Ok(json_path) = paths.version_json(&id) else {
                continue;
            };
            let Ok(Some(json)) =
                super::storage::read_json::<super::version::VersionJson>(&json_path)
            else {
                continue;
            };
            if json.inherits_from.as_deref() != Some(minecraft_version) {
                continue;
            }
            if lower.contains(&needle) {
                return Ok(id);
            }
            fallback.get_or_insert(id);
        }
    }

    fallback.ok_or_else(|| {
        YuhinaError::new(
            ErrorCode::InstallFailed,
            format!(
                "The {} installer finished but its version profile was not found.",
                loader.label()
            ),
        )
    })
}

/// Picks the Java major version the installer itself needs. Installers are
/// generally built for the same Java as their game version, with a safe floor
/// for the newer NeoForge series.
pub fn installer_java_major(
    paths: &Paths,
    minecraft_version: &str,
    loader: LoaderType,
    loader_version: &str,
) -> u32 {
    let game = super::install::read_installed(paths, minecraft_version)
        .ok()
        .flatten()
        .and_then(|version| version.java_version.and_then(|java| java.major_version))
        .unwrap_or_else(|| instances::required_java_major(minecraft_version));
    match loader {
        LoaderType::Neoforge => {
            let major = version_sort_key(loader_version)
                .first()
                .copied()
                .unwrap_or(21);
            if major >= 21 {
                game.max(21)
            } else {
                game.max(17)
            }
        }
        LoaderType::Forge => {
            let minor = minecraft_version
                .split('.')
                .nth(1)
                .and_then(|part| part.parse::<u32>().ok())
                .unwrap_or(0);
            if minor >= 17 {
                game.max(17)
            } else {
                game.max(8)
            }
        }
        _ => game,
    }
}

fn tail(text: &str, limit: usize) -> String {
    let count = text.chars().count();
    if count <= limit {
        return text.trim().to_string();
    }
    text.chars()
        .skip(count - limit)
        .collect::<String>()
        .trim()
        .to_string()
}

const MAX_OUTPUT_LINE: usize = 512;
const MAX_CAPTURED_OUTPUT: usize = 16 * 1024;

/// Streams one installer pipe line by line and keeps a tail for error text.
fn stream_output<R, G>(reader: R, on_output: &G, captured: &Mutex<String>)
where
    R: Read,
    G: Fn(&str) + Send + Sync,
{
    let mut reader = BufReader::new(reader);
    let mut buffer = String::new();
    loop {
        buffer.clear();
        match reader.read_line(&mut buffer) {
            Ok(0) => break,
            Ok(_) => {
                let line = buffer.trim();
                if line.is_empty() {
                    continue;
                }
                let line = truncate_line(line, MAX_OUTPUT_LINE);
                on_output(&line);
                let mut captured = captured.lock().unwrap();
                if captured.len() < MAX_CAPTURED_OUTPUT {
                    if !captured.is_empty() {
                        captured.push('\n');
                    }
                    captured.push_str(&line);
                }
            }
            Err(_) => break,
        }
    }
}

fn truncate_line(line: &str, limit: usize) -> String {
    if line.chars().count() <= limit {
        return line.to_string();
    }
    let mut value: String = line.chars().take(limit).collect();
    value.push('…');
    value
}

fn run_installer<G>(
    java: &Path,
    installer: &Path,
    minecraft_dir: &Path,
    on_output: &G,
) -> Result<()>
where
    G: Fn(&str) + Send + Sync,
{
    let mut command = Command::new(java);
    command
        .args(installer_arguments(installer))
        .current_dir(minecraft_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().map_err(|error| {
        YuhinaError::new(
            ErrorCode::InstallFailed,
            format!("The installer could not be started: {error}"),
        )
    })?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let captured = Arc::new(Mutex::new(String::new()));

    let status = std::thread::scope(|scope| {
        if let Some(stdout) = stdout {
            let captured = Arc::clone(&captured);
            scope.spawn(move || stream_output(stdout, on_output, &captured));
        }
        if let Some(stderr) = stderr {
            let captured = Arc::clone(&captured);
            scope.spawn(move || stream_output(stderr, on_output, &captured));
        }
        child.wait()
    })
    .map_err(|error| {
        YuhinaError::new(
            ErrorCode::InstallFailed,
            format!("The installer could not be waited on: {error}"),
        )
    })?;

    if !status.success() {
        let text = captured.lock().unwrap().clone();
        let details = tail(&text, 1500);
        return Err(YuhinaError::new(
            ErrorCode::InstallFailed,
            if details.is_empty() {
                format!("The installer exited with {status}.")
            } else {
                format!("The installer exited with {status}. {details}")
            },
        ));
    }
    Ok(())
}

/// Downloads and runs an installer, then returns the produced version id.
///
/// `on_progress` receives values in the range 0.0–1.0 plus a short status line
/// that the UI can display while the installer runs. Installer output is
/// captured for the failure message but not streamed; use
/// [`install_with_output`] to receive it line by line.
pub fn install<F>(
    paths: &Paths,
    java_settings: &JavaSettings,
    minecraft_version: &str,
    loader: LoaderType,
    loader_version: &str,
    cancel: Arc<AtomicBool>,
    on_progress: F,
) -> Result<String>
where
    F: Fn(f32, &str) + Send + Sync,
{
    install_with_output(
        paths,
        java_settings,
        minecraft_version,
        loader,
        loader_version,
        cancel,
        on_progress,
        |_: &str| {},
    )
}

/// Downloads and runs an installer, forwarding its stdout and stderr to
/// `on_output` as each line arrives.
#[allow(clippy::too_many_arguments)]
pub fn install_with_output<F, G>(
    paths: &Paths,
    java_settings: &JavaSettings,
    minecraft_version: &str,
    loader: LoaderType,
    loader_version: &str,
    cancel: Arc<AtomicBool>,
    on_progress: F,
    on_output: G,
) -> Result<String>
where
    F: Fn(f32, &str) + Send + Sync,
    G: Fn(&str) + Send + Sync,
{
    if !matches!(loader, LoaderType::Forge | LoaderType::Neoforge) {
        return Err(YuhinaError::new(
            ErrorCode::Unsupported,
            format!("{} does not use an installer.", loader.label()),
        ));
    }

    on_progress(0.02, "Preparing the loader installer");
    let major = installer_java_major(paths, minecraft_version, loader, loader_version);
    let runtime = java::ensure(paths, java_settings, major, |progress, message| {
        on_progress(0.04 + progress * 0.14, message);
    })?;

    let url = installer_url(loader, minecraft_version, loader_version)?;
    let file_name = url
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| YuhinaError::new(ErrorCode::Parse, "The installer URL has no file name."))?;
    let relative = super::paths::normalize_relative(file_name)?;
    let installer_path = Paths::resolve_inside(&paths.downloads_dir(), &relative)?;
    fs::create_dir_all(paths.downloads_dir())?;

    on_progress(0.2, "Downloading the loader installer");
    let task = DownloadTask {
        destination: installer_path.clone(),
        urls: vec![url],
        sha1: None,
        sha512: None,
        size: None,
        label: format!("{} {loader_version} installer", loader.label()),
    };
    download::download_all(vec![task], 1, Arc::clone(&cancel), |progress| {
        let ratio = if progress.bytes_total > 0 {
            (progress.bytes_received as f32 / progress.bytes_total as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        on_progress(0.2 + ratio * 0.45, "Downloading the loader installer");
    })?;

    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(YuhinaError::new(
            ErrorCode::InstallFailed,
            "The installation was cancelled.",
        ));
    }

    fs::create_dir_all(paths.minecraft_dir())?;
    let before = existing_version_ids(paths);
    on_progress(0.68, "Running the loader installer");
    run_installer(
        Path::new(&runtime.path),
        &installer_path,
        &paths.minecraft_dir(),
        &on_output,
    )?;

    on_progress(0.94, "Checking the installed loader");
    let id = detect_installed_version(paths, minecraft_version, loader, loader_version, &before)?;
    // Fail early if the profile cannot be resolved through its parent chain;
    // this is the same path the launcher uses at launch time.
    let _ = super::install::resolve_local(paths, &id)?;
    on_progress(1.0, "Loader installed");
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forge_promotions_prefer_recommended() {
        let body = r#"{
            "homepage": "https://files.minecraftforge.net/",
            "promos": {
                "1.20.1-latest": "47.4.10",
                "1.20.1-recommended": "47.4.0",
                "1.19.2-latest": "43.4.0"
            }
        }"#;
        let versions = parse_forge_promotions(body, "1.20.1").unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version, "47.4.0");
        assert!(versions[0].recommended);
        assert_eq!(versions[1].version, "47.4.10");
        assert!(!versions[1].recommended);
    }

    #[test]
    fn forge_promotions_fall_back_to_latest() {
        let body = r#"{"promos": {"1.19.2-latest": "43.4.0"}}"#;
        let versions = parse_forge_promotions(body, "1.19.2").unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, "43.4.0");
        assert!(versions[0].recommended);
        assert!(parse_forge_promotions(body, "1.21.1").is_err());
    }

    #[test]
    fn neoforge_metadata_is_parsed_and_filtered() {
        let xml = r#"<?xml version="1.0"?>
            <metadata>
              <groupId>net.neoforged</groupId>
              <versioning>
                <latest>21.4.100</latest>
                <versions>
                  <version>21.1.209</version>
                  <version>21.1.210-beta</version>
                  <version>21.4.100</version>
                  <version>20.6.119</version>
                  <version>21.0.167</version>
                  <version>21.10.1</version>
                </versions>
              </versioning>
            </metadata>"#;
        let versions = parse_neoforge_versions(xml).unwrap();
        assert_eq!(versions.len(), 6);

        let compatible = compatible_neoforge_versions(&versions, "1.21.1");
        assert_eq!(compatible, vec!["21.1.209", "21.1.210-beta"]);

        let compatible = compatible_neoforge_versions(&versions, "1.21.4");
        assert_eq!(compatible, vec!["21.4.100"]);

        let compatible = compatible_neoforge_versions(&versions, "1.21");
        assert_eq!(compatible, vec!["21.0.167"]);
    }

    #[test]
    fn neoforge_prefix_matches_the_minecraft_series() {
        assert_eq!(neoforge_prefix("1.21").as_deref(), Some("21.0"));
        assert_eq!(neoforge_prefix("1.21.1").as_deref(), Some("21.1"));
        assert_eq!(neoforge_prefix("1.20.6").as_deref(), Some("20.6"));
        assert_eq!(neoforge_prefix("2.0").as_deref(), None);
    }

    #[test]
    fn installer_urls_point_at_official_maven() {
        assert_eq!(
            installer_url(LoaderType::Forge, "1.20.1", "47.3.0").unwrap(),
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.3.0/forge-1.20.1-47.3.0-installer.jar"
        );
        assert_eq!(
            installer_url(LoaderType::Neoforge, "1.21.1", "21.1.209").unwrap(),
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.209/neoforge-21.1.209-installer.jar"
        );
        assert!(installer_url(LoaderType::Fabric, "1.21.1", "0.16.9").is_err());
    }

    #[test]
    fn installer_arguments_are_headless() {
        let args = installer_arguments(Path::new("installer.jar"));
        assert_eq!(args, vec!["-jar", "installer.jar", "--installClient"]);
    }

    #[test]
    fn candidate_ids_cover_both_schemes() {
        assert_eq!(
            candidate_version_ids(LoaderType::Forge, "1.20.1", "47.3.0"),
            vec!["1.20.1-forge-47.3.0"]
        );
        assert_eq!(
            candidate_version_ids(LoaderType::Neoforge, "1.21.1", "21.1.209"),
            vec![
                "neoforge-21.1.209",
                "1.21.1-neoforge-21.1.209",
                "1.21.1-forge-21.1.209"
            ]
        );
    }

    #[test]
    fn installer_output_is_streamed_and_captured() {
        let captured = Mutex::new(String::new());
        let lines = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&lines);
        stream_output(
            &b"first line\r\nsecond line\n\nthird line\n"[..],
            &|line: &str| sink.lock().unwrap().push(line.to_string()),
            &captured,
        );
        assert_eq!(
            *lines.lock().unwrap(),
            vec!["first line", "second line", "third line"]
        );
        assert_eq!(
            captured.lock().unwrap().as_str(),
            "first line\nsecond line\nthird line"
        );

        let long = "x".repeat(MAX_OUTPUT_LINE + 10);
        let truncated = truncate_line(&long, MAX_OUTPUT_LINE);
        assert_eq!(truncated.chars().count(), MAX_OUTPUT_LINE + 1);
        assert!(truncated.ends_with('…'));
    }

    #[test]
    fn detects_the_version_written_by_the_installer() {
        let root = std::env::temp_dir().join(format!("yuhina-loader-{}", uuid::Uuid::new_v4()));
        let paths = Paths::at(root.clone());
        let id = "neoforge-21.1.209";
        let dir = paths.version_dir(id).unwrap();
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(format!("{id}.json")),
            r#"{"id":"neoforge-21.1.209","inheritsFrom":"1.21.1"}"#,
        )
        .unwrap();

        let before = HashSet::new();
        let detected =
            detect_installed_version(&paths, "1.21.1", LoaderType::Neoforge, "21.1.209", &before)
                .unwrap();
        assert_eq!(detected, id);

        let stale = HashSet::from([id.to_string()]);
        assert!(
            detect_installed_version(&paths, "1.21.1", LoaderType::Neoforge, "21.2.0", &stale)
                .is_err()
        );

        let _ = fs::remove_dir_all(&root);
    }
}
