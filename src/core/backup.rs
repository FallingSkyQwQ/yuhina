use std::fs::{self, File};
use std::io;
use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;

use super::files;
use super::install::{InstallPhase, InstallProgress};
use super::paths::{Paths, assert_safe_id, normalize_relative};
use super::storage::{read_json, write_json};
use super::{ErrorCode, Result, YuhinaError};

/// A backup archive plus the sidecar metadata the UI lists.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupManifest {
    /// File stem shared by `<id>.zip` and `<id>.json`.
    pub id: String,
    pub instance_id: String,
    #[serde(default)]
    pub label: Option<String>,
    pub created_at: String,
    pub bytes: u64,
    pub entries: usize,
}

pub fn backups_dir(paths: &Paths, instance_id: &str) -> Result<PathBuf> {
    paths.instance_backups_dir(instance_id)
}

fn timestamp() -> String {
    Utc::now().format("%Y%m%d-%H%M%S-%3f").to_string()
}

fn unique_id(directory: &std::path::Path) -> String {
    let base = timestamp();
    let mut candidate = base.clone();
    let mut index = 1;
    while directory.join(format!("{candidate}.zip")).exists()
        || directory.join(format!("{candidate}.json")).exists()
    {
        candidate = format!("{base}-{index}");
        index += 1;
    }
    candidate
}

/// Archives the instance's `minecraft/` directory. Progress uses the shared
/// installer progress type so the Activity page can treat it uniformly.
pub fn create<F>(
    paths: &Paths,
    instance_id: &str,
    label: Option<&str>,
    on_progress: F,
) -> Result<BackupManifest>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    let record = super::instances::load(paths, instance_id)?;
    let directory = paths.instance_backups_dir(&record.id)?;
    fs::create_dir_all(&directory)?;

    let game = paths.game_dir(&record.id)?;
    let files = files::collect_files(&game)?;
    let total = files.len();
    let id = unique_id(&directory);
    let zip_path = directory.join(format!("{id}.zip"));
    let part_path = directory.join(format!("{id}.zip.part"));
    let json_path = directory.join(format!("{id}.json"));

    on_progress(InstallProgress::new(InstallPhase::Backup, 0.0));

    let write_result = (|| -> Result<()> {
        let file = File::create(&part_path)?;
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);
        for (index, (relative, path)) in files.iter().enumerate() {
            writer.start_file(relative, options)?;
            let mut input = File::open(path)?;
            io::copy(&mut input, &mut writer)?;
            report(&on_progress, InstallPhase::Backup, index + 1, total, None);
        }
        writer.finish()?;
        fs::rename(&part_path, &zip_path)?;
        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_file(&part_path);
        return Err(error);
    }

    let bytes = fs::metadata(&zip_path).map(|meta| meta.len()).unwrap_or(0);
    let manifest = BackupManifest {
        id: id.clone(),
        instance_id: record.id,
        label: label
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        created_at: Utc::now().to_rfc3339(),
        bytes,
        entries: total,
    };
    if let Err(error) = write_json(&json_path, &manifest) {
        let _ = fs::remove_file(&zip_path);
        return Err(error);
    }

    report(&on_progress, InstallPhase::Backup, total, total, None);
    Ok(manifest)
}

/// Lists backups for one instance, newest first. Archives without a readable
/// manifest are ignored rather than surfacing a half-written entry.
pub fn list(paths: &Paths, instance_id: &str) -> Result<Vec<BackupManifest>> {
    let directory = paths.instance_backups_dir(instance_id)?;
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };

    let mut manifests = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            continue;
        }
        if let Ok(Some(manifest)) = read_json::<BackupManifest>(&path)
            && assert_safe_id(&manifest.id).is_ok()
            && manifest.instance_id == instance_id
            && directory.join(format!("{}.zip", manifest.id)).is_file()
        {
            manifests.push(manifest);
        }
    }
    manifests.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(manifests)
}

/// Restores a backup after creating a safety backup of the current game folder.
pub fn restore<F>(
    paths: &Paths,
    instance_id: &str,
    backup_id: &str,
    on_progress: F,
) -> Result<BackupManifest>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    let manifest = load_manifest(paths, instance_id, backup_id)?;
    let directory = paths.instance_backups_dir(instance_id)?;
    let zip_path = directory.join(format!("{}.zip", manifest.id));
    let game = paths.game_dir(instance_id)?;

    let archive_file = File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(archive_file)?;

    // Validate every entry before anything is deleted or written.
    let mut entries = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let safe = normalize_relative(entry.name())?;
        entries.push((index, safe));
    }

    let safety = create(paths, instance_id, Some("before-restore"), |_| {})?;

    on_progress(InstallProgress {
        phase: InstallPhase::Restore,
        percent: 0.0,
        files_done: 0,
        files_total: entries.len(),
        bytes_received: 0,
        bytes_total: 0,
        message: Some(safety.id.clone()),
    });

    let replace_result = (|| -> Result<()> {
        if game.exists() {
            fs::remove_dir_all(&game)?;
        }
        fs::create_dir_all(&game)?;
        for (done, (index, relative)) in entries.iter().enumerate() {
            let mut entry = archive.by_index(*index)?;
            let target = Paths::resolve_inside(&game, relative)?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output = File::create(&target)?;
            io::copy(&mut entry, &mut output)?;
            report(
                &on_progress,
                InstallPhase::Restore,
                done + 1,
                entries.len(),
                None,
            );
        }
        Ok(())
    })();

    replace_result?;
    report(
        &on_progress,
        InstallPhase::Restore,
        entries.len(),
        entries.len(),
        None,
    );
    Ok(manifest)
}

pub fn delete(paths: &Paths, instance_id: &str, backup_id: &str) -> Result<()> {
    assert_safe_id(backup_id)?;
    let manifest = load_manifest(paths, instance_id, backup_id)?;
    let directory = paths.instance_backups_dir(instance_id)?;
    for extension in ["zip", "json"] {
        let path = directory.join(format!("{}.{extension}", manifest.id));
        if path.is_file() {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn load_manifest(paths: &Paths, instance_id: &str, backup_id: &str) -> Result<BackupManifest> {
    assert_safe_id(backup_id)?;
    let directory = paths.instance_backups_dir(instance_id)?;
    let path = directory.join(format!("{backup_id}.json"));
    let manifest = read_json::<BackupManifest>(&path)?
        .ok_or_else(|| YuhinaError::new(ErrorCode::NotFound, "That backup no longer exists."))?;
    if manifest.instance_id != instance_id || manifest.id != backup_id {
        return Err(YuhinaError::new(
            ErrorCode::InvalidInput,
            "That backup belongs to another instance.",
        ));
    }
    if !directory.join(format!("{}.zip", backup_id)).is_file() {
        return Err(YuhinaError::new(
            ErrorCode::NotFound,
            "The backup archive is missing.",
        ));
    }
    Ok(manifest)
}

fn report<F>(
    on_progress: &F,
    phase: InstallPhase,
    done: usize,
    total: usize,
    message: Option<String>,
) where
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
        message,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::instances::{self, Loader, MemoryMode, NewInstance};
    use std::io::Write;

    fn temp_root() -> PathBuf {
        let path = std::env::temp_dir().join(format!("yuhina-backup-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn make_instance(paths: &Paths) -> String {
        instances::create(
            paths,
            NewInstance {
                name: "Backups".to_string(),
                version_id: "1.21.4".to_string(),
                loader: Loader::vanilla(),
                memory_mb: 2048,
                memory_mode: MemoryMode::Manual,
                icon: None,
            },
        )
        .unwrap()
        .id
    }

    fn write_zip(path: &std::path::Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        for (name, bytes) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn backup_round_trip_restores_files() {
        let root = temp_root();
        let paths = Paths::at(&root);
        let id = make_instance(&paths);
        let game = paths.game_dir(&id).unwrap();
        fs::write(game.join("options.txt"), b"fov:90").unwrap();
        fs::create_dir_all(game.join("saves/world")).unwrap();
        fs::write(game.join("saves/world/level.dat"), b"level").unwrap();

        let manifest = create(&paths, &id, Some("before mods"), |_| {}).unwrap();
        assert_eq!(manifest.entries, 2);
        assert!(manifest.bytes > 0);
        assert!(
            backups_dir(&paths, &id)
                .unwrap()
                .join(format!("{}.zip", manifest.id))
                .is_file()
        );

        fs::write(game.join("options.txt"), b"fov:30").unwrap();
        fs::remove_file(game.join("saves/world/level.dat")).unwrap();

        restore(&paths, &id, &manifest.id, |_| {}).unwrap();
        assert_eq!(fs::read(game.join("options.txt")).unwrap(), b"fov:90");
        assert_eq!(
            fs::read(game.join("saves/world/level.dat")).unwrap(),
            b"level"
        );

        let manifests = list(&paths, &id).unwrap();
        assert_eq!(manifests.len(), 2, "restore creates a safety backup");

        let original = manifests
            .iter()
            .find(|item| item.id == manifest.id)
            .unwrap()
            .clone();
        delete(&paths, &id, &original.id).unwrap();
        assert_eq!(list(&paths, &id).unwrap().len(), 1);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn restore_rejects_traversal_entries() {
        let root = temp_root();
        let paths = Paths::at(&root);
        let id = make_instance(&paths);
        let directory = paths.instance_backups_dir(&id).unwrap();
        fs::create_dir_all(&directory).unwrap();

        let manifest = BackupManifest {
            id: "evil".to_string(),
            instance_id: id.clone(),
            label: None,
            created_at: Utc::now().to_rfc3339(),
            bytes: 0,
            entries: 1,
        };
        write_json(&directory.join("evil.json"), &manifest).unwrap();
        write_zip(&directory.join("evil.zip"), &[("../escape.txt", b"oops")]);

        let error = restore(&paths, &id, "evil", |_| {}).unwrap_err();
        assert_eq!(error.code, ErrorCode::InstallFailed);
        assert!(!paths.instance_dir(&id).unwrap().join("escape.txt").exists());
        assert!(!root.join("escape.txt").exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_backups_list_as_empty() {
        let root = temp_root();
        let paths = Paths::at(&root);
        let id = make_instance(&paths);
        assert!(list(&paths, &id).unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
