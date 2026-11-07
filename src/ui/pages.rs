use super::*;
use crate::core::accounts::AccountKind;
use crate::core::instances::{self, InstanceRecord, MemoryMode};

// 8pt rhythm and the Material shape scale. These are `rem`, so the whole
// layout follows the text-scale preference: at 200% the gutters, row padding,
// icon tiles and card padding grow with the type instead of clipping it.
const SPACE_XS: Rems = rem(4.);
const SPACE_S: Rems = rem(8.);
const SPACE_M: Rems = rem(12.);
const SPACE_L: Rems = rem(16.);
const SPACE_XL: Rems = rem(24.);
const SPACE_2XL: Rems = rem(32.);
const RADIUS_ROW: Rems = rem(16.);
const RADIUS_CARD: Rems = rem(24.);
const RADIUS_HERO: Rems = rem(32.);
const PAGE_MAX_WIDTH: Rems = rem(1120.);
const PAGE_GUTTER: Rems = rem(32.);

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

fn width_class(window: &Window) -> WindowWidthClass {
    WindowWidthClass::of(f32::from(window.viewport_size().width))
}

fn page_frame(app: &Yuhina, window: &Window, header: AnyElement, body: AnyElement) -> AnyElement {
    let compact = width_class(window) == WindowWidthClass::Compact;
    let scroll = app.page_scroll(app.page);
    div()
        .id(format!("page-scroll-{}", app.page.id()))
        .size_full()
        .min_w_0()
        .overflow_y_scroll()
        .overflow_x_hidden()
        .track_scroll(&scroll)
        .child(
            div().w_full().min_w_0().flex().flex_col().child(
                div()
                    .w_full()
                    .max_w(PAGE_MAX_WIDTH)
                    .min_w_0()
                    .px(if compact { SPACE_XL } else { PAGE_GUTTER })
                    .pt(SPACE_2XL)
                    .pb(SPACE_2XL)
                    .flex()
                    .flex_col()
                    .items_stretch()
                    .gap(SPACE_XL)
                    .child(header)
                    .child(body),
            ),
        )
        .into_any_element()
}

/// The one heading for a page. The top app bar deliberately carries no title:
/// the window title names the page/instance, and the content owns the visible
/// H1. This removes the duplicated title the audit found on six pages.
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
        .gap(SPACE_L)
        .flex_wrap()
        .child(
            div()
                .flex()
                .flex_col()
                .gap(SPACE_XS)
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
                .gap(SPACE_S)
                .child(actions)
        }))
        .into_any_element()
}

fn card_surface() -> Div {
    div()
        .rounded(RADIUS_CARD)
        .bg(color(ColorRole::SURFACE_CONTAINER_LOW))
        .p(SPACE_XL)
}

fn section_label(app: &Yuhina, key: &str) -> Div {
    type_scale(div(), TypeRole::LabelLarge.emphasized())
        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
        .child(app.t(key))
}

pub(super) fn muted(text: impl Into<SharedString>) -> Div {
    type_role(div(), TypeRole::BodySmall)
        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
        .child(text.into())
}

fn body_text(text: impl Into<SharedString>) -> Div {
    type_role(div(), TypeRole::BodyMedium).child(text.into())
}

/// The primary identifying line of a row. BodyMedium, not BodySmall: the
/// version/loader/memory tuple is what the user scans for.
fn meta_text(text: impl Into<SharedString>) -> Div {
    type_role(div(), TypeRole::BodyMedium)
        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
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

fn instance_meta(app: &Yuhina, record: &InstanceRecord) -> String {
    app.t("instances.row_meta")
        .replace("{version}", &record.version_id)
        .replace("{loader}", &loader_text(app, &record.loader))
        .replace(
            "{mb}",
            &instances::effective_memory_mb(&app.paths, record).to_string(),
        )
}

/// Show only exceptional states. "Installed" is the normal case and stays
/// quiet; Running is the single green status, Not installed the warning.
fn status_badge(app: &Yuhina, running: bool, installed: bool) -> Option<AnyElement> {
    if running {
        return Some(
            status_chip(StatusKind::Success)
                .child(app.t("instances.running"))
                .into_any_element(),
        );
    }
    if !installed {
        return Some(
            status_chip(StatusKind::Warning)
                .child(app.t("instances.not_installed"))
                .into_any_element(),
        );
    }
    None
}

fn active_badge(app: &Yuhina) -> AnyElement {
    Badge::large("account-active-badge", app.t("accounts.active"))
        .color(BadgeColor::Secondary)
        .into_any_element()
}

pub(super) fn error_panel(
    app: &Yuhina,
    id: &'static str,
    message: &str,
    retry: Option<AnyElement>,
) -> AnyElement {
    let mut banner = notice_banner(id, NoticeKind::Danger, message.to_string());
    if let Some(retry) = retry {
        banner = banner.child(retry);
    }
    let _ = app;
    banner.into_any_element()
}

pub(super) fn loading_panel(app: &Yuhina, id: &'static str, label_key: &str) -> AnyElement {
    div()
        .w_full()
        .rounded(RADIUS_CARD)
        .bg(color(ColorRole::SURFACE_CONTAINER_LOW))
        .p(SPACE_XL)
        .flex()
        .items_center()
        .gap(SPACE_M)
        .child(LoadingIndicator::new(id, app.t(label_key)).size(24.))
        .child(body_text(app.t(label_key)))
        .into_any_element()
}

fn empty_state(
    app: &Yuhina,
    id: &'static str,
    icon: &str,
    title_key: &str,
    body_key: &str,
    action: Option<AnyElement>,
) -> AnyElement {
    MaterialCard::new(id, CardVariant::Filled)
        .p(SPACE_XL)
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .items_center()
                .gap(SPACE_M)
                .child(
                    div()
                        .size(rem(64.))
                        .rounded(RADIUS_CARD)
                        .bg(color(ColorRole::SECONDARY_CONTAINER))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(color(ColorRole::ON_SECONDARY_CONTAINER))
                        .child(material_icon(icon, 32.0)),
                )
                .child(
                    type_scale(div(), TypeRole::HeadlineSmall.emphasized()).child(app.t(title_key)),
                )
                .child(
                    type_role(div(), TypeRole::BodyMedium)
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .text_center()
                        .child(app.t(body_key)),
                )
                .children(action),
        )
        .into_any_element()
}

/// Equal-column card grid. The column count follows the window class, never
/// the item count, and short rows are padded with empty cells so the last card
/// never grows to fill the line.
fn card_grid(items: Vec<AnyElement>, columns: usize) -> Div {
    let columns = columns.max(1);
    let mut grid = div().w_full().flex().flex_col().gap(SPACE_L);
    let mut items = items.into_iter().peekable();
    while items.peek().is_some() {
        let mut cells: Vec<AnyElement> = Vec::with_capacity(columns);
        for _ in 0..columns {
            match items.next() {
                Some(item) => cells.push(div().flex_1().min_w_0().child(item).into_any_element()),
                None => cells.push(div().flex_1().min_w_0().into_any_element()),
            }
        }
        grid = grid.child(
            div()
                .w_full()
                .flex()
                .items_stretch()
                .gap(SPACE_L)
                .children(cells),
        );
    }
    grid
}

fn columns_for(class: WindowWidthClass) -> usize {
    match class {
        WindowWidthClass::Compact => 1,
        WindowWidthClass::Medium | WindowWidthClass::Expanded => 2,
        WindowWidthClass::Large | WindowWidthClass::ExtraLarge => 3,
    }
}

fn progress_value(fraction: f32) -> f32 {
    fraction.clamp(0.0, 1.0)
}

/// A list row and its trailing action share one surface: the row itself is the
/// navigation target, the button stays a separate focus stop.
fn row_with_action(row: ListItem, action: Option<AnyElement>) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .rounded(RADIUS_ROW)
        .bg(color(ColorRole::SURFACE_CONTAINER_LOW))
        .flex()
        .items_center()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(row.container(ColorRole::SURFACE_CONTAINER_LOW)),
        )
        .children(action.map(|action| div().flex_shrink_0().pr(SPACE_S).child(action)))
        .into_any_element()
}

fn menu_trigger(id: String, label: &'static str, cx: &mut Context<Yuhina>) -> IconButton {
    IconButton::new(id.clone(), material_icon("more_vert", 24.0), label).on_click(cx.listener(
        move |this, _, _, cx| {
            this.menu_open = if this.menu_open.as_ref() == Some(&id) {
                None
            } else {
                Some(id.clone())
            };
            cx.notify();
        },
    ))
}

fn menu_open(app: &Yuhina, id: &str) -> bool {
    app.menu_open.as_deref() == Some(id)
}

// ---------------------------------------------------------------------------
// Home
// ---------------------------------------------------------------------------

fn home(app: &mut Yuhina, window: &mut Window, cx: &mut Context<Yuhina>) -> AnyElement {
    let name = app
        .accounts
        .iter()
        .find(|account| account.active)
        .map(|account| account.name.clone());
    let greeting = match name {
        Some(name) => app.t("home.greeting").replace("{name}", &name),
        None => app.t("home.greeting_guest").to_string(),
    };

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
            let mut card = MaterialCard::new(format!("recent-{id}"), CardVariant::Filled)
                .on_click(
                    cx.listener(move |this, _, _, cx| this.open_instance(open_id.clone(), cx)),
                )
                .p(SPACE_L)
                .h_full();
            let content = div()
                .flex()
                .flex_col()
                .gap(SPACE_M)
                .h_full()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(SPACE_M)
                        .child(
                            div()
                                .size(rem(44.))
                                .rounded(RADIUS_ROW)
                                .bg(color(ColorRole::TERTIARY_CONTAINER))
                                .text_color(color(ColorRole::ON_TERTIARY_CONTAINER))
                                .flex()
                                .flex_shrink_0()
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
                                        .truncate()
                                        .child(record.name.clone()),
                                )
                                .child(meta_text(instance_meta(app, &record)).truncate()),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(SPACE_S)
                        .children(status_badge(app, running, installed))
                        .child(if running {
                            MaterialButton::new(format!("home-stop-{id}"))
                                .variant(ButtonVariant::Tonal)
                                .button_size(ButtonSize::Small)
                                .icon(material_icon("stop", 20.0))
                                .child(app.t("instances.stop"))
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.stop_game(&play_id, cx)),
                                )
                                .into_any_element()
                        } else {
                            MaterialButton::new(format!("home-play-{id}"))
                                .variant(ButtonVariant::Tonal)
                                .button_size(ButtonSize::Small)
                                .icon(material_icon("play_arrow", 20.0))
                                .child(app.t("home.play"))
                                .disabled(!installed)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.launch_instance(play_id.clone(), None, cx)
                                }))
                                .into_any_element()
                        }),
                );
            card = card.child(content);
            card.into_any_element()
        })
        .collect();

    let news_cards: Vec<AnyElement> = app
        .news
        .items
        .iter()
        .take(4)
        .map(|item| {
            let url = item.url.clone();
            MaterialCard::new(format!("news-{}", item.id), CardVariant::Filled)
                .p(SPACE_L)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(SPACE_S)
                        .child(
                            type_role(div(), TypeRole::TitleSmall)
                                .line_clamp(2)
                                .child(item.title.clone()),
                        )
                        .child(muted(item.date.clone()))
                        .child(
                            type_role(div(), TypeRole::BodyMedium)
                                .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                                .line_clamp(2)
                                .child(item.text.clone()),
                        )
                        .child(
                            MaterialButton::new(format!("news-{}", item.id))
                                .variant(ButtonVariant::Text)
                                .button_size(ButtonSize::ExtraSmall)
                                .icon(material_icon("open_in_new", 20.0))
                                .child(app.t("home.read_more"))
                                .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))),
                        ),
                )
                .into_any_element()
        })
        .collect();

    let hero = div()
        .rounded(RADIUS_HERO)
        .bg(color(ColorRole::PRIMARY_CONTAINER))
        .text_color(color(ColorRole::ON_PRIMARY_CONTAINER))
        .p(SPACE_2XL)
        .flex()
        .flex_col()
        .gap(SPACE_M)
        .child(type_scale(div(), TypeRole::LabelLarge.emphasized()).child(app.t("app.tagline")))
        .child(type_scale(div(), TypeRole::DisplaySmall.emphasized()).child(greeting))
        .child(type_role(div(), TypeRole::BodyLarge).child(app.t("home.subtitle")))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(SPACE_M)
                .mt(SPACE_S)
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
                            .variant(ButtonVariant::Tonal)
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
        .gap(SPACE_M)
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
            loading_panel(app, "home-instances-loading", "instances.loading")
        } else if let Some(error) = app
            .instance_error
            .as_ref()
            .filter(|_| recent_cards.is_empty())
        {
            let retry = MaterialButton::new("home-instances-retry")
                .variant(ButtonVariant::Text)
                .icon(material_icon("refresh", 20.0))
                .child(app.t("common.retry"))
                .on_click(cx.listener(|this, _, _, cx| this.refresh_instances(cx)))
                .into_any_element();
            error_panel(app, "home-instances-error", error, Some(retry))
        } else if recent_cards.is_empty() {
            empty_state(
                app,
                "home-empty",
                "grid_view",
                "home.no_instances",
                "home.no_instances_body",
                Some(
                    MaterialButton::new("home-create-empty")
                        .variant(ButtonVariant::Text)
                        .icon(material_icon("add", 20.0))
                        .child(app.t("home.create"))
                        .on_click(cx.listener(|this, _, _, cx| this.open_create_dialog(cx)))
                        .into_any_element(),
                ),
            )
        } else {
            card_grid(recent_cards, columns_for(width_class(window))).into_any_element()
        });

    let news_section = div()
        .flex()
        .flex_col()
        .gap(SPACE_M)
        .child(section_label(app, "home.news"))
        .child(if app.loading.news && news_cards.is_empty() {
            loading_panel(app, "home-news-loading", "home.news_loading")
        } else if let Some(error) = app.news_error.as_ref().filter(|_| news_cards.is_empty()) {
            let retry = MaterialButton::new("news-retry")
                .variant(ButtonVariant::Text)
                .icon(material_icon("refresh", 20.0))
                .child(app.t("common.retry"))
                .on_click(cx.listener(|this, _, _, cx| this.refresh_news(cx, true)))
                .into_any_element();
            error_panel(app, "home-news-error", error, Some(retry))
        } else if news_cards.is_empty() {
            empty_state(
                app,
                "home-news-empty",
                "newspaper",
                "common.empty",
                "home.news_empty",
                Some(
                    MaterialButton::new("home-news-retry")
                        .variant(ButtonVariant::Text)
                        .icon(material_icon("refresh", 20.0))
                        .child(app.t("common.retry"))
                        .on_click(cx.listener(|this, _, _, cx| this.refresh_news(cx, true)))
                        .into_any_element(),
                ),
            )
        } else {
            card_grid(news_cards, columns_for(width_class(window))).into_any_element()
        });

    page_frame(
        app,
        window,
        hero.into_any_element(),
        div()
            .flex()
            .flex_col()
            .gap(SPACE_XL)
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
            .flex_wrap()
            .gap(SPACE_S)
            .child(
                MaterialButton::new("instances-import")
                    .variant(ButtonVariant::Text)
                    .icon(material_icon("download", 20.0))
                    .child(app.t("backups.import"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.dialog = Some(Dialog::ImportInstance);
                        this.dialog_busy = false;
                        this.dialog_field_error = None;
                        this.dialog_error = None;
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

    let total = app.instances.len();
    let rows: Vec<AnyElement> = app
        .instances
        .clone()
        .into_iter()
        .enumerate()
        .map(|(index, summary)| {
            let id = summary.record.id.clone();
            let open_id = id.clone();
            let play_id = id.clone();
            let running = app.is_game_running(&id);
            let installed = summary.installed;
            let row = ListItem::new(format!("instance-row-{id}"), summary.record.name.clone())
                .leading(ListLeading::Tinted(
                    material_icon("sports_esports", 24.0),
                    Tint::Teal,
                    40.,
                ))
                .supporting(instance_meta(app, &summary.record))
                .container(ColorRole::SURFACE_CONTAINER_LOW)
                .divider(index + 1 < total)
                .on_click(
                    cx.listener(move |this, _, _, cx| this.open_instance(open_id.clone(), cx)),
                );
            let action = if running {
                MaterialButton::new(format!("stop-{id}"))
                    .variant(ButtonVariant::Tonal)
                    .button_size(ButtonSize::Small)
                    .icon(material_icon("stop", 20.0))
                    .child(app.t("instances.stop"))
                    .on_click(cx.listener(move |this, _, _, cx| this.stop_game(&play_id, cx)))
                    .into_any_element()
            } else {
                MaterialButton::new(format!("play-{id}"))
                    .variant(ButtonVariant::Tonal)
                    .button_size(ButtonSize::Small)
                    .icon(material_icon("play_arrow", 20.0))
                    .child(app.t("instances.play"))
                    .disabled(!installed)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.launch_instance(play_id.clone(), None, cx)
                    }))
                    .into_any_element()
            };
            let action = div()
                .flex()
                .items_center()
                .gap(SPACE_S)
                .children(status_badge(app, running, installed))
                .child(action)
                .into_any_element();
            row_with_action(row, Some(action))
        })
        .collect();

    let body = if app.loading.instances && rows.is_empty() {
        loading_panel(app, "instances-loading", "instances.loading")
    } else if let Some(error) = app.instance_error.as_ref().filter(|_| rows.is_empty()) {
        let retry = MaterialButton::new("instances-retry")
            .variant(ButtonVariant::Text)
            .icon(material_icon("refresh", 20.0))
            .child(app.t("common.retry"))
            .on_click(cx.listener(|this, _, _, cx| this.refresh_instances(cx)))
            .into_any_element();
        error_panel(app, "instances-error", error, Some(retry))
    } else if rows.is_empty() {
        empty_state(
            app,
            "instances-empty",
            "grid_view",
            "instances.empty",
            "instances.empty_body",
            Some(
                MaterialButton::new("instances-create-empty")
                    .variant(ButtonVariant::Text)
                    .icon(material_icon("add", 20.0))
                    .child(app.t("instances.new"))
                    .on_click(cx.listener(|this, _, _, cx| this.open_create_dialog(cx)))
                    .into_any_element(),
            ),
        )
    } else {
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(SPACE_S)
            .children(rows)
            .into_any_element()
    };

    page_frame(app, window, header, body)
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
    let menu_id = format!("detail-menu-{id}");

    let menu_items = [
        MaterialMenuItem::new(app.t("instances.open_folder"))
            .leading(material_icon("folder_open", 20.0))
            .on_click(cx.listener({
                let id = id.clone();
                move |this, _, _, cx| {
                    this.menu_open = None;
                    this.open_folder(&id, cx);
                }
            })),
        MaterialMenuItem::new(app.t("instances.duplicate"))
            .leading(material_icon("content_copy", 20.0))
            .on_click(cx.listener({
                let id = id.clone();
                move |this, _, _, cx| {
                    this.menu_open = None;
                    this.duplicate_instance(id.clone(), cx);
                }
            })),
        MaterialMenuItem::new(app.t("backups.export"))
            .leading(material_icon("upload", 20.0))
            .on_click(cx.listener({
                let id = id.clone();
                move |this, _, _, cx| {
                    this.menu_open = None;
                    this.export_instance(id.clone(), cx);
                }
            })),
        MaterialMenuItem::new(app.t("instances.delete"))
            .leading(material_icon("delete", 20.0))
            .divider(true)
            .on_click(cx.listener({
                let id = id.clone();
                let name = record.name.clone();
                move |this, _, _, cx| {
                    this.menu_open = None;
                    this.dialog = Some(Dialog::DeleteInstance {
                        id: id.clone(),
                        name: name.clone(),
                    });
                    this.dialog_busy = false;
                    cx.notify();
                }
            })),
    ];
    let menu = MaterialMenu::new(menu_id.clone()).items(menu_items);
    let trigger = menu_trigger(menu_id.clone(), app.t("common.more"), cx);
    let overflow = MenuPopup::new(format!("{menu_id}-popup"), trigger, menu)
        .open(menu_open(app, &menu_id))
        .on_open_change(cx.listener({
            let menu_id = menu_id.clone();
            move |this, open: &bool, _, cx| {
                this.menu_open = open.then(|| menu_id.clone());
                cx.notify();
            }
        }))
        .into_any_element();

    let primary_actions = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(SPACE_S)
        .child(if running {
            MaterialButton::new("detail-stop")
                .variant(ButtonVariant::Tonal)
                .icon(material_icon("stop", 20.0))
                .child(app.t("instances.stop"))
                .on_click(cx.listener({
                    let id = id.clone();
                    move |this, _, _, cx| this.stop_game(&id, cx)
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
        .child(overflow);

    let header = div()
        .flex()
        .flex_col()
        .gap(SPACE_L)
        .child(
            div()
                .flex()
                .items_center()
                .gap(SPACE_L)
                .flex_wrap()
                .child(
                    div()
                        .size(rem(64.))
                        .rounded(RADIUS_CARD)
                        .bg(color(ColorRole::TERTIARY_CONTAINER))
                        .text_color(color(ColorRole::ON_TERTIARY_CONTAINER))
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .justify_center()
                        .child(material_icon("sports_esports", 32.0)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(SPACE_XS)
                        .flex_1()
                        .min_w(rem(220.))
                        .child(
                            type_scale(div(), TypeRole::HeadlineLarge.emphasized())
                                .child(record.name.clone()),
                        )
                        .child(meta_text(instance_meta(app, &record))),
                )
                .child(primary_actions),
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
        app,
        window,
        header.into_any_element(),
        div()
            .flex()
            .flex_col()
            .gap(SPACE_L)
            .child(
                // Six labels exceed a compact or 200% content width; the tab
                // strip scrolls horizontally instead of clipping a destination.
                div()
                    .id("instance-tabs-scroll")
                    .w_full()
                    .min_w_0()
                    .overflow_x_scroll()
                    .child(div().min_w(rem(520.)).child(tabs)),
            )
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
        app.t("instances.install_detail")
            .replace("{phase}", app.phase_label(task.phase))
            .replace("{percent}", &format!("{:.0}", task.percent))
    };
    let error = task.error.is_some();
    div()
        .w_full()
        .rounded(RADIUS_ROW)
        .bg(color(if error {
            ColorRole::ERROR_CONTAINER
        } else {
            ColorRole::SECONDARY_CONTAINER
        }))
        .text_color(color(if error {
            ColorRole::ON_ERROR_CONTAINER
        } else {
            ColorRole::ON_SECONDARY_CONTAINER
        }))
        .p(SPACE_L)
        .flex()
        .flex_col()
        .gap(SPACE_S)
        .child(type_role(div(), TypeRole::TitleSmall).child(label))
        .child(
            LinearProgress::new("install-progress", app.t("instances.install_progress_a11y"))
                .value(Some(fraction))
                .thickness(px(4.)),
        )
        .children(task.message.as_ref().map(|message| {
            type_role(div(), TypeRole::BodySmall)
                .font_family("monospace")
                .line_clamp(2)
                .child(message.clone())
        }))
        .into_any_element()
}

fn overview_tab(app: &Yuhina, record: &InstanceRecord, summary: &InstanceSummary) -> AnyElement {
    let info = [
        (app.t("common.version"), record.version_id.clone()),
        (app.t("common.loader"), loader_text(app, &record.loader)),
        (
            app.t("common.memory"),
            app.t("instances.memory_value").replace(
                "{mb}",
                &instances::effective_memory_mb(&app.paths, record).to_string(),
            ),
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
                .gap(SPACE_XS)
                .min_w(rem(160.))
                .flex_1()
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
                .font_family("monospace")
                .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                .child(line.text)
                .into_any_element()
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .gap(SPACE_L)
        .child(
            card_surface()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(SPACE_XL)
                .children(info_rows),
        )
        .child(
            card_surface()
                .flex()
                .flex_col()
                .gap(SPACE_S)
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
        .children(
            (summary.required_java_major.is_some() && !summary.installed).then(|| {
                card_surface().child(meta_text(app.t("instances.java_required").replace(
                    "{major}",
                    &summary.required_java_major.unwrap_or(21).to_string(),
                )))
            }),
        )
        .into_any_element()
}

fn logs_tab(app: &Yuhina, id: &str) -> AnyElement {
    let Some(game) = app.running.get(id) else {
        return card_surface()
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
                .font_family("monospace")
                .text_color(color(tint))
                .child(line.text.clone())
                .into_any_element()
        })
        .collect();
    div()
        .id("log-scroll")
        .w_full()
        .h(rem(520.))
        .rounded(RADIUS_ROW)
        .bg(color(ColorRole::SURFACE_CONTAINER_LOWEST))
        .p(SPACE_L)
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
                let row = ListItem::new(format!("content-{project_type}-{name}"), name)
                    .leading(ListLeading::Icon(material_icon("description", 24.0)))
                    .supporting(format!("{:.1} MB", size as f64 / 1_048_576.0))
                    .container(ColorRole::SURFACE_CONTAINER_LOW);
                let action = IconButton::new(
                    format!("remove-{project_type}-{}", remove_name),
                    material_icon("delete", 20.0),
                    app.t("instances.remove_mod"),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.dialog = Some(Dialog::ConfirmRemoveContent {
                        instance_id: remove_id.clone(),
                        project_type: remove_type.clone(),
                        filename: remove_name.clone(),
                    });
                    this.dialog_busy = false;
                    cx.notify();
                }));
                row_with_action(row, Some(action.into_any_element()))
            })
            .collect();
        sections.push(
            card_surface()
                .flex()
                .flex_col()
                .gap(SPACE_S)
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
                                .icon(material_icon("explore", 20.0))
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
                    div()
                        .flex()
                        .flex_col()
                        .gap(SPACE_S)
                        .children(rows)
                        .into_any_element()
                })
                .into_any_element(),
        );
    }
    div()
        .flex()
        .flex_col()
        .gap(SPACE_L)
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
        .gap(SPACE_M)
        .flex_wrap()
        .child(section_label(app, "instances.tab.backups"))
        .child(
            MaterialButton::new("backup-create")
                .variant(ButtonVariant::Filled)
                .button_size(ButtonSize::Small)
                .icon(material_icon("archive", 20.0))
                .child(app.t("backups.create"))
                .disabled(busy || running)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.create_backup(window, create_id.clone(), cx)
                })),
        );

    let label_field = div().flex().flex_col().gap(SPACE_XS).w_full().child(
        TextField::new("backup-label", &app.backup_label)
            .w_full()
            .label(app.t("backups.label"))
            .placeholder(app.t("backups.label_placeholder")),
    );

    let total = app.backups.len();
    let rows: Vec<AnyElement> = app
        .backups
        .iter()
        .enumerate()
        .map(|(index, backup)| {
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
            let summary = app
                .t("backups.summary_format")
                .replace("{time}", &backup_time(&backup.created_at))
                .replace("{bytes}", &format_bytes(backup.bytes))
                .replace("{files}", &backup.entries.to_string());
            let row = ListItem::new(format!("backup-{}", backup.id), label)
                .leading(ListLeading::Tinted(
                    material_icon("archive", 24.0),
                    Tint::Teal,
                    40.,
                ))
                .supporting(summary)
                .container(ColorRole::SURFACE_CONTAINER_LOW)
                .divider(index + 1 < total);
            let action = div()
                .flex()
                .items_center()
                .gap(SPACE_XS)
                .child(
                    MaterialButton::new(format!("backup-restore-{}", backup.id))
                        .variant(ButtonVariant::Text)
                        .button_size(ButtonSize::Small)
                        .icon(material_icon("settings_backup_restore", 20.0))
                        .child(app.t("backups.restore"))
                        .disabled(running)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.dialog = Some(Dialog::RestoreBackup {
                                instance_id: restore_instance.clone(),
                                backup_id: restore_backup_id.clone(),
                                label: restore_label.clone(),
                            });
                            this.dialog_busy = false;
                            cx.notify();
                        })),
                )
                .child(
                    IconButton::new(
                        format!("backup-delete-{}", backup.id),
                        material_icon("delete", 20.0),
                        app.t("backups.delete"),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.dialog = Some(Dialog::DeleteBackup {
                            instance_id: delete_instance.clone(),
                            backup_id: delete_backup_id.clone(),
                            label: delete_label.clone(),
                        });
                        this.dialog_busy = false;
                        cx.notify();
                    })),
                )
                .into_any_element();
            row_with_action(row, Some(action))
        })
        .collect();

    let list: AnyElement = if busy && app.backups.is_empty() {
        loading_panel(app, "backups-loading", "files.loading")
    } else if rows.is_empty() {
        empty_state(
            app,
            "backups-empty",
            "archive",
            "backups.empty",
            "backups.empty_body",
            None,
        )
    } else {
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(SPACE_S)
            .children(rows)
            .into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .gap(SPACE_L)
        .child(
            card_surface()
                .flex()
                .flex_col()
                .gap(SPACE_M)
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
            .icon(material_icon("folder_open", 20.0))
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
                .child(material_icon("chevron_right", 20.0))
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

    let total = app.files_entries.len();
    let rows: Vec<AnyElement> = app
        .files_entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
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
                .leading(ListLeading::Icon(material_icon(icon, 24.0)))
                .trailing_text(trailing)
                .trailing(if is_dir {
                    ListTrailing::Icon(material_icon("chevron_right", 20.0))
                } else {
                    ListTrailing::None
                })
                .container(ColorRole::SURFACE_CONTAINER_LOW)
                .divider(index + 1 < total)
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
        loading_panel(app, "files-loading", "files.loading")
    } else if rows.is_empty() {
        empty_state(
            app,
            "files-empty",
            "folder",
            "files.empty",
            "files.empty_body",
            Some(
                MaterialButton::new("files-open-folder")
                    .variant(ButtonVariant::Text)
                    .icon(material_icon("folder_open", 20.0))
                    .child(app.t("instances.open_folder"))
                    .on_click(cx.listener({
                        let id = id.to_string();
                        move |this, _, _, cx| this.open_folder(&id, cx)
                    }))
                    .into_any_element(),
            ),
        )
    } else {
        div()
            .flex()
            .flex_col()
            .gap(SPACE_XS)
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
                        .font_family("monospace")
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .child(line.to_string())
                        .into_any_element()
                })
                .collect();
            card_surface()
                .flex()
                .flex_col()
                .gap(SPACE_M)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(SPACE_M)
                        .flex_wrap()
                        .child(section_label(app, "files.preview"))
                        .child(muted(preview.relative.clone())),
                )
                .children(preview.truncated.then(|| {
                    status_chip(StatusKind::Neutral)
                        .child(app.t("files.truncated"))
                        .into_any_element()
                }))
                .child(
                    div()
                        .id("file-preview")
                        .w_full()
                        .max_h(rem(420.))
                        .overflow_y_scroll()
                        .rounded(RADIUS_ROW)
                        .bg(color(ColorRole::SURFACE_CONTAINER_LOWEST))
                        .p(SPACE_L)
                        .children(lines),
                )
                .into_any_element()
        }
        None => card_surface()
            .child(muted(app.t("files.preview_hint")))
            .into_any_element(),
    };

    div()
        .flex()
        .flex_col()
        .gap(SPACE_L)
        .child(
            card_surface()
                .flex()
                .flex_row()
                .items_center()
                .flex_wrap()
                .gap(SPACE_XS)
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
    let presets = [2048u32, 4096, 6144, 8192, 12288];
    let selected = presets
        .iter()
        .position(|mb| record.memory_mode == MemoryMode::Manual && record.memory_mb == *mb)
        .or_else(|| (record.memory_mode == MemoryMode::Manual).then_some(0));
    let options: Vec<String> = presets
        .iter()
        .map(|mb| {
            app.t("instances.memory_value")
                .replace("{mb}", &mb.to_string())
        })
        .collect();
    let memory = ExposedDropdown::new("instance-memory", app.t("common.memory"))
        .options(options)
        .selected(selected)
        .full_width()
        .on_select(cx.listener({
            let id = id.clone();
            move |this, index: &usize, _, cx| {
                let Some(mb) = presets.get(*index).copied() else {
                    return;
                };
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
        }));

    let info_row = |label: &str, value: String| {
        ListItem::new(format!("instance-info-{label}"), label)
            .supporting(value)
            .container(ColorRole::SURFACE_CONTAINER_LOW)
            .into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .gap(SPACE_L)
        .child(
            card_surface()
                .flex()
                .flex_col()
                .gap(SPACE_M)
                .child(section_label(app, "common.memory"))
                .child(muted(app.t("instances.memory_hint")))
                .child(memory),
        )
        .child(
            card_surface()
                .flex()
                .flex_col()
                .gap(SPACE_XS)
                .child(section_label(app, "instances.java_path"))
                .child(info_row(
                    app.t("instances.java_path"),
                    record
                        .java_path
                        .clone()
                        .unwrap_or_else(|| app.t("common.none").to_string()),
                ))
                .child(info_row(
                    app.t("instances.jvm_args"),
                    if record.jvm_args.is_empty() {
                        app.t("common.none").to_string()
                    } else {
                        record.jvm_args.clone()
                    },
                ))
                .child(info_row(
                    app.t("instances.game_args"),
                    if record.game_args.is_empty() {
                        app.t("common.none").to_string()
                    } else {
                        record.game_args.clone()
                    },
                )),
        )
        .child(
            card_surface().child(meta_text(
                app.t("settings.data_dir_format")
                    .replace("{label}", app.t("settings.data_dir"))
                    .replace(
                        "{path}",
                        &app.paths
                            .game_dir(&record.id)
                            .unwrap_or_default()
                            .display()
                            .to_string(),
                    ),
            )),
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
                this.dialog_busy = false;
                this.dialog_field_error = None;
                this.dialog_error = None;
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

    let instances = app.instances.clone();
    let target_names: Vec<String> = instances
        .iter()
        .map(|summary| summary.record.name.clone())
        .collect();
    let target_selected = app.library_target.as_ref().and_then(|id| {
        instances
            .iter()
            .position(|summary| &summary.record.id == id)
    });
    let target = if instances.is_empty() {
        muted(app.t("library.no_instance")).into_any_element()
    } else {
        ExposedDropdown::new("library-target", app.t("library.target"))
            .options(target_names)
            .selected(target_selected)
            .full_width()
            .on_select(cx.listener(|this, index: &usize, _, cx| {
                let ids: Vec<String> = this
                    .instances
                    .iter()
                    .map(|summary| summary.record.id.clone())
                    .collect();
                if let Some(id) = ids.get(*index) {
                    this.library_target = Some(id.clone());
                    this.search_library(cx);
                }
            }))
            .into_any_element()
    };

    let query = app.search_input.read(cx).value().to_string();
    let mut search = SearchField::new(&app.search_input)
        .id("library-search")
        .field_style(SearchFieldStyle::Standard)
        .leading(material_icon("search", 24.0))
        .w_full();
    if !query.is_empty() {
        search = search.trailing(
            IconButton::new(
                "library-search-clear",
                material_icon("close", 24.0),
                app.t("common.clear"),
            )
            .on_click(cx.listener(|this, _, window, cx| this.clear_library_search(window, cx))),
        );
    }

    let results = app.search_results.clone();
    let total_hits = results
        .as_ref()
        .map(|result| result.total_hits)
        .unwrap_or(0);
    let result_rows: Vec<AnyElement> = results
        .map(|result| result.hits)
        .unwrap_or_default()
        .into_iter()
        .map(|hit| {
            let project_id = hit.id.clone();
            let supporting = if hit.description.trim().is_empty() {
                format!("{} · {}", hit.author, compact_number(hit.downloads))
            } else {
                format!(
                    "{}\n{} · {}",
                    hit.description,
                    hit.author,
                    compact_number(hit.downloads)
                )
            };
            let row = ListItem::new(format!("library-result-{}", hit.id), hit.title.clone())
                .leading(ListLeading::Tinted(
                    material_icon("extension", 24.0),
                    Tint::Teal,
                    40.,
                ))
                .supporting(supporting)
                .three_line(true)
                .container(ColorRole::SURFACE_CONTAINER_LOW);
            let action = MaterialButton::new(format!("library-install-{}", hit.id))
                .variant(ButtonVariant::Tonal)
                .button_size(ButtonSize::Small)
                .icon(material_icon("download", 20.0))
                .child(app.t("library.install"))
                .disabled(app.library_type != "modpack" && app.library_target.is_none())
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.install_from_library(project_id.clone(), cx)
                }))
                .into_any_element();
            row_with_action(row, Some(action))
        })
        .collect();

    let body =
        div()
            .flex()
            .flex_col()
            .gap(SPACE_L)
            .child(
                card_surface()
                    .flex()
                    .flex_col()
                    .gap(SPACE_M)
                    .child(search)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(SPACE_S)
                            .child(section_label(app, "library.type"))
                            .child(div().flex().flex_wrap().gap(SPACE_S).children(type_chips)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(SPACE_S)
                            .child(section_label(app, "library.target"))
                            .child(target),
                    ),
            )
            .child(if app.loading.modpack {
                loading_panel(app, "library-modpack-loading", "library.importing")
            } else if app.loading.library {
                loading_panel(app, "library-search-loading", "library.searching")
            } else if let Some(error) = &app.library_error {
                let retry = MaterialButton::new("library-retry")
                    .variant(ButtonVariant::Text)
                    .icon(material_icon("refresh", 20.0))
                    .child(app.t("common.retry"))
                    .on_click(cx.listener(|this, _, _, cx| this.search_library(cx)))
                    .into_any_element();
                error_panel(app, "library-error", error, Some(retry))
            } else if result_rows.is_empty() {
                empty_state(
                    app,
                    "library-empty",
                    "explore",
                    "library.no_results",
                    "library.no_results_body",
                    Some(
                        MaterialButton::new("library-clear-empty")
                            .variant(ButtonVariant::Text)
                            .icon(material_icon("close", 20.0))
                            .child(app.t("common.clear"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.clear_library_search(window, cx)
                            }))
                            .into_any_element(),
                    ),
                )
            } else {
                div()
                    .flex()
                    .flex_col()
                    .gap(SPACE_M)
                    .child(
                        type_role(div(), TypeRole::LabelLarge)
                            .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                            .child(
                                app.t("library.result_count")
                                    .replace("{count}", &total_hits.to_string()),
                            ),
                    )
                    .child(div().flex().flex_col().gap(SPACE_S).children(result_rows))
                    .into_any_element()
            })
            .into_any_element();

    page_frame(app, window, header, body)
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
                this.dialog_field_error = None;
                this.dialog_busy = false;
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
            let remove_name = account.name.clone();
            let kind = match account.kind {
                AccountKind::Microsoft => app.t("accounts.microsoft"),
                AccountKind::Offline => app.t("accounts.offline"),
                AccountKind::Littleskin => app.t("accounts.littleskin"),
            };
            let avatar = Avatar::new(
                format!("account-avatar-{}", account.id),
                account.name.clone(),
            );
            let avatar = match app.skins.get(&account.id).and_then(|path| path.clone()) {
                Some(path) => avatar.image(path),
                None => avatar.monogram(monogram_initials(&account.name)),
            };
            let row = ListItem::new(format!("account-{}", account.id), account.name.clone())
                .leading(ListLeading::Image(Some(
                    avatar.size(40.).into_any_element(),
                )))
                .supporting(kind)
                .selected(account.active)
                .container(ColorRole::SURFACE_CONTAINER_LOW)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if !this
                        .accounts
                        .iter()
                        .any(|item| item.id == set_id && item.active)
                    {
                        this.set_active_account(set_id.clone(), cx);
                    }
                }));
            let menu_id = format!("account-menu-{}", account.id);
            let menu = MaterialMenu::new(menu_id.clone()).items([MaterialMenuItem::new(
                app.t("accounts.remove"),
            )
            .leading(material_icon("person_remove", 20.0))
            .on_click(cx.listener({
                let remove_id = remove_id.clone();
                let remove_name = remove_name.clone();
                move |this, _, _, cx| {
                    this.menu_open = None;
                    this.prompt_remove_account(remove_id.clone(), remove_name.clone(), cx);
                }
            }))]);
            let trigger = menu_trigger(menu_id.clone(), app.t("common.more"), cx);
            let overflow = MenuPopup::new(format!("{menu_id}-popup"), trigger, menu)
                .open(menu_open(app, &menu_id))
                .on_open_change(cx.listener({
                    let menu_id = menu_id.clone();
                    move |this, open: &bool, _, cx| {
                        this.menu_open = open.then(|| menu_id.clone());
                        cx.notify();
                    }
                }))
                .into_any_element();
            let action = div()
                .flex()
                .items_center()
                .gap(SPACE_S)
                .children(account.active.then(|| active_badge(app)))
                .child(overflow)
                .into_any_element();
            row_with_action(row, Some(action))
        })
        .collect();

    let body = if app.loading.accounts && rows.is_empty() {
        loading_panel(app, "accounts-loading", "accounts.loading")
    } else if let Some(error) = app.account_error.as_ref().filter(|_| rows.is_empty()) {
        let retry = MaterialButton::new("accounts-retry")
            .variant(ButtonVariant::Text)
            .icon(material_icon("refresh", 20.0))
            .child(app.t("common.retry"))
            .on_click(cx.listener(|this, _, _, cx| this.refresh_accounts(cx)))
            .into_any_element();
        error_panel(app, "accounts-error", error, Some(retry))
    } else if rows.is_empty() {
        empty_state(
            app,
            "accounts-empty",
            "person",
            "accounts.empty",
            "accounts.empty_body",
            Some(
                MaterialButton::new("accounts-add-empty")
                    .variant(ButtonVariant::Text)
                    .icon(material_icon("person_add", 20.0))
                    .child(app.t("accounts.add"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.dialog = Some(Dialog::AddAccount);
                        this.dialog_error = None;
                        this.dialog_field_error = None;
                        this.dialog_busy = false;
                        cx.notify();
                    }))
                    .into_any_element(),
            ),
        )
    } else {
        div()
            .flex()
            .flex_col()
            .gap(SPACE_S)
            .children(rows)
            .into_any_element()
    };
    page_frame(app, window, header, body)
}

// ---------------------------------------------------------------------------
// Servers
// ---------------------------------------------------------------------------

fn servers_page(app: &mut Yuhina, window: &mut Window, cx: &mut Context<Yuhina>) -> AnyElement {
    let actions = Some(
        div()
            .flex()
            .flex_wrap()
            .gap(SPACE_S)
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
                        this.dialog_field_error = None;
                        this.dialog_busy = false;
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
            let remove_name = server.name.clone();
            let address = servers::format_address(&server.address, server.port);
            let supporting = match &status {
                Some(status) => {
                    let motd = status.motd.replace(['\n', '\r'], " ");
                    app.t("servers.status_format")
                        .replace("{motd}", motd.trim())
                        .replace("{players}", &status.players_online.to_string())
                        .replace("{max}", &status.players_max.to_string())
                        .replace("{latency}", &status.latency_ms.to_string())
                        .replace("{version}", &status.version_name)
                }
                None => app.t("servers.offline").to_string(),
            };
            let row = ListItem::new(format!("server-{}", server.id), server.name.clone())
                .leading(ListLeading::Tinted(
                    material_icon("dns", 24.0),
                    Tint::Blue,
                    40.,
                ))
                .supporting(supporting)
                .three_line(true)
                .trailing_text(address)
                .container(ColorRole::SURFACE_CONTAINER_LOW);
            let menu_id = format!("server-menu-{}", server.id);
            let menu = MaterialMenu::new(menu_id.clone()).items([
                MaterialMenuItem::new(app.t("servers.ping"))
                    .leading(material_icon("refresh", 20.0))
                    .on_click(cx.listener({
                        let ping_server = ping_server.clone();
                        move |this, _, _, cx| {
                            this.menu_open = None;
                            this.ping_server(ping_server.clone(), cx);
                        }
                    })),
                MaterialMenuItem::new(app.t("servers.remove"))
                    .leading(material_icon("delete", 20.0))
                    .divider(true)
                    .on_click(cx.listener({
                        let remove_id = remove_id.clone();
                        let remove_name = remove_name.clone();
                        move |this, _, _, cx| {
                            this.menu_open = None;
                            this.prompt_remove_server(remove_id.clone(), remove_name.clone(), cx);
                        }
                    })),
            ]);
            let trigger = menu_trigger(menu_id.clone(), app.t("common.more"), cx);
            let overflow = MenuPopup::new(format!("{menu_id}-popup"), trigger, menu)
                .open(menu_open(app, &menu_id))
                .on_open_change(cx.listener({
                    let menu_id = menu_id.clone();
                    move |this, open: &bool, _, cx| {
                        this.menu_open = open.then(|| menu_id.clone());
                        cx.notify();
                    }
                }))
                .into_any_element();
            let action = div()
                .flex()
                .items_center()
                .gap(SPACE_S)
                .children(status.as_ref().map(|status| {
                    Badge::large(
                        format!("server-players-{}", server.id),
                        format!("{}/{}", status.players_online, status.players_max),
                    )
                    .color(BadgeColor::Info)
                    .into_any_element()
                }))
                .child(overflow)
                .child(
                    MaterialButton::new(format!("server-join-{}", server.id))
                        .variant(ButtonVariant::Tonal)
                        .button_size(ButtonSize::Small)
                        .icon(material_icon("login", 20.0))
                        .child(app.t("servers.join"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            join_server_target(this, &join_server, cx)
                        })),
                )
                .into_any_element();
            row_with_action(row, Some(action))
        })
        .collect();

    let body = if app.loading.servers && rows.is_empty() {
        loading_panel(app, "servers-loading", "servers.loading")
    } else if let Some(error) = app.server_error.as_ref().filter(|_| rows.is_empty()) {
        let retry = MaterialButton::new("servers-retry")
            .variant(ButtonVariant::Text)
            .icon(material_icon("refresh", 20.0))
            .child(app.t("common.retry"))
            .on_click(cx.listener(|this, _, _, cx| this.refresh_servers(cx)))
            .into_any_element();
        error_panel(app, "servers-error", error, Some(retry))
    } else if rows.is_empty() {
        empty_state(
            app,
            "servers-empty",
            "dns",
            "servers.empty",
            "servers.empty_body",
            Some(
                MaterialButton::new("servers-add-empty")
                    .variant(ButtonVariant::Text)
                    .icon(material_icon("add", 20.0))
                    .child(app.t("servers.add"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.dialog = Some(Dialog::AddServer);
                        this.dialog_error = None;
                        this.dialog_field_error = None;
                        this.dialog_busy = false;
                        cx.notify();
                    }))
                    .into_any_element(),
            ),
        )
    } else {
        div()
            .flex()
            .flex_col()
            .gap(SPACE_S)
            .children(rows)
            .into_any_element()
    };
    page_frame(app, window, header, body)
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
    let finished = app.installs.values().any(|task| task.finished);
    let actions = finished.then(|| {
        MaterialButton::new("activity-clear-finished")
            .variant(ButtonVariant::Text)
            .icon(material_icon("clear_all", 20.0))
            .child(app.t("activity.dismiss"))
            .on_click(cx.listener(|this, _, _, cx| {
                this.installs.retain(|_, task| !task.finished);
                cx.notify();
            }))
            .into_any_element()
    });
    let header = page_header(app, "activity.title", "activity.subtitle", actions);

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
                        .font_family("monospace")
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .child(line.text.clone())
                        .into_any_element()
                })
                .collect();
            card_surface()
                .flex()
                .flex_col()
                .gap(SPACE_S)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(SPACE_S)
                        .child(type_role(div(), TypeRole::TitleMedium).child(name))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(SPACE_S)
                                .child(
                                    status_chip(StatusKind::Success)
                                        .child(app.t("instances.running")),
                                )
                                .child(
                                    MaterialButton::new(format!("activity-stop-{id}"))
                                        .variant(ButtonVariant::Tonal)
                                        .button_size(ButtonSize::Small)
                                        .icon(material_icon("stop", 20.0))
                                        .child(app.t("instances.stop"))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.stop_game(&stop_id, cx)
                                        })),
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
            let dismiss_id = task.instance_id.clone();
            card_surface()
                .flex()
                .flex_col()
                .gap(SPACE_S)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(SPACE_S)
                        .child(
                            type_role(div(), TypeRole::TitleMedium).child(
                                app.t("activity.install_format")
                                    .replace("{name}", &name)
                                    .replace("{phase}", app.phase_label(task.phase)),
                            ),
                        )
                        .children(task.finished.then(|| {
                            IconButton::new(
                                format!("activity-dismiss-{dismiss_id}"),
                                material_icon("close", 20.0),
                                app.t("activity.dismiss"),
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.installs.remove(&dismiss_id);
                                    cx.notify();
                                },
                            ))
                        })),
                )
                .children(task.error.as_ref().map(|error| {
                    type_role(div(), TypeRole::BodySmall)
                        .text_color(color(ColorRole::ERROR))
                        .child(error.clone())
                }))
                .child(
                    LinearProgress::new(
                        format!("activity-progress-{}", task.instance_id),
                        app.t("instances.install_progress_a11y"),
                    )
                    .value(Some(progress_value(task.percent / 100.0)))
                    .thickness(px(4.)),
                )
                .children(task.message.as_ref().map(|message| {
                    type_role(div(), TypeRole::BodySmall)
                        .font_family("monospace")
                        .line_clamp(3)
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .child(message.clone())
                }))
                .children(task.error.is_some().then(|| {
                    MaterialButton::new(format!("activity-retry-{retry_id}"))
                        .variant(ButtonVariant::Text)
                        .button_size(ButtonSize::Small)
                        .icon(material_icon("refresh", 20.0))
                        .child(app.t("common.retry"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.start_install(retry_id.clone(), cx)
                        }))
                }))
                .into_any_element()
        })
        .collect();

    let browse = || {
        MaterialButton::new("activity-browse")
            .variant(ButtonVariant::Text)
            .icon(material_icon("grid_view", 20.0))
            .child(app.t("activity.browse_instances"))
            .on_click(cx.listener(|this, _, _, cx| this.set_page(Page::Instances, cx)))
            .into_any_element()
    };

    let body = div()
        .flex()
        .flex_col()
        .gap(SPACE_XL)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(SPACE_M)
                .child(section_label(app, "activity.running"))
                .child(if running.is_empty() {
                    empty_state(
                        app,
                        "activity-running-empty",
                        "sports_esports",
                        "activity.running_empty",
                        "activity.running_empty_body",
                        Some(browse()),
                    )
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .gap(SPACE_S)
                        .children(running)
                        .into_any_element()
                }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(SPACE_M)
                .child(section_label(app, "activity.installs"))
                .child(if installs.is_empty() {
                    empty_state(
                        app,
                        "activity-installs-empty",
                        "download",
                        "activity.installs_empty",
                        "activity.empty_body",
                        Some(browse()),
                    )
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .gap(SPACE_S)
                        .children(installs)
                        .into_any_element()
                }),
        )
        .into_any_element();

    page_frame(app, window, header, body)
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

fn settings_page(app: &mut Yuhina, window: &mut Window, cx: &mut Context<Yuhina>) -> AnyElement {
    let header = page_header(app, "settings.title", "settings.subtitle", None);

    let theme_selected = match app.settings.theme {
        ThemePreference::Dark => "dark",
        ThemePreference::Light => "light",
        ThemePreference::System => "system",
    };
    let theme = ConnectedButtonGroup::new("settings-theme", app.t("settings.theme"))
        .options([
            ("dark", app.t("settings.theme_dark")),
            ("light", app.t("settings.theme_light")),
            ("system", app.t("settings.theme_system")),
        ])
        .selected(theme_selected)
        .selection(GroupSelection::Single)
        .segmented()
        .on_change(cx.listener(|this, value: &SharedString, window, cx| {
            let preference = match value.as_ref() {
                "light" => ThemePreference::Light,
                "system" => ThemePreference::System,
                _ => ThemePreference::Dark,
            };
            this.set_theme(preference, window, cx);
        }));

    let language_selected = match app.settings.language_mode {
        LanguageMode::System => "system",
        LanguageMode::ZhCn => "zh",
        LanguageMode::EnUs => "en",
    };
    let language = ConnectedButtonGroup::new("settings-language", app.t("settings.language"))
        .options([
            ("system", app.t("settings.language_system")),
            ("zh", "中文"),
            ("en", "English"),
        ])
        .selected(language_selected)
        .selection(GroupSelection::Single)
        .segmented()
        .on_change(cx.listener(|this, value: &SharedString, window, cx| {
            let mode = match value.as_ref() {
                "zh" => LanguageMode::ZhCn,
                "en" => LanguageMode::EnUs,
                _ => LanguageMode::System,
            };
            this.set_language_mode(mode, window, cx);
        }));

    let motion_selected = match app.settings.reduce_motion {
        ReduceMotion::System => "system",
        ReduceMotion::On => "on",
        ReduceMotion::Off => "off",
    };
    let motion = ConnectedButtonGroup::new("settings-motion", app.t("settings.motion"))
        .options([
            ("system", app.t("settings.motion_system")),
            ("on", app.t("settings.motion_on")),
            ("off", app.t("settings.motion_off")),
        ])
        .selected(motion_selected)
        .selection(GroupSelection::Single)
        .segmented()
        .on_change(cx.listener(|this, value: &SharedString, _, cx| {
            let preference = match value.as_ref() {
                "on" => ReduceMotion::On,
                "off" => ReduceMotion::Off,
                _ => ReduceMotion::System,
            };
            this.set_reduce_motion_preference(preference, cx);
        }));

    let scale_percent = (app.settings.text_scale * 100.).round() as u32;
    let scale_selected = format!("{scale_percent}");
    let text_scale = ConnectedButtonGroup::new("settings-text-scale", app.t("settings.text_scale"))
        .options([
            ("100", "100%"),
            ("125", "125%"),
            ("150", "150%"),
            ("200", "200%"),
        ])
        .selected(scale_selected)
        .selection(GroupSelection::Single)
        .segmented()
        .on_change(cx.listener(|this, value: &SharedString, window, cx| {
            if let Ok(percent) = value.parse::<f32>() {
                this.set_text_scale(percent / 100., window, cx);
            }
        }));

    let source_selected = match app.settings.download_source {
        DownloadSource::Auto => "auto",
        DownloadSource::Bmcl => "bmcl",
        DownloadSource::Official => "official",
    };
    let source = ConnectedButtonGroup::new("settings-source", app.t("settings.download_source"))
        .options([
            ("auto", app.t("settings.download_auto")),
            ("bmcl", app.t("settings.download_bmcl")),
            ("official", app.t("settings.download_official")),
        ])
        .selected(source_selected)
        .selection(GroupSelection::Single)
        .segmented()
        .on_change(cx.listener(|this, value: &SharedString, _, cx| {
            let source = match value.as_ref() {
                "bmcl" => DownloadSource::Bmcl,
                "official" => DownloadSource::Official,
                _ => DownloadSource::Auto,
            };
            if this.settings.download_source != source {
                this.settings.download_source = source;
                let _ = core_settings::save(&this.paths, &this.settings);
                cx.notify();
            }
        }));

    let memory_value = app.settings.instance_defaults.memory_mb;
    let memory_unit = app.t("settings.memory_value").to_string();
    let memory = MaterialSlider::new(&app.memory_slider, app.t("settings.memory"))
        .format(move |value| {
            memory_unit
                .replace("{mb}", &format!("{:.0}", value.round()))
                .into()
        })
        .size(SliderSize::Medium);

    let java_rows: Vec<AnyElement> = app
        .java_runtimes
        .iter()
        .enumerate()
        .map(|(index, runtime)| {
            let row = ListItem::new(
                format!("java-runtime-{index}"),
                app.t("settings.java_version")
                    .replace("{major}", &runtime.major.to_string()),
            )
            .leading(ListLeading::Tinted(
                material_icon("coffee", 24.0),
                Tint::Amber,
                40.,
            ))
            .supporting(runtime.path.clone())
            .container(ColorRole::SURFACE_CONTAINER_LOW);
            let trailing = runtime.managed.then(|| {
                Badge::large(
                    format!("java-managed-{index}"),
                    app.t("settings.java_managed"),
                )
                .color(BadgeColor::Tertiary)
                .into_any_element()
            });
            row_with_action(row, trailing)
        })
        .collect();

    let body = div()
        .flex()
        .flex_col()
        .gap(SPACE_L)
        .child(
            card_surface()
                .flex()
                .flex_col()
                .gap(SPACE_L)
                .child(section_label(app, "settings.appearance"))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(SPACE_S)
                        .child(muted(app.t("settings.text_scale")))
                        .child(muted(app.t("settings.text_scale_hint")))
                        .child(text_scale)
                        .child(
                            MaterialButton::new("settings-text-scale-reset")
                                .variant(ButtonVariant::Text)
                                .button_size(ButtonSize::Small)
                                .child(app.t("settings.text_scale_reset"))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.reset_text_scale(window, cx)
                                })),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(SPACE_S)
                        .child(muted(app.t("settings.theme")))
                        .child(theme),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(SPACE_S)
                        .child(muted(app.t("settings.language")))
                        .child(language),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(SPACE_S)
                        .child(muted(app.t("settings.motion")))
                        .child(motion),
                ),
        )
        .child(
            card_surface()
                .flex()
                .flex_col()
                .gap(SPACE_L)
                .child(section_label(app, "settings.downloads"))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(SPACE_S)
                        .child(muted(app.t("settings.download_source")))
                        .child(source),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(SPACE_S)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(muted(app.t("settings.memory")))
                                .child(
                                    type_role(div(), TypeRole::TitleSmall).child(
                                        app.t("settings.memory_value")
                                            .replace("{mb}", &memory_value.to_string()),
                                    ),
                                ),
                        )
                        .child(memory),
                ),
        )
        .child(
            card_surface()
                .flex()
                .flex_col()
                .gap(SPACE_M)
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
                                .icon(material_icon("refresh", 20.0))
                                .child(app.t("settings.java_rescan"))
                                .on_click(cx.listener(|this, _, _, cx| this.refresh_java(cx))),
                        ),
                )
                .child(
                    ListItem::new("java-auto-download", app.t("settings.java_auto_download"))
                        .supporting(app.t("settings.java_auto_download_hint"))
                        .trailing(ListTrailing::Switch(app.settings.java.auto_download))
                        .container(ColorRole::SURFACE_CONTAINER_LOW)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.settings.java.auto_download = !this.settings.java.auto_download;
                            let _ = core_settings::save(&this.paths, &this.settings);
                            cx.notify();
                        })),
                )
                .child(section_label(app, "settings.java_detected"))
                .child(if app.loading.java {
                    loading_panel(app, "java-loading", "common.loading")
                } else if java_rows.is_empty() {
                    muted(app.t("common.empty")).into_any_element()
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .gap(SPACE_S)
                        .children(java_rows)
                        .into_any_element()
                }),
        )
        .child(
            card_surface()
                .flex()
                .flex_col()
                .gap(SPACE_M)
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
                        .gap(SPACE_XS)
                        .child(muted(app.t("settings.data_dir")))
                        .child(meta_text(app.paths.root().display().to_string())),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(SPACE_M)
                        .flex_wrap()
                        .child(if app.loading.update {
                            muted(app.t("settings.checking")).into_any_element()
                        } else if let Some(update) = &app.update {
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    type_role(div(), TypeRole::TitleSmall).child(
                                        app.t("settings.update_available_version")
                                            .replace("{label}", app.t("settings.update_available"))
                                            .replace("{version}", &update.version),
                                    ),
                                )
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
                                .gap(SPACE_S)
                                .child(
                                    MaterialButton::new("check-updates")
                                        .variant(ButtonVariant::Text)
                                        .button_size(ButtonSize::Small)
                                        .icon(material_icon("refresh", 20.0))
                                        .child(app.t("settings.check_updates"))
                                        .disabled(app.loading.update)
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.check_updates(cx)),
                                        ),
                                )
                                .children(app.update.as_ref().map(|update| {
                                    let url = update.page_url.clone();
                                    MaterialButton::new("open-release")
                                        .variant(ButtonVariant::Tonal)
                                        .button_size(ButtonSize::Small)
                                        .icon(material_icon("open_in_new", 20.0))
                                        .child(app.t("settings.open_release"))
                                        .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url)))
                                        .into_any_element()
                                })),
                        ),
                ),
        )
        .into_any_element();

    page_frame(app, window, header, body)
}

#[cfg(test)]
mod tests {
    use super::{PAGE_GUTTER, RADIUS_CARD, SPACE_XL, columns_for};
    use gpui_kit_material::components::WindowWidthClass;
    use gpui_kit_material::tokens::typography::BASE_PX;

    #[test]
    fn the_card_grid_uses_the_window_class_not_the_item_count() {
        assert_eq!(columns_for(WindowWidthClass::Compact), 1);
        assert_eq!(columns_for(WindowWidthClass::Medium), 2);
        assert_eq!(columns_for(WindowWidthClass::Expanded), 2);
        assert_eq!(columns_for(WindowWidthClass::Large), 3);
        assert_eq!(columns_for(WindowWidthClass::ExtraLarge), 3);
    }

    #[test]
    fn page_geometry_tokens_are_rem_based() {
        // 32dp at the 100% baseline; the same token grows at 200%.
        assert_eq!(PAGE_GUTTER.0, 32.0 / BASE_PX);
        assert_eq!(SPACE_XL.0, 24.0 / BASE_PX);
        assert_eq!(RADIUS_CARD.0, 24.0 / BASE_PX);
    }
}
