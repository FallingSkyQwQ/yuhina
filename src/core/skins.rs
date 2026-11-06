//! Skin head avatars: download a player's skin once, crop the 8×8 face (and
//! hat layer when present), and cache a 64×64 PNG for the account list.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use sha1::{Digest, Sha1};

use super::download::{DownloadTask, download_all};
use super::paths::Paths;
use super::{ErrorCode, Result, YuhinaError};

const SKIN_SIZE: u32 = 64;
const FACE_OFFSET: u32 = 8;
const HAT_OFFSET: u32 = 40;
const HEAD_SCALE: u32 = 64;

/// Cache key includes a short hash of the skin URL so a changed skin does not
/// keep showing the previous head.
pub fn head_path(paths: &Paths, account_id: &str, skin_url: &str) -> PathBuf {
    let safe_id: String = account_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .take(48)
        .collect();
    let digest = Sha1::digest(skin_url.as_bytes());
    let fingerprint = hex::encode(&digest[..8]);
    paths
        .root()
        .join("cache")
        .join("skins")
        .join(format!("{safe_id}-{fingerprint}.png"))
}

/// Downloads and crops a skin if the cached head is missing.
pub fn ensure_head(paths: &Paths, account_id: &str, skin_url: &str) -> Result<PathBuf> {
    let target = head_path(paths, account_id, skin_url);
    if target.is_file() {
        return Ok(target);
    }
    validate_skin_url(skin_url)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = target.with_extension("skin.png");
    let _ = fs::remove_file(&raw);
    let task = DownloadTask {
        destination: raw.clone(),
        urls: vec![skin_url.to_string()],
        sha1: None,
        sha512: None,
        size: None,
        label: "skin".to_string(),
    };
    let result = download_all(vec![task], 1, Arc::new(AtomicBool::new(false)), |_| {})
        .and_then(|_| crop_skin_head(&raw, &target));
    let _ = fs::remove_file(&raw);
    result?;
    Ok(target)
}

fn validate_skin_url(value: &str) -> Result<()> {
    let url = url::Url::parse(value).map_err(|_| {
        YuhinaError::new(ErrorCode::InvalidInput, "The account skin URL is invalid.")
    })?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(YuhinaError::new(
            ErrorCode::InvalidInput,
            "The account skin URL is unsafe.",
        ));
    }
    Ok(())
}

/// Crops the face and hat layers into a square PNG. Exposed for tests.
pub fn crop_skin_head(skin: &std::path::Path, target: &std::path::Path) -> Result<()> {
    let image = image::open(skin).map_err(|error| {
        YuhinaError::new(
            ErrorCode::Parse,
            format!("The skin image is invalid: {error}"),
        )
    })?;
    let (width, height) = (image.width(), image.height());
    if width < SKIN_SIZE || height < SKIN_SIZE / 2 {
        return Err(YuhinaError::new(
            ErrorCode::InvalidInput,
            "The skin image is smaller than 64×32.",
        ));
    }
    let unit = (width / SKIN_SIZE).max(1);
    let face = image.crop_imm(
        FACE_OFFSET * unit,
        FACE_OFFSET * unit,
        FACE_OFFSET * unit,
        FACE_OFFSET * unit,
    );
    let mut head = image::imageops::resize(
        &face,
        HEAD_SCALE,
        HEAD_SCALE,
        image::imageops::FilterType::Nearest,
    );
    let hat = image.crop_imm(
        HAT_OFFSET * unit,
        FACE_OFFSET * unit,
        FACE_OFFSET * unit,
        FACE_OFFSET * unit,
    );
    let hat = image::imageops::resize(
        &hat,
        HEAD_SCALE,
        HEAD_SCALE,
        image::imageops::FilterType::Nearest,
    );
    image::imageops::overlay(&mut head, &hat, 0, 0);

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = target.with_extension("tmp.png");
    head.save_with_format(&tmp, image::ImageFormat::Png)
        .map_err(|error| YuhinaError::new(ErrorCode::Io, error.to_string()))?;
    fs::rename(&tmp, target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_cache_names_are_stable_and_url_specific() {
        let paths = Paths::at("data");
        let a = head_path(&paths, "account-id", "https://example.com/a.png");
        let b = head_path(&paths, "account-id", "https://example.com/a.png");
        let c = head_path(&paths, "account-id", "https://example.com/b.png");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn cropping_takes_the_face_pixels() {
        let root = std::env::temp_dir().join(format!("yuhina-skin-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let skin = root.join("skin.png");
        let head = root.join("head.png");

        let mut source = image::RgbaImage::new(64, 64);
        for x in 8..16 {
            for y in 8..16 {
                source.put_pixel(x, y, image::Rgba([220, 20, 20, 255]));
            }
        }
        source.save(&skin).unwrap();

        crop_skin_head(&skin, &head).unwrap();
        let result = image::open(&head).unwrap().to_rgba8();
        assert_eq!(result.width(), 64);
        assert_eq!(result.height(), 64);
        assert_eq!(result.get_pixel(0, 0).0[0..3], [220, 20, 20]);
        let _ = fs::remove_dir_all(root);
    }
}
