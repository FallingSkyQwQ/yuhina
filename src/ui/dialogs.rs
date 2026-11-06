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
    content.px(px(24.)).pb(px(8.))
}

fn inline_error(app: &Yuhina) -> Option<AnyElement> {
    app.dialog_error.as_ref().map(|error| {
        type_role(div(), TypeRole::BodySmall)
            .text_color(color(ColorRole::ERROR))
            .child(error.clone())
            .into_any_element()
    })
}

/// The version chips put curated modded versions first, then the selected
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
        if out.len() >= 12 {
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

/// Renders the currently open dialog as a modal overlay.
pub fn render(
    app: &mut Yuhina,
    _window: &mut Window,
    cx: &mut Context<Yuhina>,
) -> Option<AnyElement> {
    let dialog = app.dialog.clone()?;
    let lang = app.lang;
    let weak = cx.entity().downgrade();
    let dismiss = move |_: &mut Window, cx: &mut App| {
        let _ = weak.update(cx, |this: &mut Yuhina, cx: &mut Context<Yuhina>| {
            this.dialog = None;
            cx.notify();
        });
    };
    let close_listener = cx.listener(|this, _: &ClickEvent, _, cx| {
        this.dialog = None;
        cx.notify();
    });
    let _ = close_listener;
    let cancel_button = |cx: &mut Context<Yuhina>, lang: Language| {
        MaterialButton::new("dialog-cancel")
            .variant(ButtonVariant::Text)
            .child(i18n::tr(lang, "common.cancel"))
            .on_click(cx.listener(|this, _, _, cx| {
                this.dialog = None;
                cx.notify();
            }))
    };

    match dialog {
        Dialog::CreateInstance => {
            let versions = version_options(app);
            let version_chips: Vec<AnyElement> = versions
                .into_iter()
                .map(|version| {
                    let value = version.clone();
                    FilterChip::new(format!("create-version-{version}"), version)
                        .selected(app.create_version == value)
                        .on_toggle(cx.listener(move |this, on: &bool, _, cx| {
                            if *on {
                                this.choose_create_version(value.clone(), cx);
                            }
                        }))
                        .into_any_element()
                })
                .collect();
            let loaders = [
                LoaderType::Vanilla,
                LoaderType::Fabric,
                LoaderType::Quilt,
                LoaderType::Forge,
                LoaderType::Neoforge,
            ];
            let loader_chips: Vec<AnyElement> = loaders
                .into_iter()
                .map(|loader| {
                    let label = app.loader_label(loader);
                    FilterChip::new(format!("create-loader-{label}"), label)
                        .selected(app.create_loader == loader)
                        .on_toggle(cx.listener(move |this, on: &bool, _, cx| {
                            if *on {
                                this.choose_create_loader(loader, cx);
                            }
                        }))
                        .into_any_element()
                })
                .collect();
            let loader_version_section =
                if matches!(app.create_loader, LoaderType::Forge | LoaderType::Neoforge) {
                    let body: AnyElement = if app.loader_versions_loading {
                        type_role(div(), TypeRole::BodySmall)
                            .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                            .child(i18n::tr(lang, "instances.loader_versions_loading"))
                            .into_any_element()
                    } else if let Some(error) = &app.loader_versions_error {
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                type_role(div(), TypeRole::BodySmall)
                                    .text_color(color(ColorRole::ERROR))
                                    .child(error.clone()),
                            )
                            .child(
                                MaterialButton::new("create-loader-retry")
                                    .variant(ButtonVariant::Text)
                                    .child(i18n::tr(lang, "common.retry"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.refresh_loader_versions(cx)
                                    })),
                            )
                            .into_any_element()
                    } else if app.loader_versions.is_empty() {
                        type_role(div(), TypeRole::BodySmall)
                            .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                            .child(i18n::tr(lang, "instances.loader_versions_empty"))
                            .into_any_element()
                    } else {
                        let chips: Vec<AnyElement> = app
                            .loader_versions
                            .iter()
                            .map(|option| {
                                let value = option.version.clone();
                                let label = if option.recommended {
                                    format!(
                                        "{} · {}",
                                        option.version,
                                        i18n::tr(lang, "instances.loader_recommended")
                                    )
                                } else {
                                    option.version.clone()
                                };
                                FilterChip::new(format!("create-loader-version-{value}"), label)
                                    .selected(
                                        app.create_loader_version.as_deref()
                                            == Some(value.as_str()),
                                    )
                                    .on_toggle(cx.listener(move |this, on: &bool, _, cx| {
                                        if *on {
                                            this.create_loader_version = Some(value.clone());
                                            cx.notify();
                                        }
                                    }))
                                    .into_any_element()
                            })
                            .collect();
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(8.))
                            .children(chips)
                            .into_any_element()
                    };
                    Some(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                type_role(div(), TypeRole::BodySmall)
                                    .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                                    .child(i18n::tr(lang, "instances.loader_version_hint")),
                            )
                            .child(body),
                    )
                } else {
                    None
                };
            let memory_chips: Vec<AnyElement> = [2048u32, 4096, 6144, 8192]
                .into_iter()
                .map(|mb| {
                    FilterChip::new(format!("create-memory-{mb}"), format!("{mb} MB"))
                        .selected(app.create_memory == mb)
                        .on_toggle(cx.listener(move |this, on: &bool, _, cx| {
                            if *on {
                                this.create_memory = mb;
                                cx.notify();
                            }
                        }))
                        .into_any_element()
                })
                .collect();

            let content = dialog_content(div())
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(
                    div().w_full().child(
                        TextField::new("create-name", &app.create_name)
                            .w_full()
                            .label(i18n::tr(lang, "instances.name"))
                            .placeholder(i18n::tr(lang, "instances.name_placeholder")),
                    ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(
                            type_role(div(), TypeRole::BodySmall)
                                .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                                .child(i18n::tr(lang, "instances.version_hint")),
                        )
                        .child(if version_chips.is_empty() {
                            type_role(div(), TypeRole::BodySmall)
                                .child(i18n::tr(lang, "common.loading"))
                                .into_any_element()
                        } else {
                            div()
                                .flex()
                                .flex_wrap()
                                .gap(px(8.))
                                .children(version_chips)
                                .into_any_element()
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(
                            type_role(div(), TypeRole::BodySmall)
                                .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                                .child(i18n::tr(lang, "instances.loader_hint")),
                        )
                        .child(div().flex().flex_wrap().gap(px(8.)).children(loader_chips)),
                )
                .children(loader_version_section)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(
                            type_role(div(), TypeRole::BodySmall)
                                .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                                .child(i18n::tr(lang, "instances.memory_hint")),
                        )
                        .child(div().flex().flex_wrap().gap(px(8.)).children(memory_chips)),
                )
                .children(inline_error(app));

            Some(
                MaterialDialog::new("create-instance-dialog")
                    .headline(i18n::tr(lang, "dialog.create_instance"))
                    .size(DialogSize::Small)
                    .max_width(560.0)
                    .dividers(true)
                    .modal(true)
                    .content(content)
                    .action(cancel_button(cx, lang))
                    .action(
                        MaterialButton::new("create-instance-confirm")
                            .variant(ButtonVariant::Filled)
                            .child(i18n::tr(lang, "common.create"))
                            .on_click(cx.listener(|this, _, _, cx| this.create_instance(cx))),
                    )
                    .on_dismiss(dismiss)
                    .into_any_element(),
            )
        }
        Dialog::AddAccount => {
            let tabs = [
                (AccountTab::Offline, i18n::tr(lang, "accounts.offline")),
                (AccountTab::Microsoft, i18n::tr(lang, "accounts.microsoft")),
                (
                    AccountTab::LittleSkin,
                    i18n::tr(lang, "accounts.littleskin"),
                ),
            ];
            let chips: Vec<AnyElement> = tabs
                .into_iter()
                .map(|(tab, label)| {
                    FilterChip::new(format!("account-tab-{label}"), label)
                        .selected(app.account_tab == tab)
                        .on_toggle(cx.listener(move |this, on: &bool, _, cx| {
                            if *on {
                                this.account_tab = tab;
                                cx.notify();
                            }
                        }))
                        .into_any_element()
                })
                .collect();

            let form: AnyElement = match app.account_tab {
                AccountTab::Offline => div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div().w_full().child(
                            TextField::new("offline-name", &app.account_name)
                                .w_full()
                                .label(i18n::tr(lang, "accounts.name"))
                                .placeholder(i18n::tr(lang, "accounts.name_placeholder")),
                        ),
                    )
                    .child(
                        type_role(div(), TypeRole::BodySmall)
                            .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                            .child(i18n::tr(lang, "accounts.offline_hint")),
                    )
                    .into_any_element(),
                AccountTab::Microsoft => div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        type_role(div(), TypeRole::BodySmall)
                            .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                            .child(i18n::tr(lang, "accounts.microsoft_hint")),
                    )
                    .into_any_element(),
                AccountTab::LittleSkin => div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div().w_full().child(
                            TextField::new("authlib-server", &app.authlib_server)
                                .w_full()
                                .label(i18n::tr(lang, "accounts.server_url"))
                                .placeholder(i18n::tr(lang, "accounts.server_url_placeholder")),
                        ),
                    )
                    .child(
                        div().w_full().child(
                            TextField::new("authlib-user", &app.authlib_user)
                                .w_full()
                                .label(i18n::tr(lang, "accounts.email"))
                                .input_type(TextFieldInputType::Email),
                        ),
                    )
                    .child(
                        div().w_full().child(
                            TextField::new("authlib-password", &app.authlib_password)
                                .w_full()
                                .label(i18n::tr(lang, "accounts.password"))
                                .input_type(TextFieldInputType::Password),
                        ),
                    )
                    .into_any_element(),
            };

            let content = dialog_content(div())
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(div().flex().flex_wrap().gap(px(8.)).children(chips))
                .child(form)
                .children(inline_error(app));

            let confirm = match app.account_tab {
                AccountTab::Offline => MaterialButton::new("add-offline-confirm")
                    .variant(ButtonVariant::Filled)
                    .child(i18n::tr(lang, "common.add"))
                    .on_click(
                        cx.listener(|this, _, window, cx| this.add_offline_account(window, cx)),
                    )
                    .into_any_element(),
                AccountTab::Microsoft => MaterialButton::new("add-microsoft-confirm")
                    .variant(ButtonVariant::Filled)
                    .child(i18n::tr(lang, "accounts.login"))
                    .on_click(cx.listener(|this, _, _, cx| this.start_microsoft_login(cx)))
                    .into_any_element(),
                AccountTab::LittleSkin => MaterialButton::new("add-authlib-confirm")
                    .variant(ButtonVariant::Filled)
                    .child(i18n::tr(lang, "accounts.login"))
                    .on_click(cx.listener(|this, _, _, cx| this.add_authlib_account(cx)))
                    .into_any_element(),
            };

            Some(
                MaterialDialog::new("add-account-dialog")
                    .headline(i18n::tr(lang, "dialog.add_account"))
                    .size(DialogSize::Small)
                    .max_width(520.0)
                    .dividers(true)
                    .modal(true)
                    .content(content)
                    .action(cancel_button(cx, lang))
                    .action(confirm)
                    .on_dismiss(dismiss)
                    .into_any_element(),
            )
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
                .gap(px(16.))
                .child(
                    type_role(div(), TypeRole::BodyMedium)
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .text_center()
                        .child(i18n::tr(lang, "accounts.microsoft_waiting")),
                )
                .child(
                    div()
                        .w_full()
                        .rounded(px(20.))
                        .bg(color(ColorRole::SURFACE_CONTAINER_HIGHEST))
                        .p(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            type_scale(div(), TypeRole::HeadlineLarge.emphasized()).child(
                                if user_code.is_empty() {
                                    i18n::tr(lang, "common.loading").to_string()
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
                        .gap(px(8.))
                        .child(
                            MaterialButton::new("microsoft-copy-code")
                                .variant(ButtonVariant::Text)
                                .icon(material_icon("content_copy", 18.0))
                                .child(i18n::tr(lang, "common.copy"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(code.clone()));
                                    this.show_toast(this.t("common.copied"), false, cx);
                                })),
                        )
                        .child(
                            MaterialButton::new("microsoft-open-browser")
                                .variant(ButtonVariant::Tonal)
                                .icon(material_icon("open_in_new", 18.0))
                                .child(i18n::tr(lang, "accounts.microsoft"))
                                .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))),
                        )
                        .into_any_element()
                }))
                .child(if waiting {
                    type_role(div(), TypeRole::BodySmall)
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .child(i18n::tr(lang, "common.loading"))
                        .into_any_element()
                } else {
                    div().into_any_element()
                });

            Some(
                MaterialDialog::new("microsoft-login-dialog")
                    .headline(i18n::tr(lang, "accounts.microsoft"))
                    .size(DialogSize::Small)
                    .max_width(480.0)
                    .modal(true)
                    .content(content)
                    .action(cancel_button(cx, lang))
                    .on_dismiss(dismiss)
                    .into_any_element(),
            )
        }
        Dialog::AddServer => {
            let content = dialog_content(div())
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(
                    div().w_full().child(
                        TextField::new("server-name", &app.server_name)
                            .w_full()
                            .label(i18n::tr(lang, "servers.name")),
                    ),
                )
                .child(
                    div().w_full().child(
                        TextField::new("server-address", &app.server_address)
                            .w_full()
                            .label(i18n::tr(lang, "servers.address"))
                            .placeholder(i18n::tr(lang, "servers.address_placeholder")),
                    ),
                )
                .children(inline_error(app));
            Some(
                MaterialDialog::new("add-server-dialog")
                    .headline(i18n::tr(lang, "dialog.add_server"))
                    .size(DialogSize::Small)
                    .max_width(480.0)
                    .modal(true)
                    .content(content)
                    .action(cancel_button(cx, lang))
                    .action(
                        MaterialButton::new("add-server-confirm")
                            .variant(ButtonVariant::Filled)
                            .child(i18n::tr(lang, "common.add"))
                            .on_click(cx.listener(|this, _, _, cx| this.add_server(cx))),
                    )
                    .on_dismiss(dismiss)
                    .into_any_element(),
            )
        }
        Dialog::ImportModpack => {
            let content = dialog_content(div())
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(
                    type_role(div(), TypeRole::BodySmall)
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .child(i18n::tr(lang, "library.import_hint")),
                )
                .child(
                    div().w_full().child(
                        TextField::new("modpack-path", &app.modpack_path)
                            .w_full()
                            .label(i18n::tr(lang, "library.import_path")),
                    ),
                );
            Some(
                MaterialDialog::new("import-modpack-dialog")
                    .headline(i18n::tr(lang, "library.import_title"))
                    .size(DialogSize::Small)
                    .max_width(560.0)
                    .modal(true)
                    .content(content)
                    .action(cancel_button(cx, lang))
                    .action(
                        MaterialButton::new("import-modpack-confirm")
                            .variant(ButtonVariant::Filled)
                            .icon(material_icon("upload_file", 18.0))
                            .child(i18n::tr(lang, "library.import_confirm"))
                            .on_click(cx.listener(|this, _, _, cx| this.import_modpack(cx))),
                    )
                    .on_dismiss(dismiss)
                    .into_any_element(),
            )
        }
        Dialog::DeleteInstance(id) => {
            let target = id.clone();
            Some(
                MaterialDialog::new("delete-instance-dialog")
                    .headline(i18n::tr(lang, "dialog.delete_instance"))
                    .supporting_text(i18n::tr(lang, "dialog.delete_confirm"))
                    .size(DialogSize::Small)
                    .modal(true)
                    .action(cancel_button(cx, lang))
                    .action(
                        destructive_button(
                            "delete-instance-confirm",
                            i18n::tr(lang, "common.delete"),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.delete_instance(target.clone(), cx)
                        })),
                    )
                    .on_dismiss(dismiss)
                    .into_any_element(),
            )
        }
        Dialog::ConfirmRemoveContent {
            instance_id,
            project_type,
            filename,
        } => Some(
            MaterialDialog::new("remove-content-dialog")
                .headline(i18n::tr(lang, "instances.remove_mod"))
                .supporting_text(filename.clone())
                .size(DialogSize::Small)
                .modal(true)
                .action(cancel_button(cx, lang))
                .action(
                    destructive_button(
                        "remove-content-confirm",
                        i18n::tr(lang, "instances.remove_mod"),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.remove_content(
                            instance_id.clone(),
                            project_type.clone(),
                            filename.clone(),
                            cx,
                        )
                    })),
                )
                .on_dismiss(dismiss)
                .into_any_element(),
        ),
        Dialog::ImportInstance => {
            let content = dialog_content(div())
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    div().w_full().child(
                        TextField::new("import-path", &app.import_path)
                            .w_full()
                            .label(i18n::tr(lang, "backups.import_path"))
                            .placeholder(i18n::tr(lang, "backups.import_placeholder")),
                    ),
                )
                .child(
                    type_role(div(), TypeRole::BodySmall)
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .child(i18n::tr(lang, "backups.import_hint")),
                );
            Some(
                MaterialDialog::new("import-instance-dialog")
                    .headline(i18n::tr(lang, "dialog.import_instance"))
                    .size(DialogSize::Small)
                    .max_width(520.0)
                    .modal(true)
                    .content(content)
                    .action(cancel_button(cx, lang))
                    .action(
                        MaterialButton::new("import-instance-confirm")
                            .variant(ButtonVariant::Filled)
                            .icon(material_icon("download", 18.0))
                            .child(i18n::tr(lang, "common.import"))
                            .on_click(
                                cx.listener(|this, _, window, cx| this.import_instance(window, cx)),
                            ),
                    )
                    .on_dismiss(dismiss)
                    .into_any_element(),
            )
        }
        Dialog::RestoreBackup {
            instance_id,
            backup_id,
            label,
        } => {
            let target_instance = instance_id.clone();
            let target_backup = backup_id.clone();
            Some(
                MaterialDialog::new("restore-backup-dialog")
                    .headline(i18n::tr(lang, "dialog.restore_backup"))
                    .supporting_text(format!(
                        "{}\n{}",
                        label,
                        i18n::tr(lang, "dialog.restore_confirm")
                    ))
                    .size(DialogSize::Small)
                    .max_width(480.0)
                    .modal(true)
                    .action(cancel_button(cx, lang))
                    .action(
                        MaterialButton::new("restore-backup-confirm")
                            .variant(ButtonVariant::Filled)
                            .icon(material_icon("settings_backup_restore", 18.0))
                            .child(i18n::tr(lang, "backups.restore"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.restore_backup(
                                    target_instance.clone(),
                                    target_backup.clone(),
                                    cx,
                                )
                            })),
                    )
                    .on_dismiss(dismiss)
                    .into_any_element(),
            )
        }
        Dialog::DeleteBackup {
            instance_id,
            backup_id,
            label,
        } => {
            let target_instance = instance_id.clone();
            let target_backup = backup_id.clone();
            Some(
                MaterialDialog::new("delete-backup-dialog")
                    .headline(i18n::tr(lang, "dialog.delete_backup"))
                    .supporting_text(format!(
                        "{}\n{}",
                        label,
                        i18n::tr(lang, "dialog.delete_backup_confirm")
                    ))
                    .size(DialogSize::Small)
                    .max_width(480.0)
                    .modal(true)
                    .action(cancel_button(cx, lang))
                    .action(
                        destructive_button(
                            "delete-backup-confirm",
                            i18n::tr(lang, "common.delete"),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.delete_backup(target_instance.clone(), target_backup.clone(), cx)
                        })),
                    )
                    .on_dismiss(dismiss)
                    .into_any_element(),
            )
        }
    }
}
