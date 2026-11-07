use std::borrow::Cow;

use gpui_kit::{AssetSource, Result, SharedString};

/// In-app brand mark, tinted with `currentColor`.
pub const BRAND_MARK: &str = "brand/yuhina-mark.svg";
/// 1024×1024 application icon source.
pub const BRAND_ICON: &str = "brand/yuhina-icon.svg";

/// Serves the Yuhina brand SVGs alongside the Material Symbols asset source.
pub struct BrandAssets;

impl AssetSource for BrandAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match path {
            BRAND_MARK => Ok(Some(Cow::Borrowed(include_bytes!(
                "../design/brand/yuhina-mark.svg"
            )))),
            BRAND_ICON => Ok(Some(Cow::Borrowed(include_bytes!(
                "../design/brand/yuhina-icon.svg"
            )))),
            _ => Ok(None),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        if path.is_empty() {
            Ok(vec![BRAND_MARK.into(), BRAND_ICON.into()])
        } else {
            Ok(Vec::new())
        }
    }
}
