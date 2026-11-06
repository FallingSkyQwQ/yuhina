use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::install::split_args;
use super::paths::Paths;
use super::version::{RuleContext, VersionJson, resolve_arguments};
use super::{ErrorCode, Result, YuhinaError};

#[derive(Clone, Debug)]
pub struct LaunchCredentials {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub user_type: String,
}

#[derive(Clone, Debug, Default)]
pub struct AuthlibInjection {
    pub jar_path: String,
    pub authlib_url: String,
    pub prefetched: String,
}

#[derive(Clone, Debug)]
pub struct ServerTarget {
    pub host: String,
    pub port: u16,
    pub quick_play: bool,
}

#[derive(Clone, Debug)]
pub struct LaunchRequest {
    pub version: VersionJson,
    pub version_name: String,
    pub version_type: String,
    pub classpath: Vec<PathBuf>,
    pub natives_directory: PathBuf,
    pub game_directory: PathBuf,
    pub assets_root: PathBuf,
    pub assets_index_name: String,
    pub library_directory: PathBuf,
    pub credentials: LaunchCredentials,
    pub memory_mb: u32,
    pub launcher_version: String,
    pub jvm_args: String,
    pub game_args: String,
    pub authlib: Option<AuthlibInjection>,
    pub server: Option<ServerTarget>,
    pub resolution: Option<(u32, u32)>,
}

#[derive(Clone, Debug)]
pub struct LaunchCommand {
    pub args: Vec<String>,
    pub redacted: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct GameLine {
    pub at: String,
    pub stream: LogStream,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogStream {
    Stdout,
    Stderr,
    System,
}

impl LogStream {
    pub fn as_str(self) -> &'static str {
        match self {
            LogStream::Stdout => "stdout",
            LogStream::Stderr => "stderr",
            LogStream::System => "system",
        }
    }
}

pub struct GameProcess {
    pub instance_id: String,
    pub child: Child,
    pub pid: u32,
    pub started: Instant,
    pub started_at: String,
    pub log_file: PathBuf,
    pub lines: Receiver<GameLine>,
    pub killed_by_user: Arc<AtomicBool>,
}

impl GameProcess {
    pub fn kill(&mut self) {
        self.killed_by_user.store(true, Ordering::Relaxed);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            let mut command = Command::new("taskkill");
            command
                .args(["/PID", &self.pid.to_string(), "/T", "/F"])
                .creation_flags(0x08000000)
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            if command.status().is_err() {
                let _ = self.child.kill();
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = self.child.kill();
        }
    }

    /// Non-blocking check for the process exit code.
    pub fn try_wait(&mut self) -> Option<Option<i32>> {
        match self.child.try_wait() {
            Ok(Some(status)) => Some(status.code()),
            Ok(None) => None,
            Err(_) => Some(None),
        }
    }
}

impl Drop for GameProcess {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
        }
    }
}

fn parse_version_triple(id: &str) -> Option<(u32, u32, u32)> {
    let mut pieces = id.split(['.', '-', '+']);
    let major = pieces.next()?.parse().ok()?;
    let minor = pieces.next()?.parse().ok()?;
    let patch = pieces
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    Some((major, minor, patch))
}

pub fn supports_quick_play(version_id: &str) -> bool {
    parse_version_triple(version_id)
        .map(|(major, minor, _)| major > 1 || (major == 1 && minor >= 20))
        .unwrap_or(false)
}

fn needs_log4j_fix(version_id: &str) -> bool {
    if version_id.len() > 4 && version_id.as_bytes()[2] == b'w' {
        return true;
    }
    match parse_version_triple(version_id) {
        Some((1, minor, patch)) => minor < 18 || (minor == 18 && patch <= 1),
        Some((major, _, _)) => major == 1,
        None => false,
    }
}

fn placeholder_map(request: &LaunchRequest) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let credentials = &request.credentials;
    let mut insert = |key: &str, value: String| {
        map.insert(key.to_string(), value);
    };
    insert("auth_player_name", credentials.name.clone());
    insert("version_name", request.version_name.clone());
    insert(
        "game_directory",
        request.game_directory.to_string_lossy().to_string(),
    );
    insert(
        "assets_root",
        request.assets_root.to_string_lossy().to_string(),
    );
    insert(
        "game_assets",
        request.assets_root.to_string_lossy().to_string(),
    );
    insert("assets_index_name", request.assets_index_name.clone());
    insert("auth_uuid", credentials.uuid.replace('-', ""));
    insert("auth_access_token", credentials.access_token.clone());
    insert("auth_session", credentials.access_token.clone());
    insert("user_type", credentials.user_type.clone());
    insert("auth_session_type", credentials.user_type.clone());
    insert("version_type", request.version_type.clone());
    insert(
        "natives_directory",
        request.natives_directory.to_string_lossy().to_string(),
    );
    insert("launcher_name", "Yuhina".to_string());
    insert("launcher_version", request.launcher_version.clone());
    insert(
        "classpath",
        request
            .classpath
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(super::version::classpath_separator()),
    );
    insert(
        "classpath_separator",
        super::version::classpath_separator().to_string(),
    );
    insert(
        "library_directory",
        request.library_directory.to_string_lossy().to_string(),
    );
    insert("auth_xuid", String::new());
    insert("clientid", String::new());
    let (width, height) = request.resolution.unwrap_or((854, 480));
    insert("resolution_width", width.to_string());
    insert("resolution_height", height.to_string());
    insert("user_properties", "{}".to_string());
    map
}

pub fn replace_placeholders(value: &str, map: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find('}') {
            let key = &after[..end];
            if let Some(replacement) = map.get(key) {
                result.push_str(replacement);
            } else {
                result.push_str(&rest[start..start + 2 + end + 1]);
            }
            rest = &after[end + 1..];
        } else {
            result.push_str(&rest[start..]);
            rest = "";
        }
    }
    result.push_str(rest);
    result
}

pub fn build_command(request: &LaunchRequest) -> Result<LaunchCommand> {
    let main_class = request.version.main_class.clone().ok_or_else(|| {
        YuhinaError::new(
            ErrorCode::LaunchFailed,
            "The version metadata does not define a main class.",
        )
    })?;
    let map = placeholder_map(request);
    let context = RuleContext::current();
    let arguments = request.version.arguments.clone().unwrap_or_default();

    let jvm_template: Vec<String> = resolve_arguments(&arguments.jvm, &context)
        .into_iter()
        .map(|value| replace_placeholders(&value, &map))
        .collect();
    let modern_jvm = !jvm_template.is_empty();
    let game_template: Vec<String> = if arguments.game.is_empty() {
        split_args(request.version.minecraft_arguments.as_deref().unwrap_or(""))
            .into_iter()
            .map(|value| replace_placeholders(&value, &map))
            .collect()
    } else {
        resolve_arguments(&arguments.game, &context)
            .into_iter()
            .map(|value| replace_placeholders(&value, &map))
            .collect()
    };

    let mut args = Vec::new();
    let natives = request.natives_directory.to_string_lossy().to_string();
    if !jvm_template
        .iter()
        .any(|arg| arg == &format!("-Djava.library.path={natives}"))
    {
        args.push(format!("-Djava.library.path={natives}"));
    }
    args.push(format!("-Xmx{}M", request.memory_mb));
    args.push(format!("-Xms{}M", (request.memory_mb / 2).min(1024)));
    if needs_log4j_fix(&request.version_name) {
        args.push("-Dlog4j2.formatMsgNoLookups=true".to_string());
    }
    args.push("-Dminecraft.launcher.brand=Yuhina".to_string());
    args.push(format!(
        "-Dminecraft.launcher.version={}",
        request.launcher_version
    ));
    if let Some(authlib) = &request.authlib {
        args.push(format!(
            "-javaagent:{}={}",
            authlib.jar_path, authlib.authlib_url
        ));
        args.push(format!(
            "-Dauthlibinjector.yggdrasil.prefetched={}",
            authlib.prefetched
        ));
    }
    if modern_jvm {
        args.extend(jvm_template);
    }
    if !args.iter().any(|arg| arg == "-cp" || arg == "-classpath") {
        args.push("-cp".to_string());
        args.push(
            request
                .classpath
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(super::version::classpath_separator()),
        );
    }
    let custom_jvm = split_args(&request.jvm_args);
    args.extend(
        custom_jvm
            .into_iter()
            .map(|value| replace_placeholders(&value, &map)),
    );
    args.push(main_class);

    let mut game = game_template;
    if let Some(server) = &request.server {
        if server.quick_play {
            game.push("--quickPlayMultiplayer".to_string());
            game.push(if server.port == 25565 {
                server.host.clone()
            } else {
                format!("{}:{}", server.host, server.port)
            });
        } else {
            game.push("--server".to_string());
            game.push(server.host.clone());
            game.push("--port".to_string());
            game.push(server.port.to_string());
        }
    }
    let custom_game = split_args(&request.game_args);
    game.extend(
        custom_game
            .into_iter()
            .map(|value| replace_placeholders(&value, &map)),
    );
    args.extend(game);

    let redacted = args
        .iter()
        .map(|arg| {
            if arg.starts_with("-Dauthlibinjector.yggdrasil.prefetched=") {
                "-Dauthlibinjector.yggdrasil.prefetched=***".to_string()
            } else if arg == &request.credentials.access_token {
                "***".to_string()
            } else {
                arg.clone()
            }
        })
        .collect();

    Ok(LaunchCommand { args, redacted })
}

pub fn spawn(
    paths: &Paths,
    instance_id: &str,
    java_path: &Path,
    request: &LaunchRequest,
) -> Result<GameProcess> {
    let command = build_command(request)?;
    fs::create_dir_all(&request.game_directory)?;
    let log_dir = paths.instance_logs_dir(instance_id)?;
    fs::create_dir_all(&log_dir)?;
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let log_file = log_dir.join(format!("{timestamp}.log"));
    let log = Arc::new(Mutex::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?,
    ));

    let started_at = chrono::Utc::now().to_rfc3339();
    write_log(
        &log,
        &GameLine {
            at: started_at.clone(),
            stream: LogStream::System,
            text: format!(
                "Launching {} with {}",
                request.version_name,
                java_path.display()
            ),
        },
    );
    write_log(
        &log,
        &GameLine {
            at: started_at.clone(),
            stream: LogStream::System,
            text: format!(
                "Command: {} {}",
                java_path.display(),
                command.redacted.join(" ")
            ),
        },
    );

    let mut child_command = Command::new(java_path);
    child_command
        .args(&command.args)
        .current_dir(&request.game_directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        child_command.creation_flags(0x08000000);
    }
    let mut child = child_command.spawn().map_err(|error| {
        YuhinaError::new(
            ErrorCode::LaunchFailed,
            format!("The Java process could not be started: {error}"),
        )
    })?;
    let pid = child.id();
    let (sender, receiver) = mpsc::channel();

    if let Some(stdout) = child.stdout.take() {
        spawn_reader(stdout, LogStream::Stdout, sender.clone(), Arc::clone(&log));
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_reader(stderr, LogStream::Stderr, sender.clone(), Arc::clone(&log));
    }
    drop(sender);

    Ok(GameProcess {
        instance_id: instance_id.to_string(),
        child,
        pid,
        started: Instant::now(),
        started_at,
        log_file,
        lines: receiver,
        killed_by_user: Arc::new(AtomicBool::new(false)),
    })
}

fn spawn_reader<R: std::io::Read + Send + 'static>(
    reader: R,
    stream: LogStream,
    sender: mpsc::Sender<GameLine>,
    log: Arc<Mutex<File>>,
) {
    std::thread::spawn(move || {
        let reader = BufReader::new(reader);
        for line in reader.lines() {
            let Ok(text) = line else { break };
            let text = if text.chars().count() > 4000 {
                text.chars().take(4000).collect::<String>() + "…"
            } else {
                text
            };
            let line = GameLine {
                at: chrono::Utc::now().to_rfc3339(),
                stream,
                text,
            };
            write_log(&log, &line);
            if sender.send(line).is_err() {
                break;
            }
        }
    });
}

fn write_log(log: &Arc<Mutex<File>>, line: &GameLine) {
    if let Ok(mut file) = log.lock() {
        let _ = writeln!(
            file,
            "[{}] [{}] {}",
            line.at,
            line.stream.as_str(),
            line.text
        );
    }
}

pub struct CrashAssessment {
    pub crashed: bool,
    pub reason: Option<&'static str>,
}

/// Classifies a non-zero exit from the tail of its log.
pub fn assess_crash(
    exit_code: Option<i32>,
    lines: &[String],
    killed_by_user: bool,
) -> CrashAssessment {
    if killed_by_user {
        return CrashAssessment {
            crashed: false,
            reason: None,
        };
    }
    let crashed = exit_code != Some(0);
    if !crashed {
        return CrashAssessment {
            crashed: false,
            reason: None,
        };
    }
    let text = lines.join("\n").to_ascii_lowercase();
    let reason =
        if text.contains("unsupportedclassversionerror") || text.contains("class file version") {
            Some("java-version")
        } else if text.contains("outofmemoryerror") {
            Some("out-of-memory")
        } else if text.contains("pixel format")
            || text.contains("opengl")
            || text.contains("gfx")
            || text.contains("directx")
        {
            Some("graphics")
        } else if text.contains("exception") || text.contains("caused by") {
            Some("exception")
        } else {
            Some("exit-code")
        };
    CrashAssessment { crashed, reason }
}

/// Sends a system line into an active process's channel for the UI.
pub fn system_line(text: impl Into<String>) -> GameLine {
    GameLine {
        at: chrono::Utc::now().to_rfc3339(),
        stream: LogStream::System,
        text: text.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_play_needs_1_20() {
        assert!(supports_quick_play("1.20"));
        assert!(supports_quick_play("1.21.4"));
        assert!(!supports_quick_play("1.19.4"));
        assert!(!supports_quick_play("24w14a"));
    }
}
