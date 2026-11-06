use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use serde::Deserialize;

use super::download::{DownloadProgress, DownloadTask, download_all};
use super::http;
use super::paths::{Paths, normalize_relative};
use super::settings::DownloadSource;
use super::storage::{read_json, write_json, write_text};
use super::version::{
    ArgumentEntry, RuleContext, VersionJson, VersionManifest, asset_object_path, candidate_urls,
    manifest_url, merge_version_json, resolve_libraries,
};
use super::{ErrorCode, Result, YuhinaError};

pub const NATIVES_MARKER: &str = ".yuhina-natives.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallPhase {
    Metadata,
    Client,
    Libraries,
    Natives,
    Assets,
    Modpack,
    Overrides,
    Done,
    Error,
}

impl InstallPhase {
    pub fn label(self) -> &'static str {
        match self {
            InstallPhase::Metadata => "metadata",
            InstallPhase::Client => "client",
            InstallPhase::Libraries => "libraries",
            InstallPhase::Natives => "natives",
            InstallPhase::Assets => "assets",
            InstallPhase::Modpack => "modpack",
            InstallPhase::Overrides => "overrides",
            InstallPhase::Done => "done",
            InstallPhase::Error => "error",
        }
    }

    fn range(self) -> (f32, f32) {
        match self {
            InstallPhase::Metadata => (0.0, 3.0),
            InstallPhase::Client => (3.0, 12.0),
            InstallPhase::Libraries => (12.0, 32.0),
            InstallPhase::Natives => (32.0, 40.0),
            InstallPhase::Assets => (40.0, 100.0),
            InstallPhase::Modpack => (40.0, 92.0),
            InstallPhase::Overrides => (92.0, 99.0),
            InstallPhase::Done | InstallPhase::Error => (100.0, 100.0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct InstallProgress {
    pub phase: InstallPhase,
    pub percent: f32,
    pub files_done: usize,
    pub files_total: usize,
    pub bytes_received: u64,
    pub bytes_total: u64,
    pub message: Option<String>,
}

impl InstallProgress {
    pub fn new(phase: InstallPhase, percent: f32) -> Self {
        Self {
            phase,
            percent,
            files_done: 0,
            files_total: 0,
            bytes_received: 0,
            bytes_total: 0,
            message: None,
        }
    }
}

fn phase_progress(phase: InstallPhase, progress: DownloadProgress) -> InstallProgress {
    let (start, end) = phase.range();
    let ratio = if progress.bytes_total > 0 {
        (progress.bytes_received as f32 / progress.bytes_total as f32).clamp(0.0, 1.0)
    } else if progress.files_total > 0 {
        progress.files_done as f32 / progress.files_total as f32
    } else {
        0.0
    };
    let mut result = InstallProgress::new(phase, start + (end - start) * ratio);
    result.files_done = progress.files_done;
    result.files_total = progress.files_total;
    result.bytes_received = progress.bytes_received;
    result.bytes_total = progress.bytes_total;
    result
}

fn manifest_cache(paths: &Paths) -> PathBuf {
    paths.minecraft_dir().join("version_manifest.json")
}

pub fn list_versions(
    paths: &Paths,
    source: DownloadSource,
    force: bool,
) -> Result<VersionManifest> {
    let cache = manifest_cache(paths);
    if !force && let Some(manifest) = read_json::<VersionManifest>(&cache)? {
        return Ok(manifest);
    }

    let mut urls = vec![manifest_url(source).to_string()];
    if source == DownloadSource::Auto {
        urls.push(super::version::OFFICIAL_MANIFEST.to_string());
        urls.dedup();
    }

    let mut last_error = None;
    for url in urls {
        match http::get_json::<VersionManifest>(&url) {
            Ok(manifest) => {
                let _ = write_json(&cache, &manifest);
                return Ok(manifest);
            }
            Err(error) => last_error = Some(error),
        }
        if source == DownloadSource::Bmcl {
            break;
        }
    }

    // A stale cache is better than no version list at all.
    if let Some(manifest) = read_json::<VersionManifest>(&cache)? {
        return Ok(manifest);
    }
    Err(last_error.unwrap_or_else(|| {
        YuhinaError::new(
            ErrorCode::Network,
            "The Minecraft version list is unavailable.",
        )
    }))
}

/// Resolves a version, merging `inheritsFrom` parents, and writes the merged
/// JSON to disk under `versions/<id>/<id>.json`.
pub fn resolve_version(
    paths: &Paths,
    version_id: &str,
    source: DownloadSource,
) -> Result<VersionJson> {
    resolve_version_inner(paths, version_id, source, 0)
}

fn resolve_version_inner(
    paths: &Paths,
    version_id: &str,
    source: DownloadSource,
    depth: usize,
) -> Result<VersionJson> {
    super::paths::assert_safe_id(version_id)?;
    if depth > 8 {
        return Err(YuhinaError::new(
            ErrorCode::VersionNotFound,
            "The version inheritance chain is too deep.",
        ));
    }

    let manifest = list_versions(paths, source, false)?;
    let entry = manifest
        .versions
        .iter()
        .find(|entry| entry.id == version_id)
        .ok_or_else(|| {
            YuhinaError::new(
                ErrorCode::VersionNotFound,
                format!("Minecraft {version_id} is not in the version list."),
            )
        })?;

    let urls = candidate_urls(&entry.url, Some(version_id), "json", source);
    let mut last_error = None;
    let mut version: Option<VersionJson> = None;
    for url in urls {
        match http::get_json::<VersionJson>(&url) {
            Ok(parsed) => {
                version = Some(parsed);
                break;
            }
            Err(error) => last_error = Some(error),
        }
    }
    let mut version = version.ok_or_else(|| {
        last_error.unwrap_or_else(|| {
            YuhinaError::new(
                ErrorCode::VersionNotFound,
                format!("Could not load the metadata for {version_id}."),
            )
        })
    })?;

    if let Some(parent_id) = version.inherits_from.clone() {
        let parent = resolve_version_inner(paths, &parent_id, source, depth + 1)?;
        version = merge_version_json(&parent, &version);
        version.id = version_id.to_string();
    }

    let json_path = paths.version_json(version_id)?;
    if let Some(parent) = json_path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_json(&json_path, &version)?;
    Ok(version)
}

pub fn read_installed(paths: &Paths, version_id: &str) -> Result<Option<VersionJson>> {
    let path = paths.version_json(version_id)?;
    read_json(&path)
}

pub fn resolve_local(paths: &Paths, version_id: &str) -> Result<(VersionJson, String)> {
    let raw = read_installed(paths, version_id)?.ok_or_else(|| {
        YuhinaError::new(
            ErrorCode::VersionNotFound,
            format!("The launcher version {version_id} is not installed."),
        )
    })?;
    if let Some(parent_id) = raw.inherits_from.clone() {
        let (parent, root) = resolve_local(paths, &parent_id)?;
        let mut merged = merge_version_json(&parent, &raw);
        merged.id = version_id.to_string();
        Ok((merged, root))
    } else {
        Ok((raw, version_id.to_string()))
    }
}

pub fn is_installed(paths: &Paths, version_id: &str) -> bool {
    let Ok((version, root)) = resolve_local(paths, version_id) else {
        return false;
    };
    let Ok(client) = paths.client_jar(&root) else {
        return false;
    };
    if !client.is_file() {
        return false;
    }
    if version_id != root
        && let Ok(own) = paths.client_jar(version_id)
    {
        let _ = own;
    }
    let asset_id = version
        .asset_index
        .as_ref()
        .map(|index| index.id.clone())
        .or_else(|| version.assets.clone());
    if let Some(asset_id) = asset_id
        && !paths
            .indexes_dir()
            .join(format!("{asset_id}.json"))
            .is_file()
    {
        return false;
    }
    paths
        .natives_dir(&root)
        .map(|dir| dir.join(NATIVES_MARKER).is_file())
        .unwrap_or(false)
}

pub fn install<F>(
    paths: &Paths,
    version_id: &str,
    source: DownloadSource,
    repair: bool,
    cancel: Arc<AtomicBool>,
    on_progress: F,
) -> Result<()>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    on_progress(InstallProgress::new(InstallPhase::Metadata, 0.0));
    let (version, root) = match resolve_local(paths, version_id) {
        Ok((version, root)) => (version, root),
        Err(_) => {
            let version = resolve_version(paths, version_id, source)?;
            (version, version_id.to_string())
        }
    };
    on_progress(InstallProgress {
        percent: 3.0,
        files_done: 1,
        files_total: 1,
        ..InstallProgress::new(InstallPhase::Metadata, 3.0)
    });

    let context = RuleContext::current();
    let selected = resolve_libraries(version.libraries.as_deref().unwrap_or(&[]), &context);

    install_client(
        paths,
        &root,
        &version,
        source,
        repair,
        &cancel,
        |progress| {
            on_progress(phase_progress(InstallPhase::Client, progress));
        },
    )?;

    let library_tasks = build_library_tasks(paths, &selected.classpath, source, repair)?;
    download_all(library_tasks, 8, Arc::clone(&cancel), |progress| {
        on_progress(phase_progress(InstallPhase::Libraries, progress));
    })?;

    let native_tasks = build_library_tasks(paths, &selected.natives, source, repair)?;
    download_all(native_tasks, 8, Arc::clone(&cancel), |progress| {
        on_progress(phase_progress(InstallPhase::Natives, progress));
    })?;
    install_natives(paths, &root, &selected.natives, repair)?;
    on_progress(InstallProgress::new(InstallPhase::Natives, 40.0));

    install_assets(paths, &version, source, repair, &cancel, |progress| {
        on_progress(phase_progress(InstallPhase::Assets, progress));
    })?;

    on_progress(InstallProgress::new(InstallPhase::Done, 100.0));
    Ok(())
}

/// Writes a Fabric or Quilt loader profile JSON that inherits from a vanilla
/// version. The caller installs the parent first, then this profile.
pub fn install_loader_profile(
    paths: &Paths,
    minecraft_version: &str,
    loader: super::instances::LoaderType,
    loader_version: Option<&str>,
) -> Result<String> {
    use super::instances::LoaderType;
    let base = match loader {
        LoaderType::Fabric => "https://meta.fabricmc.net/v2/versions/loader",
        LoaderType::Quilt => "https://meta.quiltmc.org/v3/versions/loader",
        _ => {
            return Err(YuhinaError::new(
                ErrorCode::Unsupported,
                "This loader needs its own installer and is not supported yet.",
            ));
        }
    };

    let version = match loader_version {
        Some(version) => version.to_string(),
        None => {
            let versions: Vec<serde_json::Value> =
                http::get_json(&format!("{base}/{minecraft_version}"))?;
            versions
                .iter()
                .find(|entry| entry["loader"]["stable"].as_bool().unwrap_or(true))
                .or_else(|| versions.first())
                .and_then(|entry| entry["loader"]["version"].as_str())
                .map(str::to_string)
                .ok_or_else(|| {
                    YuhinaError::new(
                        ErrorCode::NotFound,
                        format!(
                            "No {} loader is available for {minecraft_version}.",
                            loader.label()
                        ),
                    )
                })?
        }
    };

    let url = format!("{base}/{minecraft_version}/{version}/profile/json");
    let response = http::request("GET", &url, &[("Accept", "application/json")], None)?;
    if !response.is_success() {
        return Err(YuhinaError::new(
            ErrorCode::Network,
            format!(
                "The {} loader metadata could not be loaded.",
                loader.label()
            ),
        ));
    }
    let mut profile: VersionJson = serde_json::from_str(&response.body)?;
    let id = profile.id.clone();
    if id.is_empty() {
        return Err(YuhinaError::new(
            ErrorCode::Parse,
            "The loader profile has no id.",
        ));
    }
    // The canonical id is deterministic for both loaders.
    let id = match loader {
        LoaderType::Fabric => format!("fabric-loader-{version}-{minecraft_version}"),
        LoaderType::Quilt => format!("quilt-loader-{version}-{minecraft_version}"),
        _ => id,
    };
    profile.id = id.clone();
    super::paths::assert_safe_id(&id)?;
    let path = paths.version_json(&id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_json(&path, &profile)?;
    Ok(id)
}

fn install_client<F>(
    paths: &Paths,
    version_id: &str,
    version: &VersionJson,
    source: DownloadSource,
    repair: bool,
    cancel: &Arc<AtomicBool>,
    on_progress: F,
) -> Result<()>
where
    F: Fn(DownloadProgress) + Send + Sync,
{
    let Some(client) = version
        .downloads
        .as_ref()
        .and_then(|downloads| downloads.client.as_ref())
    else {
        return Ok(());
    };
    let destination = paths.client_jar(version_id)?;
    if !repair
        && destination.is_file()
        && let Ok(meta) = fs::metadata(&destination)
        && meta.len() > 0
    {
        on_progress(DownloadProgress {
            files_done: 1,
            files_total: 1,
            bytes_received: meta.len(),
            bytes_total: meta.len(),
        });
        return Ok(());
    }
    let task = DownloadTask {
        destination,
        urls: candidate_urls(&client.url, Some(version_id), "client", source),
        sha1: client.sha1.clone(),
        sha512: None,
        size: client.size,
        label: format!("{version_id} client"),
    };
    download_all(vec![task], 1, Arc::clone(cancel), on_progress)
}

fn build_library_tasks(
    paths: &Paths,
    libraries: &[super::version::ResolvedLibrary],
    source: DownloadSource,
    repair: bool,
) -> Result<Vec<DownloadTask>> {
    let mut tasks = Vec::new();
    for library in libraries {
        let relative = normalize_relative(&library.relative_path)?;
        let destination = Paths::resolve_inside(&paths.libraries_dir(), &relative)?;
        if !repair
            && let Ok(meta) = fs::metadata(&destination)
            && let Some(size) = library.size
            && size > 0
            && meta.len() == size
        {
            continue;
        }
        tasks.push(DownloadTask {
            destination,
            urls: candidate_urls(&library.url, None, "library", source),
            sha1: library.sha1.clone(),
            sha512: None,
            size: library.size,
            label: library.name.clone(),
        });
    }
    Ok(tasks)
}

fn install_natives(
    paths: &Paths,
    version_id: &str,
    natives: &[super::version::ResolvedLibrary],
    repair: bool,
) -> Result<()> {
    let natives_dir = paths.natives_dir(version_id)?;
    let marker = natives_dir.join(NATIVES_MARKER);
    if !repair && marker.is_file() {
        return Ok(());
    }
    if natives_dir.exists() {
        fs::remove_dir_all(&natives_dir)?;
    }
    fs::create_dir_all(&natives_dir)?;

    let mut entries = Vec::new();
    for library in natives {
        let relative = normalize_relative(&library.relative_path)?;
        let jar = Paths::resolve_inside(&paths.libraries_dir(), &relative)?;
        if !jar.is_file() {
            continue;
        }
        extract_natives_jar(&jar, &natives_dir, &library.extract_exclude, &mut entries)?;
    }
    write_text(
        &marker,
        &serde_json::json!({
            "version": 1,
            "extractedAt": chrono::Utc::now().to_rfc3339(),
            "entries": entries,
        })
        .to_string(),
    )?;
    Ok(())
}

fn extract_natives_jar(
    jar: &Path,
    destination: &Path,
    exclude: &[String],
    entries: &mut Vec<String>,
) -> Result<()> {
    let file = File::open(jar)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        if name.starts_with("META-INF/") {
            continue;
        }
        if exclude
            .iter()
            .any(|pattern| name == *pattern || name.starts_with(&format!("{pattern}/")))
        {
            continue;
        }
        let safe = match normalize_relative(&name) {
            Ok(safe) => safe,
            Err(_) => continue,
        };
        let target = Paths::resolve_inside(destination, &safe)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = File::create(&target)?;
        io::copy(&mut entry, &mut output)?;
        if !entries.contains(&safe) {
            entries.push(safe);
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct AssetIndexFile {
    #[serde(default)]
    objects: HashMap<String, AssetObject>,
}

#[derive(Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

fn install_assets<F>(
    paths: &Paths,
    version: &VersionJson,
    source: DownloadSource,
    repair: bool,
    cancel: &Arc<AtomicBool>,
    on_progress: F,
) -> Result<()>
where
    F: Fn(DownloadProgress) + Send + Sync,
{
    let Some(index) = version.asset_index.as_ref() else {
        return Ok(());
    };
    if index.url.is_empty() {
        return Ok(());
    }
    let index_id = if index.id.is_empty() {
        version
            .assets
            .clone()
            .unwrap_or_else(|| "legacy".to_string())
    } else {
        index.id.clone()
    };
    fs::create_dir_all(paths.indexes_dir())?;
    let index_path = paths.indexes_dir().join(format!("{index_id}.json"));
    if !index_path.is_file() || repair {
        let task = DownloadTask {
            destination: index_path.clone(),
            urls: candidate_urls(&index.url, None, "asset-index", source),
            sha1: index.sha1.clone(),
            sha512: None,
            size: index.size,
            label: format!("{index_id} asset index"),
        };
        download_all(vec![task], 1, Arc::clone(cancel), &on_progress)?;
    }

    let parsed: AssetIndexFile = match read_json(&index_path)? {
        Some(value) => value,
        None => return Ok(()),
    };
    if parsed.objects.is_empty() {
        return Ok(());
    }

    let mut tasks = Vec::new();
    for object in parsed.objects.values() {
        let relative = asset_object_path(&object.hash)?;
        let destination = Paths::resolve_inside(&paths.assets_dir(), &relative)?;
        if !repair
            && let Ok(meta) = fs::metadata(&destination)
            && meta.len() == object.size
        {
            continue;
        }
        let prefix = &object.hash[..2];
        tasks.push(DownloadTask {
            destination,
            urls: candidate_urls(
                &format!(
                    "https://resources.download.minecraft.net/{prefix}/{}",
                    object.hash
                ),
                None,
                "asset",
                source,
            ),
            sha1: Some(object.hash.clone()),
            sha512: None,
            size: Some(object.size),
            label: "asset".to_string(),
        });
    }
    download_all(tasks, 12, Arc::clone(cancel), on_progress)
}

/// Reads an already-installed version's argument list for the launcher.
pub fn version_arguments(
    version: &VersionJson,
    context: &RuleContext,
) -> (Vec<String>, Vec<String>) {
    let arguments = version.arguments.clone().unwrap_or_default();
    let jvm = super::version::resolve_arguments(&arguments.jvm, context);
    let game = if arguments.game.is_empty() {
        split_args(version.minecraft_arguments.as_deref().unwrap_or(""))
    } else {
        super::version::resolve_arguments(&arguments.game, context)
    };
    (jvm, game)
}

pub fn split_args(value: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaping = false;
    for ch in value.chars() {
        if escaping {
            current.push(ch);
            escaping = false;
            continue;
        }
        if ch == '\\' && quote != Some('\'') {
            escaping = true;
            continue;
        }
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        if ch.is_whitespace() {
            if !current.is_empty() {
                args.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(ch);
    }
    if escaping {
        current.push('\\');
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

/// Parses the version's `arguments.jvm` entries into raw strings (used by the
/// launcher's placeholder substitution).
pub fn jvm_argument_entries(version: &VersionJson, context: &RuleContext) -> Vec<String> {
    let arguments = version.arguments.clone().unwrap_or_default();
    let mut values = Vec::new();
    for entry in &arguments.jvm {
        match entry {
            ArgumentEntry::Plain(value) => values.push(value.clone()),
            ArgumentEntry::Conditional { rules, value } => {
                if super::version::rules_allow(rules, context) {
                    values.push(match value {
                        super::version::ArgumentValue::One(value) => value.clone(),
                        super::version::ArgumentValue::Many(values) => values.join(" "),
                    });
                }
            }
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_like_arguments_split() {
        assert_eq!(
            split_args(r#"-Xmx2G -Dfoo="a b""#),
            vec!["-Xmx2G", "-Dfoo=a b"]
        );
        assert_eq!(split_args("--a 'b c'"), vec!["--a", "b c"]);
    }
}
