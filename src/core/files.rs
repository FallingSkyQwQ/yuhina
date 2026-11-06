use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use super::paths::{Paths, assert_safe_id, normalize_relative};
use super::{ErrorCode, Result, YuhinaError};

/// Text previews stop here so a huge log or NBT blob cannot stall the renderer.
pub const PREVIEW_LIMIT: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    /// Path relative to the instance's `minecraft/` directory, `/`-separated.
    pub relative: String,
    pub is_dir: bool,
    pub bytes: u64,
}

#[derive(Clone, Debug)]
pub struct TextPreview {
    pub relative: String,
    pub content: String,
    pub bytes: u64,
    pub truncated: bool,
}

fn game_root(paths: &Paths, instance_id: &str) -> Result<PathBuf> {
    assert_safe_id(instance_id)?;
    paths.game_dir(instance_id)
}

fn resolve(paths: &Paths, instance_id: &str, relative: &str) -> Result<PathBuf> {
    let root = game_root(paths, instance_id)?;
    if relative.trim().is_empty() {
        return Ok(root);
    }
    let safe = normalize_relative(relative)?;
    let path = Paths::resolve_inside(&root, &safe)?;
    if !super::paths::is_inside(&root, &path) {
        return Err(YuhinaError::new(
            ErrorCode::InvalidInput,
            "That file is outside the instance folder.",
        ));
    }
    Ok(path)
}

/// Lists one directory under the instance game folder. Symlinks are skipped so
/// a link cannot expose files outside the instance.
pub fn list_dir(paths: &Paths, instance_id: &str, relative: &str) -> Result<Vec<FileEntry>> {
    let directory = resolve(paths, instance_id, relative)?;
    let base = if relative.trim().is_empty() {
        String::new()
    } else {
        normalize_relative(relative)?
    };
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };

    let mut result = Vec::new();
    for entry in entries.flatten() {
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        if file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.is_empty() || name.contains('/') || name.contains('\\') || name == ".." {
            continue;
        }
        let child = if base.is_empty() {
            name.clone()
        } else {
            format!("{base}/{name}")
        };
        let Ok(safe) = normalize_relative(&child) else {
            continue;
        };
        let bytes = if file_type.is_file() {
            entry.metadata().map(|meta| meta.len()).unwrap_or(0)
        } else {
            0
        };
        result.push(FileEntry {
            name,
            relative: safe,
            is_dir: file_type.is_dir(),
            bytes,
        });
    }

    result.sort_by(|left, right| {
        right
            .is_dir
            .cmp(&left.is_dir)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(result)
}

/// Reads a UTF-8 text preview, capped at [`PREVIEW_LIMIT`] bytes.
pub fn read_text_preview(paths: &Paths, instance_id: &str, relative: &str) -> Result<TextPreview> {
    let safe = normalize_relative(relative)?;
    let path = resolve(paths, instance_id, &safe)?;
    let metadata = fs::metadata(&path)?;
    if !metadata.is_file() {
        return Err(YuhinaError::new(
            ErrorCode::InvalidInput,
            "That path is not a file.",
        ));
    }

    let file = File::open(&path)?;
    let mut buffer = Vec::new();
    file.take(PREVIEW_LIMIT as u64 + 1)
        .read_to_end(&mut buffer)?;
    let truncated = buffer.len() > PREVIEW_LIMIT;
    if truncated {
        buffer.truncate(PREVIEW_LIMIT);
    }
    if buffer.contains(&0) {
        return Err(YuhinaError::new(
            ErrorCode::Unsupported,
            "That file is not a text file.",
        ));
    }
    let content = String::from_utf8(buffer)
        .map_err(|_| YuhinaError::new(ErrorCode::Unsupported, "That file is not UTF-8 text."))?;
    Ok(TextPreview {
        relative: safe,
        content,
        bytes: metadata.len(),
        truncated,
    })
}

/// Walks a directory and returns `(relative path, absolute path)` pairs for
/// every regular file, sorted for deterministic archives. Symlinks are skipped.
pub fn collect_files(root: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut files = Vec::new();
    if !root.exists() || !root.is_dir() {
        return Ok(files);
    }
    walk(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn walk(root: &Path, directory: &Path, files: &mut Vec<(String, PathBuf)>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            walk(root, &path, files)?;
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| YuhinaError::new(ErrorCode::Io, "A file left the instance folder."))?;
            let relative = relative.to_string_lossy().replace('\\', "/");
            let safe = normalize_relative(&relative)?;
            files.push((safe, path));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::instances::{self, NewInstance};
    use crate::core::instances::{Loader, MemoryMode};

    fn temp_root() -> PathBuf {
        let path = std::env::temp_dir().join(format!("yuhina-files-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn make_instance(paths: &Paths) -> String {
        let record = instances::create(
            paths,
            NewInstance {
                name: "Files".to_string(),
                version_id: "1.21.4".to_string(),
                loader: Loader::vanilla(),
                memory_mb: 2048,
                memory_mode: MemoryMode::Manual,
                icon: None,
            },
        )
        .unwrap();
        record.id
    }

    #[test]
    fn listing_rejects_traversal_and_orders_directories_first() {
        let root = temp_root();
        let paths = Paths::at(&root);
        let id = make_instance(&paths);
        let game = paths.game_dir(&id).unwrap();
        fs::create_dir_all(game.join("mods")).unwrap();
        fs::write(game.join("options.txt"), b"fov:90").unwrap();
        fs::write(game.join("mods/example.jar"), b"jar").unwrap();

        let entries = list_dir(&paths, &id, "").unwrap();
        assert_eq!(entries[0].name, "mods");
        assert!(entries[0].is_dir);
        assert_eq!(entries[1].name, "options.txt");
        assert_eq!(entries[1].relative, "options.txt");

        let nested = list_dir(&paths, &id, "mods").unwrap();
        assert_eq!(nested[0].relative, "mods/example.jar");

        assert!(list_dir(&paths, &id, "../").is_err());
        assert!(list_dir(&paths, &id, "/etc").is_err());
        assert!(list_dir(&paths, &id, "mods/../../").is_err());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn previews_are_text_only_and_bounded() {
        let root = temp_root();
        let paths = Paths::at(&root);
        let id = make_instance(&paths);
        let game = paths.game_dir(&id).unwrap();
        fs::write(game.join("readme.txt"), b"hello").unwrap();
        fs::write(game.join("big.txt"), vec![b'a'; PREVIEW_LIMIT + 32]).unwrap();
        fs::write(game.join("binary.dat"), [0u8, 1, 2]).unwrap();

        let preview = read_text_preview(&paths, &id, "readme.txt").unwrap();
        assert_eq!(preview.content, "hello");
        assert!(!preview.truncated);

        let preview = read_text_preview(&paths, &id, "big.txt").unwrap();
        assert!(preview.truncated);
        assert_eq!(preview.content.len(), PREVIEW_LIMIT);

        assert!(read_text_preview(&paths, &id, "binary.dat").is_err());
        assert!(read_text_preview(&paths, &id, "../instance.json").is_err());

        fs::remove_dir_all(root).unwrap();
    }
}
