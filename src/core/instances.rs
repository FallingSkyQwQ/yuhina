use std::fs;
use std::path::Path;

use super::paths::Paths;
use super::storage::{read_json, write_json};
use super::{ErrorCode, Result, YuhinaError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const MEMORY_MIN_MB: u32 = 1024;
pub const MEMORY_MAX_MB: u32 = 32768;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderType {
    Vanilla,
    Fabric,
    Forge,
    Neoforge,
    Quilt,
}

impl LoaderType {
    pub fn label(self) -> &'static str {
        match self {
            LoaderType::Vanilla => "Vanilla",
            LoaderType::Fabric => "Fabric",
            LoaderType::Forge => "Forge",
            LoaderType::Neoforge => "NeoForge",
            LoaderType::Quilt => "Quilt",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Loader {
    #[serde(rename = "type")]
    pub kind: LoaderType,
    pub version: Option<String>,
}

impl Loader {
    pub fn vanilla() -> Self {
        Self {
            kind: LoaderType::Vanilla,
            version: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum MemoryMode {
    #[default]
    Auto,
    Manual,
}

fn default_memory() -> u32 {
    4096
}
fn default_memory_mode() -> MemoryMode {
    MemoryMode::Auto
}
fn now() -> String {
    Utc::now().to_rfc3339()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceRecord {
    pub id: String,
    pub name: String,
    pub version_id: String,
    pub loader: Loader,
    #[serde(default)]
    pub launch_version_id: Option<String>,
    #[serde(default = "default_memory")]
    pub memory_mb: u32,
    #[serde(default = "default_memory_mode")]
    pub memory_mode: MemoryMode,
    #[serde(default)]
    pub java_path: Option<String>,
    #[serde(default)]
    pub jvm_args: String,
    #[serde(default)]
    pub game_args: String,
    #[serde(default = "now")]
    pub created_at: String,
    #[serde(default)]
    pub last_played_at: Option<String>,
    #[serde(default)]
    pub playtime_ms: u64,
    #[serde(default)]
    pub last_exit_code: Option<i32>,
    #[serde(default)]
    pub icon: Option<String>,
}

impl InstanceRecord {
    pub fn launch_version(&self) -> &str {
        self.launch_version_id
            .as_deref()
            .unwrap_or(&self.version_id)
    }

    pub fn loader_label(&self) -> String {
        match self.loader.kind {
            LoaderType::Vanilla => "Vanilla".to_string(),
            kind => match &self.loader.version {
                Some(version) => format!("{} {}", kind.label(), version),
                None => kind.label().to_string(),
            },
        }
    }

    pub fn playtime_label(&self) -> String {
        let minutes = self.playtime_ms / 60_000;
        if minutes < 60 {
            format!("{minutes}m")
        } else {
            format!("{}h {}m", minutes / 60, minutes % 60)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSummary {
    #[serde(flatten)]
    pub record: InstanceRecord,
    pub installed: bool,
    pub required_java_major: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct NewInstance {
    pub name: String,
    pub version_id: String,
    pub loader: Loader,
    pub memory_mb: u32,
    pub memory_mode: MemoryMode,
    pub icon: Option<String>,
}

pub fn load(paths: &Paths, id: &str) -> Result<InstanceRecord> {
    let file = paths.instance_file(id)?;
    read_json::<InstanceRecord>(&file)?
        .ok_or_else(|| YuhinaError::new(ErrorCode::NotFound, "That instance no longer exists."))
}

pub fn save(paths: &Paths, record: &InstanceRecord) -> Result<()> {
    let file = paths.instance_file(&record.id)?;
    write_json(&file, record)
}

pub fn list(paths: &Paths) -> Result<Vec<InstanceRecord>> {
    let dir = paths.instances_dir();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut records = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let file = entry.path().join("instance.json");
        if let Ok(Some(record)) = read_json::<InstanceRecord>(&file) {
            records.push(record);
        }
    }
    records.sort_by(|a, b| {
        b.last_played_at
            .as_deref()
            .unwrap_or(&b.created_at)
            .cmp(a.last_played_at.as_deref().unwrap_or(&a.created_at))
    });
    Ok(records)
}

pub fn create(paths: &Paths, new: NewInstance) -> Result<InstanceRecord> {
    let id = uuid::Uuid::new_v4().to_string();
    let record = InstanceRecord {
        id: id.clone(),
        name: new.name,
        version_id: new.version_id,
        loader: new.loader,
        launch_version_id: None,
        memory_mb: new.memory_mb.clamp(MEMORY_MIN_MB, MEMORY_MAX_MB),
        memory_mode: new.memory_mode,
        java_path: None,
        jvm_args: String::new(),
        game_args: String::new(),
        created_at: now(),
        last_played_at: None,
        playtime_ms: 0,
        last_exit_code: None,
        icon: new.icon,
    };
    fs::create_dir_all(paths.game_dir(&id)?)?;
    save(paths, &record)?;
    Ok(record)
}

pub fn update(
    paths: &Paths,
    id: &str,
    patch: impl FnOnce(&mut InstanceRecord),
) -> Result<InstanceRecord> {
    let mut record = load(paths, id)?;
    patch(&mut record);
    record.memory_mb = record.memory_mb.clamp(MEMORY_MIN_MB, MEMORY_MAX_MB);
    save(paths, &record)?;
    Ok(record)
}

/// Records the version profile the launcher should start, which differs from
/// `version_id` once a loader installer has produced a child profile.
pub fn set_launch_version_id(
    paths: &Paths,
    id: &str,
    launch_version_id: impl Into<String>,
) -> Result<InstanceRecord> {
    let launch_version_id = launch_version_id.into();
    update(paths, id, |record| {
        record.launch_version_id = Some(launch_version_id);
    })
}

pub fn delete(paths: &Paths, id: &str, keep_files: bool) -> Result<()> {
    let record = load(paths, id)?;
    if !keep_files {
        let dir = paths.instance_dir(&record.id)?;
        fs::remove_dir_all(&dir)?;
    } else {
        fs::remove_file(paths.instance_file(&record.id)?)?;
    }
    Ok(())
}

pub fn duplicate(paths: &Paths, id: &str) -> Result<InstanceRecord> {
    let source = load(paths, id)?;
    let mut copy = source.clone();
    copy.id = uuid::Uuid::new_v4().to_string();
    copy.name = format!("{} (copy)", source.name);
    copy.created_at = now();
    copy.last_played_at = None;
    copy.playtime_ms = 0;
    copy.last_exit_code = None;
    fs::create_dir_all(paths.game_dir(&copy.id)?)?;
    copy_game_dir(&paths.game_dir(&source.id)?, &paths.game_dir(&copy.id)?)?;
    save(paths, &copy)?;
    Ok(copy)
}

fn copy_game_dir(source: &Path, destination: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&target)?;
            copy_game_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

pub fn mark_launch_started(paths: &Paths, id: &str) -> Result<()> {
    update(paths, id, |record| {
        record.last_played_at = Some(now());
    })?;
    Ok(())
}

pub fn mark_exited(
    paths: &Paths,
    id: &str,
    exit_code: Option<i32>,
    playtime_ms: u64,
) -> Result<()> {
    update(paths, id, |record| {
        record.last_exit_code = exit_code;
        record.playtime_ms = record.playtime_ms.saturating_add(playtime_ms);
    })?;
    Ok(())
}

/// `auto` memory: a base of 2 GiB, 256 MiB per mod, capped by 60% of system RAM.
pub fn recommended_memory_mb(mod_count: usize) -> u32 {
    let base = 2048u32 + (mod_count.min(64) as u32) * 256;
    base.clamp(MEMORY_MIN_MB, 8192)
}

pub fn effective_memory_mb(paths: &Paths, record: &InstanceRecord) -> u32 {
    match record.memory_mode {
        MemoryMode::Manual => record.memory_mb,
        MemoryMode::Auto => {
            let mods = fs::read_dir(paths.game_dir(&record.id).unwrap_or_default().join("mods"))
                .map(|entries| {
                    entries
                        .flatten()
                        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".jar"))
                        .count()
                })
                .unwrap_or(0);
            recommended_memory_mb(mods)
        }
    }
}

pub fn required_java_major(version_id: &str) -> u32 {
    // Mojang's metadata normally supplies this; this is the offline fallback.
    let parsed = version_id
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .find(|part| part.contains('.'))
        .and_then(|part| {
            let mut pieces = part.split('.');
            let major = pieces.next()?.parse::<u32>().ok()?;
            let minor = pieces.next()?.parse::<u32>().ok()?;
            Some((major, minor))
        });
    match parsed {
        Some((1, 20..=21)) => 21,
        Some((1, 18..=19)) => 17,
        Some((1, 17)) => 16,
        Some((1, 12..=16)) => 8,
        Some((1, 0..=11)) => 8,
        _ => 21,
    }
}

pub fn relative_created(created_at: &str) -> String {
    let Ok(parsed) = DateTime::parse_from_rfc3339(created_at) else {
        return created_at.to_string();
    };
    let days = (Utc::now() - parsed.with_timezone(&Utc)).num_days();
    match days {
        0 => "today".to_string(),
        1 => "yesterday".to_string(),
        n if n < 30 => format!("{n} days ago"),
        n if n < 365 => format!("{} months ago", n / 30),
        n => format!("{} years ago", n / 365),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_version_id_is_persisted() {
        let root = std::env::temp_dir().join(format!("yuhina-instances-{}", uuid::Uuid::new_v4()));
        let paths = Paths::at(root.clone());
        let record = create(
            &paths,
            NewInstance {
                name: "Test".to_string(),
                version_id: "1.21.1".to_string(),
                loader: Loader {
                    kind: LoaderType::Neoforge,
                    version: Some("21.1.209".to_string()),
                },
                memory_mb: 4096,
                memory_mode: MemoryMode::Manual,
                icon: None,
            },
        )
        .unwrap();

        set_launch_version_id(&paths, &record.id, "neoforge-21.1.209").unwrap();
        let loaded = load(&paths, &record.id).unwrap();
        assert_eq!(loaded.launch_version(), "neoforge-21.1.209");
        assert_eq!(loaded.loader.version.as_deref(), Some("21.1.209"));
        assert_eq!(loaded.version_id, "1.21.1");

        let _ = fs::remove_dir_all(&root);
    }
}
