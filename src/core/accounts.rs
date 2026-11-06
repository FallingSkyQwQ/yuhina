use std::fs;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use base64::Engine;
use chrono::{DateTime, Utc};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::http;
use super::paths::Paths;
use super::storage::{read_json, write_json};
use super::{ErrorCode, Result, YuhinaError};

const MICROSOFT_CLIENT_ID: &str = "5cde052b-0132-4f10-a523-186d372365ff";
const MICROSOFT_SCOPE: &str = "XboxLive.signin offline_access";
const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBOX_USER_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MINECRAFT_LOGIN_URL: &str =
    "https://api.minecraftservices.com/authentication/login_with_xbox";
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const MINECRAFT_ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";
const KEYRING_SERVICE: &str = "com.yuhina.accounts";
const REFRESH_WINDOW_MS: i64 = 5 * 60 * 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountKind {
    Microsoft,
    Offline,
    Littleskin,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRecord {
    pub id: String,
    pub kind: AccountKind,
    pub name: String,
    pub uuid: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub last_used_at: String,
    #[serde(default)]
    pub owns_minecraft: Option<bool>,
    #[serde(default)]
    pub token_expires_at: Option<String>,
    #[serde(default)]
    pub authlib_url: Option<String>,
    #[serde(default)]
    pub server_name: Option<String>,
    #[serde(default)]
    pub skin_url: Option<String>,
    #[serde(default)]
    pub secrets: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountsFile {
    pub version: u32,
    pub accounts: Vec<AccountRecord>,
    pub active_account_id: Option<String>,
}

impl Default for AccountsFile {
    fn default() -> Self {
        Self {
            version: 1,
            accounts: Vec::new(),
            active_account_id: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    pub id: String,
    pub kind: AccountKind,
    pub name: String,
    pub uuid: String,
    pub active: bool,
    pub created_at: String,
    pub last_used_at: String,
    pub authlib_url: Option<String>,
    pub server_name: Option<String>,
    pub owns_minecraft: Option<bool>,
    pub token_expires_at: Option<String>,
    pub skin_url: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountSecrets {
    pub microsoft_refresh_token: Option<String>,
    pub minecraft_access_token: Option<String>,
    pub authlib_access_token: Option<String>,
    pub authlib_client_token: Option<String>,
}

#[derive(Clone, Debug)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Clone, Debug)]
pub struct MicrosoftSession {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: String,
    pub uuid: String,
    pub name: String,
    pub skin_url: Option<String>,
    pub owns_minecraft: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct LaunchCredentials {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub user_type: String,
    pub authlib: Option<AuthlibCredential>,
}

#[derive(Clone, Debug)]
pub struct AuthlibCredential {
    pub jar_path: PathBuf,
    pub url: String,
    pub prefetched: String,
}

fn accounts_path(paths: &Paths) -> PathBuf {
    paths.accounts_file()
}

fn load_file(paths: &Paths) -> Result<AccountsFile> {
    Ok(read_json::<AccountsFile>(&accounts_path(paths))?.unwrap_or_default())
}

fn save_file(paths: &Paths, file: &AccountsFile) -> Result<()> {
    write_json(&accounts_path(paths), file)
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn summarize(file: &AccountsFile) -> Vec<AccountSummary> {
    file.accounts
        .iter()
        .map(|account| AccountSummary {
            id: account.id.clone(),
            kind: account.kind,
            name: account.name.clone(),
            uuid: account.uuid.clone(),
            active: file.active_account_id.as_deref() == Some(account.id.as_str()),
            created_at: account.created_at.clone(),
            last_used_at: account.last_used_at.clone(),
            authlib_url: account.authlib_url.clone(),
            server_name: account.server_name.clone(),
            owns_minecraft: account.owns_minecraft,
            token_expires_at: account.token_expires_at.clone(),
            skin_url: account.skin_url.clone(),
        })
        .collect()
}

pub fn list(paths: &Paths) -> Result<Vec<AccountSummary>> {
    Ok(summarize(&load_file(paths)?))
}

pub fn active_account(paths: &Paths) -> Result<Option<AccountRecord>> {
    let file = load_file(paths)?;
    Ok(file
        .accounts
        .iter()
        .find(|account| Some(&account.id) == file.active_account_id.as_ref())
        .or_else(|| file.accounts.first())
        .cloned())
}

pub fn set_active(paths: &Paths, id: &str) -> Result<Vec<AccountSummary>> {
    let mut file = load_file(paths)?;
    let account = file
        .accounts
        .iter_mut()
        .find(|account| account.id == id)
        .ok_or_else(|| YuhinaError::new(ErrorCode::NotFound, "That account no longer exists."))?;
    account.last_used_at = now();
    file.active_account_id = Some(id.to_string());
    save_file(paths, &file)?;
    Ok(summarize(&file))
}

pub fn remove(paths: &Paths, id: &str) -> Result<Vec<AccountSummary>> {
    let mut file = load_file(paths)?;
    file.accounts.retain(|account| account.id != id);
    if file.active_account_id.as_deref() == Some(id) {
        file.active_account_id = file.accounts.first().map(|account| account.id.clone());
    }
    let _ = delete_secrets(id);
    save_file(paths, &file)?;
    Ok(summarize(&file))
}

pub fn is_valid_offline_name(name: &str) -> bool {
    let name = name.trim();
    (3..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn offline_uuid(name: &str) -> String {
    let digest = Md5::digest(format!("OfflinePlayer:{}", name.trim()).as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest);
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex = hex::encode(bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

pub fn format_uuid(compact: &str) -> String {
    let hex = compact.replace('-', "").to_ascii_lowercase();
    if hex.len() != 32 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return compact.to_string();
    }
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

pub fn add_offline(paths: &Paths, name: &str) -> Result<Vec<AccountSummary>> {
    let name = name.trim();
    if !is_valid_offline_name(name) {
        return Err(YuhinaError::new(
            ErrorCode::InvalidInput,
            "Use 3–16 letters, numbers or underscores for an offline name.",
        ));
    }
    let mut file = load_file(paths)?;
    let uuid = offline_uuid(name);
    let timestamp = now();
    let index = file
        .accounts
        .iter()
        .position(|account| account.kind == AccountKind::Offline && account.uuid == uuid);
    let id = match index {
        Some(index) => {
            file.accounts[index].name = name.to_string();
            file.accounts[index].last_used_at = timestamp;
            file.accounts[index].id.clone()
        }
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            file.accounts.push(AccountRecord {
                id: id.clone(),
                kind: AccountKind::Offline,
                name: name.to_string(),
                uuid,
                created_at: timestamp.clone(),
                last_used_at: timestamp,
                owns_minecraft: None,
                token_expires_at: None,
                authlib_url: None,
                server_name: None,
                skin_url: None,
                secrets: None,
            });
            id
        }
    };
    file.active_account_id = Some(id);
    save_file(paths, &file)?;
    Ok(summarize(&file))
}

// ---------------------------------------------------------------------------
// Secret storage
// ---------------------------------------------------------------------------

fn secret_entry(id: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, id).map_err(|error| {
        YuhinaError::new(
            ErrorCode::SecretStorage,
            format!("The system secret store is unavailable: {error}"),
        )
    })
}

fn save_secrets(id: &str, secrets: &AccountSecrets) -> Result<()> {
    let payload = serde_json::to_string(secrets)?;
    secret_entry(id)?.set_password(&payload).map_err(|error| {
        YuhinaError::new(
            ErrorCode::SecretStorage,
            format!("The account token could not be encrypted: {error}"),
        )
    })
}

fn load_secrets(id: &str) -> Result<Option<AccountSecrets>> {
    let entry = secret_entry(id)?;
    match entry.get_password() {
        Ok(payload) => Ok(serde_json::from_str(&payload).ok()),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(YuhinaError::new(
            ErrorCode::SecretStorage,
            format!("The account token could not be read: {error}"),
        )),
    }
}

fn delete_secrets(id: &str) -> Result<()> {
    match secret_entry(id) {
        Ok(entry) => {
            let _ = entry.delete_credential();
            Ok(())
        }
        Err(_) => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// Microsoft device code
// ---------------------------------------------------------------------------

pub fn start_device_code() -> Result<DeviceCode> {
    let response = http::post_form(
        DEVICE_CODE_URL,
        &[
            ("client_id", MICROSOFT_CLIENT_ID),
            ("scope", MICROSOFT_SCOPE),
        ],
    )?;
    if !response.is_success() {
        return Err(YuhinaError::new(
            ErrorCode::Auth,
            "Microsoft sign-in could not be started.",
        ));
    }
    let value: Value = response.json()?;
    let device_code = value["device_code"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let user_code = value["user_code"].as_str().unwrap_or_default().to_string();
    let verification_uri = value["verification_uri"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    if device_code.is_empty() || user_code.is_empty() || verification_uri.is_empty() {
        return Err(YuhinaError::new(
            ErrorCode::Auth,
            "Microsoft returned an unexpected sign-in response.",
        ));
    }
    Ok(DeviceCode {
        device_code,
        user_code,
        verification_uri,
        expires_in: value["expires_in"].as_u64().unwrap_or(900),
        interval: value["interval"].as_u64().unwrap_or(5),
    })
}

/// Polls the device-code endpoint until the user completes sign-in, the code
/// expires, or `on_status` asks to stop (the callback cannot stop it yet; the
/// UI cancels by dropping the task).
pub fn complete_microsoft_login<F>(
    paths: &Paths,
    code: &DeviceCode,
    on_status: F,
) -> Result<AccountSummary>
where
    F: Fn(&str),
{
    let deadline = std::time::Instant::now() + Duration::from_secs(code.expires_in);
    let mut interval = code.interval.max(1);
    loop {
        if std::time::Instant::now() >= deadline {
            return Err(YuhinaError::new(
                ErrorCode::AuthExpired,
                "The Microsoft sign-in code expired.",
            ));
        }
        on_status("waiting");
        sleep(Duration::from_secs(interval));
        let response = http::post_form(
            TOKEN_URL,
            &[
                ("client_id", MICROSOFT_CLIENT_ID),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", &code.device_code),
            ],
        )?;
        if !response.is_success() {
            let value: Value = response.json().unwrap_or_default();
            match value["error"].as_str().unwrap_or_default() {
                "authorization_pending" => continue,
                "slow_down" => {
                    interval += 5;
                    continue;
                }
                "expired_token" => {
                    return Err(YuhinaError::new(
                        ErrorCode::AuthExpired,
                        "The Microsoft sign-in code expired.",
                    ));
                }
                "authorization_declined" | "access_denied" => {
                    return Err(YuhinaError::new(
                        ErrorCode::Auth,
                        "Microsoft sign-in was declined.",
                    ));
                }
                _ => {
                    return Err(YuhinaError::new(
                        ErrorCode::Auth,
                        "Microsoft sign-in failed.",
                    ));
                }
            }
        }
        let value: Value = response.json()?;
        let access_token = value["access_token"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let refresh_token = value["refresh_token"].as_str().map(str::to_string);
        on_status("exchanging");
        let session = complete_xbox_login(&access_token, refresh_token)?;
        return persist_microsoft_session(paths, session);
    }
}

pub fn refresh_microsoft(paths: &Paths, account_id: &str) -> Result<AccountSummary> {
    let mut file = load_file(paths)?;
    let index = file
        .accounts
        .iter()
        .position(|account| account.id == account_id)
        .ok_or_else(|| YuhinaError::new(ErrorCode::NotFound, "That account no longer exists."))?;
    let secrets = load_secrets(account_id)?.ok_or_else(|| {
        YuhinaError::new(
            ErrorCode::AuthExpired,
            "The Microsoft session has expired. Sign in again.",
        )
    })?;
    let refresh = secrets.microsoft_refresh_token.ok_or_else(|| {
        YuhinaError::new(
            ErrorCode::AuthExpired,
            "The Microsoft session has expired. Sign in again.",
        )
    })?;
    let response = http::post_form(
        TOKEN_URL,
        &[
            ("client_id", MICROSOFT_CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh),
            ("scope", MICROSOFT_SCOPE),
        ],
    )?;
    if !response.is_success() {
        return Err(YuhinaError::new(
            ErrorCode::AuthExpired,
            "The Microsoft session has expired. Sign in again.",
        ));
    }
    let value: Value = response.json()?;
    let access_token = value["access_token"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let refresh_token = value["refresh_token"]
        .as_str()
        .map(str::to_string)
        .or(Some(refresh));
    let session = complete_xbox_login(&access_token, refresh_token)?;
    let mut account = file.accounts[index].clone();
    apply_microsoft_session(&mut account, &session);
    let secrets = AccountSecrets {
        microsoft_refresh_token: session.refresh_token.clone(),
        minecraft_access_token: Some(session.access_token.clone()),
        ..load_secrets(account_id)?.unwrap_or_default()
    };
    save_secrets(account_id, &secrets)?;
    account.secrets = Some(json!({ "persisted": true }));
    file.accounts[index] = account;
    save_file(paths, &file)?;
    Ok(summarize(&file)
        .into_iter()
        .find(|summary| summary.id == account_id)
        .expect("the account was just saved"))
}

fn complete_xbox_login(
    microsoft_access_token: &str,
    refresh_token: Option<String>,
) -> Result<MicrosoftSession> {
    let xbox = http::post_json(
        XBOX_USER_URL,
        &json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={microsoft_access_token}"),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }),
    )?;
    if !xbox.is_success() {
        return Err(YuhinaError::new(
            ErrorCode::Auth,
            "Xbox Live rejected the Microsoft token.",
        ));
    }
    let xbox: Value = xbox.json()?;
    let xbox_token = xbox["Token"].as_str().unwrap_or_default();
    let uhs = xbox["DisplayClaims"]["xui"]
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item["uhs"].as_str())
        .unwrap_or_default();
    if xbox_token.is_empty() || uhs.is_empty() {
        return Err(YuhinaError::new(
            ErrorCode::Auth,
            "Xbox Live returned an unexpected response.",
        ));
    }

    let xsts = http::post_json(
        XSTS_URL,
        &json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbox_token],
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }),
    )?;
    if !xsts.is_success() {
        let value: Value = xsts.json().unwrap_or_default();
        return Err(map_xsts_error(value["XErr"].as_i64()));
    }
    let xsts: Value = xsts.json()?;
    let xsts_token = xsts["Token"].as_str().unwrap_or_default();
    if xsts_token.is_empty() {
        return Err(YuhinaError::new(
            ErrorCode::Auth,
            "Xbox Live returned an unexpected response.",
        ));
    }

    let login = http::post_json(
        MINECRAFT_LOGIN_URL,
        &json!({ "identityToken": format!("XBL3.0 x={uhs};{xsts_token}") }),
    )?;
    if !login.is_success() {
        return Err(YuhinaError::new(
            ErrorCode::Auth,
            "Minecraft authentication failed.",
        ));
    }
    let login: Value = login.json()?;
    let access_token = login["access_token"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    if access_token.is_empty() {
        return Err(YuhinaError::new(
            ErrorCode::Auth,
            "Minecraft authentication failed.",
        ));
    }

    let profile = http::get_json_with_headers(
        MINECRAFT_PROFILE_URL,
        &[("Authorization", &format!("Bearer {access_token}"))],
    )?;
    if profile.status == 404 {
        return Err(YuhinaError::new(
            ErrorCode::AuthMissingProfile,
            "This Microsoft account has no Minecraft profile.",
        ));
    }
    if !profile.is_success() {
        return Err(YuhinaError::new(
            ErrorCode::Auth,
            "The Minecraft profile could not be loaded.",
        ));
    }
    let profile: Value = profile.json()?;
    let uuid = format_uuid(profile["id"].as_str().unwrap_or_default());
    let name = profile["name"].as_str().unwrap_or_default().to_string();
    if uuid.is_empty() || name.is_empty() {
        return Err(YuhinaError::new(
            ErrorCode::Auth,
            "The Minecraft profile could not be loaded.",
        ));
    }
    let skin_url = profile["skins"]
        .as_array()
        .and_then(|skins| {
            skins
                .iter()
                .find(|skin| skin["state"].as_str() == Some("ACTIVE"))
        })
        .and_then(|skin| skin["url"].as_str())
        .map(str::to_string);

    let owns_minecraft = http::get_json_with_headers(
        MINECRAFT_ENTITLEMENTS_URL,
        &[("Authorization", &format!("Bearer {access_token}"))],
    )
    .ok()
    .filter(|response| response.is_success())
    .and_then(|response| response.json().ok())
    .and_then(|value: Value| value["items"].as_array().map(|items| !items.is_empty()));

    let expires_at = Utc::now()
        .checked_add_signed(chrono::Duration::seconds(
            login["expires_in"].as_i64().unwrap_or(86_400),
        ))
        .unwrap_or_else(Utc::now)
        .to_rfc3339();

    Ok(MicrosoftSession {
        access_token,
        refresh_token,
        expires_at,
        uuid,
        name,
        skin_url,
        owns_minecraft,
    })
}

fn map_xsts_error(xerr: Option<i64>) -> YuhinaError {
    let message = match xerr {
        Some(2148916233) => "This Microsoft account has no Xbox profile yet.",
        Some(2148916235) => "Xbox Live is not available in this account's region.",
        Some(2148916236) | Some(2148916237) => {
            "This account needs adult verification before Xbox Live can be used."
        }
        Some(2148916238) => "A child account must join a Microsoft family first.",
        _ => "Xbox Live could not authorize this account.",
    };
    YuhinaError::new(ErrorCode::Auth, message)
}

fn apply_microsoft_session(account: &mut AccountRecord, session: &MicrosoftSession) {
    account.kind = AccountKind::Microsoft;
    account.name = session.name.clone();
    account.uuid = session.uuid.clone();
    account.token_expires_at = Some(session.expires_at.clone());
    account.owns_minecraft = session.owns_minecraft;
    account.skin_url = session.skin_url.clone();
}

fn persist_microsoft_session(paths: &Paths, session: MicrosoftSession) -> Result<AccountSummary> {
    let mut file = load_file(paths)?;
    let timestamp = now();
    let index = file
        .accounts
        .iter()
        .position(|account| account.kind == AccountKind::Microsoft && account.uuid == session.uuid);
    let (id, created_at) = match index {
        Some(index) => (
            file.accounts[index].id.clone(),
            file.accounts[index].created_at.clone(),
        ),
        None => (uuid::Uuid::new_v4().to_string(), timestamp.clone()),
    };

    save_secrets(
        &id,
        &AccountSecrets {
            microsoft_refresh_token: session.refresh_token.clone(),
            minecraft_access_token: Some(session.access_token.clone()),
            ..Default::default()
        },
    )?;

    let account = AccountRecord {
        id: id.clone(),
        kind: AccountKind::Microsoft,
        name: session.name.clone(),
        uuid: session.uuid.clone(),
        created_at,
        last_used_at: timestamp,
        owns_minecraft: session.owns_minecraft,
        token_expires_at: Some(session.expires_at.clone()),
        authlib_url: None,
        server_name: None,
        skin_url: session.skin_url.clone(),
        secrets: Some(json!({ "persisted": true })),
    };
    match index {
        Some(index) => file.accounts[index] = account,
        None => file.accounts.push(account),
    }
    file.active_account_id = Some(id.clone());
    save_file(paths, &file)?;
    Ok(summarize(&file)
        .into_iter()
        .find(|summary| summary.id == id)
        .expect("the account was just saved"))
}

// ---------------------------------------------------------------------------
// authlib / LittleSkin
// ---------------------------------------------------------------------------

pub fn normalize_authlib_url(input: &str) -> Result<String> {
    let trimmed = input.trim().trim_end_matches('/');
    let mut url = if trimmed.ends_with("/api/yggdrasil") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/api/yggdrasil")
    };
    let parsed = url::Url::parse(&url)?;
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return Err(YuhinaError::new(
            ErrorCode::InvalidInput,
            "Enter a valid http(s) LittleSkin address.",
        ));
    }
    url = parsed.to_string();
    Ok(url.trim_end_matches('/').to_string())
}

pub fn add_authlib(
    paths: &Paths,
    server_url: &str,
    username: &str,
    password: &str,
) -> Result<AccountSummary> {
    let base = normalize_authlib_url(server_url)?;
    if username.trim().is_empty() || password.is_empty() {
        return Err(YuhinaError::new(
            ErrorCode::InvalidInput,
            "Enter your LittleSkin email and password.",
        ));
    }
    let response = http::post_json(
        &format!("{base}/authserver/authenticate"),
        &json!({
            "username": username.trim(),
            "password": password,
            "clientToken": uuid::Uuid::new_v4().to_string(),
            "agent": { "name": "Minecraft", "version": 1 },
            "requestUser": false,
        }),
    )?;
    if !response.is_success() {
        return Err(YuhinaError::new(
            ErrorCode::Auth,
            "LittleSkin rejected that email or password.",
        ));
    }
    let value: Value = response.json()?;
    let access_token = value["accessToken"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let client_token = value["clientToken"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let profile = &value["selectedProfile"];
    let uuid = format_uuid(profile["id"].as_str().unwrap_or_default());
    let name = profile["name"].as_str().unwrap_or_default().to_string();
    if access_token.is_empty() || uuid.is_empty() || name.is_empty() {
        return Err(YuhinaError::new(
            ErrorCode::Auth,
            "LittleSkin returned an unexpected login response.",
        ));
    }

    let _ = ensure_authlib_injector(paths);

    let mut file = load_file(paths)?;
    let timestamp = now();
    let index = file.accounts.iter().position(|account| {
        account.kind == AccountKind::Littleskin
            && account.uuid == uuid
            && account.authlib_url.as_deref() == Some(base.as_str())
    });
    let (id, created_at) = match index {
        Some(index) => (
            file.accounts[index].id.clone(),
            file.accounts[index].created_at.clone(),
        ),
        None => (uuid::Uuid::new_v4().to_string(), timestamp.clone()),
    };
    save_secrets(
        &id,
        &AccountSecrets {
            authlib_access_token: Some(access_token),
            authlib_client_token: Some(client_token),
            ..Default::default()
        },
    )?;
    let account = AccountRecord {
        id: id.clone(),
        kind: AccountKind::Littleskin,
        name,
        uuid,
        created_at,
        last_used_at: timestamp,
        owns_minecraft: None,
        token_expires_at: None,
        authlib_url: Some(base),
        server_name: Some("LittleSkin".to_string()),
        skin_url: None,
        secrets: Some(json!({ "persisted": true })),
    };
    match index {
        Some(index) => file.accounts[index] = account,
        None => file.accounts.push(account),
    }
    file.active_account_id = Some(id.clone());
    save_file(paths, &file)?;
    Ok(summarize(&file)
        .into_iter()
        .find(|summary| summary.id == id)
        .expect("the account was just saved"))
}

pub fn authlib_jar_path(paths: &Paths) -> PathBuf {
    paths.java_dir().join("authlib-injector.jar")
}

pub fn ensure_authlib_injector(paths: &Paths) -> Result<PathBuf> {
    let destination = authlib_jar_path(paths);
    if destination.is_file()
        && let Ok(meta) = fs::metadata(&destination)
        && meta.len() > 0
    {
        return Ok(destination);
    }
    fs::create_dir_all(paths.java_dir())?;
    let value: Value = http::get_json("https://authlib-injector.yushi.moe/artifact/latest.json")?;
    let url = value["download_url"]
        .as_str()
        .ok_or_else(|| YuhinaError::new(ErrorCode::Network, "authlib-injector is unavailable."))?;
    let task = super::download::DownloadTask {
        destination: destination.clone(),
        urls: vec![url.to_string()],
        sha1: None,
        sha512: None,
        size: None,
        label: "authlib-injector".to_string(),
    };
    super::download::download_all(
        vec![task],
        1,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        |_| {},
    )?;
    Ok(destination)
}

fn authlib_prefetched(base: &str) -> Result<String> {
    let metadata = http::get_text(base)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(metadata.as_bytes()))
}

pub fn launch_credentials(paths: &Paths, account_id: Option<&str>) -> Result<LaunchCredentials> {
    let file = load_file(paths)?;
    let account = account_id
        .and_then(|id| file.accounts.iter().find(|account| account.id == id))
        .or_else(|| {
            file.accounts
                .iter()
                .find(|account| Some(&account.id) == file.active_account_id.as_ref())
        })
        .or_else(|| file.accounts.first())
        .cloned()
        .ok_or_else(|| {
            YuhinaError::new(
                ErrorCode::AccountRequired,
                "Add an account before launching the game.",
            )
        })?;

    match account.kind {
        AccountKind::Offline => Ok(LaunchCredentials {
            name: account.name.clone(),
            uuid: account.uuid.clone(),
            access_token: "0".to_string(),
            user_type: "legacy".to_string(),
            authlib: None,
        }),
        AccountKind::Littleskin => {
            let secrets = load_secrets(&account.id)?.ok_or_else(|| {
                YuhinaError::new(
                    ErrorCode::AuthExpired,
                    "The LittleSkin session has expired. Sign in again.",
                )
            })?;
            let access_token = secrets.authlib_access_token.ok_or_else(|| {
                YuhinaError::new(
                    ErrorCode::AuthExpired,
                    "The LittleSkin session has expired. Sign in again.",
                )
            })?;
            let base = account.authlib_url.clone().ok_or_else(|| {
                YuhinaError::new(
                    ErrorCode::Auth,
                    "This LittleSkin account has no server URL.",
                )
            })?;
            let jar_path = ensure_authlib_injector(paths)?;
            let prefetched = authlib_prefetched(&base)?;
            Ok(LaunchCredentials {
                name: account.name.clone(),
                uuid: account.uuid.clone(),
                access_token,
                user_type: "mojang".to_string(),
                authlib: Some(AuthlibCredential {
                    jar_path,
                    url: base,
                    prefetched,
                }),
            })
        }
        AccountKind::Microsoft => {
            let mut secrets = load_secrets(&account.id)?.ok_or_else(|| {
                YuhinaError::new(
                    ErrorCode::AuthExpired,
                    "The Microsoft session has expired. Sign in again.",
                )
            })?;
            let expires = account
                .token_expires_at
                .as_deref()
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok());
            let expiring = secrets.minecraft_access_token.is_none()
                || expires
                    .map(|expires| {
                        expires.timestamp_millis() - Utc::now().timestamp_millis()
                            <= REFRESH_WINDOW_MS
                    })
                    .unwrap_or(true);
            if expiring {
                refresh_microsoft(paths, &account.id)?;
                secrets = load_secrets(&account.id)?.unwrap_or_default();
            }
            let access_token = secrets.minecraft_access_token.ok_or_else(|| {
                YuhinaError::new(
                    ErrorCode::AuthExpired,
                    "The Microsoft session has expired. Sign in again.",
                )
            })?;
            Ok(LaunchCredentials {
                name: account.name.clone(),
                uuid: account.uuid.clone(),
                access_token,
                user_type: "mojang".to_string(),
                authlib: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_is_deterministic_and_rfc4122() {
        let first = offline_uuid("Steve");
        let second = offline_uuid(" Steve ");
        assert_eq!(first, second);
        assert_eq!(first.len(), 36);
        assert_eq!(&first[14..15], "3");
        assert!(matches!(&first[19..20], "8" | "9" | "a" | "b"));
    }

    #[test]
    fn offline_names_are_strict() {
        assert!(is_valid_offline_name("Steve_01"));
        assert!(!is_valid_offline_name("ab"));
        assert!(!is_valid_offline_name("a b"));
        assert!(!is_valid_offline_name("玩家名字"));
    }

    #[test]
    fn uuids_are_formatted() {
        assert_eq!(
            format_uuid("069a79f444e94726a5befca90e38aaf5"),
            "069a79f4-44e9-4726-a5be-fca90e38aaf5"
        );
    }
}
