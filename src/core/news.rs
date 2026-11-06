use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::http;
use super::paths::Paths;
use super::storage::read_json;
use super::{Result, YuhinaError};

const NEWS_URL: &str = "https://launchercontent.mojang.com/v2/news.json";
const IMAGE_BASE: &str = "https://launchercontent.mojang.com";
const NEWS_TTL_SECONDS: i64 = 10 * 60;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    pub date: String,
    pub text: String,
    pub url: String,
    pub image_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsState {
    pub items: Vec<NewsItem>,
    pub fetched_at: String,
    pub source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewsCache {
    fetched_at: String,
    items: Vec<NewsItem>,
}

fn fallback_items() -> Vec<NewsItem> {
    vec![
        NewsItem {
            id: "fallback-java-news".to_string(),
            title: "Minecraft: Java Edition news".to_string(),
            date: "2026-01-01".to_string(),
            text: "Read the latest Java Edition announcements on minecraft.net.".to_string(),
            url: "https://www.minecraft.net/en-us/news".to_string(),
            image_url: None,
        },
        NewsItem {
            id: "fallback-java-updates".to_string(),
            title: "Java Edition release notes".to_string(),
            date: "2026-01-01".to_string(),
            text: "Version changelogs and snapshot notes from Mojang.".to_string(),
            url: "https://www.minecraft.net/en-us/news".to_string(),
            image_url: None,
        },
    ]
}

fn cache_path(paths: &Paths) -> std::path::PathBuf {
    paths.root().join("cache").join("mojang-news.json")
}

pub fn load(paths: &Paths, force: bool) -> Result<NewsState> {
    let cached = read_json::<NewsCache>(&cache_path(paths))?.filter(|cache| {
        !cache.items.is_empty()
            && chrono::DateTime::parse_from_rfc3339(&cache.fetched_at)
                .map(|fetched| {
                    chrono::Utc::now().signed_duration_since(fetched.with_timezone(&chrono::Utc))
                        < chrono::Duration::seconds(NEWS_TTL_SECONDS)
                })
                .unwrap_or(false)
    });

    if !force && let Some(cache) = cached {
        return Ok(NewsState {
            items: cache.items,
            fetched_at: cache.fetched_at,
            source: "cache".to_string(),
        });
    }

    match fetch() {
        Ok(items) if !items.is_empty() => {
            let fetched_at = chrono::Utc::now().to_rfc3339();
            let _ = super::storage::write_json(
                &cache_path(paths),
                &NewsCache {
                    fetched_at: fetched_at.clone(),
                    items: items.clone(),
                },
            );
            Ok(NewsState {
                items,
                fetched_at,
                source: "network".to_string(),
            })
        }
        _ => {
            if let Some(cache) = read_json::<NewsCache>(&cache_path(paths))?
                && !cache.items.is_empty()
            {
                return Ok(NewsState {
                    items: cache.items,
                    fetched_at: cache.fetched_at,
                    source: "cache".to_string(),
                });
            }
            Ok(NewsState {
                items: fallback_items(),
                fetched_at: chrono::Utc::now().to_rfc3339(),
                source: "fallback".to_string(),
            })
        }
    }
}

fn fetch() -> Result<Vec<NewsItem>> {
    let value: Value = http::get_json(NEWS_URL)?;
    let entries = value["entries"]
        .as_array()
        .ok_or_else(|| YuhinaError::invalid("Mojang news returned an unexpected format."))?;
    let mut items: Vec<NewsItem> = entries
        .iter()
        .filter(|entry| is_java_entry(entry))
        .filter_map(to_item)
        .collect();
    items.sort_by(|a, b| b.date.cmp(&a.date));
    items.truncate(6);
    Ok(items)
}

fn is_java_entry(entry: &Value) -> bool {
    let tags: Vec<String> = entry["newsType"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|value| value.to_ascii_lowercase()))
                .collect()
        })
        .unwrap_or_default();
    if tags.iter().any(|tag| {
        matches!(
            tag.as_str(),
            "bedrock" | "dungeons" | "dungeonsii" | "legends" | "education"
        )
    }) {
        return false;
    }
    let category = entry["category"].as_str().unwrap_or("");
    let category_lower = category.to_ascii_lowercase();
    if category_lower.contains("bedrock")
        || category_lower.contains("dungeons")
        || category_lower.contains("legends")
        || category_lower.contains("education")
    {
        return false;
    }
    let haystack = format!(
        "{} {} {}",
        entry["title"].as_str().unwrap_or(""),
        entry["text"].as_str().unwrap_or(""),
        entry["readMoreLink"].as_str().unwrap_or("")
    )
    .to_ascii_lowercase();
    if haystack.contains("realms") {
        return false;
    }
    if tags.iter().any(|tag| tag == "java") || category_lower.contains("java edition") {
        return true;
    }
    let non_java = [
        "bedrock edition",
        "minecraft dungeons",
        "minecraft legends",
        "education edition",
        "marketplace",
    ];
    if non_java.iter().any(|needle| haystack.contains(needle)) {
        return false;
    }
    ["java edition", "snapshot", "release candidate", "changelog"]
        .iter()
        .any(|needle| haystack.contains(needle))
}

fn to_item(entry: &Value) -> Option<NewsItem> {
    let url = entry["readMoreLink"].as_str()?;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return None;
    }
    let image = entry["newsPageImage"]["url"]
        .as_str()
        .or_else(|| entry["playPageImage"]["url"].as_str())
        .map(|path| format!("{IMAGE_BASE}{path}"))
        .filter(|url| url.starts_with("https://") || url.starts_with("http://"));
    Some(NewsItem {
        id: entry["id"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("news-{}", entry["date"].as_str().unwrap_or(""))),
        title: entry["title"]
            .as_str()
            .unwrap_or("Minecraft news")
            .to_string(),
        date: entry["date"].as_str().unwrap_or("").to_string(),
        text: entry["text"].as_str().unwrap_or("").to_string(),
        url: url.to_string(),
        image_url: image,
    })
}
