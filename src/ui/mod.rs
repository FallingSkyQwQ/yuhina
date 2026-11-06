pub mod dialogs;
pub mod i18n;
pub mod pages;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui_kit::base::input::InputState;
use gpui_kit::{prelude::*, *};
use gpui_kit_material::color::themes::{Contrast, ThemeId, ThemeMode};
use gpui_kit_material::icons::{Icon, Symbol};
use gpui_kit_material::prelude::*;
use gpui_kit_material::theme;

use crate::core::accounts::{self, AccountSummary};
use crate::core::backup::{self, BackupManifest};
use crate::core::export;
use crate::core::files::{self, FileEntry, TextPreview};
use crate::core::install::{self, InstallPhase, InstallProgress};
use crate::core::instances::{self, InstanceRecord, InstanceSummary, LoaderType, MemoryMode};
use crate::core::java::{self, JavaRuntime};
use crate::core::launch::{
    self, GameLine, GameProcess, LaunchCredentials, LaunchRequest, ServerTarget,
};
use crate::core::loader::{self, LoaderVersion};
use crate::core::modpack;
use crate::core::modrinth::{self, SearchResult};
use crate::core::news::{self, NewsState};
use crate::core::servers::{self, ServerRecord, ServerStatus};
use crate::core::settings::{
    self as core_settings, DownloadSource, JavaSettings, Language, Settings, ThemePreference,
};
use crate::core::skins;
use crate::core::updates::{self, UpdateInfo};
use crate::core::version::{RuleContext, VersionManifest};
use crate::core::{ErrorCode, Paths, YuhinaError};

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The one colour scheme the application uses. Both startup and the animated
/// theme transition read this constant, so the two cannot disagree.
pub const APP_THEME: ThemeId = ThemeId::Expressive;
/// Standard contrast for now; a persisted low-vision setting can replace it.
pub const APP_CONTRAST: Contrast = Contrast::Standard;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Home,
    Instances,
    Library,
    Accounts,
    Servers,
    Activity,
    Settings,
}

impl Page {
    pub fn id(self) -> &'static str {
        match self {
            Page::Home => "home",
            Page::Instances => "instances",
            Page::Library => "library",
            Page::Accounts => "accounts",
            Page::Servers => "servers",
            Page::Activity => "activity",
            Page::Settings => "settings",
        }
    }

    pub fn title_key(self) -> &'static str {
        match self {
            Page::Home => "nav.home",
            Page::Instances => "nav.instances",
            Page::Library => "nav.library",
            Page::Accounts => "nav.accounts",
            Page::Servers => "nav.servers",
            Page::Activity => "nav.activity",
            Page::Settings => "nav.settings",
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "instances" => Page::Instances,
            "library" => Page::Library,
            "accounts" => Page::Accounts,
            "servers" => Page::Servers,
            "activity" => Page::Activity,
            "settings" => Page::Settings,
            _ => Page::Home,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstanceTab {
    Overview,
    Logs,
    Content,
    Backups,
    Files,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameStatus {
    Starting,
    Running,
    Exited,
    Crashed,
}

#[derive(Default, Debug)]
pub struct Loading {
    pub instances: bool,
    pub accounts: bool,
    pub servers: bool,
    pub news: bool,
    pub versions: bool,
    pub java: bool,
    pub update: bool,
    pub library: bool,
    pub backups: bool,
    pub files: bool,
    pub modpack: bool,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub text: String,
    pub error: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountTab {
    Offline,
    Microsoft,
    LittleSkin,
}

#[derive(Clone, Debug)]
pub enum Dialog {
    CreateInstance,
    AddAccount,
    MicrosoftLogin {
        user_code: String,
        verification_uri: String,
        waiting: bool,
    },
    AddServer,
    ImportModpack,
    DeleteInstance(String),
    ConfirmRemoveContent {
        instance_id: String,
        project_type: String,
        filename: String,
    },
    ImportInstance,
    RestoreBackup {
        instance_id: String,
        backup_id: String,
        label: String,
    },
    DeleteBackup {
        instance_id: String,
        backup_id: String,
        label: String,
    },
}

pub struct RunningGame {
    pub process: GameProcess,
    pub logs: Vec<GameLine>,
    pub status: GameStatus,
    pub exit_code: Option<i32>,
    pub scroll: ScrollHandle,
}

impl RunningGame {
    fn new(process: GameProcess) -> Self {
        Self {
            process,
            logs: Vec::new(),
            status: GameStatus::Starting,
            exit_code: None,
            scroll: ScrollHandle::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct InstallTask {
    pub instance_id: String,
    pub percent: f32,
    pub phase: InstallPhase,
    pub message: Option<String>,
    pub finished: bool,
    pub error: Option<String>,
}

pub struct Yuhina {
    pub paths: Paths,
    pub settings: Settings,
    pub lang: Language,
    pub dark: bool,
    pub page: Page,
    pub instances: Vec<InstanceSummary>,
    pub accounts: Vec<AccountSummary>,
    pub skins: HashMap<String, Option<PathBuf>>,
    pub servers: Vec<ServerRecord>,
    pub statuses: HashMap<String, ServerStatus>,
    pub news: NewsState,
    pub versions: Option<VersionManifest>,
    pub running: HashMap<String, RunningGame>,
    pub installs: HashMap<String, InstallTask>,
    pub selected_instance: Option<String>,
    pub instance_tab: InstanceTab,
    pub search_query: String,
    pub search_results: Option<SearchResult>,
    pub library_type: String,
    pub library_target: Option<String>,
    pub java_runtimes: Vec<JavaRuntime>,
    pub update: Option<UpdateInfo>,
    pub update_checked: bool,
    pub toast: Option<Toast>,
    pub dialog: Option<Dialog>,
    pub dialog_error: Option<String>,
    pub overflow_open: bool,
    pub loading: Loading,
    pub instance_error: Option<String>,
    pub account_error: Option<String>,
    pub server_error: Option<String>,
    pub library_error: Option<String>,
    pub news_error: Option<String>,
    pub create_name: Entity<InputState>,
    pub create_version: String,
    pub create_loader: LoaderType,
    pub create_loader_version: Option<String>,
    pub loader_versions: Vec<LoaderVersion>,
    pub loader_versions_loading: bool,
    pub loader_versions_error: Option<String>,
    loader_versions_key: Option<(LoaderType, String)>,
    pub create_memory: u32,
    pub account_name: Entity<InputState>,
    pub account_tab: AccountTab,
    pub server_name: Entity<InputState>,
    pub server_address: Entity<InputState>,
    pub modpack_path: Entity<InputState>,
    pub authlib_server: Entity<InputState>,
    pub authlib_user: Entity<InputState>,
    pub authlib_password: Entity<InputState>,
    pub search_input: Entity<InputState>,
    pub import_path: Entity<InputState>,
    pub backup_label: Entity<InputState>,
    pub backups: Vec<BackupManifest>,
    pub files_entries: Vec<FileEntry>,
    pub files_relative: String,
    pub file_preview: Option<TextPreview>,
    theme_pending: bool,
}

impl Yuhina {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let paths = Paths::discover().unwrap_or_else(|_| Paths::at("."));
        let settings = core_settings::load(&paths).unwrap_or_default();
        let lang = settings.effective_language();
        let dark_override = std::env::var("YUHINA_DARK").ok().map(|value| value != "0");
        let dark = dark_override.unwrap_or_else(|| settings.effective_theme());
        let mut settings = settings;
        if let Some(dark) = dark_override {
            settings.theme = if dark {
                ThemePreference::Dark
            } else {
                ThemePreference::Light
            };
        }
        let page = std::env::var("YUHINA_PAGE")
            .map(|value| Page::from_id(&value))
            .unwrap_or(Page::Home);
        let dialog = match std::env::var("YUHINA_DIALOG").as_deref() {
            Ok("create-instance") => Some(Dialog::CreateInstance),
            Ok("add-account") => Some(Dialog::AddAccount),
            Ok("add-server") => Some(Dialog::AddServer),
            Ok("import-modpack") => Some(Dialog::ImportModpack),
            _ => None,
        };
        let selected_instance = std::env::var("YUHINA_INSTANCE").ok();
        let instance_tab = match std::env::var("YUHINA_TAB").as_deref() {
            Ok("logs") => InstanceTab::Logs,
            Ok("content") => InstanceTab::Content,
            Ok("settings") => InstanceTab::Settings,
            Ok("backups") => InstanceTab::Backups,
            Ok("files") => InstanceTab::Files,
            _ => InstanceTab::Overview,
        };
        let create_loader = match std::env::var("YUHINA_LOADER").as_deref() {
            Ok("fabric") => LoaderType::Fabric,
            Ok("quilt") => LoaderType::Quilt,
            Ok("forge") => LoaderType::Forge,
            Ok("neoforge") => LoaderType::Neoforge,
            _ => LoaderType::Vanilla,
        };
        let create_version = std::env::var("YUHINA_MC").unwrap_or_default();
        let create_name = cx.new(|cx| InputState::new(window, cx));
        let account_name = cx.new(|cx| InputState::new(window, cx));
        let server_name = cx.new(|cx| InputState::new(window, cx));
        let server_address = cx.new(|cx| InputState::new(window, cx));
        let modpack_path = cx.new(|cx| InputState::new(window, cx));
        let authlib_server = cx.new(|cx| InputState::new(window, cx));
        let authlib_user = cx.new(|cx| InputState::new(window, cx));
        let authlib_password = cx.new(|cx| InputState::new(window, cx));
        let search_input = cx.new(|cx| InputState::new(window, cx));
        let import_path = cx.new(|cx| InputState::new(window, cx));
        let backup_label = cx.new(|cx| InputState::new(window, cx));

        let mut this = Self {
            paths,
            settings,
            lang,
            dark,
            page,
            instances: Vec::new(),
            accounts: Vec::new(),
            skins: HashMap::new(),
            servers: Vec::new(),
            statuses: HashMap::new(),
            news: NewsState {
                items: Vec::new(),
                fetched_at: String::new(),
                source: "fallback".to_string(),
            },
            versions: None,
            running: HashMap::new(),
            installs: HashMap::new(),
            selected_instance,
            instance_tab,
            search_query: String::new(),
            search_results: None,
            library_type: "mod".to_string(),
            library_target: None,
            java_runtimes: Vec::new(),
            update: None,
            update_checked: false,
            toast: None,
            dialog,
            dialog_error: None,
            overflow_open: false,
            loading: Loading::default(),
            instance_error: None,
            account_error: None,
            server_error: None,
            library_error: None,
            news_error: None,
            create_name,
            create_version,
            create_loader,
            create_loader_version: None,
            loader_versions: Vec::new(),
            loader_versions_loading: false,
            loader_versions_error: None,
            loader_versions_key: None,
            create_memory: 4096,
            account_name,
            account_tab: AccountTab::Offline,
            server_name,
            server_address,
            modpack_path,
            authlib_server,
            authlib_user,
            authlib_password,
            search_input,
            import_path,
            backup_label,
            backups: Vec::new(),
            files_entries: Vec::new(),
            files_relative: String::new(),
            file_preview: None,
            theme_pending: false,
        };
        this.refresh_all(cx);
        // A Forge/NeoForge instance preselected at launch (or restored in the
        // create dialog) needs its installer builds without waiting for the
        // version manifest round-trip.
        if matches!(this.create_loader, LoaderType::Forge | LoaderType::Neoforge)
            && !this.create_version.is_empty()
        {
            this.refresh_loader_versions(cx);
        }
        if page == Page::Library {
            this.search_library(cx);
        }
        this
    }

    pub fn t(&self, key: &str) -> &'static str {
        i18n::tr(self.lang, key)
    }

    /// A localized user message for a domain error. Core keeps its stable
    /// English message; the UI maps the machine-readable code.
    pub fn error_text(&self, error: &YuhinaError) -> String {
        let key = match error.code {
            ErrorCode::NotFound => "error.not_found",
            ErrorCode::InvalidInput => "error.invalid_input",
            ErrorCode::Io => "error.io",
            ErrorCode::Network => "error.network",
            ErrorCode::Http => "error.http",
            ErrorCode::Checksum => "error.checksum",
            ErrorCode::Unsupported => "error.unsupported",
            ErrorCode::Auth => "error.auth",
            ErrorCode::AuthExpired => "error.auth_expired",
            ErrorCode::AuthMissingProfile => "error.auth_missing_profile",
            ErrorCode::AccountRequired => "error.no_account",
            ErrorCode::GameAlreadyRunning => "error.game_already_running",
            ErrorCode::GameNotRunning => "error.game_not_running",
            ErrorCode::LaunchFailed => "error.launch",
            ErrorCode::InstallFailed => "error.install",
            ErrorCode::VersionNotFound => "error.version_not_found",
            ErrorCode::JavaRequired => "error.no_java",
            ErrorCode::JavaInvalid => "error.java_invalid",
            ErrorCode::SecretStorage => "error.secret_storage",
            ErrorCode::Parse => "error.parse",
            ErrorCode::External => "error.external",
        };
        let message = self.t(key);
        if message == "…" {
            error.message.clone()
        } else {
            message.to_string()
        }
    }

    pub fn loader_label(&self, loader: LoaderType) -> &'static str {
        match loader {
            LoaderType::Vanilla => self.t("loader.vanilla"),
            LoaderType::Fabric => self.t("loader.fabric"),
            LoaderType::Quilt => self.t("loader.quilt"),
            LoaderType::Forge => self.t("loader.forge"),
            LoaderType::Neoforge => self.t("loader.neoforge"),
        }
    }

    pub fn loader_text(&self, loader: &instances::Loader) -> String {
        let name = self.loader_label(loader.kind);
        match (&loader.kind, &loader.version) {
            (LoaderType::Vanilla, _) | (_, None) => name.to_string(),
            (_, Some(version)) => format!("{name} {version}"),
        }
    }

    pub fn phase_label(&self, phase: InstallPhase) -> &'static str {
        match phase {
            InstallPhase::Metadata => self.t("instances.phase.metadata"),
            InstallPhase::Client => self.t("instances.phase.client"),
            InstallPhase::Libraries => self.t("instances.phase.libraries"),
            InstallPhase::Natives => self.t("instances.phase.natives"),
            InstallPhase::Assets => self.t("instances.phase.assets"),
            InstallPhase::Loader => self.t("instances.phase.loader"),
            InstallPhase::Backup => self.t("instances.phase.backup"),
            InstallPhase::Restore => self.t("instances.phase.restore"),
            InstallPhase::Export => self.t("instances.phase.export"),
            InstallPhase::Import => self.t("instances.phase.import"),
            InstallPhase::Modpack => self.t("instances.phase.modpack"),
            InstallPhase::Overrides => self.t("instances.phase.overrides"),
            InstallPhase::Done => self.t("instances.phase.done"),
            InstallPhase::Error => self.t("instances.phase.error"),
        }
    }

    pub fn refresh_all(&mut self, cx: &mut Context<Self>) {
        self.refresh_instances(cx);
        self.refresh_accounts(cx);
        self.refresh_servers(cx);
        self.refresh_news(cx, false);
        self.refresh_versions(cx, false);
        self.refresh_java(cx);
        self.check_updates(cx);
    }

    pub fn refresh_instances(&mut self, cx: &mut Context<Self>) {
        if self.loading.instances {
            return;
        }
        self.loading.instances = true;
        cx.notify();
        let paths = self.paths.clone();
        spawn_task(
            cx,
            move || -> crate::core::Result<Vec<InstanceSummary>> {
                let records = instances::list(&paths)?;
                let mut summaries = Vec::with_capacity(records.len());
                for record in records {
                    let launch_id = record.launch_version().to_string();
                    let installed = install::is_installed(&paths, &launch_id);
                    let required = install::read_installed(&paths, &launch_id)
                        .ok()
                        .flatten()
                        .and_then(|version| {
                            version.java_version.and_then(|java| java.major_version)
                        })
                        .or_else(|| Some(instances::required_java_major(&record.version_id)));
                    summaries.push(InstanceSummary {
                        record,
                        installed,
                        required_java_major: required,
                    });
                }
                Ok(summaries)
            },
            |this, result, cx| {
                this.loading.instances = false;
                match result {
                    Ok(items) => {
                        this.instances = items;
                        this.instance_error = None;
                    }
                    Err(error) => {
                        this.instance_error = Some(this.error_text(&error));
                        this.show_error(error, cx);
                    }
                }
            },
        );
    }

    pub fn refresh_accounts(&mut self, cx: &mut Context<Self>) {
        if self.loading.accounts {
            return;
        }
        self.loading.accounts = true;
        cx.notify();
        let paths = self.paths.clone();
        spawn_task(
            cx,
            move || accounts::list(&paths),
            |this, result, cx| {
                this.loading.accounts = false;
                match result {
                    Ok(items) => {
                        this.accounts = items;
                        this.account_error = None;
                        this.refresh_skins(cx);
                    }
                    Err(error) => {
                        this.account_error = Some(this.error_text(&error));
                        this.show_error(error, cx);
                    }
                }
            },
        );
    }

    /// Downloads missing skin heads in the background. `None` marks a lookup
    /// in flight so a refresh does not spawn it twice.
    pub fn refresh_skins(&mut self, cx: &mut Context<Self>) {
        let pending: Vec<(String, String)> = self
            .accounts
            .iter()
            .filter(|account| !self.skins.contains_key(&account.id))
            .filter_map(|account| {
                account
                    .skin_url
                    .clone()
                    .map(|url| (account.id.clone(), url))
            })
            .collect();
        for (id, url) in pending {
            self.skins.insert(id.clone(), None);
            let paths = self.paths.clone();
            let task_id = id.clone();
            cx.spawn(async move |this, cx| {
                let result = cx
                    .background_executor()
                    .spawn(async move { skins::ensure_head(&paths, &id, &url) })
                    .await;
                let _ = this.update(cx, |this, cx| {
                    match result {
                        Ok(path) => {
                            this.skins.insert(task_id, Some(path));
                        }
                        Err(_) => {
                            this.skins.remove(&task_id);
                        }
                    }
                    cx.notify();
                });
            })
            .detach();
        }
    }

    pub fn refresh_servers(&mut self, cx: &mut Context<Self>) {
        if self.loading.servers {
            return;
        }
        self.loading.servers = true;
        cx.notify();
        let paths = self.paths.clone();
        spawn_task(
            cx,
            move || servers::list(&paths),
            |this, result, cx| {
                this.loading.servers = false;
                match result {
                    Ok(items) => {
                        this.servers = items;
                        this.server_error = None;
                    }
                    Err(error) => {
                        this.server_error = Some(this.error_text(&error));
                        this.show_error(error, cx);
                    }
                }
            },
        );
    }

    pub fn refresh_news(&mut self, cx: &mut Context<Self>, force: bool) {
        if self.loading.news {
            return;
        }
        self.loading.news = true;
        cx.notify();
        let paths = self.paths.clone();
        spawn_task(
            cx,
            move || news::load(&paths, force),
            |this, result, cx| {
                this.loading.news = false;
                match result {
                    Ok(state) => {
                        this.news = state;
                        this.news_error = None;
                    }
                    Err(error) => {
                        this.news_error = Some(this.error_text(&error));
                        this.show_error(error, cx);
                    }
                }
            },
        );
    }

    pub fn refresh_versions(&mut self, cx: &mut Context<Self>, force: bool) {
        if self.loading.versions || (self.versions.is_some() && !force) {
            return;
        }
        self.loading.versions = true;
        cx.notify();
        let paths = self.paths.clone();
        let source = self.settings.download_source;
        spawn_task(
            cx,
            move || install::list_versions(&paths, source, force),
            |this, result, cx| {
                this.loading.versions = false;
                match result {
                    Ok(manifest) => {
                        if this.create_version.is_empty() {
                            this.create_version = manifest.latest.release.clone();
                            if matches!(
                                this.create_loader,
                                LoaderType::Forge | LoaderType::Neoforge
                            ) {
                                this.refresh_loader_versions(cx);
                            }
                        }
                        this.versions = Some(manifest);
                    }
                    Err(error) => this.show_error(error, cx),
                }
            },
        );
    }

    pub fn refresh_java(&mut self, cx: &mut Context<Self>) {
        if self.loading.java {
            return;
        }
        self.loading.java = true;
        cx.notify();
        let paths = self.paths.clone();
        let java_settings = self.settings.java.clone();
        spawn_task(
            cx,
            move || java::detect(&paths, &java_settings),
            |this, scan, _cx| {
                this.loading.java = false;
                this.java_runtimes = scan.runtimes;
            },
        );
    }

    pub fn check_updates(&mut self, cx: &mut Context<Self>) {
        if self.loading.update {
            return;
        }
        self.loading.update = true;
        cx.notify();
        spawn_task(
            cx,
            move || updates::check(APP_VERSION),
            |this, result, _cx| {
                this.loading.update = false;
                this.update_checked = true;
                match result {
                    Ok(update) => this.update = update,
                    Err(_) => this.update = None,
                }
            },
        );
    }

    pub fn show_toast(&mut self, text: impl Into<String>, error: bool, cx: &mut Context<Self>) {
        self.toast = Some(Toast {
            text: text.into(),
            error,
        });
        let weak = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            cx.background_executor().timer(Duration::from_secs(4)).await;
            let _ = weak.update(cx, |this, cx| {
                this.toast = None;
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub fn show_error(&mut self, error: YuhinaError, cx: &mut Context<Self>) {
        let message = self.error_text(&error);
        self.show_toast(message, true, cx);
    }

    pub fn set_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        self.selected_instance = None;
        match page {
            Page::Home => {
                if self.instances.is_empty() {
                    self.refresh_instances(cx);
                }
                if self.news.items.is_empty() {
                    self.refresh_news(cx, false);
                }
            }
            Page::Instances => {
                if self.instances.is_empty() {
                    self.refresh_instances(cx);
                }
            }
            Page::Library => {
                if self.search_results.is_none() {
                    self.search_library(cx);
                }
            }
            Page::Accounts => {
                if self.accounts.is_empty() {
                    self.refresh_accounts(cx);
                }
            }
            Page::Servers => {
                if self.servers.is_empty() {
                    self.refresh_servers(cx);
                }
            }
            Page::Activity => {}
            Page::Settings => {
                if self.java_runtimes.is_empty() {
                    self.refresh_java(cx);
                }
            }
        }
        cx.notify();
    }

    pub fn is_game_running(&self, id: &str) -> bool {
        self.running
            .get(id)
            .map(|game| matches!(game.status, GameStatus::Starting | GameStatus::Running))
            .unwrap_or(false)
    }

    pub fn open_instance(&mut self, id: String, cx: &mut Context<Self>) {
        self.selected_instance = Some(id);
        self.instance_tab = InstanceTab::Overview;
        self.backups.clear();
        self.files_entries.clear();
        self.files_relative.clear();
        self.file_preview = None;
        self.page = Page::Instances;
        cx.notify();
    }

    // -----------------------------------------------------------------
    // Theme and language
    // -----------------------------------------------------------------

    pub fn toggle_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dark = !self.dark;
        self.settings.theme = if self.dark {
            ThemePreference::Dark
        } else {
            ThemePreference::Light
        };
        let _ = core_settings::save(&self.paths, &self.settings);
        self.animate_theme(window, cx);
        cx.notify();
    }

    fn animate_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let scheme = APP_THEME.scheme(
            if self.dark {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            },
            APP_CONTRAST,
        );
        let running = theme::advance_component_theme(scheme, 1.0, window, cx);
        if running && !self.theme_pending {
            self.theme_pending = true;
            let view = cx.entity().downgrade();
            window.on_next_frame(move |window, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.theme_pending = false;
                    this.animate_theme(window, cx);
                });
            });
        }
    }

    pub fn toggle_language(&mut self, cx: &mut Context<Self>) {
        self.lang = match self.lang {
            Language::ZhCn => Language::EnUs,
            Language::EnUs => Language::ZhCn,
        };
        self.settings.language = self.lang;
        self.settings.language_mode = match self.lang {
            Language::ZhCn => crate::core::settings::LanguageMode::ZhCn,
            Language::EnUs => crate::core::settings::LanguageMode::EnUs,
        };
        let _ = core_settings::save(&self.paths, &self.settings);
        cx.notify();
    }

    // -----------------------------------------------------------------
    // Instance commands
    // -----------------------------------------------------------------

    pub fn open_create_dialog(&mut self, cx: &mut Context<Self>) {
        self.dialog = Some(Dialog::CreateInstance);
        self.dialog_error = None;
        if matches!(self.create_loader, LoaderType::Forge | LoaderType::Neoforge)
            && !self.create_version.is_empty()
        {
            let key = Some((self.create_loader, self.create_version.clone()));
            if self.loader_versions_key != key
                || (self.loader_versions.is_empty()
                    && self.loader_versions_error.is_none()
                    && !self.loader_versions_loading)
            {
                self.refresh_loader_versions(cx);
            }
        }
        cx.notify();
    }

    pub fn choose_create_loader(&mut self, loader: LoaderType, cx: &mut Context<Self>) {
        if self.create_loader == loader {
            return;
        }
        self.create_loader = loader;
        self.create_loader_version = None;
        self.loader_versions.clear();
        self.loader_versions_error = None;
        self.loader_versions_loading = false;
        if matches!(loader, LoaderType::Forge | LoaderType::Neoforge) {
            self.refresh_loader_versions(cx);
        } else {
            self.loader_versions_key = None;
            cx.notify();
        }
    }

    pub fn choose_create_version(&mut self, version: String, cx: &mut Context<Self>) {
        if self.create_version == version {
            return;
        }
        self.create_version = version;
        if matches!(self.create_loader, LoaderType::Forge | LoaderType::Neoforge) {
            self.refresh_loader_versions(cx);
        } else {
            cx.notify();
        }
    }

    /// Loads the Forge/NeoForge builds for the selected Minecraft version.
    pub fn refresh_loader_versions(&mut self, cx: &mut Context<Self>) {
        let loader = self.create_loader;
        let minecraft_version = self.create_version.clone();
        self.create_loader_version = None;
        self.loader_versions.clear();
        self.loader_versions_error = None;

        if !matches!(loader, LoaderType::Forge | LoaderType::Neoforge)
            || minecraft_version.is_empty()
        {
            self.loader_versions_loading = false;
            self.loader_versions_key = None;
            cx.notify();
            return;
        }

        self.loader_versions_loading = true;
        self.loader_versions_key = Some((loader, minecraft_version.clone()));
        cx.notify();
        let discover_version = minecraft_version.clone();
        spawn_task(
            cx,
            move || loader::discover(&discover_version, loader),
            move |this, result, cx| {
                if this.loader_versions_key.as_ref() != Some(&(loader, minecraft_version.clone())) {
                    return;
                }
                this.loader_versions_loading = false;
                match result {
                    Ok(versions) => {
                        this.create_loader_version =
                            versions.first().map(|option| option.version.clone());
                        this.loader_versions = versions;
                        this.loader_versions_error = None;
                    }
                    Err(error) => {
                        this.loader_versions.clear();
                        this.loader_versions_error = Some(this.error_text(&error));
                    }
                }
                cx.notify();
            },
        );
    }

    pub fn create_instance(&mut self, cx: &mut Context<Self>) {
        let name = self.create_name.read(cx).value().to_string();
        let loader_name = self.loader_label(self.create_loader);
        let name = if name.trim().is_empty() {
            format!("{loader_name} {}", self.create_version)
        } else {
            name.trim().to_string()
        };
        if self.create_version.is_empty() {
            let message = self.t("instances.unknown_version").to_string();
            self.dialog_error = Some(message.clone());
            self.show_toast(message, true, cx);
            return;
        }
        let uses_installer = matches!(self.create_loader, LoaderType::Forge | LoaderType::Neoforge);
        if uses_installer && self.create_loader_version.is_none() {
            let message = self.t("instances.loader_version_required").to_string();
            self.dialog_error = Some(message.clone());
            self.show_toast(message, true, cx);
            return;
        }
        let loader_version = if uses_installer {
            self.create_loader_version.clone()
        } else {
            None
        };
        let paths = self.paths.clone();
        let new = instances::NewInstance {
            name,
            version_id: self.create_version.clone(),
            loader: instances::Loader {
                kind: self.create_loader,
                version: loader_version,
            },
            memory_mb: self.create_memory,
            memory_mode: MemoryMode::Manual,
            icon: None,
        };
        match instances::create(&paths, new) {
            Ok(record) => {
                self.dialog = None;
                self.dialog_error = None;
                self.open_instance(record.id.clone(), cx);
                self.refresh_instances(cx);
                self.start_install(record.id, cx);
            }
            Err(error) => {
                self.dialog_error = Some(self.error_text(&error));
                self.show_error(error, cx);
            }
        }
    }

    pub fn duplicate_instance(&mut self, id: String, cx: &mut Context<Self>) {
        let paths = self.paths.clone();
        let suffix = self.t("instances.copy_suffix").to_string();
        spawn_task(
            cx,
            move || instances::duplicate(&paths, &id, &suffix),
            |this, result, cx| match result {
                Ok(record) => {
                    this.refresh_instances(cx);
                    this.open_instance(record.id, cx);
                }
                Err(error) => this.show_error(error, cx),
            },
        );
    }

    pub fn delete_instance(&mut self, id: String, cx: &mut Context<Self>) {
        let paths = self.paths.clone();
        let target = id.clone();
        spawn_task(
            cx,
            move || instances::delete(&paths, &id, false),
            move |this, result, cx| match result {
                Ok(()) => {
                    this.dialog = None;
                    if this.selected_instance.as_deref() == Some(target.as_str()) {
                        this.selected_instance = None;
                    }
                    this.refresh_instances(cx);
                    this.show_toast(this.t("toast.removed"), false, cx);
                }
                Err(error) => this.show_error(error, cx),
            },
        );
    }

    pub fn open_folder(&mut self, id: &str, cx: &mut Context<Self>) {
        let path = self.paths.game_dir(id).unwrap_or_default();
        let _ = std::fs::create_dir_all(&path);
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer").arg(&path).spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&path).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
        }
        self.show_toast(self.t("toast.folder_opened"), false, cx);
    }

    pub fn start_install(&mut self, id: String, cx: &mut Context<Self>) {
        if let Some(task) = self.installs.get(&id) {
            if !task.finished {
                return;
            }
            self.installs.remove(&id);
        }
        let paths = self.paths.clone();
        let record = match instances::load(&paths, &id) {
            Ok(record) => record,
            Err(error) => {
                self.show_error(error, cx);
                return;
            }
        };
        let source = self.settings.download_source;
        let java_settings = self.settings.java.clone();
        let progress = Arc::new(Mutex::new(InstallProgress::new(
            InstallPhase::Metadata,
            0.0,
        )));
        let done = Arc::new(AtomicBool::new(false));
        let outcome: Arc<Mutex<Option<crate::core::Result<()>>>> = Arc::new(Mutex::new(None));

        self.installs.insert(
            id.clone(),
            InstallTask {
                instance_id: id.clone(),
                percent: 0.0,
                phase: InstallPhase::Metadata,
                message: None,
                finished: false,
                error: None,
            },
        );
        cx.notify();

        let background_progress = Arc::clone(&progress);
        let background_done = Arc::clone(&done);
        let background_outcome = Arc::clone(&outcome);
        cx.background_executor()
            .spawn(async move {
                let result = install_instance_blocking(
                    &paths,
                    &record,
                    source,
                    &java_settings,
                    Arc::clone(&background_progress),
                );
                *background_outcome.lock().unwrap() = Some(result);
                background_done.store(true, Ordering::Relaxed);
            })
            .detach();

        let target = id.clone();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
                let is_done = done.load(Ordering::Relaxed);
                let latest = progress.lock().unwrap().clone();
                let result = outcome.lock().unwrap().clone();
                let _ = this.update(cx, |this, cx| {
                    let error_text = result
                        .as_ref()
                        .and_then(|value| value.as_ref().err().map(|error| this.error_text(error)));
                    if let Some(task) = this.installs.get_mut(&target) {
                        task.percent = latest.percent;
                        task.phase = latest.phase;
                        task.message = latest.message.clone();
                        if is_done {
                            task.finished = true;
                            task.error = error_text;
                        }
                    }
                    if is_done {
                        this.refresh_instances(cx);
                    }
                    cx.notify();
                });
                if is_done {
                    break;
                }
            }
        })
        .detach();
    }

    pub fn launch_instance(
        &mut self,
        id: String,
        server: Option<ServerTarget>,
        cx: &mut Context<Self>,
    ) {
        if self.is_game_running(&id) {
            return;
        }
        let paths = self.paths.clone();
        let java_settings = self.settings.java.clone();
        let source = self.settings.download_source;
        let target = id.clone();
        spawn_task(
            cx,
            move || do_launch(&paths, &java_settings, source, &id, server),
            move |this, result, cx| match result {
                Ok(process) => {
                    this.running
                        .insert(target.clone(), RunningGame::new(process));
                    this.show_toast(this.t("toast.launched"), false, cx);
                    this.start_game_pump(target, cx);
                }
                Err(error) => this.show_error(error, cx),
            },
        );
    }

    pub fn stop_game(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(game) = self.running.get_mut(id) {
            game.process.kill();
            game.status = GameStatus::Exited;
        }
        cx.notify();
    }

    fn start_game_pump(&mut self, id: String, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
                let finished = this
                    .update(cx, |this, cx| this.pump_game(&id, cx))
                    .unwrap_or(true);
                if finished {
                    break;
                }
            }
        })
        .detach();
    }

    fn pump_game(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        let (finished, exit_code, playtime_ms, crashed) = {
            let Some(game) = self.running.get_mut(id) else {
                return true;
            };
            while let Ok(line) = game.process.lines.try_recv() {
                game.logs.push(line);
            }
            if game.logs.len() > 5000 {
                let drain = game.logs.len() - 5000;
                game.logs.drain(0..drain);
            }
            game.scroll.scroll_to_bottom();
            if game.status == GameStatus::Starting && game.process.started.elapsed().as_secs() >= 1
            {
                game.status = GameStatus::Running;
            }
            match game.process.try_wait() {
                Some(code) => {
                    let killed = game.process.killed_by_user.load(Ordering::Relaxed);
                    let crashed = !killed && code != Some(0);
                    game.status = if crashed {
                        GameStatus::Crashed
                    } else {
                        GameStatus::Exited
                    };
                    game.exit_code = code;
                    (
                        true,
                        code,
                        game.process.started.elapsed().as_millis() as u64,
                        crashed,
                    )
                }
                None => (false, None, 0, false),
            }
        };

        if finished {
            let paths = self.paths.clone();
            let id = id.to_string();
            let _ = instances::mark_exited(&paths, &id, exit_code, playtime_ms);
            if crashed {
                self.show_toast(self.t("instances.crashed"), true, cx);
            }
            self.refresh_instances(cx);
        }
        cx.notify();
        finished
    }

    // -----------------------------------------------------------------
    // Accounts / servers / library
    // -----------------------------------------------------------------

    pub fn add_offline_account(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.account_name.read(cx).value().to_string();
        let paths = self.paths.clone();
        match accounts::add_offline(&paths, &name) {
            Ok(items) => {
                self.accounts = items;
                self.dialog = None;
                self.dialog_error = None;
                self.account_name
                    .update(cx, |input, cx| input.set_value("", window, cx));
                self.show_toast(self.t("toast.account_added"), false, cx);
            }
            Err(error) => {
                self.dialog_error = Some(self.error_text(&error));
                self.show_error(error, cx);
            }
        }
    }

    pub fn start_microsoft_login(&mut self, cx: &mut Context<Self>) {
        self.dialog_error = None;
        self.dialog = Some(Dialog::MicrosoftLogin {
            user_code: String::new(),
            verification_uri: String::new(),
            waiting: true,
        });
        spawn_task(
            cx,
            accounts::start_device_code,
            |this, result, cx| match result {
                Ok(code) => {
                    cx.open_url(&code.verification_uri);
                    this.dialog = Some(Dialog::MicrosoftLogin {
                        user_code: code.user_code.clone(),
                        verification_uri: code.verification_uri.clone(),
                        waiting: false,
                    });
                    let paths = this.paths.clone();
                    let code_clone = code.clone();
                    spawn_task(
                        cx,
                        move || accounts::complete_microsoft_login(&paths, &code_clone, |_| {}),
                        |this, result, cx| match result {
                            Ok(_) => {
                                this.dialog = None;
                                this.dialog_error = None;
                                this.refresh_accounts(cx);
                                this.show_toast(this.t("toast.account_added"), false, cx);
                            }
                            Err(error) => {
                                this.dialog_error = Some(this.error_text(&error));
                                this.show_error(error, cx);
                            }
                        },
                    );
                }
                Err(error) => this.show_error(error, cx),
            },
        );
    }

    pub fn add_authlib_account(&mut self, cx: &mut Context<Self>) {
        let server = self.authlib_server.read(cx).value().to_string();
        let user = self.authlib_user.read(cx).value().to_string();
        let password = self.authlib_password.read(cx).value().to_string();
        let paths = self.paths.clone();
        spawn_task(
            cx,
            move || accounts::add_authlib(&paths, &server, &user, &password),
            |this, result, cx| match result {
                Ok(_) => {
                    this.dialog = None;
                    this.dialog_error = None;
                    this.refresh_accounts(cx);
                    this.show_toast(this.t("toast.account_added"), false, cx);
                }
                Err(error) => {
                    this.dialog_error = Some(this.error_text(&error));
                    this.show_error(error, cx);
                }
            },
        );
    }

    pub fn set_active_account(&mut self, id: String, cx: &mut Context<Self>) {
        let paths = self.paths.clone();
        spawn_task(
            cx,
            move || accounts::set_active(&paths, &id),
            |this, result, cx| match result {
                Ok(items) => this.accounts = items,
                Err(error) => this.show_error(error, cx),
            },
        );
    }

    pub fn remove_account(&mut self, id: String, cx: &mut Context<Self>) {
        let paths = self.paths.clone();
        spawn_task(
            cx,
            move || accounts::remove(&paths, &id),
            |this, result, cx| match result {
                Ok(items) => this.accounts = items,
                Err(error) => this.show_error(error, cx),
            },
        );
    }

    pub fn add_server(&mut self, cx: &mut Context<Self>) {
        let name = self.server_name.read(cx).value().to_string();
        let address = self.server_address.read(cx).value().to_string();
        let paths = self.paths.clone();
        match servers::add(&paths, &name, &address, None) {
            Ok(_) => {
                self.dialog = None;
                self.dialog_error = None;
                self.refresh_servers(cx);
                self.show_toast(self.t("toast.server_added"), false, cx);
            }
            Err(error) => {
                self.dialog_error = Some(self.error_text(&error));
                self.show_error(error, cx);
            }
        }
    }

    pub fn remove_server(&mut self, id: String, cx: &mut Context<Self>) {
        let paths = self.paths.clone();
        spawn_task(
            cx,
            move || servers::remove(&paths, &id),
            |this, result, cx| match result {
                Ok(items) => {
                    this.servers = items;
                    this.statuses.clear();
                }
                Err(error) => this.show_error(error, cx),
            },
        );
    }

    pub fn ping_all_servers(&mut self, cx: &mut Context<Self>) {
        let servers = self.servers.clone();
        for server in servers {
            self.ping_server(server, cx);
        }
    }

    pub fn ping_server(&mut self, server: ServerRecord, cx: &mut Context<Self>) {
        let id = server.id.clone();
        let host = server.address.clone();
        let port = server.port;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { servers::ping(&host, port) })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(status) => {
                        this.statuses.insert(id, status);
                    }
                    Err(error) => {
                        this.statuses.remove(&id);
                        this.show_error(error, cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn search_library(&mut self, cx: &mut Context<Self>) {
        if self.loading.library {
            return;
        }
        self.loading.library = true;
        self.library_error = None;
        self.search_query = self.search_input.read(cx).value().to_string();
        let query = self.search_query.clone();
        let project_type = self.library_type.clone();
        let loader = self.library_loader();
        let game_version = self.library_game_version();
        cx.notify();
        spawn_task(
            cx,
            move || {
                modrinth::search(
                    &query,
                    &project_type,
                    loader.as_deref(),
                    game_version.as_deref(),
                    24,
                    0,
                )
            },
            |this, result, cx| {
                this.loading.library = false;
                match result {
                    Ok(results) => {
                        this.search_results = Some(results);
                        this.library_error = None;
                    }
                    Err(error) => {
                        this.library_error = Some(this.error_text(&error));
                        this.search_results = Some(SearchResult {
                            hits: Vec::new(),
                            total_hits: 0,
                            offset: 0,
                            limit: 0,
                        });
                        this.show_error(error, cx);
                    }
                }
            },
        );
    }

    pub fn library_loader(&self) -> Option<String> {
        self.library_target
            .as_ref()
            .and_then(|id| self.instances.iter().find(|item| &item.record.id == id))
            .and_then(|item| match item.record.loader.kind {
                LoaderType::Fabric => Some("fabric".to_string()),
                LoaderType::Quilt => Some("quilt".to_string()),
                LoaderType::Forge => Some("forge".to_string()),
                LoaderType::Neoforge => Some("neoforge".to_string()),
                LoaderType::Vanilla => None,
            })
    }

    pub fn library_game_version(&self) -> Option<String> {
        self.library_target
            .as_ref()
            .and_then(|id| self.instances.iter().find(|item| &item.record.id == id))
            .map(|item| item.record.version_id.clone())
    }

    pub fn set_library_type(&mut self, kind: String, cx: &mut Context<Self>) {
        self.library_type = kind;
        self.search_library(cx);
    }

    pub fn install_from_library(&mut self, project_id: String, cx: &mut Context<Self>) {
        if self.library_type == "modpack" {
            self.install_modpack_project(project_id, cx);
            return;
        }
        let Some(target) = self.library_target.clone() else {
            self.show_toast(self.t("library.no_instance"), true, cx);
            return;
        };
        let project_type = self.library_type.clone();
        let loader = self.library_loader();
        let game_version = self.library_game_version();
        let paths = self.paths.clone();
        let id = project_id.clone();
        spawn_task(
            cx,
            move || {
                modrinth::install(
                    &paths,
                    &target,
                    &project_type,
                    &id,
                    None,
                    loader.as_deref(),
                    game_version.as_deref(),
                )
            },
            |this, result, cx| match result {
                Ok(files) => this.show_toast(
                    format!("{} ({})", this.t("library.install_done"), files.len()),
                    false,
                    cx,
                ),
                Err(error) => this.show_error(error, cx),
            },
        );
    }

    // -----------------------------------------------------------------
    // Backups, export/import and the file browser
    // -----------------------------------------------------------------

    pub fn set_instance_tab(&mut self, tab: InstanceTab, cx: &mut Context<Self>) {
        self.instance_tab = tab;
        let Some(id) = self.selected_instance.clone() else {
            cx.notify();
            return;
        };
        match tab {
            InstanceTab::Backups => self.refresh_backups(&id, cx),
            InstanceTab::Files => {
                let relative = self.files_relative.clone();
                self.refresh_files(&id, relative, cx);
            }
            _ => cx.notify(),
        }
    }

    pub fn refresh_backups(&mut self, instance_id: &str, cx: &mut Context<Self>) {
        if self.loading.backups {
            return;
        }
        self.loading.backups = true;
        cx.notify();
        let paths = self.paths.clone();
        let task_id = instance_id.to_string();
        let target = instance_id.to_string();
        spawn_task(
            cx,
            move || backup::list(&paths, &task_id),
            move |this, result, cx| {
                this.loading.backups = false;
                match result {
                    Ok(items) => {
                        if this.selected_instance.as_deref() == Some(target.as_str()) {
                            this.backups = items;
                        }
                    }
                    Err(error) => this.show_error(error, cx),
                }
            },
        );
    }

    pub fn create_backup(
        &mut self,
        window: &mut Window,
        instance_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.loading.backups {
            return;
        }
        if self.is_game_running(&instance_id) {
            self.show_toast(self.t("backups.game_running"), true, cx);
            return;
        }
        let label = self.backup_label.read(cx).value().trim().to_string();
        self.backup_label
            .update(cx, |input, cx| input.set_value("", window, cx));
        let paths = self.paths.clone();
        let target = instance_id.clone();
        self.loading.backups = true;
        cx.notify();
        spawn_task(
            cx,
            move || {
                let label = (!label.is_empty()).then_some(label.as_str());
                backup::create(&paths, &instance_id, label, |_| {})
            },
            move |this, result, cx| {
                this.loading.backups = false;
                match result {
                    Ok(_) => {
                        this.refresh_backups(&target, cx);
                        this.show_toast(this.t("backups.created_toast"), false, cx);
                    }
                    Err(error) => this.show_error(error, cx),
                }
            },
        );
    }

    pub fn restore_backup(
        &mut self,
        instance_id: String,
        backup_id: String,
        cx: &mut Context<Self>,
    ) {
        self.dialog = None;
        cx.notify();
        if self.is_game_running(&instance_id) {
            self.show_toast(self.t("backups.game_running"), true, cx);
            return;
        }
        let paths = self.paths.clone();
        let target = instance_id.clone();
        spawn_task(
            cx,
            move || backup::restore(&paths, &instance_id, &backup_id, |_| {}),
            move |this, result, cx| match result {
                Ok(_) => {
                    this.refresh_backups(&target, cx);
                    this.show_toast(this.t("backups.restored"), false, cx);
                }
                Err(error) => this.show_error(error, cx),
            },
        );
    }

    pub fn delete_backup(
        &mut self,
        instance_id: String,
        backup_id: String,
        cx: &mut Context<Self>,
    ) {
        self.dialog = None;
        cx.notify();
        let paths = self.paths.clone();
        let target = instance_id.clone();
        spawn_task(
            cx,
            move || backup::delete(&paths, &instance_id, &backup_id),
            move |this, result, cx| match result {
                Ok(()) => {
                    this.refresh_backups(&target, cx);
                    this.show_toast(this.t("backups.deleted_toast"), false, cx);
                }
                Err(error) => this.show_error(error, cx),
            },
        );
    }

    pub fn export_instance(&mut self, instance_id: String, cx: &mut Context<Self>) {
        let paths = self.paths.clone();
        spawn_task(
            cx,
            move || export::export_instance(&paths, &instance_id, |_| {}),
            |this, result, cx| match result {
                Ok(path) => {
                    let filename = path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_default();
                    this.show_toast(
                        format!("{} {}", this.t("backups.export_done"), filename),
                        false,
                        cx,
                    );
                }
                Err(error) => this.show_error(error, cx),
            },
        );
    }

    pub fn import_instance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let raw = self
            .import_path
            .read(cx)
            .value()
            .trim()
            .trim_matches('"')
            .to_string();
        if raw.is_empty() {
            self.show_toast(self.t("backups.import_path_required"), true, cx);
            return;
        }
        self.import_path
            .update(cx, |input, cx| input.set_value("", window, cx));
        let paths = self.paths.clone();
        let source = PathBuf::from(raw);
        spawn_task(
            cx,
            move || export::import(&paths, &source, |_| {}),
            |this, result, cx| match result {
                Ok(record) => {
                    this.dialog = None;
                    this.refresh_instances(cx);
                    this.open_instance(record.id, cx);
                    this.show_toast(this.t("backups.imported_toast"), false, cx);
                }
                Err(error) => this.show_error(error, cx),
            },
        );
    }

    pub fn refresh_files(&mut self, instance_id: &str, relative: String, cx: &mut Context<Self>) {
        let requested = relative;
        self.files_relative = requested.clone();
        self.file_preview = None;
        self.loading.files = true;
        cx.notify();
        let paths = self.paths.clone();
        let task_id = instance_id.to_string();
        let task_relative = requested.clone();
        let target = instance_id.to_string();
        let guard = requested;
        spawn_task(
            cx,
            move || files::list_dir(&paths, &task_id, &task_relative),
            move |this, result, cx| {
                if this.selected_instance.as_deref() == Some(target.as_str())
                    && this.files_relative == guard
                {
                    this.loading.files = false;
                    match result {
                        Ok(items) => this.files_entries = items,
                        Err(error) => this.show_error(error, cx),
                    }
                }
            },
        );
    }

    pub fn open_files_dir(&mut self, instance_id: &str, relative: String, cx: &mut Context<Self>) {
        self.refresh_files(instance_id, relative, cx);
    }

    pub fn preview_file(&mut self, instance_id: &str, relative: String, cx: &mut Context<Self>) {
        self.file_preview = None;
        cx.notify();
        let paths = self.paths.clone();
        let task_id = instance_id.to_string();
        let target = instance_id.to_string();
        spawn_task(
            cx,
            move || files::read_text_preview(&paths, &task_id, &relative),
            move |this, result, cx| {
                if this.selected_instance.as_deref() == Some(target.as_str()) {
                    match result {
                        Ok(preview) => this.file_preview = Some(preview),
                        Err(error) => this.show_error(error, cx),
                    }
                }
            },
        );
    }

    /// Modpacks are not content dropped into an existing instance: installing
    /// one creates a fresh instance with its own loader and game directory.
    pub fn install_modpack_project(&mut self, project_id: String, cx: &mut Context<Self>) {
        if self.loading.modpack {
            return;
        }
        self.loading.modpack = true;
        let paths = self.paths.clone();
        let source = self.settings.download_source;
        let cancel = Arc::new(AtomicBool::new(false));
        cx.notify();
        spawn_task(
            cx,
            move || modpack::install_modrinth(&paths, &project_id, None, source, cancel, |_| {}),
            |this, result, cx| {
                this.loading.modpack = false;
                match result {
                    Ok(record) => {
                        this.refresh_instances(cx);
                        this.open_instance(record.id, cx);
                        this.show_toast(this.t("library.modpack_done"), false, cx);
                    }
                    Err(error) => this.show_error(error, cx),
                }
            },
        );
    }

    pub fn import_modpack(&mut self, cx: &mut Context<Self>) {
        if self.loading.modpack {
            return;
        }
        let value = self.modpack_path.read(cx).value().to_string();
        let value = value.trim().to_string();
        if value.is_empty() {
            self.show_toast(self.t("library.import_empty"), true, cx);
            return;
        }
        let paths = self.paths.clone();
        let source = self.settings.download_source;
        let cancel = Arc::new(AtomicBool::new(false));
        let path = PathBuf::from(value);
        self.loading.modpack = true;
        self.dialog = None;
        cx.notify();
        spawn_task(
            cx,
            move || modpack::import_local(&paths, &path, source, cancel, |_| {}),
            |this, result, cx| {
                this.loading.modpack = false;
                match result {
                    Ok(record) => {
                        this.refresh_instances(cx);
                        this.open_instance(record.id, cx);
                        this.show_toast(this.t("library.modpack_done"), false, cx);
                    }
                    Err(error) => this.show_error(error, cx),
                }
            },
        );
    }

    pub fn remove_content(
        &mut self,
        instance_id: String,
        project_type: String,
        filename: String,
        cx: &mut Context<Self>,
    ) {
        let paths = self.paths.clone();
        match modrinth::remove_file(&paths, &instance_id, &project_type, &filename) {
            Ok(()) => {
                self.dialog = None;
                self.show_toast(self.t("toast.removed"), false, cx);
            }
            Err(error) => self.show_error(error, cx),
        }
    }
}

fn spawn_task<T, F, D>(cx: &mut Context<Yuhina>, task: F, done: D)
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
    D: FnOnce(&mut Yuhina, T, &mut Context<Yuhina>) + 'static,
{
    cx.spawn(async move |this, cx| {
        let result = cx.background_executor().spawn(async move { task() }).await;
        let _ = this.update(cx, move |this, cx| {
            done(this, result, cx);
            cx.notify();
        });
    })
    .detach();
}

fn install_instance_blocking(
    paths: &Paths,
    record: &InstanceRecord,
    source: DownloadSource,
    java_settings: &JavaSettings,
    progress: Arc<Mutex<InstallProgress>>,
) -> crate::core::Result<()> {
    let cancel = Arc::new(AtomicBool::new(false));
    let report = |progress: Arc<Mutex<InstallProgress>>| {
        move |value: InstallProgress| {
            *progress.lock().unwrap() = value;
        }
    };

    install::install(
        paths,
        &record.version_id,
        source,
        false,
        Arc::clone(&cancel),
        report(Arc::clone(&progress)),
    )?;

    if record.loader.kind != LoaderType::Vanilla {
        let loader_id = install::install_loader(
            paths,
            install::LoaderInstallRequest {
                minecraft_version: &record.version_id,
                loader: record.loader.kind,
                loader_version: record.loader.version.as_deref(),
                java_settings,
                source,
            },
            Arc::clone(&cancel),
            report(Arc::clone(&progress)),
        )?;
        instances::set_launch_version_id(paths, &record.id, loader_id)?;
    }

    Ok(())
}

fn do_launch(
    paths: &Paths,
    java_settings: &crate::core::settings::JavaSettings,
    source: DownloadSource,
    instance_id: &str,
    server: Option<ServerTarget>,
) -> crate::core::Result<GameProcess> {
    let record = instances::load(paths, instance_id)?;
    let launch_id = record.launch_version().to_string();
    if !install::is_installed(paths, &launch_id) {
        return Err(YuhinaError::new(
            crate::core::ErrorCode::InstallFailed,
            "Install this version before launching it.",
        ));
    }
    let (version, root) = install::resolve_local(paths, &launch_id)?;
    let context = RuleContext::current();
    let selected = crate::core::version::resolve_libraries(
        version.libraries.as_deref().unwrap_or(&[]),
        &context,
    );
    let client = paths.client_jar(&root)?;
    let mut classpath = vec![client];
    for library in &selected.classpath {
        let relative = crate::core::paths::normalize_relative(&library.relative_path)?;
        classpath.push(Paths::resolve_inside(&paths.libraries_dir(), &relative)?);
    }

    let major = version
        .java_version
        .as_ref()
        .and_then(|java| java.major_version)
        .unwrap_or_else(|| instances::required_java_major(&record.version_id));
    let java_path = match &record.java_path {
        Some(path) => PathBuf::from(path),
        None => {
            let runtime = java::ensure(paths, java_settings, major, |_, _| {})?;
            PathBuf::from(runtime.path)
        }
    };

    let credentials = accounts::launch_credentials(paths, None)?;
    let memory_mb = instances::effective_memory_mb(paths, &record);
    let game_dir = paths.game_dir(&record.id)?;
    std::fs::create_dir_all(&game_dir)?;

    let request = LaunchRequest {
        version_name: version.id.clone(),
        version_type: version
            .kind
            .clone()
            .unwrap_or_else(|| "release".to_string()),
        assets_index_name: version
            .asset_index
            .as_ref()
            .map(|index| index.id.clone())
            .or_else(|| version.assets.clone())
            .unwrap_or_else(|| "legacy".to_string()),
        version,
        classpath,
        natives_directory: paths.natives_dir(&root)?,
        game_directory: game_dir,
        assets_root: paths.assets_dir(),
        library_directory: paths.libraries_dir(),
        credentials: LaunchCredentials {
            name: credentials.name,
            uuid: credentials.uuid,
            access_token: credentials.access_token,
            user_type: credentials.user_type,
        },
        memory_mb,
        launcher_version: APP_VERSION.to_string(),
        jvm_args: record.jvm_args.clone(),
        game_args: record.game_args.clone(),
        authlib: credentials.authlib.map(|authlib| launch::AuthlibInjection {
            jar_path: authlib.jar_path.to_string_lossy().to_string(),
            authlib_url: authlib.url,
            prefetched: authlib.prefetched,
        }),
        server: server.map(|target| ServerTarget {
            host: target.host,
            port: target.port,
            quick_play: target.quick_play && launch::supports_quick_play(&record.version_id),
        }),
        resolution: None,
    };

    let process = launch::spawn(paths, &record.id, &java_path, &request)?;
    instances::mark_launch_started(paths, &record.id)?;
    let _ = source;
    Ok(process)
}

impl Render for Yuhina {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog = dialogs::render(self, window, cx);
        let page = self.page;
        let lang = self.lang;
        let nav_items = navigation_items(lang);
        let selected = page.id();
        let title = self.t(page.title_key());
        let is_dark = self.dark;
        // The compact window class cannot fit the two secondary destinations
        // beside the theme, language and title. Keep them reachable in one
        // overflow menu instead of letting the bar clip them.
        let viewport_width = f32::from(window.viewport_size().width);
        let compact_actions = viewport_width < 600.;

        let content = pages::render(self, window, cx);

        let theme_icon = if is_dark { "light_mode" } else { "dark_mode" };
        let theme_label = if is_dark {
            self.t("topbar.theme_light")
        } else {
            self.t("topbar.theme_dark")
        };
        let language_label = if lang == Language::ZhCn {
            self.t("topbar.language_en")
        } else {
            self.t("topbar.language_zh")
        };

        let mut actions: Vec<AnyElement> = vec![
            IconButton::new("toggle-theme", material_icon(theme_icon, 22.0), theme_label)
                .on_click(cx.listener(|this, _, window, cx| this.toggle_theme(window, cx)))
                .into_any_element(),
            IconButton::new(
                "toggle-language",
                material_icon("translate", 22.0),
                language_label,
            )
            .on_click(cx.listener(|this, _, _, cx| this.toggle_language(cx)))
            .into_any_element(),
        ];

        if compact_actions {
            let items = [
                MaterialMenuItem::new(self.t("nav.activity"))
                    .leading(material_icon("monitoring", 20.0))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.overflow_open = false;
                        this.set_page(Page::Activity, cx);
                    })),
                MaterialMenuItem::new(self.t("nav.settings"))
                    .leading(material_icon("settings", 20.0))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.overflow_open = false;
                        this.set_page(Page::Settings, cx);
                    })),
            ];
            let trigger = IconButton::new(
                "topbar-more",
                material_icon("more_vert", 22.0),
                self.t("topbar.more"),
            )
            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                if matches!(event, ClickEvent::Keyboard(_)) {
                    this.overflow_open = !this.overflow_open;
                    cx.notify();
                }
            }));
            let menu = MaterialMenu::new("topbar-more-menu").items(items);
            actions.push(
                MenuPopup::new("topbar-more-popup", trigger, menu)
                    .open(self.overflow_open)
                    .on_open_change(cx.listener(|this, open: &bool, _, cx| {
                        this.overflow_open = *open;
                        cx.notify();
                    }))
                    .into_any_element(),
            );
        } else {
            actions.push(
                IconButton::new(
                    "open-activity",
                    material_icon("monitoring", 22.0),
                    self.t("nav.activity"),
                )
                .on_click(cx.listener(|this, _, _, cx| this.set_page(Page::Activity, cx)))
                .into_any_element(),
            );
            actions.push(
                IconButton::new(
                    "open-settings",
                    material_icon("settings", 22.0),
                    self.t("nav.settings"),
                )
                .on_click(cx.listener(|this, _, _, cx| this.set_page(Page::Settings, cx)))
                .into_any_element(),
            );
        }

        let top_bar = TopAppBar::new("top-bar", TopAppBarVariant::Small)
            .title(title)
            .container(ColorRole::SURFACE)
            .navigation_icon(logo())
            .actions(actions);

        let compact = viewport_width < 600.;
        let mut primary = Some(
            Fab::new(
                "new-instance-fab",
                material_icon("add", 24.0),
                self.t("instances.new"),
            )
            .on_click(cx.listener(|this, _, _, cx| this.open_create_dialog(cx))),
        );

        let suite = NavigationSuite::new("main-navigation", self.t("nav.landmark"))
            .items(nav_items)
            .selected(selected)
            .layout_type(compact.then_some(NavigationSuiteType::None))
            .expanded_type(NavigationSuiteType::RailExpanded)
            .expanded_from(Some(WindowWidthClass::Expanded))
            .on_change(cx.listener(|this, id: &SharedString, _, cx| {
                this.set_page(Page::from_id(id.as_ref()), cx);
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(div().flex_1().min_h_0().min_w_0().w_full().child(content)),
            );
        let suite = if compact {
            suite
        } else {
            suite.primary_action(primary.take().expect("the compact branch keeps the FAB"))
        };

        let compact_bar = compact.then(|| {
            let bar = NavigationBar::new("main-navigation", self.t("nav.landmark"))
                .items(navigation_items(lang))
                .selected(selected)
                .layout(NavItemLayout::Vertical)
                .on_change(cx.listener(|this, id: &SharedString, _, cx| {
                    this.set_page(Page::from_id(id.as_ref()), cx);
                }));
            // The bar's lane is wider than some compact windows; clip the bar
            // rather than letting it widen the page content.
            div()
                .w_full()
                .min_w_0()
                .flex_shrink_0()
                .overflow_hidden()
                .child(div().w(px(viewport_width)).child(bar))
        });
        let compact_fab = primary.map(|fab| {
            div()
                .absolute()
                .bottom(px(96.))
                .right(px(16.))
                .child(fab)
                .into_any_element()
        });

        let snackbar = self.toast.clone().map(|toast| {
            let duration = if toast.error {
                SnackbarDuration::Long
            } else {
                SnackbarDuration::Short
            };
            div().absolute().left(px(24.)).bottom(px(24.)).child(
                Snackbar::new("toast", toast.text)
                    .open(true)
                    .duration(duration)
                    .close(
                        i18n::tr(lang, "common.close"),
                        cx.listener(|this, _, _, cx| {
                            this.toast = None;
                            cx.notify();
                        }),
                    ),
            )
        });

        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .min_w_0()
            .overflow_hidden()
            .bg(color(ColorRole::SURFACE))
            .text_color(color(ColorRole::ON_SURFACE))
            .font_family("Roboto")
            .child(div().w_full().min_w_0().child(top_bar))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .w(px(viewport_width))
                    .overflow_hidden()
                    .child(suite),
            )
            .children(compact_bar)
            .children(compact_fab)
            .children(dialog)
            .children(snackbar)
    }
}

fn navigation_items(lang: Language) -> Vec<NavItem> {
    [
        ("home", "nav.home", "home"),
        ("instances", "nav.instances", "grid_view"),
        ("library", "nav.library", "explore"),
        ("accounts", "nav.accounts", "person"),
        ("servers", "nav.servers", "dns"),
    ]
    .into_iter()
    .map(|(id, label, icon)| NavItem::new(id, i18n::tr(lang, label), NavIcon::symbol(symbol(icon))))
    .collect()
}

pub fn symbol(name: &str) -> Symbol {
    Symbol::named(name).unwrap_or(Symbol::SETTINGS)
}

pub fn material_icon(name: &str, size: f32) -> Icon {
    Icon::from(symbol(name).icon()).size(rem(size))
}

fn logo() -> impl IntoElement {
    div()
        .size(px(32.))
        .rounded(px(10.))
        .bg(color(ColorRole::PRIMARY))
        .text_color(color(ColorRole::ON_PRIMARY))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(16.))
        .font_weight(FontWeight::BOLD)
        .child("Y")
}
