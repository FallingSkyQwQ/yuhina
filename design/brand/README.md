# Yuhina brand

## Files

- `yuhina-mark.svg` — in-app mark, `currentColor`, transparent background.
  Use it in the top bar and any brand lockup; tint it with a semantic role.
- `yuhina-icon.svg` — 1024×1024 app-icon source, transparent background,
  neutral light mark for dark taskbars/docks.
- `yuhina-icon.png` — 1024×1024 raster of the icon source.

The mark is a single geometric Y: two swept wings meeting at the center and a
tapered stem. Keep it monochrome; do not add a background, gradient, shadow or
game texture.

## Usage

- In-app: `svg().path("brand/yuhina-mark.svg").text_color(color(ColorRole::PRIMARY))`
  through `BrandAssets`; the mark is decorative next to the accessible title.
- Executable: `resources/icons/yuhina.ico` is a multi-size ICO (16, 24, 32, 48,
  64, 128, 256) embedded by `build.rs` with `winres` on Windows.
- Do not substitute text, an emoji or a generated placeholder for the mark.

## Regenerating the icon

Rasterize `yuhina-icon.svg` at 16/24/32/48/64/128/256/1024 with a transparent
background (fill `#EDEDED`), write the multi-size ICO, and place it at
`resources/icons/yuhina.ico`. The current assets were generated with
System.Drawing + a small ICO packer; any SVG rasterizer that preserves the
transparent background is acceptable. Commit the ICO and the 1024 PNG when the
source changes.

`build.rs` sets `ProductName` and `FileDescription` to `Yuhina`.
