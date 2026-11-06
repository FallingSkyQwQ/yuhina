use std::fs;
use std::io::Write;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::{Result, YuhinaError};

/// Reads JSON, returning `Ok(None)` when the file does not exist.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    match fs::read(path) {
        Ok(bytes) => {
            let value = serde_json::from_slice(&bytes)?;
            Ok(Some(value))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(YuhinaError::io(format!("read {}", path.display()))(error)),
    }
}

/// Writes JSON atomically: a sibling temp file, flush, then rename.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(YuhinaError::io(format!("create {}", parent.display())))?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|s| s.to_str()).unwrap_or("json")
    ));
    let bytes = serde_json::to_vec_pretty(value)?;
    {
        let mut file =
            fs::File::create(&tmp).map_err(YuhinaError::io(format!("create {}", tmp.display())))?;
        file.write_all(&bytes)
            .map_err(YuhinaError::io(format!("write {}", tmp.display())))?;
        file.sync_all()
            .map_err(YuhinaError::io(format!("sync {}", tmp.display())))?;
    }
    fs::rename(&tmp, path).map_err(YuhinaError::io(format!("replace {}", path.display())))?;
    Ok(())
}

/// Writes text atomically, used for small metadata files.
pub fn write_text(path: &Path, value: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, value)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn read_text(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}
