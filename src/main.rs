mod brand_assets;
mod core;
mod ui;

use gpui_kit::{prelude::*, *};
use gpui_kit_material::color::themes::ThemeMode;
use gpui_kit_material::{icons::Assets, theme};

use brand_assets::BrandAssets;
use core::Paths;
use core::settings::{self, Language, LanguageMode, ReduceMotion, ThemePreference};
use ui::{APP_CONTRAST, APP_THEME, Yuhina};

/// The smallest window the compact shell is designed for. The product is
/// checked at 800x600, but keeping the floor at 480x600 lets the compact
/// navigation and 500x700 review capture work without overlap.
const MIN_WINDOW_WIDTH: f32 = 480.;
const MIN_WINDOW_HEIGHT: f32 = 600.;

fn window_size() -> Size<Pixels> {
    if let Ok(value) = std::env::var("YUHINA_SIZE")
        && let Some((width, height)) = value.split_once('x')
        && let (Ok(width), Ok(height)) = (width.parse::<f32>(), height.parse::<f32>())
    {
        return size(px(width), px(height));
    }
    size(px(1440.), px(900.))
}

/// Keep the opening window on-screen: clamp it to the primary display's
/// visible area and centre it there. A fixed 1440x900 frame at (24, 24) is
/// larger than a 1366x768 laptop work area.
fn initial_bounds(cx: &App) -> Bounds<Pixels> {
    let wanted = window_size();
    let Some(display) = cx.primary_display() else {
        return Bounds::new(point(px(24.), px(24.)), wanted);
    };
    let area = display.visible_bounds();
    let margin = 24.;
    let area_width = f32::from(area.size.width);
    let area_height = f32::from(area.size.height);
    let width = f32::from(wanted.width)
        .min((area_width - margin * 2.).max(MIN_WINDOW_WIDTH))
        .max(MIN_WINDOW_WIDTH.min(area_width));
    let height = f32::from(wanted.height)
        .min((area_height - margin * 2.).max(MIN_WINDOW_HEIGHT))
        .max(MIN_WINDOW_HEIGHT.min(area_height));
    let left = f32::from(area.origin.x) + (area_width - width) / 2.;
    let top = f32::from(area.origin.y) + (area_height - height) / 2.;
    Bounds::new(point(px(left), px(top)), size(px(width), px(height)))
}

/// Resolve the persisted preference against the OS appearance.
fn resolve_dark(preference: ThemePreference, system_dark: bool) -> bool {
    match preference {
        ThemePreference::Dark => true,
        ThemePreference::Light => false,
        ThemePreference::System => system_dark,
    }
}

fn resolve_language(preference: LanguageMode, current: Language) -> Language {
    match preference {
        LanguageMode::ZhCn => Language::ZhCn,
        LanguageMode::EnUs => Language::EnUs,
        LanguageMode::System => ui::platform::system_language().unwrap_or(current),
    }
}

fn resolve_reduce_motion(preference: ReduceMotion) -> bool {
    match preference {
        ReduceMotion::On => true,
        ReduceMotion::Off => false,
        ReduceMotion::System => ui::platform::system_reduce_motion(),
    }
}

fn main() {
    // Focus is opt-in: agent/test runs and accidental launches must not steal
    // the desktop focus. A user launch can request focus with YUHINA_FOCUS=1.
    let explicit_focus = std::env::var_os("YUHINA_FOCUS").is_some();

    gpui_kit::application()
        .with_assets(Assets::with(BrandAssets))
        .with_quit_mode(QuitMode::LastWindowClosed)
        .run(move |cx| {
            gpui_kit::init(cx);
            gpui_kit_material::init(cx);

            let paths = Paths::discover().unwrap_or_else(|_| Paths::at("."));
            let settings = settings::load(&paths).unwrap_or_default();
            let system_dark = ui::platform::system_dark(matches!(
                cx.window_appearance(),
                WindowAppearance::Dark | WindowAppearance::VibrantDark
            ));
            let dark_override = std::env::var("YUHINA_DARK").ok().map(|value| value != "0");
            let dark = dark_override.unwrap_or_else(|| resolve_dark(settings.theme, system_dark));
            let mode = if dark {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            };
            let language = resolve_language(settings.language_mode, settings.language);
            let scheme = APP_THEME.scheme(mode, APP_CONTRAST);
            theme::init_component_theme(scheme, settings.text_scale, cx);
            cx.set_reduce_motion(resolve_reduce_motion(settings.reduce_motion));
            cx.set_window_appearance(Some(if dark {
                WindowAppearance::Dark
            } else {
                WindowAppearance::Light
            }));

            let (window, _content) = gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(initial_bounds(cx))),
                    titlebar: Some(TitlebarOptions {
                        title: Some(ui::i18n::tr(language, "nav.home").into()),
                        appears_transparent: false,
                        traffic_light_position: None,
                    }),
                    window_min_size: Some(size(px(MIN_WINDOW_WIDTH), px(MIN_WINDOW_HEIGHT))),
                    focus: explicit_focus,
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| Yuhina::new(window, cx)),
            )
            .expect("the main window could not be created");

            ui::platform::sync_caption_for_handle(&window, dark, cx);
            let _ = &window;
            if explicit_focus {
                cx.activate(true);
            }
        });
}
