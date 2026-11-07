mod brand_assets;
mod core;
mod ui;

use gpui_kit::{prelude::*, *};
use gpui_kit_material::color::themes::ThemeMode;
use gpui_kit_material::{icons::Assets, theme};

use brand_assets::BrandAssets;
use core::Paths;
use core::settings::{self, ReduceMotion};
use ui::{APP_CONTRAST, APP_THEME, Yuhina};

fn window_size() -> Size<Pixels> {
    if let Ok(value) = std::env::var("YUHINA_SIZE")
        && let Some((width, height)) = value.split_once('x')
        && let (Ok(width), Ok(height)) = (width.parse::<f32>(), height.parse::<f32>())
    {
        return size(px(width), px(height));
    }
    size(px(1440.), px(900.))
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
            let dark_override = std::env::var("YUHINA_DARK").ok().map(|value| value != "0");
            let mode = if dark_override.unwrap_or_else(|| settings.effective_theme()) {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            };
            let scheme = APP_THEME.scheme(mode, APP_CONTRAST);
            theme::init_component_theme(scheme, 1.0, cx);
            cx.set_reduce_motion(matches!(settings.reduce_motion, ReduceMotion::On));

            let (window, _content) = gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(24.), px(24.)),
                        window_size(),
                    ))),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Yuhina".into()),
                        appears_transparent: false,
                        traffic_light_position: None,
                    }),
                    focus: explicit_focus,
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| Yuhina::new(window, cx)),
            )
            .expect("the main window could not be created");

            let _ = &window;
            if explicit_focus {
                cx.activate(true);
            }
        });
}
