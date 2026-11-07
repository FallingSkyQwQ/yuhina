pub mod dialogs;
pub mod i18n;
pub mod pages;
pub mod platform;

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui_kit::base::input::{InputEvent, InputState};
use gpui_kit::base::slider::{SliderEvent, SliderState};
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
    self as core_settings, DownloadSource, JavaSettings, Language, LanguageMode, ReduceMotion,
    Settings, ThemePreference,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
pub enum ToastAction {
    Instances,
    Accounts,
    Servers,
    Library,
    News,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub text: String,
    pub error: bool,
    pub action: Option<(&'static str, ToastAction)>,
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
    DeleteInstance {
        id: String,
        name: String,
    },
    DeleteServer {
        id: String,
        name: String,
    },
    RemoveAccount {
        id: String,
        name: String,
    },
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
    pub toasts: VecDeque<Toast>,
    pub dialog: Option<Dialog>,
    pub dialog_error: Option<String>,
    pub dialog_field_error: Option<&'static str>,
    pub dialog_busy: bool,
    pub dialog_scroll: ScrollHandle,
    pub menu_open: Option<String>,
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
    pub search_generation: u64,
    pub memory_slider: Entity<SliderState>,
    pub dialog_memory_slider: Entity<SliderState>,
    pub scroll: HashMap<Page, ScrollHandle>,
    pub reduce_motion: bool,
    pub window_title: String,
    theme_pending: bool,
    _subscriptions: Vec<Subscription>,
}

impl Yuhina {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let paths = Paths::discover().unwrap_or_else(|_| Paths::at("."));
        let settings = core_settings::load(&paths).unwrap_or_default();
        let lang = if settings.language_mode == LanguageMode::System {
            platform::system_language().unwrap_or_else(|| settings.effective_language())
        } else {
            settings.effective_language()
        };
        let system_dark = platform::system_dark(matches!(
            cx.window_appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        ));
        let dark_override = std::env::var("YUHINA_DARK").ok().map(|value| value != "0");
        let dark = dark_override.unwrap_or(match settings.theme {
            ThemePreference::Dark => true,
            ThemePreference::Light => false,
            ThemePreference::System => system_dark,
        });
        let reduce_motion = match settings.reduce_motion {
            ReduceMotion::On => true,
            ReduceMotion::Off => false,
            ReduceMotion::System => platform::system_reduce_motion(),
        };
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
        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(i18n::tr(lang, "library.search_placeholder"))
        });
        let import_path = cx.new(|cx| InputState::new(window, cx));
        let backup_label = cx.new(|cx| InputState::new(window, cx));
        let memory_slider = cx.new(|_| {
            SliderState::new()
                .min(1024.)
                .max(16384.)
                .step(512.)
                .default_value(settings.instance_defaults.memory_mb as f32)
        });
        let dialog_memory_slider = cx.new(|_| {
            SliderState::new()
                .min(1024.)
                .max(16384.)
                .step(512.)
                .default_value(4096.)
        });
        let scroll = [
            Page::Home,
            Page::Instances,
            Page::Library,
            Page::Accounts,
            Page::Servers,
            Page::Activity,
            Page::Settings,
        ]
        .into_iter()
        .map(|page| (page, ScrollHandle::new()))
        .collect();

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
            toasts: VecDeque::new(),
            dialog,
            dialog_error: None,
            dialog_field_error: None,
            dialog_busy: false,
            dialog_scroll: ScrollHandle::new(),
            menu_open: None,
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
            search_generation: 0,
            memory_slider,
            dialog_memory_slider,
            scroll,
            reduce_motion,
            window_title: String::new(),
            theme_pending: false,
            _subscriptions: Vec::new(),
        };
        cx.set_reduce_motion(reduce_motion);
        let search_subscription = cx.subscribe_in(
            &this.search_input,
            window,
            |this, _input, event: &InputEvent, _window, cx| match event {
                InputEvent::Change => this.schedule_library_search(cx),
                InputEvent::PressEnter { .. } => this.search_library(cx),
                _ => {}
            },
        );
        let memory_subscription = cx.subscribe_in(
            &this.memory_slider,
            window,
            |this, _slider, event: &SliderEvent, _window, cx| {
                if let SliderEvent::Change(value) = event {
                    this.settings.instance_defaults.memory_mb =
                        value.start().round().clamp(1024., 16384.) as u32;
                    let _ = core_settings::save(&this.paths, &this.settings);
                    cx.notify();
                }
            },
        );
        let dialog_memory_subscription = cx.subscribe_in(
            &this.dialog_memory_slider,
            window,
            |this, _slider, event: &SliderEvent, _window, cx| {
                if let SliderEvent::Change(value) = event {
                    this.create_memory = value.start().round().clamp(1024., 16384.) as u32;
                    cx.notify();
                }
            },
        );
        this._subscriptions = vec![
            search_subscription,
            memory_subscription,
            dialog_memory_subscription,
        ];
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
        message.to_string()
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
                        this.show_error_with(
                            error,
                            Some(("common.retry", ToastAction::Instances)),
                            cx,
                        );
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
                        this.show_error_with(
                            error,
                            Some(("common.retry", ToastAction::Accounts)),
                            cx,
                        );
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
                        this.show_error_with(
                            error,
                            Some(("common.retry", ToastAction::Servers)),
                            cx,
                        );
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
                        this.show_error_with(error, Some(("common.retry", ToastAction::News)), cx);
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

    /// Queue a transient confirmation. Queueing instead of replacing means a
    /// second failure is still readable after the first is dismissed.
    pub fn show_toast(&mut self, text: impl Into<String>, error: bool, cx: &mut Context<Self>) {
        self.push_toast(
            Toast {
                text: text.into(),
                error,
                action: None,
            },
            cx,
        );
    }

    pub fn show_error(&mut self, error: YuhinaError, cx: &mut Context<Self>) {
        self.show_error_with(error, None, cx);
    }

    /// A failure with an optional Retry action. Errors stay until dismissed;
    /// success confirmations use the component's short timer.
    pub fn show_error_with(
        &mut self,
        error: YuhinaError,
        action: Option<(&'static str, ToastAction)>,
        cx: &mut Context<Self>,
    ) {
        let message = self.error_text(&error);
        self.push_toast(
            Toast {
                text: message,
                error: true,
                action,
            },
            cx,
        );
    }

    fn push_toast(&mut self, toast: Toast, cx: &mut Context<Self>) {
        if self
            .toasts
            .back()
            .is_some_and(|queued| queued.error == toast.error && queued.text == toast.text)
        {
            return;
        }
        let is_success = !toast.error;
        self.toasts.push_back(toast);
        if is_success {
            self.arm_success_dismiss(cx);
        }
        cx.notify();
    }

    fn arm_success_dismiss(&mut self, cx: &mut Context<Self>) {
        let Some(front) = self.toasts.front() else {
            return;
        };
        if front.error {
            return;
        }
        let expected = front.text.clone();
        let weak = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            cx.background_executor().timer(Duration::from_secs(4)).await;
            let _ = weak.update(cx, |this, cx| {
                let same = this
                    .toasts
                    .front()
                    .is_some_and(|toast| toast.text == expected && !toast.error);
                if same {
                    this.toasts.pop_front();
                    this.arm_success_dismiss(cx);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub fn dismiss_toast(&mut self, cx: &mut Context<Self>) {
        self.toasts.pop_front();
        self.arm_success_dismiss(cx);
        cx.notify();
    }

    pub fn run_toast_action(&mut self, cx: &mut Context<Self>) {
        let action = self.toasts.front().and_then(|toast| toast.action.clone());
        self.dismiss_toast(cx);
        let Some((_, action)) = action else {
            return;
        };
        match action {
            ToastAction::Instances => self.refresh_instances(cx),
            ToastAction::Accounts => self.refresh_accounts(cx),
            ToastAction::Servers => self.refresh_servers(cx),
            ToastAction::Library => self.search_library(cx),
            ToastAction::News => self.refresh_news(cx, true),
        }
    }

    pub fn set_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        // Each section keeps its own detail, tab and scroll offset. Clearing
        // the selection here made Library -> Instances lose the open instance.
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
        let changed = self.selected_instance.as_deref() != Some(id.as_str());
        self.selected_instance = Some(id);
        if changed {
            self.instance_tab = InstanceTab::Overview;
            self.backups.clear();
            self.files_entries.clear();
            self.files_relative.clear();
            self.file_preview = None;
        }
        self.page = Page::Instances;
        cx.notify();
    }

    /// The standard back affordance for the instance detail.
    pub fn back_to_instances(&mut self, cx: &mut Context<Self>) {
        self.selected_instance = None;
        self.menu_open = None;
        cx.notify();
    }

    pub fn page_scroll(&self, page: Page) -> ScrollHandle {
        self.scroll
            .get(&page)
            .cloned()
            .unwrap_or_else(ScrollHandle::new)
    }

    fn window_title_text(&self) -> String {
        if self.page == Page::Instances
            && let Some(id) = &self.selected_instance
            && let Some(item) = self.instances.iter().find(|item| &item.record.id == id)
        {
            return item.record.name.clone();
        }
        self.t(self.page.title_key()).to_string()
    }

    // -----------------------------------------------------------------
    // Theme and language
    // -----------------------------------------------------------------

    pub fn set_theme(
        &mut self,
        preference: ThemePreference,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.settings.theme == preference {
            return;
        }
        self.settings.theme = preference;
        self.dark = match preference {
            ThemePreference::Dark => true,
            ThemePreference::Light => false,
            ThemePreference::System => platform::system_dark(matches!(
                cx.window_appearance(),
                WindowAppearance::Dark | WindowAppearance::VibrantDark
            )),
        };
        let _ = core_settings::save(&self.paths, &self.settings);
        cx.set_window_appearance(Some(if self.dark {
            WindowAppearance::Dark
        } else {
            WindowAppearance::Light
        }));
        platform::sync_caption(window, self.dark);
        self.animate_theme(window, cx);
        cx.notify();
    }

    /// Persist and apply a new text scale immediately. The scale travels to
    /// both the Component projection and the window's rem size, so rem-based
    /// gutters, row padding and dialog padding grow with the type.
    pub fn set_text_scale(&mut self, scale: f32, window: &mut Window, cx: &mut Context<Self>) {
        let scale = core_settings::clamp_text_scale(scale);
        if (self.settings.text_scale - scale).abs() < f32::EPSILON {
            return;
        }
        self.settings.text_scale = scale;
        let _ = core_settings::save(&self.paths, &self.settings);
        self.animate_theme(window, cx);
        cx.notify();
    }

    pub fn reset_text_scale(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set_text_scale(core_settings::TEXT_SCALE_DEFAULT, window, cx);
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
        let running = theme::advance_component_theme(scheme, self.settings.text_scale, window, cx);
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

    pub fn set_language_mode(
        &mut self,
        mode: LanguageMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.settings.language_mode == mode {
            return;
        }
        self.settings.language_mode = mode;
        let lang = match mode {
            LanguageMode::ZhCn => Language::ZhCn,
            LanguageMode::EnUs => Language::EnUs,
            LanguageMode::System => {
                platform::system_language().unwrap_or_else(|| self.settings.effective_language())
            }
        };
        self.lang = lang;
        self.settings.language = lang;
        let _ = core_settings::save(&self.paths, &self.settings);
        self.search_input.update(cx, |input, cx| {
            input.set_placeholder(i18n::tr(lang, "library.search_placeholder"), window, cx);
        });
        cx.notify();
    }

    pub fn set_reduce_motion_preference(
        &mut self,
        preference: ReduceMotion,
        cx: &mut Context<Self>,
    ) {
        if self.settings.reduce_motion == preference {
            return;
        }
        self.settings.reduce_motion = preference;
        self.reduce_motion = match preference {
            ReduceMotion::On => true,
            ReduceMotion::Off => false,
            ReduceMotion::System => platform::system_reduce_motion(),
        };
        cx.set_reduce_motion(self.reduce_motion);
        let _ = core_settings::save(&self.paths, &self.settings);
        cx.notify();
    }

    // -----------------------------------------------------------------
    // Instance commands
    // -----------------------------------------------------------------

    pub fn open_create_dialog(&mut self, cx: &mut Context<Self>) {
        self.dialog = Some(Dialog::CreateInstance);
        self.dialog_error = None;
        self.dialog_field_error = None;
        self.dialog_busy = false;
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
        if self.dialog_busy {
            return;
        }
        self.dialog_field_error = None;
        self.dialog_error = None;
        let name = self.create_name.read(cx).value().to_string();
        let loader_name = self.loader_label(self.create_loader);
        let name = if name.trim().is_empty() {
            format!("{loader_name} {}", self.create_version)
        } else {
            name.trim().to_string()
        };
        if name.is_empty() || name.chars().count() > 64 {
            self.dialog_field_error = Some("create-name");
            self.dialog_error = Some(self.t("instances.name_invalid").to_string());
            return;
        }
        if self.create_version.is_empty() {
            self.dialog_field_error = None;
            self.dialog_error = Some(self.t("instances.unknown_version").to_string());
            return;
        }
        let uses_installer = matches!(self.create_loader, LoaderType::Forge | LoaderType::Neoforge);
        if uses_installer && self.create_loader_version.is_none() {
            self.dialog_field_error = None;
            self.dialog_error = Some(self.t("instances.loader_version_required").to_string());
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
        self.dialog_busy = true;
        cx.notify();
        spawn_task(
            cx,
            move || instances::create(&paths, new),
            |this, result, cx| {
                this.dialog_busy = false;
                match result {
                    Ok(record) => {
                        this.dialog = None;
                        this.dialog_error = None;
                        this.dialog_field_error = None;
                        this.open_instance(record.id.clone(), cx);
                        this.refresh_instances(cx);
                        this.start_install(record.id, cx);
                    }
                    Err(error) => {
                        this.dialog_error = Some(this.error_text(&error));
                        this.show_error(error, cx);
                    }
                }
            },
        );
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
        if self.dialog_busy {
            return;
        }
        let paths = self.paths.clone();
        let target = id.clone();
        self.dialog_busy = true;
        cx.notify();
        spawn_task(
            cx,
            move || instances::delete(&paths, &id, false),
            move |this, result, cx| {
                this.dialog_busy = false;
                match result {
                    Ok(()) => {
                        this.dialog = None;
                        if this.selected_instance.as_deref() == Some(target.as_str()) {
                            this.selected_instance = None;
                        }
                        this.refresh_instances(cx);
                        this.show_toast(this.t("toast.removed"), false, cx);
                    }
                    Err(error) => this.show_error(error, cx),
                }
            },
        );
    }

    pub fn open_folder(&mut self, id: &str, cx: &mut Context<Self>) {
        let path = self.paths.game_dir(id).unwrap_or_default();
        let display = path.clone();
        spawn_task(
            cx,
            move || -> crate::core::Result<PathBuf> {
                std::fs::create_dir_all(&path)?;
                Ok(path)
            },
            move |this, result, cx| match result {
                Ok(path) => {
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
                    let _ = &path;
                    this.show_toast(this.t("toast.folder_opened"), false, cx);
                }
                Err(error) => {
                    let _ = &display;
                    this.show_error(error, cx);
                }
            },
        );
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
        if self.dialog_busy {
            return;
        }
        self.dialog_field_error = None;
        self.dialog_error = None;
        let name = self.account_name.read(cx).value().to_string();
        if !valid_offline_name(&name) {
            self.dialog_field_error = Some("offline-name");
            self.dialog_error = Some(self.t("accounts.offline_hint").to_string());
            return;
        }
        let paths = self.paths.clone();
        self.dialog_busy = true;
        cx.notify();
        spawn_task(
            cx,
            move || accounts::add_offline(&paths, &name),
            |this, result, cx| {
                this.dialog_busy = false;
                match result {
                    Ok(items) => {
                        this.accounts = items;
                        this.dialog = None;
                        this.dialog_error = None;
                        this.show_toast(this.t("toast.account_added"), false, cx);
                    }
                    Err(error) => {
                        this.dialog_error = Some(this.error_text(&error));
                        this.show_error(error, cx);
                    }
                }
            },
        );
        let _ = window;
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
        if self.dialog_busy {
            return;
        }
        self.dialog_field_error = None;
        self.dialog_error = None;
        let server = self.authlib_server.read(cx).value().to_string();
        let user = self.authlib_user.read(cx).value().to_string();
        let password = self.authlib_password.read(cx).value().to_string();
        if server.trim().is_empty() {
            self.dialog_field_error = Some("authlib-server");
            self.dialog_error = Some(self.t("accounts.server_url_required").to_string());
            return;
        }
        if user.trim().is_empty() {
            self.dialog_field_error = Some("authlib-user");
            self.dialog_error = Some(self.t("accounts.email_required").to_string());
            return;
        }
        if password.is_empty() {
            self.dialog_field_error = Some("authlib-password");
            self.dialog_error = Some(self.t("accounts.password_required").to_string());
            return;
        }
        let paths = self.paths.clone();
        self.dialog_busy = true;
        cx.notify();
        spawn_task(
            cx,
            move || accounts::add_authlib(&paths, &server, &user, &password),
            |this, result, cx| {
                this.dialog_busy = false;
                match result {
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
                }
            },
        );
    }

    pub fn set_active_account(&mut self, id: String, cx: &mut Context<Self>) {
        let paths = self.paths.clone();
        let target = id.clone();
        let is_littleskin = self.accounts.iter().any(|account| {
            account.id == id && account.kind == crate::core::accounts::AccountKind::Littleskin
        });
        spawn_task(
            cx,
            move || {
                // A LittleSkin session may be stale; refreshing updates the
                // access token, profile and skin URL before it is made active.
                if is_littleskin {
                    accounts::refresh_authlib(&paths, &target)?;
                }
                accounts::set_active(&paths, &target)
            },
            |this, result, cx| match result {
                Ok(items) => {
                    this.accounts = items;
                    this.refresh_skins(cx);
                }
                Err(error) => this.show_error(error, cx),
            },
        );
    }

    pub fn prompt_remove_account(&mut self, id: String, name: String, cx: &mut Context<Self>) {
        self.dialog = Some(Dialog::RemoveAccount { id, name });
        self.dialog_error = None;
        self.dialog_busy = false;
        cx.notify();
    }

    pub fn confirm_remove_account(&mut self, id: String, cx: &mut Context<Self>) {
        if self.dialog_busy {
            return;
        }
        let paths = self.paths.clone();
        self.dialog_busy = true;
        cx.notify();
        spawn_task(
            cx,
            move || accounts::remove(&paths, &id),
            |this, result, cx| {
                this.dialog_busy = false;
                match result {
                    Ok(items) => {
                        this.accounts = items;
                        this.dialog = None;
                        this.show_toast(this.t("toast.removed"), false, cx);
                    }
                    Err(error) => this.show_error(error, cx),
                }
            },
        );
    }

    pub fn add_server(&mut self, cx: &mut Context<Self>) {
        if self.dialog_busy {
            return;
        }
        self.dialog_field_error = None;
        self.dialog_error = None;
        let name = self.server_name.read(cx).value().to_string();
        let address = self.server_address.read(cx).value().to_string();
        if address.trim().is_empty() {
            self.dialog_field_error = Some("server-address");
            self.dialog_error = Some(self.t("servers.address_required").to_string());
            return;
        }
        let paths = self.paths.clone();
        self.dialog_busy = true;
        cx.notify();
        spawn_task(
            cx,
            move || servers::add(&paths, &name, &address, None),
            |this, result, cx| {
                this.dialog_busy = false;
                match result {
                    Ok(_) => {
                        this.dialog = None;
                        this.dialog_error = None;
                        this.refresh_servers(cx);
                        this.show_toast(this.t("toast.server_added"), false, cx);
                    }
                    Err(error) => {
                        this.dialog_error = Some(this.error_text(&error));
                        this.show_error(error, cx);
                    }
                }
            },
        );
    }

    pub fn prompt_remove_server(&mut self, id: String, name: String, cx: &mut Context<Self>) {
        self.dialog = Some(Dialog::DeleteServer { id, name });
        self.dialog_error = None;
        self.dialog_busy = false;
        cx.notify();
    }

    pub fn confirm_remove_server(&mut self, id: String, cx: &mut Context<Self>) {
        if self.dialog_busy {
            return;
        }
        let paths = self.paths.clone();
        self.dialog_busy = true;
        cx.notify();
        spawn_task(
            cx,
            move || servers::remove(&paths, &id),
            |this, result, cx| {
                this.dialog_busy = false;
                match result {
                    Ok(items) => {
                        this.servers = items;
                        this.statuses.clear();
                        this.dialog = None;
                        this.show_toast(this.t("toast.removed"), false, cx);
                    }
                    Err(error) => this.show_error(error, cx),
                }
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

    /// Search now (Enter, a filter change or a retry).
    pub fn search_library(&mut self, cx: &mut Context<Self>) {
        self.search_generation = self.search_generation.wrapping_add(1);
        let generation = self.search_generation;
        self.run_library_search(generation, cx);
    }

    /// Refine while typing: wait 250 ms after the last change. Any newer
    /// change invalidates the pending search, and a stale response is ignored.
    pub fn schedule_library_search(&mut self, cx: &mut Context<Self>) {
        self.search_generation = self.search_generation.wrapping_add(1);
        let generation = self.search_generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(250))
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.search_generation == generation {
                    this.run_library_search(generation, cx);
                }
            });
        })
        .detach();
    }

    fn run_library_search(&mut self, generation: u64, cx: &mut Context<Self>) {
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
            move |this, result, cx| {
                if this.search_generation != generation {
                    return;
                }
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
                        this.show_error_with(
                            error,
                            Some(("common.retry", ToastAction::Library)),
                            cx,
                        );
                    }
                }
            },
        );
    }

    pub fn clear_library_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.search_library(cx);
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
                    this.t("library.install_done_count")
                        .replace("{count}", &files.len().to_string()),
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
                        this.t("backups.export_done_name")
                            .replace("{name}", &filename),
                        false,
                        cx,
                    );
                }
                Err(error) => this.show_error(error, cx),
            },
        );
    }

    pub fn import_instance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.dialog_busy {
            return;
        }
        self.dialog_field_error = None;
        let raw = self
            .import_path
            .read(cx)
            .value()
            .trim()
            .trim_matches('"')
            .to_string();
        if raw.is_empty() {
            self.dialog_field_error = Some("import-path");
            self.dialog_error = Some(self.t("backups.import_path_required").to_string());
            return;
        }
        self.import_path
            .update(cx, |input, cx| input.set_value("", window, cx));
        let paths = self.paths.clone();
        let source = PathBuf::from(raw);
        self.dialog_busy = true;
        cx.notify();
        spawn_task(
            cx,
            move || export::import(&paths, &source, |_| {}),
            |this, result, cx| {
                this.dialog_busy = false;
                match result {
                    Ok(record) => {
                        this.dialog = None;
                        this.refresh_instances(cx);
                        this.open_instance(record.id, cx);
                        this.show_toast(this.t("backups.imported_toast"), false, cx);
                    }
                    Err(error) => this.show_error(error, cx),
                }
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
        let java_settings = self.settings.java.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        cx.notify();
        spawn_task(
            cx,
            move || {
                modpack::install_modrinth_with_java(
                    &paths,
                    &project_id,
                    None,
                    source,
                    &java_settings,
                    cancel,
                    |_| {},
                )
            },
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
        self.dialog_field_error = None;
        let value = self.modpack_path.read(cx).value().to_string();
        let value = value.trim().to_string();
        if value.is_empty() {
            self.dialog_field_error = Some("modpack-path");
            self.dialog_error = Some(self.t("library.import_empty").to_string());
            return;
        }
        let paths = self.paths.clone();
        let source = self.settings.download_source;
        let java_settings = self.settings.java.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let path = PathBuf::from(value);
        self.loading.modpack = true;
        self.dialog = None;
        cx.notify();
        spawn_task(
            cx,
            move || {
                modpack::import_local_with_java(
                    &paths,
                    &path,
                    source,
                    &java_settings,
                    cancel,
                    |_| {},
                )
            },
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
        if self.dialog_busy {
            return;
        }
        let paths = self.paths.clone();
        self.dialog_busy = true;
        cx.notify();
        spawn_task(
            cx,
            move || modrinth::remove_file(&paths, &instance_id, &project_type, &filename),
            move |this, result, cx| {
                this.dialog_busy = false;
                match result {
                    Ok(()) => {
                        this.dialog = None;
                        this.show_toast(this.t("toast.removed"), false, cx);
                    }
                    Err(error) => this.show_error(error, cx),
                }
            },
        );
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
        move |mut value: InstallProgress| {
            let mut slot = progress.lock().unwrap();
            // Loader output arrives between progress events; keep the most
            // recent line rather than clearing it on the next percentage tick.
            if value.message.is_none() {
                value.message = slot.message.clone();
            }
            *slot = value;
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
        let output_progress = Arc::clone(&progress);
        let on_output = move |line: &str| {
            let mut value = output_progress.lock().unwrap();
            value.phase = InstallPhase::Loader;
            value.message = Some(line.to_string());
        };
        let loader_id = install::install_loader_with_output(
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
            on_output,
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
        let title = self.window_title_text();
        if self.window_title != title {
            window.set_window_title(&title);
            self.window_title = title;
        }
        let class = WindowWidthClass::of(f32::from(window.viewport_size().width));
        let compact = class == WindowWidthClass::Compact;
        let expanded = matches!(
            class,
            WindowWidthClass::Expanded | WindowWidthClass::Large | WindowWidthClass::ExtraLarge
        );
        let detail = page == Page::Instances
            && self
                .selected_instance
                .as_ref()
                .is_some_and(|id| self.instances.iter().any(|item| &item.record.id == id));

        let content = pages::render(self, window, cx);
        let content: AnyElement = if self.reduce_motion {
            content
        } else {
            let key = SharedString::from(format!(
                "page-fade-{}-{}",
                page.id(),
                self.selected_instance.as_deref().unwrap_or("list")
            ));
            div()
                .child(content)
                .with_animation(
                    key,
                    Animation::new(Duration::from_millis(140)),
                    |element, delta| element.opacity(0.55 + 0.45 * delta),
                )
                .into_any_element()
        };

        let activity_active = self
            .running
            .values()
            .any(|game| matches!(game.status, GameStatus::Starting | GameStatus::Running))
            || self.installs.values().any(|task| !task.finished);

        let activity_button = |cx: &mut Context<Self>| {
            let button = IconButton::new(
                "open-activity",
                material_icon("monitoring", 24.0),
                self.t("nav.activity"),
            )
            .on_click(cx.listener(|this, _, _, cx| this.set_page(Page::Activity, cx)));
            if activity_active {
                Badge::small("activity-badge")
                    .color(BadgeColor::Info)
                    .visible(true)
                    .anchor(button)
                    .into_any_element()
            } else {
                button.into_any_element()
            }
        };

        let mut actions: Vec<AnyElement> = Vec::new();
        if compact {
            let items = [
                MaterialMenuItem::new(self.t("nav.activity"))
                    .leading(material_icon("monitoring", 20.0))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.menu_open = None;
                        this.set_page(Page::Activity, cx);
                    })),
                MaterialMenuItem::new(self.t("nav.settings"))
                    .leading(material_icon("settings", 20.0))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.menu_open = None;
                        this.set_page(Page::Settings, cx);
                    })),
            ];
            let trigger = MaterialButton::new("topbar-more")
                .variant(ButtonVariant::Text)
                .button_size(ButtonSize::Small)
                .icon(material_icon("more_vert", 20.0))
                .child(self.t("topbar.more"))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.menu_open = if this.menu_open.as_deref() == Some("topbar") {
                        None
                    } else {
                        Some("topbar".to_string())
                    };
                    cx.notify();
                }));
            let trigger = if activity_active {
                Badge::small("topbar-more-badge")
                    .color(BadgeColor::Info)
                    .visible(true)
                    .anchor(trigger)
                    .into_any_element()
            } else {
                trigger.into_any_element()
            };
            let menu = MaterialMenu::new("topbar-more-menu").items(items);
            actions.push(
                MenuPopup::new("topbar-more-popup", trigger, menu)
                    .open(self.menu_open.as_deref() == Some("topbar"))
                    .on_open_change(cx.listener(|this, open: &bool, _, cx| {
                        this.menu_open = open.then(|| "topbar".to_string());
                        cx.notify();
                    }))
                    .into_any_element(),
            );
        } else {
            actions.push(activity_button(cx));
            actions.push(
                IconButton::new(
                    "open-settings",
                    material_icon("settings", 24.0),
                    self.t("nav.settings"),
                )
                .on_click(cx.listener(|this, _, _, cx| this.set_page(Page::Settings, cx)))
                .into_any_element(),
            );
        }

        let navigation_icon = if detail {
            IconButton::new(
                "detail-back",
                material_icon("arrow_back", 24.0),
                self.t("common.back"),
            )
            .on_click(cx.listener(|this, _, _, cx| this.back_to_instances(cx)))
            .into_any_element()
        } else {
            logo().into_any_element()
        };
        let top_bar = TopAppBar::new("top-bar", TopAppBarVariant::Small)
            .container(ColorRole::SURFACE)
            .navigation_icon(navigation_icon)
            .actions(actions);

        let rail_width = if expanded { 220. } else { 96. };
        let main = if compact {
            div().w_full().h_full().min_w_0().child(content)
        } else {
            div()
                .relative()
                .w_full()
                .h_full()
                .min_h_0()
                .min_w_0()
                .child(
                    div()
                        .absolute()
                        .left(px(0.))
                        .top(px(0.))
                        .w(px(rail_width))
                        .h_full()
                        .min_w_0()
                        .overflow_hidden()
                        .child(
                            NavigationRail::new("main-navigation", self.t("nav.landmark"))
                                .items(nav_items)
                                .selected(selected)
                                .expanded(expanded)
                                .container(ColorRole::SURFACE)
                                .expanded_width(rail_width)
                                .on_change(cx.listener(|this, id: &SharedString, _, cx| {
                                    this.set_page(Page::from_id(id.as_ref()), cx);
                                })),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(rail_width))
                        .right(px(0.))
                        .top(px(0.))
                        .bottom(px(0.))
                        .min_w_0()
                        .child(content),
                )
        };

        let compact_bar = compact.then(|| {
            div()
                .w_full()
                .min_w_0()
                .flex_shrink_0()
                .overflow_hidden()
                .child(
                    div().w_full().min_w_0().child(
                        NavigationBar::new("main-navigation", self.t("nav.landmark"))
                            .items(navigation_items(lang))
                            .selected(selected)
                            .layout(NavItemLayout::Vertical)
                            .on_change(cx.listener(|this, id: &SharedString, _, cx| {
                                this.set_page(Page::from_id(id.as_ref()), cx);
                            })),
                    ),
                )
        });

        let snackbar = self.toasts.front().cloned().map(|toast| {
            let duration = if toast.error {
                SnackbarDuration::Indefinite
            } else {
                SnackbarDuration::Short
            };
            let mut bar = Snackbar::new("toast", toast.text.clone())
                .open(true)
                .duration(duration)
                .position(SnackbarPosition::Center)
                .close(
                    i18n::tr(lang, "common.close"),
                    cx.listener(|this, _, _, cx| this.dismiss_toast(cx)),
                );
            if let Some((label, _)) = toast.action {
                bar = bar.action(
                    i18n::tr(lang, label),
                    cx.listener(|this, _, _, cx| this.run_toast_action(cx)),
                );
            }
            div()
                .absolute()
                .left(px(0.))
                .right(px(0.))
                .bottom(px(if compact { 96. } else { 24. }))
                .px(px(24.))
                .flex()
                .justify_center()
                .child(div().w_full().max_w(px(672.)).child(bar))
                .into_any_element()
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
            .child(div().w_full().min_w_0().child(top_bar))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .w_full()
                    .overflow_hidden()
                    .child(main),
            )
            .children(compact_bar)
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
        .flex()
        .items_center()
        .justify_center()
        .child(
            svg()
                .path(crate::brand_assets::BRAND_MARK)
                .w(px(24.))
                .h(px(24.))
                .text_color(color(ColorRole::PRIMARY)),
        )
}

/// The offline account rules, shared by the dialog and its inline error.
fn valid_offline_name(name: &str) -> bool {
    let name = name.trim();
    (3..=16).contains(&name.chars().count())
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}
