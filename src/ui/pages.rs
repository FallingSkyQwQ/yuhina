use super::*;
use crate::core::instances::{self, InstanceRecord, MemoryMode};

pub fn render(app: &mut Yuhina, window: &mut Window, cx: &mut Context<Yuhina>) -> AnyElement {
    match app.page {
        Page::Home => home(app, window, cx),
        Page::Instances => instances_page(app, window, cx),
        Page::Library => library(app, window, cx),
        Page::Accounts => accounts_page(app, window, cx),
        Page::Servers => servers_page(app, window, cx),
        Page::Activity => activity(app, window, cx),
        Page::Settings => settings_page(app, window, cx),
    }
}

// ---------------------------------------------------------------------------
// Shared building blocks
// ---------------------------------------------------------------------------

fn type_scale<E: Styled>(element: E, scale: TypeScale) -> E {
    element
        .font_family(scale.font_family)
        .text_size(scale.font_size)
        .letter_spacing(scale.letter_spacing)
        .line_height(scale.line_height)
        .font_weight(FontWeight(scale.font_weight as f32))
}

fn type_role<E: Styled>(element: E, role: TypeRole) -> E {
    type_scale(element, role.scale())
}

fn page_frame(window: &Window, header: AnyElement, body: AnyElement) -> AnyElement {
    // An explicit width avoids percentage-width-plus-padding overflow in the
    // flex layout. The column is the viewport minus the navigation and two
    // gutters, capped at the 1120dp content measure.
    let viewport = f32::from(window.viewport_size().width);
    let navigation = if viewport >= 840. {
        220.
    } else if viewport >= 600. {
        96.
    } else {
        0.
    };
    // The extra 32dp absorbs the horizontal padding the page's own padded
    // surfaces (hero, panels) add outside their width in GPUI's layout, so a
    // full-width child never paints past the right gutter.
    let content_width = (viewport - navigation - 96.).clamp(0., 1120.);
    div()
        .id("page-scroll")
        .w_full()
        .h_full()
        .min_w_0()
        .overflow_y_scroll()
        .overflow_x_hidden()
        .child(
            div().w_full().min_w_0().flex().justify_center().child(
                div()
                    .w(px(content_width))
                    .min_w_0()
                    .py(px(32.))
                    .flex()
                    .flex_col()
                    .items_stretch()
                    .gap(px(24.))
                    .child(header)
                    .child(body),
            ),
        )
        .into_any_element()
}

fn page_header(
    app: &Yuhina,
    title_key: &str,
    subtitle_key: &str,
    actions: Option<AnyElement>,
) -> AnyElement {
    div()
        .min_w_0()
        .flex()
        .items_end()
        .justify_between()
        .gap(px(16.))
        .flex_wrap()
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .flex_1()
                .min_w_0()
                .child(
                    type_scale(div().min_w_0(), TypeRole::HeadlineLarge.emphasized())
                        .child(app.t(title_key)),
                )
                .child(
                    type_role(div().min_w_0(), TypeRole::BodyMedium)
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .child(app.t(subtitle_key)),
                ),
        )
        .children(actions.map(|actions| {
            div()
                .min_w_0()
                .flex()
                .flex_wrap()
                .justify_end()
                .gap(px(8.))
                .child(actions)
        }))
        .into_any_element()
}

fn panel() -> Div {
    div()
        .rounded(px(24.))
        .bg(color(ColorRole::SURFACE_CONTAINER_LOW))
        .p(px(20.))
}

fn section_label(app: &Yuhina, key: &str) -> Div {
    type_scale(div(), TypeRole::LabelLarge.emphasized())
        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
        .child(app.t(key))
}

fn muted(text: impl Into<SharedString>) -> Div {
    type_role(div(), TypeRole::BodySmall)
        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
        .child(text.into())
}

fn body_text(text: impl Into<SharedString>) -> Div {
    type_role(div(), TypeRole::BodyMedium).child(text.into())
}

fn chip(text: impl Into<SharedString>, tone: ColorRole, text_role: ColorRole) -> Div {
    type_role(div(), TypeRole::LabelMedium)
        .px(px(10.))
        .py(px(4.))
        .rounded(px(999.))
        .bg(color(tone))
        .text_color(color(text_role))
        .child(text.into())
}

fn loader_text(app: &Yuhina, loader: &instances::Loader) -> String {
    app.loader_text(loader)
}

fn playtime_text(app: &Yuhina, millis: u64) -> String {
    let minutes = millis / 60_000;
    if minutes < 60 {
        app.t("instances.playtime_minutes")
            .replace("{m}", &minutes.to_string())
    } else {
        app.t("instances.playtime_hours")
            .replace("{h}", &(minutes / 60).to_string())
            .replace("{m}", &(minutes % 60).to_string())
    }
}

fn error_panel(app: &Yuhina, message: &str, retry: Option<AnyElement>) -> AnyElement {
    div()
        .w_full()
        .rounded(px(24.))
        .bg(color(ColorRole::ERROR_CONTAINER))
        .text_color(color(ColorRole::ON_ERROR_CONTAINER))
        .p(px(20.))
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(type_role(div(), TypeRole::TitleSmall).child(app.t("common.error")))
        .child(body_text(message.to_string()))
        .child(muted(app.t("common.offline_hint")).text_color(color(ColorRole::ON_ERROR_CONTAINER)))
        .children(retry)
        .into_any_element()
}

fn loading_panel(app: &Yuhina, label_key: &str) -> AnyElement {
    div()
        .w_full()
        .rounded(px(24.))
        .bg(color(ColorRole::SURFACE_CONTAINER_LOW))
        .p(px(20.))
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(body_text(app.t(label_key)))
        .child(
            LinearProgress::new("inline-loading", app.t("common.loading"))
                .value(None::<f32>)
                .thickness(px(4.)),
        )
        .into_any_element()
}

fn empty_state(
    app: &Yuhina,
    icon: &str,
    title_key: &str,
    body_key: &str,
    action: Option<AnyElement>,
) -> AnyElement {
    div()
        .w_full()
        .py(px(48.))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(12.))
        .child(
            div()
                .size(px(64.))
                .rounded(px(22.))
                .bg(color(ColorRole::SECONDARY_CONTAINER))
                .flex()
                .items_center()
                .justify_center()
                .text_color(color(ColorRole::ON_SECONDARY_CONTAINER))
                .child(material_icon(icon, 30.0)),
        )
        .child(type_scale(div(), TypeRole::HeadlineSmall.emphasized()).child(app.t(title_key)))
        .child(
            type_role(div(), TypeRole::BodyMedium)
                .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                .child(app.t(body_key)),
        )
        .children(action)
        .into_any_element()
}

fn grid() -> Div {
    div().w_full().flex().flex_wrap().gap(px(16.))
}

fn card_shell() -> Div {
    div()
        .flex_1()
        .min_w(px(260.))
        .rounded(px(24.))
        .bg(color(ColorRole::SURFACE_CONTAINER))
        .p(px(20.))
        .flex()
        .flex_col()
        .gap(px(12.))
}

fn progress_value(fraction: f32) -> f32 {
    fraction.clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Home
// ---------------------------------------------------------------------------

fn home(app: &mut Yuhina, window: &mut Window, cx: &mut Context<Yuhina>) -> AnyElement {
    let name = app
        .accounts
        .iter()
        .find(|account| account.active)
        .map(|account| account.name.clone())
        .unwrap_or_else(|| "Steve".to_string());
    let greeting = app.t("home.greeting").replace("{name}", &name);

    let recent: Vec<_> = app.instances.iter().take(3).cloned().collect();
    let recent_cards: Vec<AnyElement> = recent
        .iter()
        .map(|summary| {
            let record = summary.record.clone();
            let id = record.id.clone();
            let open_id = id.clone();
            let play_id = id.clone();
            let installed = summary.installed;
            let running = app.is_game_running(&id);
            card_shell()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(
                            div()
                                .size(px(44.))
                                .rounded(px(14.))
                                .bg(color(ColorRole::TERTIARY_CONTAINER))
                                .text_color(color(ColorRole::ON_TERTIARY_CONTAINER))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(material_icon("sports_esports", 24.0)),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    type_role(div(), TypeRole::TitleMedium)
                                        .child(record.name.clone()),
                                )
                                .child(muted(format!(
                                    "{} · {}",
                                    record.version_id,
                                    loader_text(app, &record.loader)
                                ))),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(if running {
                            chip(
                                app.t("instances.running"),
                                ColorRole::TERTIARY_CONTAINER,
                                ColorRole::ON_TERTIARY_CONTAINER,
                            )
                        } else if installed {
                            chip(
                                app.t("instances.installed"),
                                ColorRole::SECONDARY_CONTAINER,
                                ColorRole::ON_SECONDARY_CONTAINER,
                            )
                        } else {
                            chip(
                                app.t("instances.not_installed"),
                                ColorRole::SURFACE_CONTAINER_HIGHEST,
                                ColorRole::ON_SURFACE_VARIANT,
                            )
                        })
                        .child(
                            div()
                                .flex()
                                .gap(px(8.))
                                .child(
                                    MaterialButton::new(format!("home-open-{id}"))
                                        .variant(ButtonVariant::Text)
                                        .button_size(ButtonSize::Small)
                                        .child(app.t("common.details"))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.open_instance(open_id.clone(), cx)
                                        })),
                                )
                                .child(if running {
                                    MaterialButton::new(format!("home-stop-{id}"))
                                        .variant(ButtonVariant::Tonal)
                                        .button_size(ButtonSize::Small)
                                        .icon(material_icon("stop", 18.0))
                                        .child(app.t("instances.stop"))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.stop_game(&play_id, cx)
                                        }))
                                        .into_any_element()
                                } else {
                                    MaterialButton::new(format!("home-play-{id}"))
                                        .variant(ButtonVariant::Filled)
                                        .button_size(ButtonSize::Small)
                                        .icon(material_icon("play_arrow", 18.0))
                                        .child(app.t("home.play"))
                                        .disabled(!installed)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.launch_instance(play_id.clone(), None, cx)
                                        }))
                                        .into_any_element()
                                }),
                        ),
                )
                .into_any_element()
        })
        .collect();

    let news_cards: Vec<AnyElement> = app
        .news
        .items
        .iter()
        .take(4)
        .map(|item| {
            let url = item.url.clone();
            div()
                .flex_1()
                .min_w(px(260.))
                .rounded(px(20.))
                .bg(color(ColorRole::SURFACE_CONTAINER_LOW))
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(type_role(div(), TypeRole::TitleSmall).child(item.title.clone()))
                .child(muted(item.date.clone()))
                .child(
                    type_role(div(), TypeRole::BodySmall)
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .child(item.text.clone()),
                )
                .child(
                    MaterialButton::new(format!("news-{}", item.id))
                        .variant(ButtonVariant::Text)
                        .button_size(ButtonSize::ExtraSmall)
                        .icon(material_icon("open_in_new", 16.0))
                        .child(app.t("home.read_more"))
                        .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))),
                )
                .into_any_element()
        })
        .collect();

    let hero = div()
        .rounded(px(32.))
        .bg(color(ColorRole::PRIMARY_CONTAINER))
        .text_color(color(ColorRole::ON_PRIMARY_CONTAINER))
        .p(px(32.))
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(type_scale(div(), TypeRole::LabelLarge.emphasized()).child(app.t("app.tagline")))
        .child(type_scale(div(), TypeRole::DisplaySmall.emphasized()).child(greeting))
        .child(type_role(div(), TypeRole::BodyLarge).child(app.t("home.subtitle")))
        .child(
            div()
                .flex()
                .gap(px(12.))
                .mt(px(8.))
                .child(
                    MaterialButton::new("home-new-instance")
                        .variant(ButtonVariant::Filled)
                        .icon(material_icon("add", 20.0))
                        .child(app.t("home.create"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_create_dialog(cx);
                        })),
                )
                .when(!recent.is_empty(), |row| {
                    let id = recent[0].record.id.clone();
                    row.child(
                        MaterialButton::new("home-play-latest")
                            .variant(ButtonVariant::Filled)
                            .icon(material_icon("play_arrow", 20.0))
                            .child(app.t("home.play"))
                            .disabled(!recent[0].installed)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.launch_instance(id.clone(), None, cx)
                            })),
                    )
                }),
        );

    let recent_section = div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(section_label(app, "home.recent"))
                .child(
                    MaterialButton::new("home-all-instances")
                        .variant(ButtonVariant::Text)
                        .button_size(ButtonSize::ExtraSmall)
                        .child(app.t("home.view_all"))
                        .on_click(cx.listener(|this, _, _, cx| this.set_page(Page::Instances, cx))),
                ),
        )
        .child(if app.loading.instances && recent_cards.is_empty() {
            loading_panel(app, "instances.loading")
        } else if let Some(error) = app
            .instance_error
            .as_ref()
            .filter(|_| recent_cards.is_empty())
        {
            let retry = MaterialButton::new("home-instances-retry")
                .variant(ButtonVariant::Text)
                .icon(material_icon("refresh", 18.0))
                .child(app.t("common.retry"))
                .on_click(cx.listener(|this, _, _, cx| this.refresh_instances(cx)))
                .into_any_element();
            error_panel(app, error, Some(retry))
        } else if recent_cards.is_empty() {
            panel()
                .child(empty_state(
                    app,
                    "grid_view",
                    "home.no_instances",
                    "home.no_instances_body",
                    Some(
                        MaterialButton::new("home-create-empty")
                            .variant(ButtonVariant::Filled)
                            .icon(material_icon("add", 20.0))
                            .child(app.t("home.create"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_create_dialog(cx);
                            }))
                            .into_any_element(),
                    ),
                ))
                .into_any_element()
        } else {
            grid().children(recent_cards).into_any_element()
        });

    let news_section = div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(section_label(app, "home.news"))
        .child(if app.loading.news && news_cards.is_empty() {
            loading_panel(app, "home.news_loading")
        } else if let Some(error) = app.news_error.as_ref().filter(|_| news_cards.is_empty()) {
            let retry = MaterialButton::new("news-retry")
                .variant(ButtonVariant::Text)
                .icon(material_icon("refresh", 18.0))
                .child(app.t("common.retry"))
                .on_click(cx.listener(|this, _, _, cx| this.refresh_news(cx, true)))
                .into_any_element();
            error_panel(app, error, Some(retry))
        } else if news_cards.is_empty() {
            panel()
                .child(muted(app.t("home.news_empty")))
                .into_any_element()
        } else {
            grid().children(news_cards).into_any_element()
        });

    page_frame(
        window,
        hero.into_any_element(),
        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(recent_section)
            .child(news_section)
            .into_any_element(),
    )
}

// ---------------------------------------------------------------------------
// Instances
// ---------------------------------------------------------------------------

fn instances_page(app: &mut Yuhina, window: &mut Window, cx: &mut Context<Yuhina>) -> AnyElement {
    if let Some(id) = app.selected_instance.clone() {
        if app.instances.iter().any(|item| item.record.id == id) {
            return instance_detail(app, window, cx, id);
        }
        app.selected_instance = None;
    }

    let actions = Some(
        div()
            .flex()
            .gap(px(8.))
            .child(
                MaterialButton::new("instances-import")
                    .variant(ButtonVariant::Tonal)
                    .icon(material_icon("download", 20.0))
                    .child(app.t("backups.import"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.dialog = Some(Dialog::ImportInstance);
                        cx.notify();
                    })),
            )
            .child(
                MaterialButton::new("instances-new")
                    .variant(ButtonVariant::Filled)
                    .icon(material_icon("add", 20.0))
                    .child(app.t("instances.new"))
                    .on_click(cx.listener(|this, _, _, cx| this.open_create_dialog(cx))),
            )
            .into_any_element(),
    );
    let header = page_header(app, "instances.title", "instances.subtitle", actions);

    let rows: Vec<AnyElement> = app
        .instances
        .clone()
        .into_iter()
        .map(|summary| {
            let id = summary.record.id.clone();
            let open_id = id.clone();
            let play_id = id.clone();
            let running = app.is_game_running(&id);
            let status = if running {
                chip(
                    app.t("instances.running"),
                    ColorRole::TERTIARY_CONTAINER,
                    ColorRole::ON_TERTIARY_CONTAINER,
                )
            } else if summary.installed {
                chip(
                    app.t("instances.installed"),
                    ColorRole::SECONDARY_CONTAINER,
                    ColorRole::ON_SECONDARY_CONTAINER,
                )
            } else {
                chip(
                    app.t("instances.not_installed"),
                    ColorRole::SURFACE_CONTAINER_HIGHEST,
                    ColorRole::ON_SURFACE_VARIANT,
                )
            };
            div()
                .w_full()
                .rounded(px(20.))
                .bg(color(ColorRole::SURFACE_CONTAINER_LOW))
                .p(px(16.))
                .flex()
                .items_center()
                .gap(px(16.))
                .child(
                    div()
                        .size(px(48.))
                        .rounded(px(16.))
                        .bg(color(ColorRole::TERTIARY_CONTAINER))
                        .text_color(color(ColorRole::ON_TERTIARY_CONTAINER))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(material_icon("sports_esports", 26.0)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .child(
                            type_role(div(), TypeRole::TitleMedium)
                                .child(summary.record.name.clone()),
                        )
                        .child(muted(format!(
                            "{} · {} · {} MB",
                            summary.record.version_id,
                            loader_text(app, &summary.record.loader),
                            summary.record.memory_mb
                        ))),
                )
                .child(status.flex_shrink_0())
                .child(
                    div()
                        .flex()
                        .flex_shrink_0()
                        .gap(px(8.))
                        .child(
                            MaterialButton::new(format!("open-{id}"))
                                .variant(ButtonVariant::Text)
                                .button_size(ButtonSize::Small)
                                .child(app.t("common.details"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.open_instance(open_id.clone(), cx)
                                })),
                        )
                        .child(if running {
                            MaterialButton::new(format!("stop-{id}"))
                                .variant(ButtonVariant::Tonal)
                                .button_size(ButtonSize::Small)
                                .icon(material_icon("stop", 18.0))
                                .child(app.t("instances.stop"))
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.stop_game(&play_id, cx)),
                                )
                                .into_any_element()
                        } else {
                            MaterialButton::new(format!("play-{id}"))
                                .variant(ButtonVariant::Filled)
                                .button_size(ButtonSize::Small)
                                .icon(material_icon("play_arrow", 18.0))
                                .child(app.t("instances.play"))
                                .disabled(!summary.installed)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.launch_instance(play_id.clone(), None, cx)
                                }))
                                .into_any_element()
                        }),
                )
                .into_any_element()
        })
        .collect();

    let body = if app.loading.instances && rows.is_empty() {
        loading_panel(app, "instances.loading")
    } else if let Some(error) = app.instance_error.as_ref().filter(|_| rows.is_empty()) {
        let retry = MaterialButton::new("instances-retry")
            .variant(ButtonVariant::Text)
            .icon(material_icon("refresh", 18.0))
            .child(app.t("common.retry"))
            .on_click(cx.listener(|this, _, _, cx| this.refresh_instances(cx)))
            .into_any_element();
        error_panel(app, error, Some(retry))
    } else if rows.is_empty() {
        panel()
            .child(empty_state(
                app,
                "grid_view",
                "instances.empty",
                "instances.empty_body",
                Some(
                    MaterialButton::new("instances-create-empty")
                        .variant(ButtonVariant::Filled)
                        .icon(material_icon("add", 20.0))
                        .child(app.t("instances.new"))
                        .on_click(cx.listener(|this, _, _, cx| this.open_create_dialog(cx)))
                        .into_any_element(),
                ),
            ))
            .into_any_element()
    } else {
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .children(rows)
            .into_any_element()
    };

    page_frame(window, header, body)
}

fn instance_detail(
    app: &mut Yuhina,
    window: &mut Window,
    cx: &mut Context<Yuhina>,
    id: String,
) -> AnyElement {
    let summary = app
        .instances
        .iter()
        .find(|item| item.record.id == id)
        .cloned()
        .expect("the instance exists");
    let record = summary.record.clone();
    let running = app.is_game_running(&id);
    let install = app.installs.get(&id).cloned();

    let header = div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            MaterialButton::new("detail-back")
                .variant(ButtonVariant::Text)
                .button_size(ButtonSize::Small)
                .icon(material_icon("arrow_back", 18.0))
                .icon_position(ButtonIconPosition::Leading)
                .child(app.t("common.back"))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.selected_instance = None;
                    cx.notify()
                })),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(16.))
                .flex_wrap()
                .child(
                    div()
                        .size(px(64.))
                        .rounded(px(22.))
                        .bg(color(ColorRole::TERTIARY_CONTAINER))
                        .text_color(color(ColorRole::ON_TERTIARY_CONTAINER))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(material_icon("sports_esports", 32.0)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .flex_1()
                        .min_w(px(220.))
                        .child(
                            type_scale(div(), TypeRole::HeadlineLarge.emphasized())
                                .child(record.name.clone()),
                        )
                        .child(muted(format!(
                            "{} · {} · {} MB",
                            record.version_id,
                            loader_text(app, &record.loader),
                            instances::effective_memory_mb(&app.paths, &record)
                        ))),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(8.))
                        .flex_wrap()
                        .child(if running {
                            let stop_id = id.clone();
                            MaterialButton::new("detail-stop")
                                .variant(ButtonVariant::Tonal)
                                .icon(material_icon("stop", 20.0))
                                .child(app.t("instances.stop"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.stop_game(&stop_id, cx);
                                }))
                                .into_any_element()
                        } else {
                            MaterialButton::new("detail-play")
                                .variant(ButtonVariant::Filled)
                                .icon(material_icon("play_arrow", 20.0))
                                .child(app.t("instances.play"))
                                .disabled(!summary.installed)
                                .on_click(cx.listener({
                                    let id = id.clone();
                                    move |this, _, _, cx| this.launch_instance(id.clone(), None, cx)
                                }))
                                .into_any_element()
                        })
                        .child(
                            MaterialButton::new("detail-install")
                                .variant(ButtonVariant::Tonal)
                                .icon(material_icon(
                                    if summary.installed {
                                        "build"
                                    } else {
                                        "download"
                                    },
                                    20.0,
                                ))
                                .child(if summary.installed {
                                    app.t("instances.repair")
                                } else {
                                    app.t("instances.install")
                                })
                                .on_click(cx.listener({
                                    let id = id.clone();
                                    move |this, _, _, cx| this.start_install(id.clone(), cx)
                                })),
                        )
                        .child(
                            IconButton::new(
                                "detail-folder",
                                material_icon("folder_open", 22.0),
                                app.t("instances.open_folder"),
                            )
                            .on_click(cx.listener({
                                let id = id.clone();
                                move |this, _, _, cx| this.open_folder(&id, cx)
                            })),
                        )
                        .child(
                            IconButton::new(
                                "detail-duplicate",
                                material_icon("content_copy", 22.0),
                                app.t("instances.duplicate"),
                            )
                            .on_click(cx.listener({
                                let id = id.clone();
                                move |this, _, _, cx| this.duplicate_instance(id.clone(), cx)
                            })),
                        )
                        .child(
                            IconButton::new(
                                "detail-export",
                                material_icon("upload", 22.0),
                                app.t("backups.export"),
                            )
                            .on_click(cx.listener({
                                let id = id.clone();
                                move |this, _, _, cx| this.export_instance(id.clone(), cx)
                            })),
                        )
                        .child(
                            IconButton::new(
                                "detail-delete",
                                material_icon("delete", 22.0),
                                app.t("instances.delete"),
                            )
                            .on_click(cx.listener({
                                let id = id.clone();
                                move |this, _, _, cx| {
                                    this.dialog = Some(Dialog::DeleteInstance(id.clone()));
                                    cx.notify();
                                }
                            })),
                        ),
                ),
        )
        .children(install.map(|task| install_banner(app, &task)));

    let tabs = MaterialTabs::new("instance-tabs", app.t("instances.sections"))
        .items([
            TabItem::new("overview", app.t("instances.tab.overview")),
            TabItem::new("logs", app.t("instances.tab.logs")),
            TabItem::new("content", app.t("instances.tab.content")),
            TabItem::new("backups", app.t("instances.tab.backups")),
            TabItem::new("files", app.t("instances.tab.files")),
            TabItem::new("settings", app.t("instances.tab.settings")),
        ])
        .selected(match app.instance_tab {
            InstanceTab::Overview => "overview",
            InstanceTab::Logs => "logs",
            InstanceTab::Content => "content",
            InstanceTab::Backups => "backups",
            InstanceTab::Files => "files",
            InstanceTab::Settings => "settings",
        })
        .on_change(cx.listener(|this, value: &SharedString, _, cx| {
            let tab = match value.as_ref() {
                "logs" => InstanceTab::Logs,
                "content" => InstanceTab::Content,
                "backups" => InstanceTab::Backups,
                "files" => InstanceTab::Files,
                "settings" => InstanceTab::Settings,
                _ => InstanceTab::Overview,
            };
            this.set_instance_tab(tab, cx);
        }));

    let body: AnyElement = match app.instance_tab {
        InstanceTab::Overview => overview_tab(app, &record, &summary),
        InstanceTab::Logs => logs_tab(app, &id),
        InstanceTab::Content => content_tab(app, cx, &id),
        InstanceTab::Backups => backups_tab(app, cx, &id),
        InstanceTab::Files => files_tab(app, cx, &id),
        InstanceTab::Settings => settings_tab(app, cx, &record),
    };

    page_frame(
        window,
        header.into_any_element(),
        div()
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(tabs)
            .child(body)
            .into_any_element(),
    )
}

fn install_banner(app: &Yuhina, task: &InstallTask) -> AnyElement {
    let fraction = progress_value(task.percent / 100.0);
    let label = if let Some(error) = &task.error {
        error.clone()
    } else if task.finished {
        app.t("instances.install_done").to_string()
    } else {
        format!(
            "{} · {} · {:.0}%",
            app.t("instances.install_progress"),
            app.phase_label(task.phase),
            task.percent
        )
    };
    div()
        .w_full()
        .rounded(px(20.))
        .bg(color(if task.error.is_some() {
            ColorRole::ERROR_CONTAINER
        } else {
            ColorRole::SECONDARY_CONTAINER
        }))
        .text_color(color(if task.error.is_some() {
            ColorRole::ON_ERROR_CONTAINER
        } else {
            ColorRole::ON_SECONDARY_CONTAINER
        }))
        .p(px(16.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(type_role(div(), TypeRole::TitleSmall).child(label))
        .child(
            div().child(
                LinearProgress::new("install-progress", app.t("instances.install_progress_a11y"))
                    .value(Some(fraction))
                    .thickness(px(4.)),
            ),
        )
        .into_any_element()
}

fn overview_tab(app: &Yuhina, record: &InstanceRecord, summary: &InstanceSummary) -> AnyElement {
    let info = [
        (app.t("common.version"), record.version_id.clone()),
        (app.t("common.loader"), loader_text(app, &record.loader)),
        (
            app.t("common.memory"),
            format!("{} MB", instances::effective_memory_mb(&app.paths, record)),
        ),
        (
            app.t("common.java"),
            record
                .java_path
                .clone()
                .unwrap_or_else(|| app.t("common.none").to_string()),
        ),
        (
            app.t("instances.last_played"),
            record
                .last_played_at
                .clone()
                .unwrap_or_else(|| app.t("instances.never_played").to_string()),
        ),
        (
            app.t("instances.playtime"),
            playtime_text(app, record.playtime_ms),
        ),
    ];
    let info_rows: Vec<AnyElement> = info
        .into_iter()
        .map(|(label, value)| {
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .min_w(px(160.))
                .child(muted(label))
                .child(type_role(div(), TypeRole::TitleSmall).child(value))
                .into_any_element()
        })
        .collect();

    let logs = app
        .running
        .get(&record.id)
        .map(|game| {
            game.logs
                .iter()
                .rev()
                .take(12)
                .rev()
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let log_lines: Vec<AnyElement> = logs
        .into_iter()
        .map(|line| {
            type_role(div(), TypeRole::BodySmall)
                .line_height(rem(18.))
                .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                .child(line.text)
                .into_any_element()
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            panel()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(px(24.))
                .children(info_rows),
        )
        .child(
            panel()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(section_label(app, "instances.tab.logs"))
                .child(if log_lines.is_empty() {
                    muted(app.t("instances.logs_empty")).into_any_element()
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .children(log_lines)
                        .into_any_element()
                }),
        )
        .child(
            if summary.required_java_major.is_some() && !summary.installed {
                panel()
                    .child(
                        type_role(div(), TypeRole::BodySmall)
                            .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                            .child(format!(
                                "{}: Java {}",
                                app.t("common.java"),
                                summary.required_java_major.unwrap_or(21)
                            )),
                    )
                    .into_any_element()
            } else {
                div().into_any_element()
            },
        )
        .into_any_element()
}

fn logs_tab(app: &Yuhina, id: &str) -> AnyElement {
    let Some(game) = app.running.get(id) else {
        return panel()
            .child(muted(app.t("instances.logs_empty")))
            .into_any_element();
    };
    let lines: Vec<AnyElement> = game
        .logs
        .iter()
        .rev()
        .take(400)
        .rev()
        .map(|line| {
            let tint = match line.stream {
                crate::core::launch::LogStream::Stderr => ColorRole::ERROR,
                crate::core::launch::LogStream::System => ColorRole::PRIMARY,
                _ => ColorRole::ON_SURFACE_VARIANT,
            };
            type_role(div(), TypeRole::BodySmall)
                .line_height(rem(18.))
                .text_color(color(tint))
                .child(line.text.clone())
                .into_any_element()
        })
        .collect();
    div()
        .id("log-scroll")
        .w_full()
        .h(px(520.))
        .rounded(px(20.))
        .bg(color(ColorRole::SURFACE_CONTAINER_LOWEST))
        .p(px(16.))
        .overflow_y_scroll()
        .track_scroll(&game.scroll)
        .children(lines)
        .into_any_element()
}

fn content_tab(app: &Yuhina, cx: &mut Context<Yuhina>, id: &str) -> AnyElement {
    let project_types = [
        ("mod", "library.mods", "mods"),
        ("resourcepack", "library.resourcepacks", "resourcepacks"),
        ("shader", "library.shaders", "shaderpacks"),
    ];
    let mut sections = Vec::new();
    for (project_type, label, folder) in project_types {
        let directory = app.paths.game_dir(id).unwrap_or_default().join(folder);
        let mut files: Vec<(String, u64)> = std::fs::read_dir(&directory)
            .map(|entries| {
                entries
                    .flatten()
                    .filter_map(|entry| {
                        let path = entry.path();
                        if path.is_file() {
                            Some((
                                entry.file_name().to_string_lossy().to_string(),
                                entry.metadata().map(|meta| meta.len()).unwrap_or(0),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        let rows: Vec<AnyElement> = files
            .into_iter()
            .map(|(name, size)| {
                let remove_id = id.to_string();
                let remove_type = project_type.to_string();
                let remove_name = name.clone();
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .py(px(8.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .min_w_0()
                            .child(type_role(div(), TypeRole::TitleSmall).child(name))
                            .child(muted(format!("{:.1} MB", size as f64 / 1_048_576.0))),
                    )
                    .child(
                        IconButton::new(
                            format!("remove-{project_type}-{}", remove_name),
                            material_icon("delete", 18.0),
                            app.t("instances.remove_mod"),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.dialog = Some(Dialog::ConfirmRemoveContent {
                                instance_id: remove_id.clone(),
                                project_type: remove_type.clone(),
                                filename: remove_name.clone(),
                            });
                            cx.notify();
                        })),
                    )
                    .into_any_element()
            })
            .collect();
        sections.push(
            panel()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(section_label(app, label))
                        .child(
                            MaterialButton::new(format!("browse-{project_type}"))
                                .variant(ButtonVariant::Text)
                                .button_size(ButtonSize::ExtraSmall)
                                .icon(material_icon("explore", 16.0))
                                .child(app.t("instances.add_mod"))
                                .on_click(cx.listener({
                                    let id = id.to_string();
                                    let project_type = project_type.to_string();
                                    move |this, _, _, cx| {
                                        this.library_target = Some(id.clone());
                                        this.library_type = project_type.clone();
                                        this.page = Page::Library;
                                        this.search_library(cx);
                                    }
                                })),
                        ),
                )
                .child(if rows.is_empty() {
                    muted(app.t("instances.content_empty")).into_any_element()
                } else {
                    div().flex().flex_col().children(rows).into_any_element()
                })
                .into_any_element(),
        );
    }
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .children(sections)
        .into_any_element()
}

fn backups_tab(app: &Yuhina, cx: &mut Context<Yuhina>, id: &str) -> AnyElement {
    let running = app.is_game_running(id);
    let busy = app.loading.backups;
    let create_id = id.to_string();
    let heading = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .flex_wrap()
        .child(section_label(app, "instances.tab.backups"))
        .child(
            MaterialButton::new("backup-create")
                .variant(ButtonVariant::Filled)
                .button_size(ButtonSize::Small)
                .icon(material_icon("archive", 18.0))
                .child(app.t("backups.create"))
                .disabled(busy || running)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.create_backup(window, create_id.clone(), cx)
                })),
        );

    let label_field = div().flex().flex_col().gap(px(6.)).w_full().child(
        TextField::new("backup-label", &app.backup_label)
            .w_full()
            .label(app.t("backups.label"))
            .placeholder(app.t("backups.label_placeholder")),
    );

    let rows: Vec<AnyElement> = app
        .backups
        .iter()
        .map(|backup| {
            let label = backup
                .label
                .clone()
                .filter(|label| !label.trim().is_empty())
                .unwrap_or_else(|| app.t("backups.untitled").to_string());
            let restore_instance = id.to_string();
            let restore_backup_id = backup.id.clone();
            let delete_instance = id.to_string();
            let delete_backup_id = backup.id.clone();
            let restore_label = label.clone();
            let delete_label = label.clone();
            div()
                .w_full()
                .rounded(px(18.))
                .bg(color(ColorRole::SURFACE_CONTAINER))
                .p(px(14.))
                .flex()
                .items_center()
                .gap(px(12.))
                .child(
                    div()
                        .size(px(40.))
                        .rounded(px(14.))
                        .bg(color(ColorRole::TERTIARY_CONTAINER))
                        .text_color(color(ColorRole::ON_TERTIARY_CONTAINER))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(material_icon("archive", 22.0)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w(px(180.))
                        .child(type_role(div(), TypeRole::TitleSmall).child(label))
                        .child(muted(format!(
                            "{} · {} · {} {}",
                            backup_time(&backup.created_at),
                            format_bytes(backup.bytes),
                            backup.entries,
                            app.t("backups.files")
                        ))),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(
                            MaterialButton::new(format!("backup-restore-{}", backup.id))
                                .variant(ButtonVariant::Text)
                                .button_size(ButtonSize::Small)
                                .icon(material_icon("settings_backup_restore", 18.0))
                                .child(app.t("backups.restore"))
                                .disabled(running)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.dialog = Some(Dialog::RestoreBackup {
                                        instance_id: restore_instance.clone(),
                                        backup_id: restore_backup_id.clone(),
                                        label: restore_label.clone(),
                                    });
                                    cx.notify();
                                })),
                        )
                        .child(
                            IconButton::new(
                                format!("backup-delete-{}", backup.id),
                                material_icon("delete", 18.0),
                                app.t("backups.delete"),
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.dialog = Some(Dialog::DeleteBackup {
                                        instance_id: delete_instance.clone(),
                                        backup_id: delete_backup_id.clone(),
                                        label: delete_label.clone(),
                                    });
                                    cx.notify();
                                },
                            )),
                        ),
                )
                .into_any_element()
        })
        .collect();

    let list: AnyElement = if busy && app.backups.is_empty() {
        panel()
            .child(muted(app.t("common.loading")))
            .into_any_element()
    } else if rows.is_empty() {
        panel()
            .child(empty_state(
                app,
                "archive",
                "backups.empty",
                "backups.empty_body",
                None,
            ))
            .into_any_element()
    } else {
        panel()
            .flex()
            .flex_col()
            .gap(px(10.))
            .children(rows)
            .into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            panel()
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(heading)
                .child(label_field)
                .children((running).then(|| muted(app.t("backups.game_running")))),
        )
        .child(list)
        .into_any_element()
}

fn files_tab(app: &Yuhina, cx: &mut Context<Yuhina>, id: &str) -> AnyElement {
    let root_id = id.to_string();
    let mut crumbs: Vec<AnyElement> = vec![
        MaterialButton::new("files-root")
            .variant(ButtonVariant::Text)
            .button_size(ButtonSize::ExtraSmall)
            .icon(material_icon("folder_open", 16.0))
            .child(app.t("files.root"))
            .on_click(
                cx.listener(move |this, _, _, cx| this.open_files_dir(&root_id, String::new(), cx)),
            )
            .into_any_element(),
    ];
    let mut path = String::new();
    for segment in app
        .files_relative
        .split('/')
        .filter(|segment| !segment.is_empty())
    {
        path = if path.is_empty() {
            segment.to_string()
        } else {
            format!("{path}/{segment}")
        };
        let target = path.clone();
        let target_id = id.to_string();
        crumbs.push(
            div()
                .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                .child(material_icon("chevron_right", 16.0))
                .into_any_element(),
        );
        crumbs.push(
            MaterialButton::new(format!("files-crumb-{target}"))
                .variant(ButtonVariant::Text)
                .button_size(ButtonSize::ExtraSmall)
                .child(segment.to_string())
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.open_files_dir(&target_id, target.clone(), cx)
                }))
                .into_any_element(),
        );
    }

    let rows: Vec<AnyElement> = app
        .files_entries
        .iter()
        .map(|entry| {
            let is_dir = entry.is_dir;
            let relative = entry.relative.clone();
            let target_id = id.to_string();
            let trailing = if is_dir {
                app.t("files.folder").to_string()
            } else {
                format_bytes(entry.bytes)
            };
            let icon = if is_dir { "folder" } else { "description" };
            ListItem::new(format!("file-{}", entry.relative), entry.name.clone())
                .leading(ListLeading::Icon(material_icon(icon, 20.0)))
                .trailing_text(trailing)
                .trailing(if is_dir {
                    ListTrailing::Icon(material_icon("chevron_right", 18.0))
                } else {
                    ListTrailing::None
                })
                .container(ColorRole::SURFACE_CONTAINER)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if is_dir {
                        this.open_files_dir(&target_id, relative.clone(), cx);
                    } else {
                        this.preview_file(&target_id, relative.clone(), cx);
                    }
                }))
                .into_any_element()
        })
        .collect();

    let list: AnyElement = if app.loading.files && app.files_entries.is_empty() {
        panel()
            .child(muted(app.t("files.loading")))
            .into_any_element()
    } else if rows.is_empty() {
        panel()
            .child(empty_state(
                app,
                "folder",
                "files.empty",
                "files.empty_body",
                None,
            ))
            .into_any_element()
    } else {
        panel()
            .flex()
            .flex_col()
            .gap(px(6.))
            .children(rows)
            .into_any_element()
    };

    let preview: AnyElement = match &app.file_preview {
        Some(preview) => {
            let lines: Vec<AnyElement> = preview
                .content
                .lines()
                .map(|line| {
                    type_role(div(), TypeRole::BodySmall)
                        .line_height(rem(18.))
                        .font_family("monospace")
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .child(line.to_string())
                        .into_any_element()
                })
                .collect();
            panel()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .flex_wrap()
                        .child(section_label(app, "files.preview"))
                        .child(muted(preview.relative.clone())),
                )
                .children(preview.truncated.then(|| {
                    chip(
                        app.t("files.truncated"),
                        ColorRole::SECONDARY_CONTAINER,
                        ColorRole::ON_SECONDARY_CONTAINER,
                    )
                }))
                .child(
                    div()
                        .id("file-preview")
                        .w_full()
                        .max_h(px(420.))
                        .overflow_y_scroll()
                        .rounded(px(16.))
                        .bg(color(ColorRole::SURFACE_CONTAINER_LOWEST))
                        .p(px(16.))
                        .children(lines),
                )
                .into_any_element()
        }
        None => panel()
            .child(muted(app.t("files.preview_hint")))
            .into_any_element(),
    };

    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            panel()
                .flex()
                .flex_row()
                .items_center()
                .flex_wrap()
                .gap(px(4.))
                .children(crumbs),
        )
        .child(list)
        .child(preview)
        .into_any_element()
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let value = bytes as f64;
    if value >= MB {
        format!("{:.1} MB", value / MB)
    } else if value >= KB {
        format!("{:.1} KB", value / KB)
    } else {
        format!("{bytes} B")
    }
}

fn backup_time(created_at: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(created_at)
        .map(|value| {
            value
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| created_at.to_string())
}

fn settings_tab(app: &Yuhina, cx: &mut Context<Yuhina>, record: &InstanceRecord) -> AnyElement {
    let id = record.id.clone();
    let presets = [2048u32, 4096, 6144, 8192];
    let chips: Vec<AnyElement> = presets
        .into_iter()
        .map(|mb| {
            let selected = record.memory_mode == MemoryMode::Manual && record.memory_mb == mb;
            FilterChip::new(format!("memory-{id}-{mb}"), format!("{mb} MB"))
                .selected(selected)
                .on_toggle(cx.listener({
                    let id = id.clone();
                    move |this, on: &bool, _, cx| {
                        if *on {
                            let paths = this.paths.clone();
                            if let Ok(updated) = instances::update(&paths, &id, |record| {
                                record.memory_mode = MemoryMode::Manual;
                                record.memory_mb = mb;
                            }) && let Some(item) = this
                                .instances
                                .iter_mut()
                                .find(|item| item.record.id == updated.id)
                            {
                                item.record = updated;
                            }
                            cx.notify();
                        }
                    }
                }))
                .into_any_element()
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            panel()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(section_label(app, "common.memory"))
                .child(muted(app.t("instances.memory_hint")))
                .child(div().flex().flex_wrap().gap(px(8.)).children(chips)),
        )
        .child(
            panel()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(section_label(app, "instances.java_path"))
                .child(
                    type_role(div(), TypeRole::BodySmall).child(
                        record
                            .java_path
                            .clone()
                            .unwrap_or_else(|| app.t("common.none").to_string()),
                    ),
                )
                .child(section_label(app, "instances.jvm_args"))
                .child(
                    type_role(div(), TypeRole::BodySmall)
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .child(if record.jvm_args.is_empty() {
                            app.t("common.none").to_string()
                        } else {
                            record.jvm_args.clone()
                        }),
                )
                .child(section_label(app, "instances.game_args"))
                .child(
                    type_role(div(), TypeRole::BodySmall)
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .child(if record.game_args.is_empty() {
                            app.t("common.none").to_string()
                        } else {
                            record.game_args.clone()
                        }),
                ),
        )
        .child(
            panel().child(
                type_role(div(), TypeRole::BodySmall)
                    .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                    .child(format!(
                        "{}: {}",
                        app.t("settings.data_dir"),
                        app.paths.game_dir(&record.id).unwrap_or_default().display()
                    )),
            ),
        )
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------

fn library(app: &mut Yuhina, window: &mut Window, cx: &mut Context<Yuhina>) -> AnyElement {
    let actions = Some(
        MaterialButton::new("library-import-modpack")
            .variant(ButtonVariant::Tonal)
            .icon(material_icon("upload_file", 20.0))
            .child(app.t("library.import"))
            .disabled(app.loading.modpack)
            .on_click(cx.listener(|this, _, _, cx| {
                this.dialog = Some(Dialog::ImportModpack);
                cx.notify();
            }))
            .into_any_element(),
    );
    let header = page_header(app, "library.title", "library.subtitle", actions);
    let types = [
        ("mod", "library.mods"),
        ("resourcepack", "library.resourcepacks"),
        ("shader", "library.shaders"),
        ("modpack", "library.modpacks"),
    ];
    let type_chips: Vec<AnyElement> = types
        .into_iter()
        .map(|(kind, label)| {
            FilterChip::new(format!("library-type-{kind}"), app.t(label))
                .selected(app.library_type == kind)
                .on_toggle(cx.listener({
                    let kind = kind.to_string();
                    move |this, on: &bool, _, cx| {
                        if *on {
                            this.set_library_type(kind.clone(), cx);
                        }
                    }
                }))
                .into_any_element()
        })
        .collect();

    let target_chips: Vec<AnyElement> = app
        .instances
        .clone()
        .into_iter()
        .map(|summary| {
            let id = summary.record.id.clone();
            FilterChip::new(format!("library-target-{id}"), summary.record.name.clone())
                .selected(app.library_target.as_deref() == Some(id.as_str()))
                .on_toggle(cx.listener({
                    let id = id.clone();
                    move |this, on: &bool, _, cx| {
                        if *on {
                            this.library_target = Some(id.clone());
                            this.search_library(cx);
                        }
                    }
                }))
                .into_any_element()
        })
        .collect();

    let search_row = div()
        .flex()
        .items_end()
        .gap(px(12.))
        .child(
            div().flex_1().min_w(px(240.)).child(
                TextField::new("library-search", &app.search_input)
                    .w_full()
                    .label(app.t("common.search")),
            ),
        )
        .child(
            MaterialButton::new("library-search-button")
                .variant(ButtonVariant::Filled)
                .icon(material_icon("search", 20.0))
                .child(app.t("common.search"))
                .on_click(cx.listener(|this, _, _, cx| this.search_library(cx))),
        );

    let results = app.search_results.clone();
    let result_rows: Vec<AnyElement> = results
        .map(|result| result.hits)
        .unwrap_or_default()
        .into_iter()
        .map(|hit| {
            let project_id = hit.id.clone();
            div()
                .w_full()
                .rounded(px(20.))
                .bg(color(ColorRole::SURFACE_CONTAINER_LOW))
                .p(px(16.))
                .flex()
                .items_center()
                .gap(px(14.))
                .child(
                    div()
                        .size(px(44.))
                        .rounded(px(14.))
                        .bg(color(ColorRole::SECONDARY_CONTAINER))
                        .text_color(color(ColorRole::ON_SECONDARY_CONTAINER))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(material_icon("extension", 24.0)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .child(type_role(div(), TypeRole::TitleMedium).child(hit.title.clone()))
                        .child(
                            type_role(div(), TypeRole::BodySmall)
                                .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                                .child(hit.description.clone()),
                        )
                        .child(muted(format!(
                            "{} · {} · {}",
                            hit.author,
                            compact_number(hit.downloads),
                            hit.categories.join(" · ")
                        ))),
                )
                .child(
                    MaterialButton::new(format!("library-install-{}", hit.id))
                        .variant(ButtonVariant::Tonal)
                        .button_size(ButtonSize::Small)
                        .icon(material_icon("download", 18.0))
                        .child(app.t("library.install"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.install_from_library(project_id.clone(), cx)
                        })),
                )
                .into_any_element()
        })
        .collect();

    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            panel()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(search_row)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(section_label(app, "library.type"))
                        .child(div().flex().flex_wrap().gap(px(8.)).children(type_chips)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(section_label(app, "library.target"))
                        .child(if target_chips.is_empty() {
                            muted(app.t("library.no_instance")).into_any_element()
                        } else {
                            div()
                                .flex()
                                .flex_wrap()
                                .gap(px(8.))
                                .children(target_chips)
                                .into_any_element()
                        }),
                ),
        )
        .child(if app.loading.modpack {
            loading_panel(app, "library.importing")
        } else if app.loading.library {
            loading_panel(app, "library.searching")
        } else if let Some(error) = &app.library_error {
            let retry = MaterialButton::new("library-retry")
                .variant(ButtonVariant::Text)
                .icon(material_icon("refresh", 18.0))
                .child(app.t("common.retry"))
                .on_click(cx.listener(|this, _, _, cx| this.search_library(cx)))
                .into_any_element();
            error_panel(app, error, Some(retry))
        } else if result_rows.is_empty() {
            panel()
                .child(empty_state(
                    app,
                    "explore",
                    "library.no_results",
                    "library.no_results_body",
                    None,
                ))
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(px(10.))
                .children(result_rows)
                .into_any_element()
        })
        .into_any_element();

    page_frame(window, header, body)
}

fn compact_number(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}K", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

// ---------------------------------------------------------------------------
// Accounts
// ---------------------------------------------------------------------------

fn accounts_page(app: &mut Yuhina, window: &mut Window, cx: &mut Context<Yuhina>) -> AnyElement {
    let actions = Some(
        MaterialButton::new("accounts-add")
            .variant(ButtonVariant::Filled)
            .icon(material_icon("person_add", 20.0))
            .child(app.t("accounts.add"))
            .on_click(cx.listener(|this, _, _, cx| {
                this.dialog = Some(Dialog::AddAccount);
                this.dialog_error = None;
                cx.notify();
            }))
            .into_any_element(),
    );
    let header = page_header(app, "accounts.title", "accounts.subtitle", actions);

    let rows: Vec<AnyElement> = app
        .accounts
        .clone()
        .into_iter()
        .map(|account| {
            let set_id = account.id.clone();
            let remove_id = account.id.clone();
            let kind = match account.kind {
                crate::core::accounts::AccountKind::Microsoft => app.t("accounts.microsoft"),
                crate::core::accounts::AccountKind::Offline => app.t("accounts.offline"),
                crate::core::accounts::AccountKind::Littleskin => app.t("accounts.littleskin"),
            };
            let avatar = Avatar::new(
                format!("account-avatar-{}", account.id),
                account.name.clone(),
            );
            let avatar = match app.skins.get(&account.id).and_then(|path| path.clone()) {
                Some(path) => avatar.image(path),
                None => avatar.monogram(monogram_initials(&account.name)),
            };
            let short_uuid: String = account.uuid.chars().take(8).collect();
            div()
                .w_full()
                .rounded(px(20.))
                .bg(color(if account.active {
                    ColorRole::SECONDARY_CONTAINER
                } else {
                    ColorRole::SURFACE_CONTAINER_LOW
                }))
                .p(px(16.))
                .flex()
                .items_center()
                .gap(px(14.))
                .child(avatar)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .child(type_role(div(), TypeRole::TitleMedium).child(account.name.clone()))
                        .child(muted(format!("{kind} · {short_uuid}"))),
                )
                .children(account.active.then(|| {
                    chip(
                        app.t("accounts.active"),
                        ColorRole::PRIMARY_CONTAINER,
                        ColorRole::ON_PRIMARY_CONTAINER,
                    )
                    .into_any_element()
                }))
                .child(
                    div()
                        .flex()
                        .gap(px(8.))
                        .children((!account.active).then(|| {
                            MaterialButton::new(format!("account-active-{}", account.id))
                                .variant(ButtonVariant::Text)
                                .button_size(ButtonSize::Small)
                                .child(app.t("accounts.set_active"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.set_active_account(set_id.clone(), cx)
                                }))
                        }))
                        .child(
                            IconButton::new(
                                format!("account-remove-{}", account.id),
                                material_icon("delete", 18.0),
                                app.t("accounts.remove"),
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| this.remove_account(remove_id.clone(), cx),
                            )),
                        ),
                )
                .into_any_element()
        })
        .collect();

    let body = if app.loading.accounts && rows.is_empty() {
        loading_panel(app, "accounts.loading")
    } else if let Some(error) = app.account_error.as_ref().filter(|_| rows.is_empty()) {
        let retry = MaterialButton::new("accounts-retry")
            .variant(ButtonVariant::Text)
            .icon(material_icon("refresh", 18.0))
            .child(app.t("common.retry"))
            .on_click(cx.listener(|this, _, _, cx| this.refresh_accounts(cx)))
            .into_any_element();
        error_panel(app, error, Some(retry))
    } else if rows.is_empty() {
        panel()
            .child(empty_state(
                app,
                "person",
                "accounts.empty",
                "accounts.empty_body",
                Some(
                    MaterialButton::new("accounts-add-empty")
                        .variant(ButtonVariant::Filled)
                        .icon(material_icon("person_add", 20.0))
                        .child(app.t("accounts.add"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dialog = Some(Dialog::AddAccount);
                            this.dialog_error = None;
                            cx.notify();
                        }))
                        .into_any_element(),
                ),
            ))
            .into_any_element()
    } else {
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .children(rows)
            .into_any_element()
    };
    page_frame(window, header, body)
}

// ---------------------------------------------------------------------------
// Servers
// ---------------------------------------------------------------------------

fn servers_page(app: &mut Yuhina, window: &mut Window, cx: &mut Context<Yuhina>) -> AnyElement {
    let actions = Some(
        div()
            .flex()
            .gap(px(8.))
            .child(
                MaterialButton::new("servers-ping")
                    .variant(ButtonVariant::Text)
                    .icon(material_icon("refresh", 20.0))
                    .child(app.t("servers.ping"))
                    .on_click(cx.listener(|this, _, _, cx| this.ping_all_servers(cx))),
            )
            .child(
                MaterialButton::new("servers-add")
                    .variant(ButtonVariant::Filled)
                    .icon(material_icon("add", 20.0))
                    .child(app.t("servers.add"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.dialog = Some(Dialog::AddServer);
                        this.dialog_error = None;
                        cx.notify();
                    })),
            )
            .into_any_element(),
    );
    let header = page_header(app, "servers.title", "servers.subtitle", actions);

    let rows: Vec<AnyElement> = app
        .servers
        .clone()
        .into_iter()
        .map(|server| {
            let status = app.statuses.get(&server.id).cloned();
            let join_server = server.clone();
            let ping_server = server.clone();
            let remove_id = server.id.clone();
            div()
                .w_full()
                .rounded(px(20.))
                .bg(color(ColorRole::SURFACE_CONTAINER_LOW))
                .p(px(16.))
                .flex()
                .items_center()
                .gap(px(14.))
                .child(
                    div()
                        .size(px(44.))
                        .rounded(px(14.))
                        .bg(color(ColorRole::SECONDARY_CONTAINER))
                        .text_color(color(ColorRole::ON_SECONDARY_CONTAINER))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(material_icon("dns", 24.0)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .child(type_role(div(), TypeRole::TitleMedium).child(server.name.clone()))
                        .child(muted(servers::format_address(&server.address, server.port)))
                        .child(match &status {
                            Some(status) => muted(format!(
                                "{} · {}/{} · {} ms · {}",
                                status.motd,
                                status.players_online,
                                status.players_max,
                                status.latency_ms,
                                status.version_name
                            )),
                            None => muted(app.t("servers.offline")),
                        }),
                )
                .children(status.as_ref().map(|status| {
                    chip(
                        format!("{}/{}", status.players_online, status.players_max),
                        ColorRole::TERTIARY_CONTAINER,
                        ColorRole::ON_TERTIARY_CONTAINER,
                    )
                    .into_any_element()
                }))
                .child(
                    div()
                        .flex()
                        .gap(px(8.))
                        .child(
                            IconButton::new(
                                format!("server-ping-{}", server.id),
                                material_icon("refresh", 18.0),
                                app.t("servers.ping"),
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| this.ping_server(ping_server.clone(), cx),
                            )),
                        )
                        .child(
                            IconButton::new(
                                format!("server-remove-{}", server.id),
                                material_icon("delete", 18.0),
                                app.t("servers.remove"),
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| this.remove_server(remove_id.clone(), cx),
                            )),
                        )
                        .child(
                            MaterialButton::new(format!("server-join-{}", server.id))
                                .variant(ButtonVariant::Filled)
                                .button_size(ButtonSize::Small)
                                .icon(material_icon("login", 18.0))
                                .child(app.t("servers.join"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    join_server_target(this, &join_server, cx)
                                })),
                        ),
                )
                .into_any_element()
        })
        .collect();

    let body = if app.loading.servers && rows.is_empty() {
        loading_panel(app, "servers.loading")
    } else if let Some(error) = app.server_error.as_ref().filter(|_| rows.is_empty()) {
        let retry = MaterialButton::new("servers-retry")
            .variant(ButtonVariant::Text)
            .icon(material_icon("refresh", 18.0))
            .child(app.t("common.retry"))
            .on_click(cx.listener(|this, _, _, cx| this.refresh_servers(cx)))
            .into_any_element();
        error_panel(app, error, Some(retry))
    } else if rows.is_empty() {
        panel()
            .child(empty_state(
                app,
                "dns",
                "servers.empty",
                "servers.empty_body",
                Some(
                    MaterialButton::new("servers-add-empty")
                        .variant(ButtonVariant::Filled)
                        .icon(material_icon("add", 20.0))
                        .child(app.t("servers.add"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dialog = Some(Dialog::AddServer);
                            this.dialog_error = None;
                            cx.notify();
                        }))
                        .into_any_element(),
                ),
            ))
            .into_any_element()
    } else {
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .children(rows)
            .into_any_element()
    };
    page_frame(window, header, body)
}

fn join_server_target(app: &mut Yuhina, server: &ServerRecord, cx: &mut Context<Yuhina>) {
    let instance = app
        .selected_instance
        .clone()
        .or_else(|| {
            app.instances
                .iter()
                .find(|item| item.installed)
                .map(|item| item.record.id.clone())
        })
        .or_else(|| app.instances.first().map(|item| item.record.id.clone()));
    let Some(instance) = instance else {
        app.show_toast(app.t("library.no_instance"), true, cx);
        return;
    };
    let host = server.address.clone();
    let port = server.port;
    // Resolve before joining, off the UI thread. Minecraft's own client does
    // the SRV lookup again for `--quickPlayMultiplayer`, so the original
    // hostname (not the SRV target) is what goes into the launch arguments.
    super::spawn_task(
        cx,
        move || servers::resolve_endpoint(&host, port),
        move |this, endpoint, cx| {
            this.launch_instance(
                instance,
                Some(ServerTarget {
                    host: endpoint.handshake_host,
                    port: endpoint.handshake_port,
                    quick_play: true,
                }),
                cx,
            )
        },
    );
}

// ---------------------------------------------------------------------------
// Activity
// ---------------------------------------------------------------------------

fn activity(app: &mut Yuhina, window: &mut Window, cx: &mut Context<Yuhina>) -> AnyElement {
    let header = page_header(app, "activity.title", "activity.subtitle", None);
    let running: Vec<AnyElement> = app
        .running
        .iter()
        .filter(|(_, game)| matches!(game.status, GameStatus::Starting | GameStatus::Running))
        .map(|(id, game)| {
            let stop_id = id.clone();
            let name = app
                .instances
                .iter()
                .find(|item| &item.record.id == id)
                .map(|item| item.record.name.clone())
                .unwrap_or_else(|| id.clone());
            let tail: Vec<AnyElement> = game
                .logs
                .iter()
                .rev()
                .take(8)
                .rev()
                .map(|line| {
                    type_role(div(), TypeRole::BodySmall)
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .child(line.text.clone())
                        .into_any_element()
                })
                .collect();
            panel()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(type_role(div(), TypeRole::TitleMedium).child(name))
                        .child(
                            MaterialButton::new(format!("activity-stop-{id}"))
                                .variant(ButtonVariant::Tonal)
                                .button_size(ButtonSize::Small)
                                .icon(material_icon("stop", 18.0))
                                .child(app.t("instances.stop"))
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.stop_game(&stop_id, cx)),
                                ),
                        ),
                )
                .child(div().flex().flex_col().children(tail))
                .into_any_element()
        })
        .collect();

    let installs: Vec<AnyElement> = app
        .installs
        .values()
        .map(|task| {
            let name = app
                .instances
                .iter()
                .find(|item| item.record.id == task.instance_id)
                .map(|item| item.record.name.clone())
                .unwrap_or_else(|| task.instance_id.clone());
            let retry_id = task.instance_id.clone();
            panel()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    type_role(div(), TypeRole::TitleMedium)
                        .child(format!("{name} · {}", app.phase_label(task.phase))),
                )
                .children(task.error.as_ref().map(|error| {
                    type_role(div(), TypeRole::BodySmall)
                        .text_color(color(ColorRole::ERROR))
                        .child(error.clone())
                }))
                .child(
                    div().child(
                        LinearProgress::new(
                            format!("activity-progress-{}", task.instance_id),
                            app.t("instances.install_progress_a11y"),
                        )
                        .value(Some(progress_value(task.percent / 100.0)))
                        .thickness(px(4.)),
                    ),
                )
                .children(task.error.is_some().then(|| {
                    MaterialButton::new(format!("activity-retry-{retry_id}"))
                        .variant(ButtonVariant::Text)
                        .button_size(ButtonSize::Small)
                        .icon(material_icon("refresh", 18.0))
                        .child(app.t("common.retry"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.start_install(retry_id.clone(), cx)
                        }))
                }))
                .into_any_element()
        })
        .collect();

    let body = div()
        .flex()
        .flex_col()
        .gap(px(24.))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(section_label(app, "activity.running"))
                .child(if running.is_empty() {
                    panel()
                        .child(empty_state(
                            app,
                            "sports_esports",
                            "activity.running_empty",
                            "activity.running_empty_body",
                            None,
                        ))
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .children(running)
                        .into_any_element()
                }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(section_label(app, "activity.installs"))
                .child(if installs.is_empty() {
                    panel()
                        .child(empty_state(
                            app,
                            "download",
                            "activity.installs_empty",
                            "activity.empty_body",
                            None,
                        ))
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .children(installs)
                        .into_any_element()
                }),
        )
        .into_any_element();

    page_frame(window, header, body)
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

fn settings_page(app: &mut Yuhina, window: &mut Window, cx: &mut Context<Yuhina>) -> AnyElement {
    let header = page_header(app, "settings.title", "settings.subtitle", None);

    let theme_chips: Vec<AnyElement> = [
        (ThemePreference::Dark, "settings.theme_dark"),
        (ThemePreference::Light, "settings.theme_light"),
    ]
    .into_iter()
    .map(|(value, label)| {
        FilterChip::new(format!("theme-{label}"), app.t(label))
            .selected(app.settings.theme == value)
            .on_toggle(cx.listener(move |this, on: &bool, window, cx| {
                if *on && this.settings.theme != value {
                    this.toggle_theme(window, cx);
                }
            }))
            .into_any_element()
    })
    .collect();

    let language_chips: Vec<AnyElement> = [(Language::ZhCn, "中文"), (Language::EnUs, "English")]
        .into_iter()
        .map(|(value, label)| {
            FilterChip::new(format!("lang-{label}"), label)
                .selected(app.lang == value)
                .on_toggle(cx.listener(move |this, on: &bool, _, cx| {
                    if *on && this.lang != value {
                        this.toggle_language(cx);
                    }
                }))
                .into_any_element()
        })
        .collect();

    let source_chips: Vec<AnyElement> = [
        (DownloadSource::Auto, "settings.download_auto"),
        (DownloadSource::Bmcl, "settings.download_bmcl"),
        (DownloadSource::Official, "settings.download_official"),
    ]
    .into_iter()
    .map(|(value, label)| {
        FilterChip::new(format!("source-{label}"), app.t(label))
            .selected(app.settings.download_source == value)
            .on_toggle(cx.listener(move |this, on: &bool, _, cx| {
                if *on && this.settings.download_source != value {
                    this.settings.download_source = value;
                    let _ = core_settings::save(&this.paths, &this.settings);
                    cx.notify();
                }
            }))
            .into_any_element()
    })
    .collect();

    let motion_chips: Vec<AnyElement> = [
        (
            crate::core::settings::ReduceMotion::System,
            "settings.motion_system",
        ),
        (
            crate::core::settings::ReduceMotion::On,
            "settings.motion_on",
        ),
        (
            crate::core::settings::ReduceMotion::Off,
            "settings.motion_off",
        ),
    ]
    .into_iter()
    .map(|(value, label)| {
        FilterChip::new(format!("motion-{label}"), app.t(label))
            .selected(app.settings.reduce_motion == value)
            .on_toggle(cx.listener(move |this, on: &bool, _, cx| {
                if *on {
                    this.settings.reduce_motion = value;
                    let _ = core_settings::save(&this.paths, &this.settings);
                    cx.set_reduce_motion(value == crate::core::settings::ReduceMotion::On);
                    cx.notify();
                }
            }))
            .into_any_element()
    })
    .collect();

    let memory_chips: Vec<AnyElement> = [2048u32, 4096, 6144, 8192, 12288]
        .into_iter()
        .map(|mb| {
            FilterChip::new(format!("default-memory-{mb}"), format!("{mb} MB"))
                .selected(app.settings.instance_defaults.memory_mb == mb)
                .on_toggle(cx.listener(move |this, on: &bool, _, cx| {
                    if *on {
                        this.settings.instance_defaults.memory_mb = mb;
                        let _ = core_settings::save(&this.paths, &this.settings);
                        cx.notify();
                    }
                }))
                .into_any_element()
        })
        .collect();

    let java_rows: Vec<AnyElement> = app
        .java_runtimes
        .iter()
        .map(|runtime| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .py(px(6.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .min_w_0()
                        .child(
                            type_role(div(), TypeRole::TitleSmall)
                                .child(format!("Java {}", runtime.major)),
                        )
                        .child(muted(runtime.path.clone())),
                )
                .children(runtime.managed.then(|| {
                    chip(
                        app.t("settings.java_managed"),
                        ColorRole::TERTIARY_CONTAINER,
                        ColorRole::ON_TERTIARY_CONTAINER,
                    )
                    .into_any_element()
                }))
                .into_any_element()
        })
        .collect();

    let settings_body = div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            panel()
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(section_label(app, "settings.appearance"))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(muted(app.t("settings.theme")))
                        .child(div().flex().flex_wrap().gap(px(8.)).children(theme_chips)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(muted(app.t("settings.language")))
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap(px(8.))
                                .children(language_chips),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(muted(app.t("settings.motion")))
                        .child(div().flex().flex_wrap().gap(px(8.)).children(motion_chips)),
                ),
        )
        .child(
            panel()
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(section_label(app, "settings.downloads"))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(muted(app.t("settings.download_source")))
                        .child(div().flex().flex_wrap().gap(px(8.)).children(source_chips)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(muted(app.t("settings.memory")))
                        .child(div().flex().flex_wrap().gap(px(8.)).children(memory_chips)),
                ),
        )
        .child(
            panel()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(section_label(app, "settings.java"))
                        .child(
                            MaterialButton::new("java-rescan")
                                .variant(ButtonVariant::Text)
                                .button_size(ButtonSize::ExtraSmall)
                                .icon(material_icon("refresh", 16.0))
                                .child(app.t("settings.java_rescan"))
                                .on_click(cx.listener(|this, _, _, cx| this.refresh_java(cx))),
                        ),
                )
                .child(
                    MaterialSwitch::new("java-auto-download", app.t("settings.java_auto_download"))
                        .checked(app.settings.java.auto_download)
                        .on_change(cx.listener(|this, on: &bool, _, cx| {
                            this.settings.java.auto_download = *on;
                            let _ = core_settings::save(&this.paths, &this.settings);
                            cx.notify();
                        })),
                )
                .child(section_label(app, "settings.java_detected"))
                .child(if java_rows.is_empty() {
                    muted(app.t("common.loading")).into_any_element()
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .children(java_rows)
                        .into_any_element()
                }),
        )
        .child(
            panel()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(section_label(app, "settings.about"))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(muted(app.t("settings.version")))
                        .child(type_role(div(), TypeRole::TitleSmall).child(APP_VERSION)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(muted(app.t("settings.data_dir")))
                        .child(
                            type_role(div(), TypeRole::BodySmall)
                                .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                                .child(app.paths.root().display().to_string()),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .child(if app.loading.update {
                            muted(app.t("settings.checking")).into_any_element()
                        } else if let Some(update) = &app.update {
                            div()
                                .flex()
                                .flex_col()
                                .child(type_role(div(), TypeRole::TitleSmall).child(format!(
                                    "{} {}",
                                    app.t("settings.update_available"),
                                    update.version
                                )))
                                .child(muted(update.tag.clone()))
                                .into_any_element()
                        } else if app.update_checked {
                            muted(app.t("settings.up_to_date")).into_any_element()
                        } else {
                            muted(app.t("settings.check_updates")).into_any_element()
                        })
                        .child(
                            div()
                                .flex()
                                .gap(px(8.))
                                .child(
                                    MaterialButton::new("check-updates")
                                        .variant(ButtonVariant::Text)
                                        .button_size(ButtonSize::Small)
                                        .icon(material_icon("refresh", 18.0))
                                        .child(app.t("settings.check_updates"))
                                        .disabled(app.loading.update)
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.check_updates(cx)),
                                        ),
                                )
                                .children(app.update.as_ref().map(|update| {
                                    let url = update.page_url.clone();
                                    MaterialButton::new("open-release")
                                        .variant(ButtonVariant::Filled)
                                        .button_size(ButtonSize::Small)
                                        .icon(material_icon("open_in_new", 18.0))
                                        .child(app.t("settings.open_release"))
                                        .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url)))
                                        .into_any_element()
                                })),
                        ),
                ),
        )
        .into_any_element();

    let _ = window;
    page_frame(window, header, settings_body)
}
