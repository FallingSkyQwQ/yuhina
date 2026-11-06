use super::*;
use crate::core::instances::LoaderType;

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
            let versions: Vec<String> = app
                .versions
                .as_ref()
                .map(|manifest| {
                    manifest
                        .versions
                        .iter()
                        .filter(|version| version.kind == "release")
                        .take(8)
                        .map(|version| version.id.clone())
                        .collect()
                })
                .unwrap_or_default();
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
                (LoaderType::Vanilla, "Vanilla"),
                (LoaderType::Fabric, "Fabric"),
                (LoaderType::Quilt, "Quilt"),
                (LoaderType::Forge, "Forge"),
                (LoaderType::Neoforge, "NeoForge"),
            ];
            let loader_chips: Vec<AnyElement> = loaders
                .into_iter()
                .map(|(loader, label)| {
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
                        div()
                            .text_size(px(13.))
                            .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                            .child(i18n::tr(lang, "instances.loader_versions_loading"))
                            .into_any_element()
                    } else if let Some(error) = &app.loader_versions_error {
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(12.))
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
                        div()
                            .text_size(px(13.))
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
                                div()
                                    .text_size(px(12.))
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

            let content = div()
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(
                    div().w_full().child(
                        TextField::new("create-name", &app.create_name)
                            .w_full()
                            .label(i18n::tr(lang, "instances.name")),
                    ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                                .child(i18n::tr(lang, "instances.version_hint")),
                        )
                        .child(if version_chips.is_empty() {
                            div()
                                .text_size(px(13.))
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
                            div()
                                .text_size(px(12.))
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
                            div()
                                .text_size(px(12.))
                                .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                                .child(i18n::tr(lang, "instances.memory_hint")),
                        )
                        .child(div().flex().flex_wrap().gap(px(8.)).children(memory_chips)),
                );

            Some(
                MaterialDialog::new("create-instance-dialog")
                    .headline(i18n::tr(lang, "dialog.create_instance"))
                    .size(DialogSize::Small)
                    .max_width(560.0)
                    .modal(true)
                    .content(content)
                    .action(cancel_button(cx, lang))
                    .action(
                        MaterialButton::new("create-instance-confirm")
                            .variant(ButtonVariant::Filled)
                            .child(i18n::tr(lang, "common.confirm"))
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
                                .label(i18n::tr(lang, "accounts.name")),
                        ),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                            .child(i18n::tr(lang, "accounts.offline_hint")),
                    )
                    .child(
                        MaterialButton::new("add-offline")
                            .variant(ButtonVariant::Tonal)
                            .icon(material_icon("person_add", 18.0))
                            .child(i18n::tr(lang, "common.add"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.add_offline_account(window, cx)
                            })),
                    )
                    .into_any_element(),
                AccountTab::Microsoft => div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                            .child(i18n::tr(lang, "accounts.microsoft_hint")),
                    )
                    .child(
                        MaterialButton::new("add-microsoft")
                            .variant(ButtonVariant::Filled)
                            .icon(material_icon("login", 18.0))
                            .child(i18n::tr(lang, "accounts.microsoft"))
                            .on_click(cx.listener(|this, _, _, cx| this.start_microsoft_login(cx))),
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
                                .label(i18n::tr(lang, "accounts.server_url")),
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
                    .child(
                        MaterialButton::new("add-authlib")
                            .variant(ButtonVariant::Filled)
                            .icon(material_icon("login", 18.0))
                            .child(i18n::tr(lang, "accounts.login"))
                            .on_click(cx.listener(|this, _, _, cx| this.add_authlib_account(cx))),
                    )
                    .into_any_element(),
            };

            let content = div()
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(div().flex().flex_wrap().gap(px(8.)).children(chips))
                .child(form);

            Some(
                MaterialDialog::new("add-account-dialog")
                    .headline(i18n::tr(lang, "dialog.add_account"))
                    .size(DialogSize::Small)
                    .max_width(520.0)
                    .modal(true)
                    .content(content)
                    .action(cancel_button(cx, lang))
                    .on_dismiss(dismiss)
                    .into_any_element(),
            )
        }
        Dialog::MicrosoftLogin {
            user_code,
            verification_uri,
            waiting,
        } => {
            let content = div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(16.))
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
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
                        .text_size(px(34.))
                        .font_weight(FontWeight::BOLD)
                        .child(if user_code.is_empty() {
                            i18n::tr(lang, "common.loading").to_string()
                        } else {
                            user_code.clone()
                        }),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(color(ColorRole::ON_SURFACE_VARIANT))
                        .child(verification_uri.clone()),
                )
                .children((!user_code.is_empty()).then(|| {
                    let url = verification_uri.clone();
                    MaterialButton::new("microsoft-open-browser")
                        .variant(ButtonVariant::Tonal)
                        .icon(material_icon("open_in_new", 18.0))
                        .child(i18n::tr(lang, "accounts.microsoft"))
                        .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url)))
                        .into_any_element()
                }))
                .child(if waiting {
                    div()
                        .text_size(px(12.))
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
            let content = div()
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
                            .label(i18n::tr(lang, "servers.address")),
                    ),
                );
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
                        MaterialButton::new("delete-instance-confirm")
                            .variant(ButtonVariant::Filled)
                            .child(i18n::tr(lang, "common.delete"))
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
                    MaterialButton::new("remove-content-confirm")
                        .variant(ButtonVariant::Filled)
                        .child(i18n::tr(lang, "common.delete"))
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
            let content = div()
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
                    div()
                        .text_size(px(12.))
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
                        MaterialButton::new("delete-backup-confirm")
                            .variant(ButtonVariant::Filled)
                            .icon(material_icon("delete", 18.0))
                            .child(i18n::tr(lang, "common.delete"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.delete_backup(
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
    }
}
