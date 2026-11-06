use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;

use super::files;
use super::install::{InstallPhase, InstallProgress};
use super::instances::{self, InstanceRecord, MEMORY_MAX_MB, MEMORY_MIN_MB};
use super::paths::{Paths, normalize_relative};
use super::{ErrorCode, Result, YuhinaError};

pub const EXPORT_MANIFEST: &str = "yuhina.instance.json";
pub const EXPORT_FORMAT: u32 = 1;
const GAME_PREFIX: &str = "minecraft/";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportManifest {
    pub format: u32,
    pub created_at: String,
    pub app_version: String,
    pub instance: InstanceRecord,
}

/// Exports the instance metadata and game directory into
/// `<root>/exports/<name>-<timestamp>.zip`.
pub fn export_instance<F>(paths: &Paths, instance_id: &str, on_progress: F) -> Result<PathBuf>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    let record = instances::load(paths, instance_id)?;
    let game = paths.game_dir(&record.id)?;
    let files = files::collect_files(&game)?;
    let total = files.len() + 1;
    let directory = paths.exports_dir();
    fs::create_dir_all(&directory)?;
    let destination = unique_export_path(&directory, &record.name);

    let manifest = ExportManifest {
        format: EXPORT_FORMAT,
        created_at: Utc::now().to_rfc3339(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        instance: record,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;

    on_progress(InstallProgress::new(InstallPhase::Export, 0.0));

    let part = destination.with_extension("zip.part");
    let result = (|| -> Result<()> {
        let file = File::create(&part)?;
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        writer.start_file(EXPORT_MANIFEST, options)?;
        writer.write_all(&manifest_bytes)?;
        progress(&on_progress, InstallPhase::Export, 1, total);

        for (index, (relative, path)) in files.iter().enumerate() {
            writer.start_file(format!("{GAME_PREFIX}{relative}"), options)?;
            let mut input = File::open(path)?;
            io::copy(&mut input, &mut writer)?;
            progress(&on_progress, InstallPhase::Export, index + 2, total);
        }
        writer.finish()?;
        fs::rename(&part, &destination)?;
        Ok(())
    })();

    if let Err(error) = result {
        let _ = fs::remove_file(&part);
        return Err(error);
    }
    Ok(destination)
}

/// Imports an export archive as a new instance with a fresh id. All entries are
/// validated before the new instance directory is created.
pub fn import<F>(paths: &Paths, source: &Path, on_progress: F) -> Result<InstanceRecord>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    if !source.is_file() {
        return Err(YuhinaError::new(
            ErrorCode::NotFound,
            "That export file does not exist.",
        ));
    }

    let archive_file = File::open(source)?;
    let mut archive = zip::ZipArchive::new(archive_file)?;
    let mut manifest_bytes = None;
    let mut entries = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let safe = normalize_relative(entry.name())?;
        if safe == EXPORT_MANIFEST {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            manifest_bytes = Some(bytes);
        } else if let Some(relative) = safe.strip_prefix(GAME_PREFIX) {
            if relative.is_empty() {
                continue;
            }
            entries.push((index, relative.to_string()));
        } else {
            return Err(YuhinaError::new(
                ErrorCode::InvalidInput,
                "That archive contains an unexpected file.",
            ));
        }
    }

    let manifest_bytes = manifest_bytes.ok_or_else(|| {
        YuhinaError::new(
            ErrorCode::InvalidInput,
            "That archive is not a Yuhina instance export.",
        )
    })?;
    let manifest: ExportManifest = serde_json::from_slice(&manifest_bytes)?;
    if manifest.format > EXPORT_FORMAT {
        return Err(YuhinaError::new(
            ErrorCode::Unsupported,
            "That export was made by a newer version of Yuhina.",
        ));
    }

    let mut record = manifest.instance;
    record.id = uuid::Uuid::new_v4().to_string();
    if record.name.trim().is_empty() {
        record.name = "Imported instance".to_string();
    }
    record.memory_mb = record.memory_mb.clamp(MEMORY_MIN_MB, MEMORY_MAX_MB);
    record.created_at = Utc::now().to_rfc3339();
    record.last_played_at = None;
    record.playtime_ms = 0;
    record.last_exit_code = None;
    record.launch_version_id = None;
    // Machine-specific Java paths from another computer are not trusted.
    record.java_path = None;

    let instance_dir = paths.instance_dir(&record.id)?;
    let game = paths.game_dir(&record.id)?;
    fs::create_dir_all(&game)?;
    if let Err(error) = instances::save(paths, &record) {
        let _ = fs::remove_dir_all(&instance_dir);
        return Err(error);
    }

    let total = entries.len();
    on_progress(InstallProgress::new(InstallPhase::Import, 0.0));
    let result = (|| -> Result<()> {
        for (done, (index, relative)) in entries.iter().enumerate() {
            let mut entry = archive.by_index(*index)?;
            let target = Paths::resolve_inside(&game, relative)?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output = File::create(&target)?;
            io::copy(&mut entry, &mut output)?;
            progress(&on_progress, InstallPhase::Import, done + 1, total);
        }
        Ok(())
    })();

    if let Err(error) = result {
        let _ = fs::remove_dir_all(&instance_dir);
        return Err(error);
    }

    progress(&on_progress, InstallPhase::Import, total, total);
    Ok(record)
}

fn unique_export_path(directory: &Path, name: &str) -> PathBuf {
    let base = format!(
        "{}-{}",
        sanitize_name(name),
        Utc::now().format("%Y%m%d-%H%M%S")
    );
    let mut candidate = format!("{base}.zip");
    let mut index = 1;
    while directory.join(&candidate).exists() {
        candidate = format!("{base}-{index}.zip");
        index += 1;
    }
    directory.join(candidate)
}

fn sanitize_name(name: &str) -> String {
    let mut cleaned: String = name
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect();
    cleaned = cleaned.trim_matches('-').to_string();
    if cleaned.is_empty() {
        return "instance".to_string();
    }
    cleaned.chars().take(48).collect()
}

fn progress<F>(on_progress: &F, phase: InstallPhase, done: usize, total: usize)
where
    F: Fn(InstallProgress),
{
    let percent = if total == 0 {
        100.0
    } else {
        (done as f32 / total as f32 * 100.0).clamp(0.0, 100.0)
    };
    on_progress(InstallProgress {
        phase,
        percent,
        files_done: done,
        files_total: total,
        bytes_received: 0,
        bytes_total: 0,
        message: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::instances::{Loader, MemoryMode, NewInstance};

    fn temp_root() -> PathBuf {
        let path = std::env::temp_dir().join(format!("yuhina-export-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn make_instance(paths: &Paths) -> InstanceRecord {
        instances::create(
            paths,
            NewInstance {
                name: "Export me".to_string(),
                version_id: "1.21.4".to_string(),
                loader: Loader::vanilla(),
                memory_mb: 4096,
                memory_mode: MemoryMode::Manual,
                icon: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn export_import_round_trip_copies_metadata_and_files() {
        let root = temp_root();
        let paths = Paths::at(&root);
        let source = make_instance(&paths);
        let game = paths.game_dir(&source.id).unwrap();
        fs::write(game.join("options.txt"), b"fov:90").unwrap();
        fs::create_dir_all(game.join("saves/world")).unwrap();
        fs::write(game.join("saves/world/level.dat"), b"level").unwrap();

        let archive = export_instance(&paths, &source.id, |_| {}).unwrap();
        assert!(archive.is_file());
        assert!(
            archive
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("Export-me-")
        );

        let imported = import(&paths, &archive, |_| {}).unwrap();
        assert_ne!(imported.id, source.id);
        assert_eq!(imported.name, source.name);
        assert_eq!(imported.version_id, source.version_id);
        assert_eq!(imported.playtime_ms, 0);

        let imported_game = paths.game_dir(&imported.id).unwrap();
        assert_eq!(
            fs::read(imported_game.join("options.txt")).unwrap(),
            b"fov:90"
        );
        assert_eq!(
            fs::read(imported_game.join("saves/world/level.dat")).unwrap(),
            b"level"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn import_rejects_traversal_and_foreign_files() {
        let root = temp_root();
        let paths = Paths::at(&root);
        let archive = root.join("evil.zip");
        let file = File::create(&archive).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        writer.start_file("../evil.txt", options).unwrap();
        writer.write_all(b"nope").unwrap();
        writer.finish().unwrap();

        assert!(import(&paths, &archive, |_| {}).is_err());
        assert!(!root.join("evil.txt").exists());
        assert!(instances::list(&paths).unwrap().is_empty());

        fs::remove_dir_all(root).unwrap();
    }
}
