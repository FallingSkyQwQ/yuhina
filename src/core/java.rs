use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use super::http;
use super::paths::{Paths, display_path};
use super::settings::JavaSettings;
use super::{ErrorCode, Result, YuhinaError};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRuntime {
    pub path: String,
    pub major: u32,
    pub version: String,
    pub vendor: String,
    pub arch: String,
    pub managed: bool,
}

impl JavaRuntime {
    pub fn label(&self) -> String {
        format!(
            "Java {} · {} · {}",
            self.major,
            if self.vendor.is_empty() {
                "Unknown"
            } else {
                &self.vendor
            },
            display_path(Path::new(&self.path))
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct JavaScan {
    pub runtimes: Vec<JavaRuntime>,
    pub warnings: Vec<String>,
}

pub fn probe(path: &Path) -> Option<JavaRuntime> {
    if !path.is_file() {
        return None;
    }
    let mut command = Command::new(path);
    command.args(["-XshowSettings:properties", "-version"]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command.output().ok()?;
    let mut text = String::from_utf8_lossy(&output.stderr).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    let mut version = String::new();
    let mut vendor = String::new();
    let mut arch = String::new();
    let mut major = 0u32;
    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("java.version = ") {
            version = value.trim().to_string();
            major = parse_major(&version).unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("java.vendor = ") {
            vendor = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("os.arch = ") {
            arch = value.trim().to_string();
        }
    }
    if major == 0 {
        // Fallback: `java version "1.8.0_402"`.
        for line in text.lines() {
            if let Some(start) = line.find("version \"") {
                let rest = &line[start + 9..];
                if let Some(end) = rest.find('"') {
                    version = rest[..end].to_string();
                    major = parse_major(&version).unwrap_or(0);
                    break;
                }
            }
        }
    }
    if major == 0 {
        return None;
    }
    Some(JavaRuntime {
        path: display_path(path),
        major,
        version,
        vendor,
        arch,
        managed: false,
    })
}

pub fn parse_major(version: &str) -> Option<u32> {
    let first = version.split(['.', '-', '+', '_']).next()?;
    let major = first.parse::<u32>().ok()?;
    if major == 1 {
        // Legacy `1.8.0_402` → 8.
        version.split('.').nth(1)?.parse().ok()
    } else {
        Some(major)
    }
}

fn java_executable(_name: &str) -> &'static str {
    if cfg!(target_os = "windows") {
        "java.exe"
    } else {
        "java"
    }
}

pub fn detect(paths: &Paths, settings: &JavaSettings) -> JavaScan {
    let mut scan = JavaScan::default();
    let mut seen = std::collections::HashSet::new();

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        candidates.push(
            Path::new(&java_home)
                .join("bin")
                .join(java_executable("java")),
        );
    }
    if let Ok(path) = std::env::var("PATH") {
        for entry in std::env::split_paths(&path) {
            candidates.push(entry.join(java_executable("java")));
        }
    }

    for root in common_java_roots() {
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                candidates.push(entry.path().join("bin").join(java_executable("java")));
            }
        }
        candidates.push(root.join("bin").join(java_executable("java")));
    }

    // Managed runtimes downloaded by Yuhina.
    for root in [paths.java_dir(), paths.runtimes_dir()] {
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                candidates.push(path.join("bin").join(java_executable("java")));
                candidates.push(path.join("jre").join("bin").join(java_executable("java")));
            }
        }
    }

    for manual in &settings.manual_paths {
        let path = Path::new(manual);
        candidates.push(if path.is_dir() {
            path.join("bin").join(java_executable("java"))
        } else {
            path.to_path_buf()
        });
    }

    for candidate in candidates {
        let key = candidate.to_string_lossy().to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        if let Some(runtime) = probe(&candidate) {
            scan.runtimes.push(runtime);
        }
    }

    scan.runtimes
        .sort_by(|a, b| b.major.cmp(&a.major).then_with(|| a.path.cmp(&b.path)));
    scan.runtimes
        .dedup_by(|a, b| a.path.eq_ignore_ascii_case(&b.path));
    scan
}

fn common_java_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(target_os = "windows")]
    {
        roots.push(PathBuf::from(r"C:\Program Files\Java"));
        roots.push(PathBuf::from(r"C:\Program Files\Eclipse Adoptium"));
        roots.push(PathBuf::from(r"C:\Program Files\Microsoft"));
        roots.push(PathBuf::from(r"C:\Program Files\Zulu"));
        roots.push(PathBuf::from(r"C:\Program Files\BellSoft"));
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            roots.push(Path::new(&local).join("Programs").join("Eclipse Adoptium"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        roots.push(PathBuf::from("/Library/Java/JavaVirtualMachines"));
        if let Ok(home) = std::env::var("HOME") {
            roots.push(Path::new(&home).join("Library/Java/JavaVirtualMachines"));
        }
    }
    #[cfg(target_os = "linux")]
    {
        roots.push(PathBuf::from("/usr/lib/jvm"));
    }
    roots
}

pub fn best_for(scan: &JavaScan, major: u32) -> Option<JavaRuntime> {
    scan.runtimes
        .iter()
        .filter(|runtime| runtime.major >= major)
        .min_by_key(|runtime| runtime.major)
        .cloned()
        .or_else(|| {
            scan.runtimes
                .iter()
                .filter(|runtime| runtime.major >= 8)
                .max_by_key(|runtime| runtime.major)
                .cloned()
        })
}

/// Ensures a compatible runtime exists, downloading Eclipse Temurin when the
/// user allows it. Progress is reported as a percentage and a status line.
pub fn ensure<F>(
    paths: &Paths,
    settings: &JavaSettings,
    major: u32,
    on_progress: F,
) -> Result<JavaRuntime>
where
    F: Fn(f32, &str) + Sync,
{
    let scan = detect(paths, settings);
    if let Some(runtime) = scan.runtimes.iter().find(|runtime| runtime.major == major) {
        return Ok(runtime.clone());
    }
    if !settings.auto_download {
        return Err(YuhinaError::new(
            ErrorCode::JavaRequired,
            format!("Java {major} is required. Install it and try again."),
        ));
    }
    on_progress(0.02, "Resolving the Java runtime");
    let runtime = download_temurin(paths, major, &on_progress)?;
    on_progress(1.0, "Java is ready");
    Ok(runtime)
}

fn adoptium_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    }
}

fn adoptium_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86" => "x86",
        "aarch64" => "aarch64",
        _ => "x64",
    }
}

fn download_temurin<F>(paths: &Paths, major: u32, on_progress: &F) -> Result<JavaRuntime>
where
    F: Fn(f32, &str) + Sync,
{
    let url = format!(
        "https://api.adoptium.net/v3/assets/latest/{major}/hotspot?architecture={}&image_type=jre&os={}&vendor=eclipse",
        adoptium_arch(),
        adoptium_os()
    );
    on_progress(0.05, "Contacting Eclipse Adoptium");
    #[derive(Deserialize)]
    struct Release {
        binary: Binary,
        version: Version,
    }
    #[derive(Deserialize)]
    struct Binary {
        package: Package,
    }
    #[derive(Deserialize)]
    struct Package {
        link: String,
        #[serde(default)]
        checksum: Option<String>,
        name: String,
    }
    #[derive(Deserialize)]
    struct Version {
        semver: String,
    }
    let releases: Vec<Release> = http::get_json(&url)?;
    let release = releases
        .into_iter()
        .next()
        .ok_or_else(|| YuhinaError::new(ErrorCode::JavaRequired, "No Java build is available."))?;

    let downloads = paths.downloads_dir().join("java");
    fs::create_dir_all(&downloads)?;
    let archive = downloads.join(&release.binary.package.name);
    on_progress(0.12, "Downloading the Java runtime");
    let task = super::download::DownloadTask {
        destination: archive.clone(),
        urls: vec![release.binary.package.link.clone()],
        sha1: None,
        size: None,
        label: format!("Java {major}"),
    };
    super::download::download_all(
        vec![task],
        1,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        |progress| {
            if progress.bytes_total > 0 {
                let ratio = progress.bytes_received as f32 / progress.bytes_total as f32;
                on_progress(0.12 + ratio * 0.6, "Downloading the Java runtime");
            }
        },
    )?;

    if let Some(expected) = release.binary.package.checksum.as_deref()
        && expected.len() > 8
    {
        let actual = sha256_file(&archive)?;
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = fs::remove_file(&archive);
            return Err(YuhinaError::new(
                ErrorCode::Checksum,
                "The Java runtime failed its checksum.",
            ));
        }
    }

    on_progress(0.75, "Extracting the Java runtime");
    let target = paths.java_dir().join(major.to_string());
    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    fs::create_dir_all(&target)?;

    if release.binary.package.name.ends_with(".zip") {
        extract_zip(&archive, &target)?;
    } else {
        extract_tar_gz(&archive, &target)?;
    }
    let _ = fs::remove_file(&archive);

    // Temurin archives contain one top-level folder; flatten it.
    let jar_home = find_java_home(&target).unwrap_or(target);
    let java = jar_home.join("bin").join(java_executable("java"));
    let mut runtime = probe(&java).ok_or_else(|| {
        YuhinaError::new(
            ErrorCode::JavaRequired,
            "The downloaded Java runtime could not be started.",
        )
    })?;
    runtime.managed = true;
    runtime.path = display_path(&java);
    Ok(runtime)
}

fn find_java_home(root: &Path) -> Option<PathBuf> {
    let direct = root.join("bin").join(java_executable("java"));
    if direct.is_file() {
        return Some(root.to_path_buf());
    }
    for entry in fs::read_dir(root).ok()?.flatten() {
        if entry.file_type().ok()?.is_dir() {
            let candidate = entry.path().join("bin").join(java_executable("java"));
            if candidate.is_file() {
                return Some(entry.path());
            }
        }
    }
    None
}

fn extract_zip(archive: &Path, target: &Path) -> Result<()> {
    let file = File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        let safe = super::paths::normalize_relative(&name)?;
        let destination = Paths::resolve_inside(target, &safe)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = File::create(&destination)?;
        io::copy(&mut entry, &mut output)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = entry.unix_mode() {
                let _ = fs::set_permissions(&destination, fs::Permissions::from_mode(mode));
            }
        }
    }
    Ok(())
}

fn extract_tar_gz(archive: &Path, target: &Path) -> Result<()> {
    let file = File::open(archive)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().to_string();
        let safe = super::paths::normalize_relative(&path)?;
        let destination = Paths::resolve_inside(target, &safe)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        entry.unpack(&destination)?;
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}
