use super::pages::{error_panel, loading_panel, muted};
use super::*;
use crate::core::instances::LoaderType;

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

fn dialog_content(content: Div) -> Div {
    content.px(rem(24.)).pb(rem(16.))
}

fn compact(window: &Window) -> bool {
    WindowWidthClass::of(f32::from(window.viewport_size().width)) == WindowWidthClass::Compact
}

fn dialog_size(window: &Window) -> DialogSize {
    dialog_size_for(WindowWidthClass::of(f32::from(
        window.viewport_size().width,
    )))
}

fn dialog_size_for(class: WindowWidthClass) -> DialogSize {
    if class == WindowWidthClass::Compact {
        DialogSize::FullScreen
    } else {
        DialogSize::Medium
    }
}

fn finish_dialog(dialog: MaterialDialog, window: &Window, lang: Language) -> MaterialDialog {
    if compact(window) {
        dialog.close_button(i18n::tr(lang, "common.close"))
    } else {
        dialog
    }
}

fn dialog_error(app: &Yuhina) -> Option<AnyElement> {
    app.dialog_error.as_ref().map(|error| {
        notice_banner("dialog-error", NoticeKind::Danger, error.clone()).into_any_element()
    })
}

fn field_error<'a>(app: &'a Yuhina, field: &str) -> Option<&'a String> {
    (app.dialog_field_error == Some(field))
        .then_some(app.dialog_error.as_ref())
        .flatten()
}

/// The version list puts curated modded versions first, then the selected
/// version, then the newest releases. Taking only the eight newest releases
/// hid 1.20.1 and 1.21.1, which most modpacks still target.
fn version_options(app: &Yuhina) -> Vec<String> {
    const CURATED: [&str; 5] = ["1.21.1", "1.20.1", "1.19.2", "1.18.2", "1.16.5"];
    let releases: Vec<String> = app
        .versions
        .as_ref()
        .map(|manifest| {
            manifest
                .versions
                .iter()
                .filter(|version| version.kind == "release")
                .map(|version| version.id.clone())
                .collect()
        })
        .unwrap_or_default();
    let mut out: Vec<String> = Vec::new();
    for version in CURATED {
        if releases.iter().any(|candidate| candidate == version) {
            out.push(version.to_string());
        }
    }
    if !app.create_version.is_empty() && !out.iter().any(|version| version == &app.create_version) {
        out.push(app.create_version.clone());
    }
    for version in releases {
        if out.len() >= 24 {
            break;
        }
        if !out.iter().any(|candidate| candidate == &version) {
            out.push(version);
        }
    }
    out
}

fn destructive_button(id: &str, label: &'static str) -> MaterialButton {
    MaterialButton::new(id.to_string())
        .variant(ButtonVariant::Text)
        .text_color(color(ColorRole::ERROR))
        .child(label)
}

fn cancel_button(cx: &mut Context<Yuhina>, lang: Language, busy: bool) -> MaterialButton {
    MaterialButton::new("dialog-cancel")
        .variant(ButtonVariant::Text)
        .disabled(busy)
        .child(i18n::tr(lang, "common.cancel"))
        .on_click(cx.listener(|this, _, _, cx| {
            if !this.dialog_busy {
                this.dialog = None;
                cx.notify();
            }
        }))
}

/// Renders the currently open dialog as a modal overlay.
pub fn render(
    app: &mut Yuhina,
    window: &mut Window,
    cx: &mut Context<Yuhina>,
) -> Option<AnyElement> {
    let dialog = app.dialog.clone()?;
    let lang = app.lang;
    let weak = cx.entity().downgrade();
    let dismiss = move |_: &mut Window, cx: &mut App| {
        let _ = weak.update(cx, |this: &mut Yuhina, cx: &mut Context<Yuhina>| {
            if !this.dialog_busy {
                this.dialog = None;
                cx.notify();
            }
        });
    };
    let size = dialog_size(window);
    let busy = app.dialog_busy;

    match dialog {
        Dialog::CreateInstance => {
            let versions = version_options(app);
            let selected_version = versions
                .iter()
                .position(|version| version == &app.create_version);
            let version_dropdown = ExposedDropdown::new("create-version", app.t("common.version"))
                .options(versions.clone())
                .selected(selected_version)
                .full_width()
                .menu_max_height(320.)
                .on_select(cx.listener(move |this, index: &usize, _, cx| {
                    if let Some(version) = versions.get(*index) {
                        this.choose_create_version(version.clone(), cx);
                    }
                }));

            let loaders = [
                (LoaderType::Vanilla, "vanilla"),
                (LoaderType::Fabric, "fabric"),
                (LoaderType::Quilt, "quilt"),
                (LoaderType::Forge, "forge"),
                (LoaderType::Neoforge, "neoforge"),
            ];
            let loader_selected = loaders
                .iter()
                .find(|(loader, _)| *loader == app.create_loader)
                .map(|(_, value)| *value)
                .unwrap_or("vanilla");
            let loader_options: Vec<(&str, &str)> = loaders
                .iter()
                .map(|(loader, value)| (*value, app.loader_label(*loader)))
                .collect();
            let loader_group = ConnectedButtonGroup::new("create-loader", app.t("common.loader"))
                .options(loader_options)
                .selected(loader_selected)
                .selection(GroupSelection::Single)
                .segmented()
                .on_change(cx.listener(|this, value: &SharedString, _, cx| {
                    let loader = match value.as_ref() {
                        "fabric" => LoaderType::Fabric,
                        "quilt" => LoaderType::Quilt,
                        "forge" => LoaderType::Forge,
                        "neoforge" => LoaderType::Neoforge,
                        _ => LoaderType::Vanilla,
                    };
                    this.choose_create_loader(loader, cx);
                }));

            let loader_version_section: Option<AnyElement> =
                if matches!(app.create_loader, LoaderType::Forge | LoaderType::Neoforge) {
                    let body: AnyElement = if app.loader_versions_loading {
                        loading_panel(
                            app,
                            "create-loader-loading",
                            "instances.loader_versions_loading",
                        )
                    } else if let Some(error) = &app.loader_versions_error {
                        let retry = MaterialButton::new("create-loader-retry")
                            .variant(ButtonVariant::Text)
                            .child(app.t("common.retry"))
                            .on_click(
                                cx.listener(|this, _, _, cx| this.refresh_loader_versions(cx)),
                            )
                            .into_any_element();
                        error_panel(app, "create-loader-error", error, Some(retry))
                    } else if app.loader_versions.is_empty() {
                        muted(app.t("instances.loader_versions_empty")).into_any_element()
                    } else {
                        let labels: Vec<String> = app
                            .loader_versions
                            .iter()
                            .map(|option| {
                                if option.recommended {
                                    format!(
                                        "{} · {}",
                                        option.version,
                                        app.t("instances.loader_recommended")
                                    )
                                } else {
                                    option.version.clone()
                                }
                            })
                            .collect();
                        let selected = app.create_loader_version.as_ref().and_then(|current| {
                            app.loader_versions
                                .iter()
                                .position(|option| &option.version == current)
                        });
                        let values: Vec<String> = app
                            .loader_versions
                            .iter()
                            .map(|option| option.version.clone())
                            .collect();
                        ExposedDropdown::new(
                            "create-loader-version",
                            app.t("instances.loader_version_hint"),
                        )
                        .options(labels)
                        .selected(selected)
                        .full_width()
                        .menu_max_height(320.)
                        .on_select(cx.listener(move |this, index: &usize, _, cx| {
                            if let Some(version) = values.get(*index) {
                                this.create_loader_version = Some(version.clone());
                                cx.notify();
                            }
                        }))
                        .into_any_element()
                    };
                    Some(
                        div()
                            .flex()
                            .flex_col()
                            .gap(rem(8.))
                            .child(muted(app.t("instances.loader_version_hint")))
                            .child(body)
                            .into_any_element(),
                    )
                } else {
                    None
                };

            let content = dialog_content(div())
                .flex()
                .flex_col()
                .gap(rem(16.))
                .child(
                    TextField::new("create-name", &app.create_name)
                        .w_full()
                        .label(app.t("instances.name"))
                        .placeholder(app.t("instances.name_placeholder"))
                        .error(field_error(app, "create-name").is_some())
                        .when_some(field_error(app, "create-name"), |field, error| {
                            field.supporting_text(error.clone())
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(rem(8.))
                        .child(muted(app.t("instances.version_hint")))
                        .child(version_dropdown),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(rem(8.))
                        .child(muted(app.t("instances.loader_hint")))
                        .child(loader_group),
                )
                .children(loader_version_section)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(rem(8.))
                        .child(muted(app.t("instances.memory_hint")))
                        .child(
                            MaterialSlider::new(&app.dialog_memory_slider, app.t("common.memory"))
                                .format(|value| format!("{:.0} MB", value.round()).into())
                                .size(SliderSize::Medium),
                        ),
                )
                .children(
                    (app.dialog_field_error.is_none())
                        .then(|| dialog_error(app))
                        .flatten(),
                );

            let confirm = MaterialButton::new("create-instance-confirm")
                .variant(ButtonVariant::Filled)
                .loading(busy, app.t("common.creating"))
                .disabled(busy)
                .child(app.t("common.create"))
                .on_click(cx.listener(|this, _, _, cx| this.create_instance(cx)));

            let mut material = MaterialDialog::new("create-instance-dialog")
                .headline(app.t("dialog.create_instance"))
                .size(size)
                .dividers(true)
                .modal(true)
                .track_body_scroll(&app.dialog_scroll)
                .content(content)
                .action(cancel_button(cx, lang, busy))
                .action(confirm)
                .on_dismiss(dismiss);
            material = finish_dialog(material, window, lang);
            Some(material.into_any_element())
        }
        Dialog::AddAccount => {
            let tabs = [
                (AccountTab::Offline, "offline", app.t("accounts.offline")),
                (
                    AccountTab::Microsoft,
                    "microsoft",
                    app.t("accounts.microsoft"),
                ),
                (
                    AccountTab::LittleSkin,
                    "littleskin",
                    app.t("accounts.littleskin"),
                ),
            ];
            let selected = tabs
                .iter()
                .find(|(tab, _, _)| *tab == app.account_tab)
                .map(|(_, value, _)| *value)
                .unwrap_or("offline");
            let options: Vec<(&str, &str)> = tabs
                .iter()
                .map(|(_, value, label)| (*value, *label))
                .collect();
            let group = ConnectedButtonGroup::new("account-tabs", app.t("accounts.add"))
                .options(options)
                .selected(selected)
                .selection(GroupSelection::Single)
                .segmented()
                .on_change(cx.listener(|this, value: &SharedString, _, cx| {
                    this.account_tab = match value.as_ref() {
                        "microsoft" => AccountTab::Microsoft,
                        "littleskin" => AccountTab::LittleSkin,
                        _ => AccountTab::Offline,
                    };
                    this.dialog_field_error = None;
                    this.dialog_error = None;
                    cx.notify();
                }));

            let form: AnyElement = match app.account_tab {
                AccountTab::Offline => div()
                    .flex()
                    .flex_col()
                    .gap(rem(12.))
                    .child(
                        TextField::new("offline-name", &app.account_name)
                            .w_full()
                            .label(app.t("accounts.name"))
                            .placeholder(app.t("accounts.name_placeholder"))
                            .error(field_error(app, "offline-name").is_some())
                            .when_some(field_error(app, "offline-name"), |field, error| {
                                field.supporting_text(error.clone())
                            }),
                    )
                    .child(muted(app.t("accounts.offline_hint")))
                    .into_any_element(),
                AccountTab::Microsoft => div()
                    .child(muted(app.t("accounts.microsoft_hint")))
                    .into_any_element(),
                AccountTab::LittleSkin => div()
                    .flex()
                    .flex_col()
                    .gap(rem(12.))
                    .child(
                        TextField::new("authlib-server", &app.authlib_server)
                            .w_full()
                            .label(app.t("accounts.server_url"))
                            .placeholder(app.t("accounts.server_url_placeholder"))
                            .error(field_error(app, "authlib-server").is_some())
                            .when_some(field_error(app, "authlib-server"), |field, error| {
                                field.supporting_text(error.clone())
                            }),
                    )
                    .child(
                        TextField::new("authlib-user", &app.authlib_user)
                            .w_full()
                            .label(app.t("accounts.email"))
                            .input_type(TextFieldInputType::Email)
                            .error(field_error(app, "authlib-user").is_some()),
                    )
                    .child(
                        TextField::new("authlib-password", &app.authlib_password)
                            .w_full()
                            .label(app.t("accounts.password"))
                            .input_type(TextFieldInputType::Password)
                            .error(field_error(app, "authlib-password").is_some()),
                    )
                    .into_any_element(),
            };

            let content = dialog_content(div())
                .flex()
                .flex_col()
                .gap(rem(16.))
                .child(group)
                .child(form)
                .children(
                    (app.dialog_field_error.is_none())
                        .then(|| dialog_error(app))
                        .flatten(),
                );

            let (label, confirm_id) = match app.account_tab {
                AccountTab::Offline => (app.t("common.add"), "add-offline-confirm"),
                AccountTab::Microsoft => (app.t("accounts.login"), "add-microsoft-confirm"),
                AccountTab::LittleSkin => (app.t("accounts.login"), "add-authlib-confirm"),
            };
            let confirm = MaterialButton::new(confirm_id)
                .variant(ButtonVariant::Filled)
                .loading(busy, app.t("common.signing_in"))
                .disabled(busy)
                .child(label)
                .on_click(cx.listener(|this, _, window, cx| match this.account_tab {
                    AccountTab::Offline => this.add_offline_account(window, cx),
                    AccountTab::Microsoft => this.start_microsoft_login(cx),
                    AccountTab::LittleSkin => this.add_authlib_account(cx),
                }));

            let mut material = MaterialDialog::new("add-account-dialog")
                .headline(app.t("dialog.add_account"))
                .size(size)
                .dividers(true)
                .modal(true)
                .track_body_scroll(&app.dialog_scroll)
                .content(content)
                .action(cancel_button(cx, lang, busy))
                .action(confirm)
                .on_dismiss(dismiss);
            material = finish_dialog(material, window, lang);
            Some(material.into_any_element())
        }
        Dialog::MicrosoftLogin {
            user_code,
            verification_uri,
            waiting,
        } => {
            let content = dialog_content(div())
                .flex()
                .flex_col()
                .items_center()
                .gap(rem(16.))
                .child(
                    type_role(div(), TypeRole::BodyMedium)
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .text_center()
                        .child(app.t("accounts.microsoft_waiting")),
                )
                .child(
                    div()
                        .w_full()
                        .rounded(rem(20.))
                        .bg(color(ColorRole::SURFACE_CONTAINER_HIGHEST))
                        .p(rem(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            type_scale(div(), TypeRole::HeadlineLarge.emphasized()).child(
                                if user_code.is_empty() {
                                    app.t("common.loading").to_string()
                                } else {
                                    user_code.clone()
                                },
                            ),
                        ),
                )
                .child(
                    type_role(div(), TypeRole::BodySmall)
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .child(verification_uri.clone()),
                )
                .children((!user_code.is_empty()).then(|| {
                    let url = verification_uri.clone();
                    let code = user_code.clone();
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(rem(8.))
                        .child(
                            MaterialButton::new("microsoft-copy-code")
                                .variant(ButtonVariant::Text)
                                .icon(material_icon("content_copy", 20.0))
                                .child(app.t("common.copy"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(code.clone()));
                                    this.show_toast(this.t("common.copied"), false, cx);
                                })),
                        )
                        .child(
                            MaterialButton::new("microsoft-open-browser")
                                .variant(ButtonVariant::Tonal)
                                .icon(material_icon("open_in_new", 20.0))
                                .child(app.t("accounts.microsoft"))
                                .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))),
                        )
                        .into_any_element()
                }))
                .child(if waiting {
                    loading_panel(app, "microsoft-waiting", "common.loading")
                } else {
                    div().into_any_element()
                });

            let mut material = MaterialDialog::new("microsoft-login-dialog")
                .headline(app.t("accounts.microsoft"))
                .size(size)
                .modal(true)
                .track_body_scroll(&app.dialog_scroll)
                .content(content)
                .action(cancel_button(cx, lang, busy))
                .on_dismiss(dismiss);
            material = finish_dialog(material, window, lang);
            Some(material.into_any_element())
        }
        Dialog::AddServer => {
            let content = dialog_content(div())
                .flex()
                .flex_col()
                .gap(rem(16.))
                .child(
                    TextField::new("server-name", &app.server_name)
                        .w_full()
                        .label(app.t("servers.name")),
                )
                .child(
                    TextField::new("server-address", &app.server_address)
                        .w_full()
                        .label(app.t("servers.address"))
                        .placeholder(app.t("servers.address_placeholder"))
                        .error(field_error(app, "server-address").is_some())
                        .when_some(field_error(app, "server-address"), |field, error| {
                            field.supporting_text(error.clone())
                        }),
                )
                .children(
                    (app.dialog_field_error.is_none())
                        .then(|| dialog_error(app))
                        .flatten(),
                );
            let confirm = MaterialButton::new("add-server-confirm")
                .variant(ButtonVariant::Filled)
                .loading(busy, app.t("common.adding"))
                .disabled(busy)
                .child(app.t("common.add"))
                .on_click(cx.listener(|this, _, _, cx| this.add_server(cx)));
            let mut material = MaterialDialog::new("add-server-dialog")
                .headline(app.t("dialog.add_server"))
                .size(size)
                .dividers(true)
                .modal(true)
                .track_body_scroll(&app.dialog_scroll)
                .content(content)
                .action(cancel_button(cx, lang, busy))
                .action(confirm)
                .on_dismiss(dismiss);
            material = finish_dialog(material, window, lang);
            Some(material.into_any_element())
        }
        Dialog::ImportModpack => {
            let content = dialog_content(div())
                .flex()
                .flex_col()
                .gap(rem(16.))
                .child(muted(app.t("library.import_hint")))
                .child(
                    TextField::new("modpack-path", &app.modpack_path)
                        .w_full()
                        .label(app.t("library.import_path"))
                        .error(field_error(app, "modpack-path").is_some())
                        .when_some(field_error(app, "modpack-path"), |field, error| {
                            field.supporting_text(error.clone())
                        }),
                );
            let confirm = MaterialButton::new("import-modpack-confirm")
                .variant(ButtonVariant::Filled)
                .icon(material_icon("upload_file", 20.0))
                .loading(busy, app.t("common.importing"))
                .disabled(busy)
                .child(app.t("library.import_confirm"))
                .on_click(cx.listener(|this, _, _, cx| this.import_modpack(cx)));
            let mut material = MaterialDialog::new("import-modpack-dialog")
                .headline(app.t("library.import_title"))
                .size(size)
                .dividers(true)
                .modal(true)
                .track_body_scroll(&app.dialog_scroll)
                .content(content)
                .action(cancel_button(cx, lang, busy))
                .action(confirm)
                .on_dismiss(dismiss);
            material = finish_dialog(material, window, lang);
            Some(material.into_any_element())
        }
        Dialog::DeleteInstance { id, name } => {
            let target = id.clone();
            let supporting = app
                .t("dialog.delete_instance_named")
                .replace("{name}", &name);
            let confirm = destructive_button("delete-instance-confirm", app.t("common.delete"))
                .loading(busy, app.t("common.removing"))
                .disabled(busy)
                .on_click(
                    cx.listener(move |this, _, _, cx| this.delete_instance(target.clone(), cx)),
                );
            let mut material = MaterialDialog::new("delete-instance-dialog")
                .headline(app.t("dialog.delete_instance"))
                .supporting_text(supporting)
                .size(size)
                .modal(true)
                .action(cancel_button(cx, lang, busy))
                .action(confirm)
                .on_dismiss(dismiss);
            material = finish_dialog(material, window, lang);
            Some(material.into_any_element())
        }
        Dialog::DeleteServer { id, name } => {
            let target = id.clone();
            let supporting = app.t("dialog.delete_server_named").replace("{name}", &name);
            let confirm = destructive_button("delete-server-confirm", app.t("servers.remove"))
                .disabled(busy)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.confirm_remove_server(target.clone(), cx)
                }));
            let mut material = MaterialDialog::new("delete-server-dialog")
                .headline(app.t("dialog.delete_server"))
                .supporting_text(supporting)
                .size(size)
                .modal(true)
                .action(cancel_button(cx, lang, busy))
                .action(confirm)
                .on_dismiss(dismiss);
            material = finish_dialog(material, window, lang);
            Some(material.into_any_element())
        }
        Dialog::RemoveAccount { id, name } => {
            let target = id.clone();
            let supporting = app
                .t("dialog.remove_account_named")
                .replace("{name}", &name);
            let confirm = destructive_button("remove-account-confirm", app.t("accounts.remove"))
                .disabled(busy)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.confirm_remove_account(target.clone(), cx)
                }));
            let mut material = MaterialDialog::new("remove-account-dialog")
                .headline(app.t("dialog.remove_account"))
                .supporting_text(supporting)
                .size(size)
                .modal(true)
                .action(cancel_button(cx, lang, busy))
                .action(confirm)
                .on_dismiss(dismiss);
            material = finish_dialog(material, window, lang);
            Some(material.into_any_element())
        }
        Dialog::ConfirmRemoveContent {
            instance_id,
            project_type,
            filename,
        } => {
            let remove_filename = filename.clone();
            let confirm =
                destructive_button("remove-content-confirm", app.t("instances.remove_mod"))
                    .disabled(busy)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.remove_content(
                            instance_id.clone(),
                            project_type.clone(),
                            remove_filename.clone(),
                            cx,
                        )
                    }));
            let mut material = MaterialDialog::new("remove-content-dialog")
                .headline(app.t("instances.remove_mod"))
                .supporting_text(filename.clone())
                .size(size)
                .modal(true)
                .action(cancel_button(cx, lang, busy))
                .action(confirm)
                .on_dismiss(dismiss);
            material = finish_dialog(material, window, lang);
            Some(material.into_any_element())
        }
        Dialog::ImportInstance => {
            let content = dialog_content(div())
                .flex()
                .flex_col()
                .gap(rem(12.))
                .child(
                    TextField::new("import-path", &app.import_path)
                        .w_full()
                        .label(app.t("backups.import_path"))
                        .placeholder(app.t("backups.import_placeholder"))
                        .error(field_error(app, "import-path").is_some())
                        .when_some(field_error(app, "import-path"), |field, error| {
                            field.supporting_text(error.clone())
                        }),
                )
                .child(muted(app.t("backups.import_hint")));
            let confirm = MaterialButton::new("import-instance-confirm")
                .variant(ButtonVariant::Filled)
                .icon(material_icon("download", 20.0))
                .loading(busy, app.t("common.importing"))
                .disabled(busy)
                .child(app.t("common.import"))
                .on_click(cx.listener(|this, _, window, cx| this.import_instance(window, cx)));
            let mut material = MaterialDialog::new("import-instance-dialog")
                .headline(app.t("dialog.import_instance"))
                .size(size)
                .modal(true)
                .track_body_scroll(&app.dialog_scroll)
                .content(content)
                .action(cancel_button(cx, lang, busy))
                .action(confirm)
                .on_dismiss(dismiss);
            material = finish_dialog(material, window, lang);
            Some(material.into_any_element())
        }
        Dialog::RestoreBackup {
            instance_id,
            backup_id,
            label,
        } => {
            let target_instance = instance_id.clone();
            let target_backup = backup_id.clone();
            let confirm = MaterialButton::new("restore-backup-confirm")
                .variant(ButtonVariant::Filled)
                .icon(material_icon("settings_backup_restore", 20.0))
                .disabled(busy)
                .child(app.t("backups.restore"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.restore_backup(target_instance.clone(), target_backup.clone(), cx)
                }));
            let mut material = MaterialDialog::new("restore-backup-dialog")
                .headline(app.t("dialog.restore_backup"))
                .supporting_text(format!("{}\n{}", label, app.t("dialog.restore_confirm")))
                .size(size)
                .modal(true)
                .action(cancel_button(cx, lang, busy))
                .action(confirm)
                .on_dismiss(dismiss);
            material = finish_dialog(material, window, lang);
            Some(material.into_any_element())
        }
        Dialog::DeleteBackup {
            instance_id,
            backup_id,
            label,
        } => {
            let target_instance = instance_id.clone();
            let target_backup = backup_id.clone();
            let confirm = destructive_button("delete-backup-confirm", app.t("common.delete"))
                .disabled(busy)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.delete_backup(target_instance.clone(), target_backup.clone(), cx)
                }));
            let mut material = MaterialDialog::new("delete-backup-dialog")
                .headline(app.t("dialog.delete_backup"))
                .supporting_text(format!(
                    "{}\n{}",
                    label,
                    app.t("dialog.delete_backup_confirm")
                ))
                .size(size)
                .modal(true)
                .action(cancel_button(cx, lang, busy))
                .action(confirm)
                .on_dismiss(dismiss);
            material = finish_dialog(material, window, lang);
            Some(material.into_any_element())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::dialog_size_for;
    use gpui_kit_material::components::{DialogSize, WindowWidthClass};

    #[test]
    fn the_create_dialog_is_wide_on_desktop() {
        assert!(matches!(
            dialog_size_for(WindowWidthClass::Compact),
            DialogSize::FullScreen
        ));
        assert!(matches!(
            dialog_size_for(WindowWidthClass::Expanded),
            DialogSize::Medium
        ));
    }
}
