//! Modrinth `.mrpack` modpack import.
//!
//! A pack is a zip with a `modrinth.index.json` manifest, optional
//! `overrides/`, `client-overrides/` and `server-overrides/` trees. The
//! launcher always installs the client side: `overrides/` is applied first
//! and `client-overrides/` last, so a client-specific file wins.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use serde::Deserialize;

use super::download::{DownloadProgress, DownloadTask, download_all};
use super::install::{self, InstallPhase, InstallProgress};
use super::instances::{self, InstanceRecord, Loader, LoaderType, MemoryMode, NewInstance};
use super::modrinth;
use super::paths::{Paths, assert_safe_id, normalize_relative};
use super::settings::{DownloadSource, JavaSettings};
use super::{ErrorCode, Result, YuhinaError};

pub const INDEX_NAME: &str = "modrinth.index.json";
const OVERRIDES: &str = "overrides";
const CLIENT_OVERRIDES: &str = "client-overrides";
const SUPPORTED_FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct ModpackHashes {
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub sha512: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct ModpackEnv {
    /// `required`, `optional` or `unsupported`.
    #[serde(default)]
    pub client: Option<String>,
    /// `required`, `optional` or `unsupported`.
    #[serde(default)]
    pub server: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModpackFile {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub hashes: ModpackHashes,
    #[serde(default)]
    pub env: Option<ModpackEnv>,
    #[serde(default)]
    pub downloads: Vec<String>,
    #[serde(default)]
    pub file_size: u64,
}

impl ModpackFile {
    pub fn applies_to_client(&self) -> bool {
        !matches!(
            self.env.as_ref().and_then(|env| env.client.as_deref()),
            Some("unsupported")
        )
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModpackIndex {
    pub format_version: u32,
    #[serde(default)]
    pub game: String,
    pub version_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub files: Vec<ModpackFile>,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

impl ModpackIndex {
    /// Rejects unsafe paths and non-HTTP download URLs before anything reaches
    /// the filesystem.
    pub fn validate(&self) -> Result<()> {
        if self.format_version != SUPPORTED_FORMAT_VERSION {
            return Err(YuhinaError::new(
                ErrorCode::Unsupported,
                format!(
                    "This modpack uses format version {}; only version 1 is supported.",
                    self.format_version
                ),
            ));
        }
        for file in &self.files {
            normalize_relative(&file.path)?;
            if !file.applies_to_client() {
                continue;
            }
            if file.downloads.is_empty() {
                return Err(YuhinaError::new(
                    ErrorCode::InvalidInput,
                    "A modpack file has no download URL.",
                ));
            }
            if file.hashes.sha1.is_none() && file.hashes.sha512.is_none() {
                return Err(YuhinaError::new(
                    ErrorCode::InvalidInput,
                    "A modpack file has no checksum.",
                ));
            }
            for url in &file.downloads {
                validate_http_url(url)?;
            }
        }
        Ok(())
    }

    /// The loader and Minecraft version this pack targets.
    pub fn loader(&self) -> Result<LoaderSelection> {
        select_loader(&self.dependencies)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoaderSelection {
    pub minecraft: String,
    pub loader: Loader,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedFile {
    pub path: String,
    pub urls: Vec<String>,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
    pub size: u64,
}

/// Parses and validates `modrinth.index.json` bytes.
pub fn parse_index(bytes: &[u8]) -> Result<ModpackIndex> {
    let index: ModpackIndex = serde_json::from_slice(bytes)?;
    index.validate()?;
    Ok(index)
}

/// Reads `modrinth.index.json` from a local `.mrpack` zip.
pub fn read_index(zip_path: &Path) -> Result<ModpackIndex> {
    let file = File::open(zip_path).map_err(YuhinaError::io("open the modpack"))?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut entry = archive.by_name(INDEX_NAME).map_err(|error| {
        YuhinaError::new(
            ErrorCode::InvalidInput,
            format!("This zip has no {INDEX_NAME}: {error}"),
        )
    })?;
    let mut bytes = Vec::new();
    io::Read::read_to_end(&mut entry, &mut bytes)?;
    parse_index(&bytes)
}

/// Maps Modrinth dependency keys to a launcher loader.
pub fn select_loader(dependencies: &HashMap<String, String>) -> Result<LoaderSelection> {
    let minecraft = dependencies
        .get("minecraft")
        .map(String::as_str)
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .ok_or_else(|| {
            YuhinaError::new(
                ErrorCode::InvalidInput,
                "This modpack does not declare a Minecraft version.",
            )
        })?
        .to_string();
    assert_safe_id(&minecraft)?;

    let candidates = [
        ("fabric-loader", LoaderType::Fabric),
        ("quilt-loader", LoaderType::Quilt),
        ("forge", LoaderType::Forge),
        ("neoforge", LoaderType::Neoforge),
    ];
    let mut selected: Option<(LoaderType, Option<String>)> = None;
    for (key, kind) in candidates {
        let Some(version) = dependencies.get(key) else {
            continue;
        };
        if selected.is_some() {
            return Err(YuhinaError::new(
                ErrorCode::InvalidInput,
                "This modpack declares more than one loader.",
            ));
        }
        let version = version.trim();
        selected = Some((kind, (!version.is_empty()).then(|| version.to_string())));
    }

    let (kind, version) = selected.unwrap_or((LoaderType::Vanilla, None));
    // Installer-driven loaders use the declared version as part of a Maven
    // artefact name and as a version-directory id, so validate it before the
    // vanilla install starts. A missing version is resolved by the installer
    // pipeline from the loader's promotion list.
    if matches!(kind, LoaderType::Forge | LoaderType::Neoforge)
        && let Some(version) = version.as_deref()
    {
        assert_safe_id(version)?;
    }

    Ok(LoaderSelection {
        minecraft,
        loader: Loader { kind, version },
    })
}

pub fn plan_files(index: &ModpackIndex) -> Result<Vec<PlannedFile>> {
    let mut files = Vec::new();
    for file in &index.files {
        if !file.applies_to_client() {
            continue;
        }
        let path = normalize_relative(&file.path)?;
        if file.downloads.is_empty() {
            return Err(YuhinaError::new(
                ErrorCode::InvalidInput,
                "A modpack file has no download URL.",
            ));
        }
        if file.hashes.sha1.is_none() && file.hashes.sha512.is_none() {
            return Err(YuhinaError::new(
                ErrorCode::InvalidInput,
                "A modpack file has no checksum.",
            ));
        }
        for url in &file.downloads {
            validate_http_url(url)?;
        }
        files.push(PlannedFile {
            path,
            urls: file.downloads.clone(),
            sha1: file.hashes.sha1.clone(),
            sha512: file.hashes.sha512.clone(),
            size: file.file_size,
        });
    }
    Ok(files)
}

fn validate_http_url(value: &str) -> Result<()> {
    let url = url::Url::parse(value).map_err(|_| {
        YuhinaError::new(
            ErrorCode::InvalidInput,
            "The modpack listed an invalid download URL.",
        )
    })?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(YuhinaError::new(
            ErrorCode::InvalidInput,
            "The modpack listed an unsafe download URL.",
        ));
    }
    Ok(())
}

/// The game-relative destination of a zip entry under `overrides/` or
/// `client-overrides/`, or `None` for every other entry.
fn override_relative(name: &str) -> Result<Option<String>> {
    let name = name.replace('\\', "/");
    for prefix in [OVERRIDES, CLIENT_OVERRIDES] {
        if let Some(rest) = name.strip_prefix(&format!("{prefix}/")) {
            if rest.is_empty() {
                return Ok(None);
            }
            return Ok(Some(normalize_relative(rest)?));
        }
    }
    Ok(None)
}

/// Extracts `overrides/` then `client-overrides/` into the instance game
/// directory. `server-overrides/` is intentionally ignored.
pub fn extract_overrides(zip_path: &Path, game_dir: &Path) -> Result<usize> {
    let file = File::open(zip_path).map_err(YuhinaError::io("open the modpack"))?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut written = 0;
    for prefix in [OVERRIDES, CLIENT_OVERRIDES] {
        written += extract_prefix(&mut archive, prefix, game_dir)?;
    }
    Ok(written)
}

fn extract_prefix(
    archive: &mut zip::ZipArchive<File>,
    prefix: &str,
    game_dir: &Path,
) -> Result<usize> {
    let mut written = 0;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let Some(relative) = override_relative(entry.name())? else {
            continue;
        };
        if !entry
            .name()
            .replace('\\', "/")
            .starts_with(&format!("{prefix}/"))
        {
            continue;
        }
        let target = Paths::resolve_inside(game_dir, &relative)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = File::create(&target)?;
        io::copy(&mut entry, &mut output)?;
        written += 1;
    }
    Ok(written)
}

/// Installs a local `.mrpack` into a brand new instance.
///
/// Uses the default Java settings for installer-driven loaders. Call
/// [`import_local_with_java`] to pass the user's Java configuration.
pub fn import_local<F>(
    paths: &Paths,
    zip_path: &Path,
    source: DownloadSource,
    cancel: Arc<AtomicBool>,
    on_progress: F,
) -> Result<InstanceRecord>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    import_local_with_java(
        paths,
        zip_path,
        source,
        &JavaSettings::default(),
        cancel,
        on_progress,
    )
}

/// Installs a local `.mrpack` into a brand new instance, using `java_settings`
/// when the pack needs a Forge or NeoForge installer.
pub fn import_local_with_java<F>(
    paths: &Paths,
    zip_path: &Path,
    source: DownloadSource,
    java_settings: &JavaSettings,
    cancel: Arc<AtomicBool>,
    on_progress: F,
) -> Result<InstanceRecord>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    on_progress(InstallProgress::new(InstallPhase::Metadata, 0.0));
    let index = read_index(zip_path)?;
    let selection = index.loader()?;
    let files = plan_files(&index)?;

    let name = if index.name.trim().is_empty() {
        format!("{} modpack", selection.minecraft)
    } else {
        index.name.trim().to_string()
    };
    let record = instances::create(
        paths,
        NewInstance {
            name,
            version_id: selection.minecraft.clone(),
            loader: selection.loader.clone(),
            memory_mb: instances::recommended_memory_mb(files.len()),
            memory_mode: MemoryMode::Auto,
            icon: None,
        },
    )?;

    let result = import_into(
        paths,
        &record,
        &selection,
        &files,
        zip_path,
        source,
        java_settings,
        cancel,
        &on_progress,
    );
    if let Err(error) = result {
        // Do not leave a half-installed instance behind.
        let _ = instances::delete(paths, &record.id, false);
        return Err(error);
    }

    on_progress(InstallProgress::new(InstallPhase::Done, 100.0));
    instances::load(paths, &record.id)
}

#[allow(clippy::too_many_arguments)]
fn import_into<F>(
    paths: &Paths,
    record: &InstanceRecord,
    selection: &LoaderSelection,
    files: &[PlannedFile],
    zip_path: &Path,
    source: DownloadSource,
    java_settings: &JavaSettings,
    cancel: Arc<AtomicBool>,
    on_progress: &F,
) -> Result<()>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    // 1. Vanilla files, then the loader. Forge and NeoForge need the vanilla
    //    parent to exist before their installer runs.
    install::install(
        paths,
        &selection.minecraft,
        source,
        false,
        Arc::clone(&cancel),
        |progress| on_progress(scale_progress(progress, 0.0, 32.0)),
    )?;

    let launch_version = match selection.loader.kind {
        LoaderType::Vanilla => selection.minecraft.clone(),
        _ => install::install_loader_with_output(
            paths,
            install::LoaderInstallRequest {
                minecraft_version: &selection.minecraft,
                loader: selection.loader.kind,
                loader_version: selection.loader.version.as_deref(),
                java_settings,
                source,
            },
            Arc::clone(&cancel),
            |progress| on_progress(scale_progress(progress, 32.0, 45.0)),
            |line| {
                let mut value = InstallProgress::new(InstallPhase::Loader, 45.0);
                value.message = Some(line.to_string());
                on_progress(value);
            },
        )?,
    };
    instances::update(paths, &record.id, |record| {
        record.launch_version_id = Some(launch_version.clone());
    })?;

    // 2. The files listed by the pack.
    let game_dir = paths.game_dir(&record.id)?;
    let mut tasks = Vec::with_capacity(files.len());
    for file in files {
        let destination = Paths::resolve_inside(&game_dir, &file.path)?;
        tasks.push(DownloadTask {
            destination,
            urls: file.urls.clone(),
            sha1: file.sha1.clone(),
            sha512: file.sha512.clone(),
            size: (file.size > 0).then_some(file.size),
            label: file.path.clone(),
        });
    }
    let total = tasks.len();
    download_all(tasks, 8, Arc::clone(&cancel), |progress| {
        let ratio = if progress.bytes_total > 0 {
            progress.bytes_received as f32 / progress.bytes_total as f32
        } else if total > 0 {
            progress.files_done as f32 / total as f32
        } else {
            0.0
        };
        let mut mapped =
            InstallProgress::new(InstallPhase::Modpack, 45.0 + 47.0 * ratio.clamp(0.0, 1.0));
        mapped.files_done = progress.files_done;
        mapped.files_total = progress.files_total;
        mapped.bytes_received = progress.bytes_received;
        mapped.bytes_total = progress.bytes_total;
        on_progress(mapped);
    })?;

    // 3. Pack overrides win over downloaded files.
    on_progress(InstallProgress::new(InstallPhase::Overrides, 92.0));
    extract_overrides(zip_path, &game_dir)?;
    on_progress(InstallProgress::new(InstallPhase::Overrides, 99.0));
    Ok(())
}

fn scale_progress(mut progress: InstallProgress, start: f32, end: f32) -> InstallProgress {
    progress.percent = start + (end - start) * (progress.percent / 100.0).clamp(0.0, 1.0);
    progress
}

/// Downloads a Modrinth modpack version and imports it as a new instance.
///
/// Uses the default Java settings for installer-driven loaders. Call
/// [`install_modrinth_with_java`] to pass the user's Java configuration.
pub fn install_modrinth<F>(
    paths: &Paths,
    project_id: &str,
    version_id: Option<&str>,
    source: DownloadSource,
    cancel: Arc<AtomicBool>,
    on_progress: F,
) -> Result<InstanceRecord>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    install_modrinth_with_java(
        paths,
        project_id,
        version_id,
        source,
        &JavaSettings::default(),
        cancel,
        on_progress,
    )
}

/// Downloads a Modrinth modpack version and imports it as a new instance,
/// using `java_settings` when the pack needs a Forge or NeoForge installer.
pub fn install_modrinth_with_java<F>(
    paths: &Paths,
    project_id: &str,
    version_id: Option<&str>,
    source: DownloadSource,
    java_settings: &JavaSettings,
    cancel: Arc<AtomicBool>,
    on_progress: F,
) -> Result<InstanceRecord>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    on_progress(InstallProgress::new(InstallPhase::Metadata, 0.0));
    let version = match version_id {
        Some(version_id) => modrinth::version(version_id)?,
        None => modrinth::versions(project_id, None, None)?
            .into_iter()
            .next()
            .ok_or_else(|| {
                YuhinaError::new(
                    ErrorCode::NotFound,
                    "No downloadable modpack version was found.",
                )
            })?,
    };
    let file = version.primary_file().ok_or_else(|| {
        YuhinaError::new(
            ErrorCode::NotFound,
            "That modpack version has no downloadable file.",
        )
    })?;
    validate_http_url(&file.url)?;
    let filename = sanitize_filename(&file.filename)?;
    let destination = paths.downloads_dir().join("modpacks").join(filename);
    let task = DownloadTask {
        destination: destination.clone(),
        urls: vec![file.url.clone()],
        sha1: file.sha1.clone(),
        sha512: file.sha512.clone(),
        size: (file.size > 0).then_some(file.size),
        label: version.name.clone(),
    };
    download_all(vec![task], 1, Arc::clone(&cancel), |progress| {
        on_progress(scale_progress(progress_to_install(progress), 0.0, 3.0))
    })?;
    import_local_with_java(
        paths,
        &destination,
        source,
        java_settings,
        cancel,
        on_progress,
    )
}

fn progress_to_install(progress: DownloadProgress) -> InstallProgress {
    let ratio = if progress.bytes_total > 0 {
        progress.bytes_received as f32 / progress.bytes_total as f32
    } else if progress.files_total > 0 {
        progress.files_done as f32 / progress.files_total as f32
    } else {
        0.0
    };
    let mut result = InstallProgress::new(InstallPhase::Metadata, 100.0 * ratio.clamp(0.0, 1.0));
    result.files_done = progress.files_done;
    result.files_total = progress.files_total;
    result.bytes_received = progress.bytes_received;
    result.bytes_total = progress.bytes_total;
    result
}

fn sanitize_filename(name: &str) -> Result<String> {
    let name = name.rsplit(['/', '\\']).next().unwrap_or(name);
    if name.is_empty() || name.contains('\0') || name == "." || name == ".." {
        return Err(YuhinaError::new(
            ErrorCode::InvalidInput,
            "Modrinth returned an invalid modpack file name.",
        ));
    }
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const FABRIC_INDEX: &str = r#"{
        "formatVersion": 1,
        "game": "minecraft",
        "versionId": "1.0.0",
        "name": "Test Pack",
        "summary": "A test pack",
        "files": [
            {
                "path": "mods/example.jar",
                "hashes": {
                    "sha1": "0123456789abcdef0123456789abcdef01234567",
                    "sha512": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                },
                "env": { "client": "required", "server": "required" },
                "downloads": ["https://cdn.modrinth.com/data/example.jar"],
                "fileSize": 1234
            }
        ],
        "dependencies": {
            "minecraft": "1.21.1",
            "fabric-loader": "0.16.9"
        }
    }"#;

    #[test]
    fn parses_index_and_maps_dependencies() {
        let index = parse_index(FABRIC_INDEX.as_bytes()).unwrap();
        assert_eq!(index.name, "Test Pack");
        assert_eq!(index.files.len(), 1);
        let selection = index.loader().unwrap();
        assert_eq!(selection.minecraft, "1.21.1");
        assert_eq!(selection.loader.kind, LoaderType::Fabric);
        assert_eq!(selection.loader.version.as_deref(), Some("0.16.9"));
        let files = plan_files(&index).unwrap();
        assert_eq!(files[0].path, "mods/example.jar");
        assert_eq!(files[0].size, 1234);
    }

    #[test]
    fn dependency_mapping_covers_every_loader() {
        let mut deps = HashMap::new();
        deps.insert("minecraft".to_string(), "1.20.1".to_string());

        let vanilla = select_loader(&deps).unwrap();
        assert_eq!(vanilla.loader.kind, LoaderType::Vanilla);
        assert_eq!(vanilla.loader.version, None);

        deps.insert("quilt-loader".to_string(), "0.24.0".to_string());
        let quilt = select_loader(&deps).unwrap();
        assert_eq!(quilt.loader.kind, LoaderType::Quilt);
        assert_eq!(quilt.loader.version.as_deref(), Some("0.24.0"));

        deps.remove("quilt-loader");
        deps.insert("forge".to_string(), "47.2.0".to_string());
        let forge = select_loader(&deps).unwrap();
        assert_eq!(forge.loader.kind, LoaderType::Forge);
        assert_eq!(forge.loader.version.as_deref(), Some("47.2.0"));

        deps.remove("forge");
        deps.insert("neoforge".to_string(), "21.1.209".to_string());
        let neoforge = select_loader(&deps).unwrap();
        assert_eq!(neoforge.loader.kind, LoaderType::Neoforge);
        assert_eq!(neoforge.loader.version.as_deref(), Some("21.1.209"));

        deps.remove("neoforge");
        deps.remove("minecraft");
        assert_eq!(
            select_loader(&deps).unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn forge_and_neoforge_versions_are_validated_before_install() {
        let mut deps = HashMap::new();
        deps.insert("minecraft".to_string(), "1.20.1".to_string());
        deps.insert("forge".to_string(), "../../evil".to_string());
        assert_eq!(
            select_loader(&deps).unwrap_err().code,
            ErrorCode::InvalidInput
        );

        deps.remove("forge");
        deps.insert("neoforge".to_string(), "21/1/209".to_string());
        assert_eq!(
            select_loader(&deps).unwrap_err().code,
            ErrorCode::InvalidInput
        );

        // An installer-driven loader may omit its version; the installer
        // pipeline then resolves the newest compatible build.
        deps.remove("neoforge");
        deps.insert("forge".to_string(), String::new());
        let forge = select_loader(&deps).unwrap();
        assert_eq!(forge.loader.kind, LoaderType::Forge);
        assert_eq!(forge.loader.version, None);
    }

    #[test]
    fn declaring_two_loaders_is_rejected() {
        let mut deps = HashMap::new();
        deps.insert("minecraft".to_string(), "1.21.1".to_string());
        deps.insert("forge".to_string(), "52.0.0".to_string());
        deps.insert("neoforge".to_string(), "21.1.209".to_string());
        assert_eq!(
            select_loader(&deps).unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn path_traversal_is_rejected() {
        for path in [
            "../evil.jar",
            "mods/../../evil.jar",
            "/etc/passwd",
            "C:/Windows/x",
        ] {
            let json = FABRIC_INDEX.replace("mods/example.jar", path);
            assert!(parse_index(json.as_bytes()).is_err(), "{path} was accepted");
        }
        assert!(override_relative("overrides/../../evil.txt").is_err());
        assert!(override_relative("client-overrides/../evil.txt").is_err());
    }

    #[test]
    fn client_unsupported_files_are_skipped() {
        let json = FABRIC_INDEX.replace(
            r#""env": { "client": "required", "server": "required" }"#,
            r#""env": { "client": "unsupported", "server": "required" }"#,
        );
        let index = parse_index(json.as_bytes()).unwrap();
        assert!(plan_files(&index).unwrap().is_empty());
    }

    #[test]
    fn overrides_are_extracted_with_client_overrides_winning() {
        let root =
            std::env::temp_dir().join(format!("yuhina-modpack-test-{}", uuid::Uuid::new_v4()));
        let game = root.join("minecraft");
        fs::create_dir_all(&game).unwrap();
        let zip_path = root.join("pack.mrpack");
        {
            let file = File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            write_zip_entry(&mut writer, options, INDEX_NAME, FABRIC_INDEX.as_bytes());
            write_zip_entry(
                &mut writer,
                options,
                "overrides/config/shared.txt",
                b"from overrides",
            );
            write_zip_entry(
                &mut writer,
                options,
                "overrides/config/only-overrides.txt",
                b"override only",
            );
            write_zip_entry(
                &mut writer,
                options,
                "client-overrides/config/shared.txt",
                b"from client",
            );
            write_zip_entry(
                &mut writer,
                options,
                "server-overrides/config/server.txt",
                b"must not be written",
            );
            writer.finish().unwrap();
        }

        let written = extract_overrides(&zip_path, &game).unwrap();
        assert_eq!(written, 3);
        assert_eq!(
            fs::read_to_string(game.join("config/shared.txt")).unwrap(),
            "from client"
        );
        assert_eq!(
            fs::read_to_string(game.join("config/only-overrides.txt")).unwrap(),
            "override only"
        );
        assert!(!game.join("config/server.txt").exists());

        let _ = fs::remove_dir_all(root);
    }

    fn write_zip_entry(
        writer: &mut zip::ZipWriter<File>,
        options: zip::write::SimpleFileOptions,
        name: &str,
        bytes: &[u8],
    ) {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }

    #[test]
    fn index_without_minecraft_dependency_is_rejected_at_install_planning() {
        let json = FABRIC_INDEX.replace(
            r#""dependencies": {
            "minecraft": "1.21.1",
            "fabric-loader": "0.16.9"
        }"#,
            r#""dependencies": { "fabric-loader": "0.16.9" }"#,
        );
        let index = parse_index(json.as_bytes()).unwrap();
        assert_eq!(index.loader().unwrap_err().code, ErrorCode::InvalidInput);
    }

    #[test]
    fn failed_import_removes_the_half_installed_instance() {
        let root =
            std::env::temp_dir().join(format!("yuhina-modpack-cleanup-{}", uuid::Uuid::new_v4()));
        let paths = Paths::at(root.clone());

        // A local version profile with no downloads keeps the vanilla install
        // network-free; the import then fails while extracting overrides.
        let version_id = "1.0.0-local";
        let dir = paths.version_dir(version_id).unwrap();
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(format!("{version_id}.json")),
            format!(r#"{{"id":"{version_id}","type":"release"}}"#),
        )
        .unwrap();

        let index = format!(
            r#"{{
                "formatVersion": 1,
                "game": "minecraft",
                "versionId": "1.0.0",
                "name": "Broken pack",
                "files": [],
                "dependencies": {{ "minecraft": "{version_id}" }}
            }}"#
        );
        let zip_path = root.join("broken.mrpack");
        {
            let file = File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            write_zip_entry(&mut writer, options, INDEX_NAME, index.as_bytes());
            write_zip_entry(&mut writer, options, "overrides/../evil.txt", b"nope");
            writer.finish().unwrap();
        }

        let error = import_local(
            &paths,
            &zip_path,
            DownloadSource::Auto,
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::InstallFailed);
        assert!(instances::list(&paths).unwrap().is_empty());

        let _ = fs::remove_dir_all(root);
    }
}
