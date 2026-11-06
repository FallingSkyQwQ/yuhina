use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::settings::DownloadSource;
use super::{ErrorCode, Result, YuhinaError};

pub const BMCL_BASE: &str = "https://bmclapi2.bangbang93.com";
pub const OFFICIAL_MANIFEST: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const BMCL_MANIFEST: &str = "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub release_time: String,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub compliance_level: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VersionJson {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub inherits_from: Option<String>,
    pub main_class: Option<String>,
    pub assets: Option<String>,
    pub asset_index: Option<AssetIndex>,
    pub downloads: Option<VersionDownloads>,
    pub libraries: Option<Vec<Library>>,
    pub java_version: Option<JavaVersion>,
    pub arguments: Option<Arguments>,
    pub minecraft_arguments: Option<String>,
    pub release_time: Option<String>,
    pub time: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub total_size: Option<u64>,
    pub url: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct VersionDownloads {
    pub client: Option<Artifact>,
    #[serde(rename = "client_mappings")]
    pub client_mappings: Option<Artifact>,
    pub server: Option<Artifact>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct JavaVersion {
    pub component: Option<String>,
    pub major_version: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Arguments {
    pub game: Vec<ArgumentEntry>,
    pub jvm: Vec<ArgumentEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentEntry {
    Plain(String),
    Conditional {
        #[serde(default)]
        rules: Vec<Rule>,
        value: ArgumentValue,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    One(String),
    Many(Vec<String>),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub name: String,
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub rules: Option<Vec<Rule>>,
    pub downloads: Option<LibraryDownloads>,
    pub natives: Option<HashMap<String, String>>,
    pub extract: Option<Extract>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LibraryDownloads {
    pub artifact: Option<Artifact>,
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Artifact {
    pub path: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub url: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Extract {
    pub exclude: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct RuleContext {
    pub os: String,
    pub arch: String,
    pub os_version: Option<String>,
    pub features: HashMap<String, bool>,
}

impl RuleContext {
    pub fn current() -> Self {
        Self {
            os: platform_name().to_string(),
            arch: platform_arch().to_string(),
            os_version: std::env::var("OS_VERSION").ok(),
            features: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResolvedLibrary {
    pub name: String,
    pub relative_path: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub native: bool,
    pub extract_exclude: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct SelectedLibraries {
    pub classpath: Vec<ResolvedLibrary>,
    pub natives: Vec<ResolvedLibrary>,
}

pub fn platform_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

pub fn platform_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86" => "x86",
        "aarch64" => "arm64",
        _ => "x64",
    }
}

pub fn classpath_separator() -> &'static str {
    if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    }
}

pub fn rules_allow(rules: &[Rule], context: &RuleContext) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for rule in rules {
        if !os_matches(rule, context) || !features_match(rule, context) {
            continue;
        }
        allowed = rule.action == "allow";
    }
    allowed
}

fn os_matches(rule: &Rule, context: &RuleContext) -> bool {
    let Some(os) = &rule.os else {
        return true;
    };
    if let Some(name) = &os.name
        && name != &context.os
    {
        return false;
    }
    if let Some(arch) = &os.arch {
        let expected = match arch.as_str() {
            "x86" => "x86",
            "arm64" | "aarch64" => "arm64",
            _ => "x64",
        };
        if expected != context.arch {
            return false;
        }
    }
    if let (Some(pattern), Some(version)) = (&os.version, &context.os_version)
        && let Ok(regex) = regex::Regex::new(pattern)
        && !regex.is_match(version)
    {
        return false;
    }
    true
}

fn features_match(rule: &Rule, context: &RuleContext) -> bool {
    let Some(features) = &rule.features else {
        return true;
    };
    features
        .iter()
        .all(|(key, expected)| context.features.get(key).copied().unwrap_or(false) == *expected)
}

pub fn library_allowed(library: &Library, context: &RuleContext) -> bool {
    rules_allow(library.rules.as_deref().unwrap_or(&[]), context)
}

pub fn native_classifier(library: &Library) -> Option<String> {
    let name = library.name.split('@').next().unwrap_or(&library.name);
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 4 {
        return None;
    }
    let classifier = parts[3];
    if classifier.starts_with("natives-") {
        Some(classifier.to_string())
    } else {
        None
    }
}

pub fn legacy_native_classifier(library: &Library, context: &RuleContext) -> Option<String> {
    let natives = library.natives.as_ref()?;
    let key = match context.os.as_str() {
        "windows" => "windows",
        "osx" => "osx",
        _ => "linux",
    };
    let classifier = natives.get(key)?;
    Some(classifier.replace("${arch}", if context.arch == "x86" { "32" } else { "64" }))
}

pub fn maven_path_from_name(name: &str) -> Option<String> {
    let (without_extension, extension) = match name.split_once('@') {
        Some((name, extension)) => (name, extension),
        None => (name, "jar"),
    };
    let parts: Vec<&str> = without_extension.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0];
    let artifact = parts[1];
    let version = parts[2];
    let classifier = parts.get(3);
    let suffix = classifier
        .map(|value| format!("-{value}"))
        .unwrap_or_default();
    Some(format!(
        "{}/{}/{}/{}-{}{}.{}",
        group.replace('.', "/"),
        artifact,
        version,
        artifact,
        version,
        suffix,
        extension
    ))
}

pub fn resolve_libraries(libraries: &[Library], context: &RuleContext) -> SelectedLibraries {
    let mut selected = SelectedLibraries::default();
    for library in libraries {
        if !library_allowed(library, context) {
            continue;
        }
        let exclude = library
            .extract
            .as_ref()
            .map(|extract| extract.exclude.clone())
            .unwrap_or_default();

        if library.natives.is_some() {
            if let Some(classifier) = legacy_native_classifier(library, context)
                && let Some(download) = library
                    .downloads
                    .as_ref()
                    .and_then(|downloads| downloads.classifiers.as_ref())
                    .and_then(|classifiers| classifiers.get(&classifier))
            {
                selected
                    .natives
                    .push(resolved(&library.name, download, true, &exclude));
            }
            continue;
        }

        if native_classifier(library).is_some() {
            if let Some(download) = library
                .downloads
                .as_ref()
                .and_then(|downloads| downloads.artifact.as_ref())
            {
                selected
                    .natives
                    .push(resolved(&library.name, download, true, &exclude));
            }
            continue;
        }

        if let Some(download) = library
            .downloads
            .as_ref()
            .and_then(|downloads| downloads.artifact.as_ref())
        {
            selected
                .classpath
                .push(resolved(&library.name, download, false, &[]));
            continue;
        }

        if let Some(path) = maven_path_from_name(&library.name) {
            let repository = library
                .url
                .as_deref()
                .filter(|url| url.starts_with("http"))
                .map(|url| url.trim_end_matches('/').to_string() + "/")
                .unwrap_or_else(|| "https://libraries.minecraft.net/".to_string());
            selected.classpath.push(ResolvedLibrary {
                name: library.name.clone(),
                relative_path: path.clone(),
                url: format!("{repository}{path}"),
                sha1: library.sha1.clone(),
                size: library.size,
                native: false,
                extract_exclude: Vec::new(),
            });
        }
    }
    selected
}

fn resolved(name: &str, artifact: &Artifact, native: bool, exclude: &[String]) -> ResolvedLibrary {
    ResolvedLibrary {
        name: name.to_string(),
        relative_path: artifact
            .path
            .clone()
            .or_else(|| maven_path_from_name(name))
            .unwrap_or_default(),
        url: artifact.url.clone(),
        sha1: artifact.sha1.clone(),
        size: artifact.size,
        native,
        extract_exclude: exclude.to_vec(),
    }
}

pub fn resolve_arguments(entries: &[ArgumentEntry], context: &RuleContext) -> Vec<String> {
    let mut values = Vec::new();
    for entry in entries {
        match entry {
            ArgumentEntry::Plain(value) => values.push(value.clone()),
            ArgumentEntry::Conditional { rules, value } => {
                if !rules_allow(rules, context) {
                    continue;
                }
                match value {
                    ArgumentValue::One(value) => values.push(value.clone()),
                    ArgumentValue::Many(many) => values.extend(many.iter().cloned()),
                }
            }
        }
    }
    values
}

fn library_key(name: &str) -> String {
    let name = name.split('@').next().unwrap_or(name);
    let parts: Vec<&str> = name.split(':').collect();
    match (parts.first(), parts.get(1), parts.get(3)) {
        (Some(group), Some(artifact), classifier) => {
            let suffix = classifier
                .map(|value| format!(":{value}"))
                .unwrap_or_default();
            format!("{group}:{artifact}{suffix}")
        }
        _ => name.to_string(),
    }
}

pub fn merge_version_json(parent: &VersionJson, child: &VersionJson) -> VersionJson {
    let child_libraries = child.libraries.clone().unwrap_or_default();
    let child_keys: Vec<String> = child_libraries
        .iter()
        .map(|l| library_key(&l.name))
        .collect();
    let parent_libraries = parent
        .libraries
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|library| !child_keys.contains(&library_key(&library.name)));

    let parent_arguments = parent.arguments.clone().unwrap_or_default();
    let child_arguments = child.arguments.clone().unwrap_or_default();

    VersionJson {
        id: child.id.clone(),
        kind: child.kind.clone().or_else(|| parent.kind.clone()),
        inherits_from: None,
        main_class: child
            .main_class
            .clone()
            .or_else(|| parent.main_class.clone()),
        assets: child.assets.clone().or_else(|| parent.assets.clone()),
        asset_index: child
            .asset_index
            .clone()
            .or_else(|| parent.asset_index.clone()),
        downloads: child.downloads.clone().or_else(|| parent.downloads.clone()),
        libraries: Some(
            parent_libraries
                .chain(child_libraries)
                .collect::<Vec<Library>>(),
        ),
        java_version: child
            .java_version
            .clone()
            .or_else(|| parent.java_version.clone()),
        arguments: Some(Arguments {
            game: parent_arguments
                .game
                .into_iter()
                .chain(child_arguments.game)
                .collect(),
            jvm: parent_arguments
                .jvm
                .into_iter()
                .chain(child_arguments.jvm)
                .collect(),
        }),
        minecraft_arguments: child
            .minecraft_arguments
            .clone()
            .or_else(|| parent.minecraft_arguments.clone()),
        release_time: child
            .release_time
            .clone()
            .or_else(|| parent.release_time.clone()),
        time: child.time.clone().or_else(|| parent.time.clone()),
    }
}

/// BMCLAPI mirror rewrite for official Mojang hosts.
pub fn rewrite_to_bmcl(url: &str) -> String {
    let Ok(parsed) = url::Url::parse(url) else {
        return url.to_string();
    };
    match parsed.host_str() {
        Some("libraries.minecraft.net") => format!("{BMCL_BASE}/maven{}", parsed.path()),
        Some("resources.download.minecraft.net") => {
            format!("{BMCL_BASE}/assets{}", parsed.path())
        }
        Some("launchermeta.mojang.com" | "piston-meta.mojang.com" | "piston-data.mojang.com") => {
            format!(
                "{BMCL_BASE}{}{}",
                parsed.path(),
                parsed.query().map(|q| format!("?{q}")).unwrap_or_default()
            )
        }
        _ => url.to_string(),
    }
}

pub fn candidate_urls(
    url: &str,
    version_id: Option<&str>,
    kind: &str,
    source: DownloadSource,
) -> Vec<String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Vec::new();
    }
    if source == DownloadSource::Official {
        return vec![url.to_string()];
    }
    let mut bmcl = Vec::new();
    if let Some(version_id) = version_id {
        match kind {
            "json" => bmcl.push(format!(
                "{BMCL_BASE}/version/{}/json",
                percent_encoding::utf8_percent_encode(
                    version_id,
                    percent_encoding::NON_ALPHANUMERIC
                )
            )),
            "client" => bmcl.push(format!(
                "{BMCL_BASE}/version/{}/client",
                percent_encoding::utf8_percent_encode(
                    version_id,
                    percent_encoding::NON_ALPHANUMERIC
                )
            )),
            _ => {}
        }
    }
    if bmcl.is_empty() {
        bmcl.push(rewrite_to_bmcl(url));
    }
    if source == DownloadSource::Bmcl {
        bmcl.dedup();
        return bmcl;
    }
    let mut urls = bmcl;
    urls.push(url.to_string());
    urls.dedup();
    urls
}

pub fn manifest_url(source: DownloadSource) -> &'static str {
    match source {
        DownloadSource::Bmcl => BMCL_MANIFEST,
        _ => OFFICIAL_MANIFEST,
    }
}

pub fn asset_object_path(hash: &str) -> Result<String> {
    if hash.len() != 40 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(YuhinaError::new(
            ErrorCode::InstallFailed,
            "An asset hash was invalid.",
        ));
    }
    let normalized = hash.to_ascii_lowercase();
    Ok(format!("objects/{}/{}", &normalized[..2], normalized))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_paths_include_classifiers() {
        assert_eq!(
            maven_path_from_name("com.example:demo:1.0").unwrap(),
            "com/example/demo/1.0/demo-1.0.jar"
        );
        assert_eq!(
            maven_path_from_name("com.example:demo:1.0:natives-windows").unwrap(),
            "com/example/demo/1.0/demo-1.0-natives-windows.jar"
        );
    }

    #[test]
    fn rules_are_last_match_wins() {
        let mut features = HashMap::new();
        features.insert("has_custom_resolution".to_string(), true);
        let context = RuleContext {
            os: "windows".into(),
            arch: "x64".into(),
            os_version: None,
            features,
        };
        let rules = vec![
            Rule {
                action: "allow".into(),
                os: Some(OsRule {
                    name: Some("windows".into()),
                    ..Default::default()
                }),
                features: None,
            },
            Rule {
                action: "deny".into(),
                os: Some(OsRule {
                    name: Some("windows".into()),
                    ..Default::default()
                }),
                features: None,
            },
        ];
        assert!(!rules_allow(&rules, &context));
    }

    #[test]
    fn asset_paths_are_validated() {
        let hash = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(
            asset_object_path(hash).unwrap(),
            "objects/01/0123456789abcdef0123456789abcdef01234567"
        );
        assert!(asset_object_path("nope").is_err());
    }
}
