// The engine exposes a small library surface that the first UI milestone does
// not call yet (backups, file browsing, modpack import, ...). Keep it compiled
// and tested rather than deleting it.
#![allow(dead_code)]

pub mod accounts;
pub mod download;
pub mod http;
pub mod install;
pub mod instances;
pub mod java;
pub mod launch;
pub mod modrinth;
pub mod news;
pub mod paths;
pub mod servers;
pub mod settings;
pub mod storage;
pub mod updates;
pub mod version;

mod error;

pub use error::{ErrorCode, Result, YuhinaError};
#[allow(unused_imports)]
pub use paths::Paths;
