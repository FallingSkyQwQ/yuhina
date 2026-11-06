use std::fmt;

/// Machine-readable error codes. The UI maps these to localized messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    NotFound,
    InvalidInput,
    Io,
    Network,
    Http,
    Checksum,
    Unsupported,
    Auth,
    AuthExpired,
    AuthMissingProfile,
    AccountRequired,
    GameAlreadyRunning,
    GameNotRunning,
    LaunchFailed,
    InstallFailed,
    VersionNotFound,
    JavaRequired,
    JavaInvalid,
    SecretStorage,
    Parse,
    External,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::InvalidInput => "INVALID_INPUT",
            ErrorCode::Io => "IO",
            ErrorCode::Network => "NETWORK",
            ErrorCode::Http => "HTTP",
            ErrorCode::Checksum => "CHECKSUM_MISMATCH",
            ErrorCode::Unsupported => "UNSUPPORTED",
            ErrorCode::Auth => "AUTH_FAILED",
            ErrorCode::AuthExpired => "AUTH_SESSION_EXPIRED",
            ErrorCode::AuthMissingProfile => "AUTH_NO_MINECRAFT_PROFILE",
            ErrorCode::AccountRequired => "ACCOUNT_REQUIRED",
            ErrorCode::GameAlreadyRunning => "GAME_ALREADY_RUNNING",
            ErrorCode::GameNotRunning => "GAME_NOT_RUNNING",
            ErrorCode::LaunchFailed => "LAUNCH_FAILED",
            ErrorCode::InstallFailed => "INSTALL_FAILED",
            ErrorCode::VersionNotFound => "VERSION_NOT_FOUND",
            ErrorCode::JavaRequired => "JAVA_REQUIRED",
            ErrorCode::JavaInvalid => "JAVA_INVALID_PATH",
            ErrorCode::SecretStorage => "SECRET_STORAGE_UNAVAILABLE",
            ErrorCode::Parse => "PARSE_ERROR",
            ErrorCode::External => "EXTERNAL_ERROR",
        }
    }
}

/// A typed domain error with a stable code and a user-facing message.
#[derive(Clone, Debug)]
pub struct YuhinaError {
    pub code: ErrorCode,
    pub message: String,
}

impl YuhinaError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, message)
    }

    pub fn io(context: impl Into<String>) -> impl FnOnce(std::io::Error) -> Self {
        let context = context.into();
        move |source| Self::new(ErrorCode::Io, format!("{}: {}", context, source))
    }
}

impl fmt::Display for YuhinaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for YuhinaError {}

impl From<std::io::Error> for YuhinaError {
    fn from(source: std::io::Error) -> Self {
        Self::new(ErrorCode::Io, source.to_string())
    }
}

impl From<serde_json::Error> for YuhinaError {
    fn from(source: serde_json::Error) -> Self {
        Self::new(ErrorCode::Parse, source.to_string())
    }
}

impl From<ureq::Error> for YuhinaError {
    fn from(source: ureq::Error) -> Self {
        match source {
            ureq::Error::Status(status, response) => Self::new(
                ErrorCode::Http,
                format!("HTTP {}: {}", status, response.status_text()),
            ),
            ureq::Error::Transport(transport) => {
                Self::new(ErrorCode::Network, transport.to_string())
            }
        }
    }
}

impl From<url::ParseError> for YuhinaError {
    fn from(source: url::ParseError) -> Self {
        Self::new(ErrorCode::InvalidInput, source.to_string())
    }
}

impl From<zip::result::ZipError> for YuhinaError {
    fn from(source: zip::result::ZipError) -> Self {
        Self::new(ErrorCode::Io, source.to_string())
    }
}

pub type Result<T> = std::result::Result<T, YuhinaError>;
