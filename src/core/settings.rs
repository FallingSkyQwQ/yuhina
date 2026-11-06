use serde::{Deserialize, Serialize};

use super::Result;
use super::paths::Paths;
use super::storage::{read_json, write_json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    Dark,
    Light,
    System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageMode {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadSource {
    Auto,
    Bmcl,
    Official,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReduceMotion {
    System,
    On,
    Off,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct JavaSettings {
    pub auto_detect: bool,
    pub auto_download: bool,
    pub default_runtime_id: Option<String>,
    pub manual_paths: Vec<String>,
}

impl Default for JavaSettings {
    fn default() -> Self {
        Self {
            auto_detect: true,
            auto_download: true,
            default_runtime_id: None,
            manual_paths: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InstanceDefaults {
    pub memory_mb: u32,
    pub java_path: Option<String>,
    pub jvm_args: String,
    pub game_args: String,
}

impl Default for InstanceDefaults {
    fn default() -> Self {
        Self {
            memory_mb: 4096,
            java_path: None,
            jvm_args: String::new(),
            game_args: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: ThemePreference,
    pub language: Language,
    pub language_mode: LanguageMode,
    pub download_source: DownloadSource,
    pub close_to_tray: bool,
    pub auto_check_updates: bool,
    pub reduce_motion: ReduceMotion,
    pub java: JavaSettings,
    pub instance_defaults: InstanceDefaults,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::Dark,
            language: Language::ZhCn,
            language_mode: LanguageMode::ZhCn,
            download_source: DownloadSource::Auto,
            close_to_tray: false,
            auto_check_updates: true,
            reduce_motion: ReduceMotion::System,
            java: JavaSettings::default(),
            instance_defaults: InstanceDefaults::default(),
        }
    }
}

impl Settings {
    pub fn effective_language(&self) -> Language {
        match self.language_mode {
            LanguageMode::System => system_language(),
            LanguageMode::ZhCn => Language::ZhCn,
            LanguageMode::EnUs => Language::EnUs,
        }
    }

    pub fn effective_theme(&self) -> bool {
        // Returns true for dark.
        match self.theme {
            ThemePreference::Dark => true,
            ThemePreference::Light => false,
            ThemePreference::System => true,
        }
    }
}

pub fn system_language() -> Language {
    let locale = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .unwrap_or_default()
        .to_ascii_lowercase();
    if locale.starts_with("zh") {
        Language::ZhCn
    } else {
        Language::EnUs
    }
}

pub fn load(paths: &Paths) -> Result<Settings> {
    Ok(read_json::<Settings>(&paths.settings_file())?.unwrap_or_default())
}

pub fn save(paths: &Paths, settings: &Settings) -> Result<()> {
    write_json(&paths.settings_file(), settings)
}
