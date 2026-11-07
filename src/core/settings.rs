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

/// User text scale. The UI passes this to the component theme initialiser.
pub const TEXT_SCALE_DEFAULT: f32 = 1.0;
pub const TEXT_SCALE_MIN: f32 = 0.8;
pub const TEXT_SCALE_MAX: f32 = 2.0;

fn default_text_scale() -> f32 {
    TEXT_SCALE_DEFAULT
}

/// Clamps a text scale to the supported range. Non-finite values fall back to
/// the default so a corrupt settings file cannot break layout.
pub fn clamp_text_scale(value: f32) -> f32 {
    if !value.is_finite() {
        return TEXT_SCALE_DEFAULT;
    }
    value.clamp(TEXT_SCALE_MIN, TEXT_SCALE_MAX)
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
    #[serde(default = "default_text_scale")]
    pub text_scale: f32,
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
            text_scale: TEXT_SCALE_DEFAULT,
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
    let mut settings = read_json::<Settings>(&paths.settings_file())?.unwrap_or_default();
    settings.text_scale = clamp_text_scale(settings.text_scale);
    Ok(settings)
}

pub fn save(paths: &Paths, settings: &Settings) -> Result<()> {
    let mut clamped = settings.clone();
    clamped.text_scale = clamp_text_scale(clamped.text_scale);
    write_json(&paths.settings_file(), &clamped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_scale_defaults_to_one() {
        assert_eq!(Settings::default().text_scale, TEXT_SCALE_DEFAULT);
        assert_eq!(TEXT_SCALE_DEFAULT, 1.0);
    }

    #[test]
    fn text_scale_is_clamped_and_non_finite_values_are_rejected() {
        assert_eq!(clamp_text_scale(0.5), TEXT_SCALE_MIN);
        assert_eq!(clamp_text_scale(3.5), TEXT_SCALE_MAX);
        assert_eq!(clamp_text_scale(1.25), 1.25);
        assert_eq!(clamp_text_scale(f32::NAN), TEXT_SCALE_DEFAULT);
        assert_eq!(clamp_text_scale(f32::INFINITY), TEXT_SCALE_DEFAULT);
    }

    #[test]
    fn old_settings_without_text_scale_get_the_default() {
        let json = r#"{
            "theme": "light",
            "language": "zh-CN",
            "languageMode": "en-US",
            "downloadSource": "bmcl",
            "closeToTray": true,
            "autoCheckUpdates": false,
            "reduceMotion": "off",
            "java": {
                "autoDetect": false,
                "autoDownload": true,
                "defaultRuntimeId": "temurin-21",
                "manualPaths": ["C:/Java/bin/java.exe"]
            },
            "instanceDefaults": {
                "memoryMb": 8192,
                "jvmArgs": "-XX:+UseG1GC",
                "gameArgs": ""
            }
        }"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.text_scale, TEXT_SCALE_DEFAULT);
        assert_eq!(settings.instance_defaults.memory_mb, 8192);
        assert!(settings.close_to_tray);
    }

    #[test]
    fn loading_and_saving_clamp_the_text_scale() {
        let root =
            std::env::temp_dir().join(format!("yuhina-settings-test-{}", uuid::Uuid::new_v4()));
        let paths = Paths::at(root.clone());
        std::fs::create_dir_all(&root).unwrap();
        let json = r#"{"textScale": 3.5}"#;
        std::fs::write(paths.settings_file(), json).unwrap();
        assert_eq!(load(&paths).unwrap().text_scale, TEXT_SCALE_MAX);

        let settings = Settings {
            text_scale: 0.1,
            ..Settings::default()
        };
        save(&paths, &settings).unwrap();
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(paths.settings_file()).unwrap()).unwrap();
        let saved_scale = saved["textScale"].as_f64().unwrap();
        assert!((saved_scale - f64::from(TEXT_SCALE_MIN)).abs() < 1e-6);
        assert_eq!(load(&paths).unwrap().text_scale, TEXT_SCALE_MIN);

        let _ = std::fs::remove_dir_all(root);
    }
}
