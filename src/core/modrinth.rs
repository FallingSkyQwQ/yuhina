use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::http;
use super::paths::Paths;
use super::{ErrorCode, Result, YuhinaError};

pub const MODRINTH_BASE: &str = "https://api.modrinth.com/v2";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub follows: u64,
    pub categories: Vec<String>,
    pub project_type: String,
    pub game_versions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    pub total_hits: u64,
    pub offset: u64,
    pub limit: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub project_type: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub followers: u64,
    pub categories: Vec<String>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub source_url: Option<String>,
    pub updated: String,
    pub published: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dependency {
    pub project_id: String,
    pub version_id: Option<String>,
    pub dependency_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub version_type: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub files: Vec<VersionFile>,
    pub dependencies: Vec<Dependency>,
    pub date_published: String,
    pub downloads: u64,
}

impl Version {
    pub fn primary_file(&self) -> Option<&VersionFile> {
        self.files
            .iter()
            .find(|file| file.primary)
            .or_else(|| self.files.first())
    }
}

fn encode_component(value: &str) -> String {
    percent_encoding::utf8_percent_encode(value, percent_encoding::NON_ALPHANUMERIC).to_string()
}

pub fn search(
    query: &str,
    project_type: &str,
    loader: Option<&str>,
    game_version: Option<&str>,
    limit: u64,
    offset: u64,
) -> Result<SearchResult> {
    let facets = build_facets(project_type, loader, game_version);
    let url = format!(
        "{MODRINTH_BASE}/search?query={}&limit={}&offset={}&index=relevance&facets={}",
        encode_component(query),
        limit.clamp(1, 50),
        offset,
        encode_component(&facets)
    );
    let value: Value = http::get_json(&url)?;
    let hits = value["hits"]
        .as_array()
        .map(|items| items.iter().map(parse_hit).collect())
        .unwrap_or_default();
    Ok(SearchResult {
        hits,
        total_hits: value["total_hits"].as_u64().unwrap_or(0),
        offset: value["offset"].as_u64().unwrap_or(0),
        limit: value["limit"].as_u64().unwrap_or(limit),
    })
}

fn build_facets(project_type: &str, loader: Option<&str>, game_version: Option<&str>) -> String {
    let mut facets: Vec<String> = Vec::new();
    if !project_type.is_empty() {
        facets.push(format!("[\"project_type:{project_type}\"]"));
    }
    if let Some(loader) = loader.filter(|loader| !loader.is_empty()) {
        facets.push(format!("[\"categories:{loader}\"]"));
    }
    if let Some(version) = game_version.filter(|version| !version.is_empty()) {
        facets.push(format!("[\"versions:{version}\"]"));
    }
    format!("[{}]", facets.join(","))
}

fn parse_hit(value: &Value) -> SearchHit {
    SearchHit {
        id: value["project_id"].as_str().unwrap_or_default().to_string(),
        slug: value["slug"].as_str().unwrap_or_default().to_string(),
        title: value["title"].as_str().unwrap_or_default().to_string(),
        description: value["description"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        author: value["author"].as_str().unwrap_or_default().to_string(),
        icon_url: value["icon_url"].as_str().map(str::to_string),
        downloads: value["downloads"].as_u64().unwrap_or(0),
        follows: value["follows"].as_u64().unwrap_or(0),
        categories: strings(&value["categories"]),
        project_type: value["project_type"].as_str().unwrap_or("mod").to_string(),
        game_versions: strings(&value["versions"]),
    }
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn project(id: &str) -> Result<Project> {
    let url = format!("{MODRINTH_BASE}/project/{}", encode_component(id));
    let value: Value = http::get_json(&url)?;
    Ok(Project {
        id: value["id"].as_str().unwrap_or_default().to_string(),
        slug: value["slug"].as_str().unwrap_or_default().to_string(),
        title: value["title"].as_str().unwrap_or_default().to_string(),
        description: value["description"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        body: value["body"].as_str().unwrap_or_default().to_string(),
        project_type: value["project_type"].as_str().unwrap_or("mod").to_string(),
        icon_url: value["icon_url"].as_str().map(str::to_string),
        downloads: value["downloads"].as_u64().unwrap_or(0),
        followers: value["followers"].as_u64().unwrap_or(0),
        categories: strings(&value["categories"]),
        game_versions: strings(&value["game_versions"]),
        loaders: strings(&value["loaders"]),
        source_url: value["source_url"].as_str().map(str::to_string),
        updated: value["updated"].as_str().unwrap_or_default().to_string(),
        published: value["published"].as_str().unwrap_or_default().to_string(),
    })
}

pub fn versions(
    project_id: &str,
    loader: Option<&str>,
    game_version: Option<&str>,
) -> Result<Vec<Version>> {
    let mut url = format!(
        "{MODRINTH_BASE}/project/{}/version",
        encode_component(project_id)
    );
    let mut params = Vec::new();
    if let Some(loader) = loader.filter(|loader| !loader.is_empty()) {
        params.push(format!(
            "loaders={}",
            encode_component(&format!("[\"{loader}\"]"))
        ));
    }
    if let Some(version) = game_version.filter(|version| !version.is_empty()) {
        params.push(format!(
            "game_versions={}",
            encode_component(&format!("[\"{version}\"]"))
        ));
    }
    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }
    let value: Value = http::get_json(&url)?;
    Ok(value
        .as_array()
        .map(|items| items.iter().map(parse_version).collect())
        .unwrap_or_default())
}

pub fn version(id: &str) -> Result<Version> {
    let url = format!("{MODRINTH_BASE}/version/{}", encode_component(id));
    let value: Value = http::get_json(&url)?;
    Ok(parse_version(&value))
}

fn parse_version(value: &Value) -> Version {
    Version {
        id: value["id"].as_str().unwrap_or_default().to_string(),
        project_id: value["project_id"].as_str().unwrap_or_default().to_string(),
        name: value["name"].as_str().unwrap_or_default().to_string(),
        version_number: value["version_number"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        version_type: value["version_type"]
            .as_str()
            .unwrap_or("release")
            .to_string(),
        game_versions: strings(&value["game_versions"]),
        loaders: strings(&value["loaders"]),
        files: value["files"]
            .as_array()
            .map(|files| {
                files
                    .iter()
                    .map(|file| VersionFile {
                        url: file["url"].as_str().unwrap_or_default().to_string(),
                        filename: file["filename"].as_str().unwrap_or_default().to_string(),
                        primary: file["primary"].as_bool().unwrap_or(false),
                        size: file["size"].as_u64().unwrap_or(0),
                        sha1: file["hashes"]["sha1"].as_str().map(str::to_string),
                        sha512: file["hashes"]["sha512"].as_str().map(str::to_string),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        dependencies: value["dependencies"]
            .as_array()
            .map(|dependencies| {
                dependencies
                    .iter()
                    .map(|dependency| Dependency {
                        project_id: dependency["project_id"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        version_id: dependency["version_id"].as_str().map(str::to_string),
                        dependency_type: dependency["dependency_type"]
                            .as_str()
                            .unwrap_or("optional")
                            .to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        date_published: value["date_published"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        downloads: value["downloads"].as_u64().unwrap_or(0),
    }
}

pub fn content_dir(
    paths: &Paths,
    instance_id: &str,
    project_type: &str,
) -> Result<std::path::PathBuf> {
    let game = paths.game_dir(instance_id)?;
    let name = match project_type {
        "resourcepack" => "resourcepacks",
        "shader" => "shaderpacks",
        "modpack" => "modpacks",
        _ => "mods",
    };
    Ok(game.join(name))
}

/// Installs a project version and its required dependencies into an instance.
/// Returns the files written.
pub fn install(
    paths: &Paths,
    instance_id: &str,
    project_type: &str,
    project_id: &str,
    version_id: Option<&str>,
    loader: Option<&str>,
    game_version: Option<&str>,
) -> Result<Vec<std::path::PathBuf>> {
    let resolved = resolve_version(project_id, version_id, loader, game_version)?;
    let mut files = Vec::new();
    let mut visited = std::collections::HashSet::new();
    install_recursive(
        paths,
        instance_id,
        project_type,
        &resolved,
        loader,
        game_version,
        0,
        &mut visited,
        &mut files,
    )?;
    Ok(files)
}

#[allow(clippy::too_many_arguments)]
fn install_recursive(
    paths: &Paths,
    instance_id: &str,
    project_type: &str,
    selected: &Version,
    loader: Option<&str>,
    game_version: Option<&str>,
    depth: usize,
    visited: &mut std::collections::HashSet<String>,
    files: &mut Vec<std::path::PathBuf>,
) -> Result<()> {
    if depth > 8 || !visited.insert(selected.id.clone()) {
        return Ok(());
    }
    let Some(file) = selected.primary_file() else {
        return Ok(());
    };
    if !(file.url.starts_with("https://") || file.url.starts_with("http://")) {
        return Err(YuhinaError::new(
            ErrorCode::InvalidInput,
            "Modrinth returned an unsafe download URL.",
        ));
    }
    let directory = content_dir(paths, instance_id, project_type)?;
    std::fs::create_dir_all(&directory)?;
    let filename = sanitize_filename(&file.filename)?;
    let destination = directory.join(filename);
    let task = super::download::DownloadTask {
        destination: destination.clone(),
        urls: vec![file.url.clone()],
        sha1: file.sha1.clone(),
        size: if file.size > 0 { Some(file.size) } else { None },
        label: selected.name.clone(),
    };
    super::download::download_all(
        vec![task],
        1,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        |_| {},
    )?;
    files.push(destination);

    for dependency in &selected.dependencies {
        if dependency.dependency_type != "required" {
            continue;
        }
        let dep = if let Some(version_id) = &dependency.version_id {
            version(version_id)
        } else {
            versions(&dependency.project_id, loader, game_version).and_then(|versions| {
                versions.into_iter().next().ok_or_else(|| {
                    YuhinaError::new(
                        ErrorCode::NotFound,
                        "A required Modrinth dependency has no compatible version.",
                    )
                })
            })
        };
        if let Ok(dep) = dep {
            install_recursive(
                paths,
                instance_id,
                project_type,
                &dep,
                loader,
                game_version,
                depth + 1,
                visited,
                files,
            )?;
        }
    }
    Ok(())
}

fn resolve_version(
    project_id: &str,
    version_id: Option<&str>,
    loader: Option<&str>,
    game_version: Option<&str>,
) -> Result<Version> {
    if let Some(version_id) = version_id {
        return version(version_id);
    }
    versions(project_id, loader, game_version).and_then(|versions| {
        versions.into_iter().next().ok_or_else(|| {
            YuhinaError::new(
                ErrorCode::NotFound,
                "No compatible version of that project was found.",
            )
        })
    })
}

fn sanitize_filename(name: &str) -> Result<String> {
    let name = name.rsplit(['/', '\\']).next().unwrap_or(name);
    if name.is_empty() || name.contains('\0') || name == "." || name == ".." {
        return Err(YuhinaError::new(
            ErrorCode::InvalidInput,
            "Modrinth returned an invalid file name.",
        ));
    }
    Ok(name.to_string())
}

pub fn remove_file(
    paths: &Paths,
    instance_id: &str,
    project_type: &str,
    filename: &str,
) -> Result<()> {
    let directory = content_dir(paths, instance_id, project_type)?;
    let safe = sanitize_filename(filename)?;
    let path = directory.join(safe);
    if path.is_file() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub fn list_installed(
    paths: &Paths,
    instance_id: &str,
    project_type: &str,
) -> Result<Vec<(String, u64)>> {
    let directory = content_dir(paths, instance_id, project_type)?;
    let entries = match std::fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            files.push((
                entry.file_name().to_string_lossy().to_string(),
                entry.metadata().map(|meta| meta.len()).unwrap_or(0),
            ));
        }
    }
    files.sort();
    Ok(files)
}
