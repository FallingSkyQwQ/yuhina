use std::path::{Component, Path, PathBuf};

use super::{Result, YuhinaError};

/// Every path Yuhina owns, rooted at one data directory.
#[derive(Clone, Debug)]
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    /// `%APPDATA%\Yuhina`, `~/Library/Application Support/Yuhina`, or
    /// `~/.config/Yuhina`, matching the original Electron layout.
    pub fn discover() -> Result<Self> {
        if let Ok(root) = std::env::var("YUHINA_DATA_DIR") {
            return Ok(Self::at(root));
        }
        let base = platform_data_dir().ok_or_else(|| {
            YuhinaError::invalid("The operating system did not expose a data directory.")
        })?;
        Ok(Self {
            root: base.join("Yuhina"),
        })
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure(&self) -> Result<()> {
        std::fs::create_dir_all(&self.root).map_err(YuhinaError::io("create data root"))
    }

    pub fn settings_file(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    pub fn accounts_file(&self) -> PathBuf {
        self.root.join("accounts.json")
    }

    pub fn servers_file(&self) -> PathBuf {
        self.root.join("servers.json")
    }

    pub fn instances_dir(&self) -> PathBuf {
        self.root.join("instances")
    }

    pub fn instance_dir(&self, id: &str) -> Result<PathBuf> {
        assert_safe_id(id)?;
        Ok(self.instances_dir().join(id))
    }

    pub fn instance_file(&self, id: &str) -> Result<PathBuf> {
        Ok(self.instance_dir(id)?.join("instance.json"))
    }

    pub fn game_dir(&self, id: &str) -> Result<PathBuf> {
        Ok(self.instance_dir(id)?.join("minecraft"))
    }

    pub fn minecraft_dir(&self) -> PathBuf {
        self.root.join("minecraft")
    }

    pub fn versions_dir(&self) -> PathBuf {
        self.minecraft_dir().join("versions")
    }

    pub fn libraries_dir(&self) -> PathBuf {
        self.minecraft_dir().join("libraries")
    }

    pub fn assets_dir(&self) -> PathBuf {
        self.minecraft_dir().join("assets")
    }

    pub fn indexes_dir(&self) -> PathBuf {
        self.assets_dir().join("indexes")
    }

    pub fn runtimes_dir(&self) -> PathBuf {
        self.minecraft_dir().join("runtimes")
    }

    pub fn java_dir(&self) -> PathBuf {
        self.root.join("java")
    }

    pub fn downloads_dir(&self) -> PathBuf {
        self.root.join("downloads")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.root.join("backups")
    }

    pub fn instance_backups_dir(&self, id: &str) -> Result<PathBuf> {
        assert_safe_id(id)?;
        Ok(self.backups_dir().join(id))
    }

    pub fn exports_dir(&self) -> PathBuf {
        self.root.join("exports")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn instance_logs_dir(&self, id: &str) -> Result<PathBuf> {
        assert_safe_id(id)?;
        Ok(self.logs_dir().join("instances").join(id))
    }

    pub fn version_dir(&self, version_id: &str) -> Result<PathBuf> {
        assert_safe_id(version_id)?;
        Ok(self.versions_dir().join(version_id))
    }

    pub fn version_json(&self, version_id: &str) -> Result<PathBuf> {
        Ok(self
            .version_dir(version_id)?
            .join(format!("{version_id}.json")))
    }

    pub fn client_jar(&self, version_id: &str) -> Result<PathBuf> {
        Ok(self
            .version_dir(version_id)?
            .join(format!("{version_id}.jar")))
    }

    pub fn natives_dir(&self, version_id: &str) -> Result<PathBuf> {
        Ok(self.version_dir(version_id)?.join("natives"))
    }

    /// Resolves `relative` under `root`, rejecting absolute paths, `..` and
    /// NUL bytes. This is the single filesystem guard for downloaded content.
    pub fn resolve_inside(root: &Path, relative: impl AsRef<str>) -> Result<PathBuf> {
        let relative = normalize_relative(relative.as_ref())?;
        let mut out = root.to_path_buf();
        for segment in relative.split('/') {
            out.push(segment);
        }
        Ok(out)
    }
}

fn platform_data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        dirs::config_dir().or_else(dirs::data_dir)
    }
    #[cfg(not(target_os = "linux"))]
    {
        dirs::data_dir()
    }
}

/// Version and instance ids are used as directory names.
pub fn assert_safe_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
    {
        return Err(YuhinaError::invalid("That name is not a valid identifier."));
    }
    if id == "." || id == ".." {
        return Err(YuhinaError::invalid("That name is not a valid identifier."));
    }
    Ok(())
}

/// Validates an installer-controlled relative path and returns it with `/`
/// separators, without `.` segments.
pub fn normalize_relative(value: &str) -> Result<String> {
    if value.is_empty() || value.contains('\0') {
        return Err(YuhinaError::new(
            super::ErrorCode::InstallFailed,
            "A download path was invalid.",
        ));
    }
    let slashed = value.replace('\\', "/");
    if slashed.starts_with('/') || has_drive_prefix(&slashed) {
        return Err(YuhinaError::new(
            super::ErrorCode::InstallFailed,
            "A download path was absolute.",
        ));
    }
    let mut segments = Vec::new();
    for segment in slashed.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return Err(YuhinaError::new(
                super::ErrorCode::InstallFailed,
                "A download path tried to escape its folder.",
            ));
        }
        segments.push(segment);
    }
    if segments.is_empty() {
        return Err(YuhinaError::new(
            super::ErrorCode::InstallFailed,
            "A download path was empty.",
        ));
    }
    Ok(segments.join("/"))
}

fn has_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// Normalizes a user-picked absolute path for display and storage.
pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

/// Returns true when `path` is lexically inside `root` after resolution.
pub fn is_inside(root: &Path, path: &Path) -> bool {
    let root = normalize_lexically(root);
    let path = normalize_lexically(path);
    path.components()
        .zip(root.components())
        .all(|(a, b)| a == b)
        && path.components().count() >= root.components().count()
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_reject_escapes() {
        assert_eq!(normalize_relative("a/b.jar").unwrap(), "a/b.jar");
        assert_eq!(normalize_relative("./a//b.jar").unwrap(), "a/b.jar");
        assert!(normalize_relative("../a.jar").is_err());
        assert!(normalize_relative("/etc/passwd").is_err());
        assert!(normalize_relative("C:/Windows").is_err());
        assert!(normalize_relative("a\0b").is_err());
    }

    #[test]
    fn ids_are_safe_directory_names() {
        assert!(assert_safe_id("1.21.4").is_ok());
        assert!(assert_safe_id("fabric-loader-0.16.9-1.21.4").is_ok());
        assert!(assert_safe_id("../x").is_err());
        assert!(assert_safe_id("a/b").is_err());
        assert!(assert_safe_id("").is_err());
    }
}
