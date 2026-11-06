use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sha1::{Digest, Sha1};
use sha2::Sha512;

use super::http::USER_AGENT;
use super::{ErrorCode, Result, YuhinaError};

#[derive(Clone, Debug)]
pub struct DownloadTask {
    pub destination: PathBuf,
    pub urls: Vec<String>,
    pub sha1: Option<String>,
    /// Used when the publisher only supplies a SHA-512 digest, as some
    /// `.mrpack` indexes do.
    pub sha512: Option<String>,
    pub size: Option<u64>,
    pub label: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DownloadProgress {
    pub files_done: usize,
    pub files_total: usize,
    pub bytes_received: u64,
    pub bytes_total: u64,
}

/// Bounded-concurrency download queue with retries and SHA-1 verification.
/// Files that already match their size (or checksum when the size is unknown)
/// are skipped without a network request.
pub fn download_all<F>(
    tasks: Vec<DownloadTask>,
    concurrency: usize,
    cancel: Arc<AtomicBool>,
    on_progress: F,
) -> Result<()>
where
    F: Fn(DownloadProgress) + Send + Sync,
{
    if tasks.is_empty() {
        return Ok(());
    }
    let total = tasks.len();
    let total_bytes: u64 = tasks.iter().map(|task| task.size.unwrap_or(0)).sum();
    let cursor = AtomicUsize::new(0);
    let files_done = AtomicUsize::new(0);
    let completed_bytes = AtomicU64::new(0);
    let active_bytes = AtomicU64::new(0);
    let last_emit = Mutex::new(Instant::now() - Duration::from_secs(1));
    let first_error: Mutex<Option<YuhinaError>> = Mutex::new(None);
    let tasks = Arc::new(tasks);

    // A shared, throttled progress reporter.
    let report = |force: bool| {
        let mut last = last_emit.lock().unwrap();
        if !force && last.elapsed() < Duration::from_millis(160) {
            return;
        }
        *last = Instant::now();
        drop(last);
        on_progress(DownloadProgress {
            files_done: files_done.load(Ordering::Relaxed),
            files_total: total,
            bytes_received: completed_bytes.load(Ordering::Relaxed)
                + active_bytes.load(Ordering::Relaxed),
            bytes_total: total_bytes,
        });
    };
    report(true);

    std::thread::scope(|scope| {
        let workers = concurrency.max(1).min(total);
        for _ in 0..workers {
            let tasks = Arc::clone(&tasks);
            let cursor = &cursor;
            let files_done = &files_done;
            let completed_bytes = &completed_bytes;
            let active_bytes = &active_bytes;
            let first_error = &first_error;
            let cancel = Arc::clone(&cancel);
            let report = &report;
            scope.spawn(move || {
                loop {
                    if cancel.load(Ordering::Relaxed) || first_error.lock().unwrap().is_some() {
                        break;
                    }
                    let index = cursor.fetch_add(1, Ordering::Relaxed);
                    let Some(task) = tasks.get(index) else {
                        break;
                    };

                    if already_valid(task) {
                        files_done.fetch_add(1, Ordering::Relaxed);
                        completed_bytes.fetch_add(task.size.unwrap_or(0), Ordering::Relaxed);
                        report(true);
                        continue;
                    }

                    let mut last_error = None;
                    for attempt in 0..3usize {
                        if cancel.load(Ordering::Relaxed) {
                            return;
                        }
                        let url = task
                            .urls
                            .get(attempt.min(task.urls.len().saturating_sub(1)))
                            .cloned();
                        let Some(url) = url else {
                            break;
                        };
                        let before = active_bytes.load(Ordering::Relaxed);
                        match download_once(task, &url, &cancel, active_bytes, report) {
                            Ok(bytes) => {
                                completed_bytes.fetch_add(bytes, Ordering::Relaxed);
                                active_bytes.fetch_sub(bytes, Ordering::Relaxed);
                                last_error = None;
                                break;
                            }
                            Err(error) => {
                                active_bytes.store(before, Ordering::Relaxed);
                                last_error = Some(error);
                                std::thread::sleep(Duration::from_millis(
                                    400 * (attempt as u64 + 1),
                                ));
                            }
                        }
                    }

                    match last_error {
                        None => {
                            files_done.fetch_add(1, Ordering::Relaxed);
                            report(true);
                        }
                        Some(error) => {
                            let mut slot = first_error.lock().unwrap();
                            if slot.is_none() {
                                *slot = Some(error);
                            }
                            break;
                        }
                    }
                }
            });
        }
    });

    report(true);
    if let Some(error) = first_error.lock().unwrap().take() {
        if cancel.load(Ordering::Relaxed) {
            return Err(YuhinaError::new(
                ErrorCode::InstallFailed,
                "The installation was cancelled.",
            ));
        }
        return Err(error);
    }
    Ok(())
}

fn already_valid(task: &DownloadTask) -> bool {
    let Ok(meta) = fs::metadata(&task.destination) else {
        return false;
    };
    if !meta.is_file() || meta.len() == 0 {
        return false;
    }
    if let Some(size) = task.size
        && size > 0
    {
        return meta.len() == size;
    }
    if let Some(expected) = &task.sha1 {
        return hash_file(&task.destination)
            .map(|actual| actual.eq_ignore_ascii_case(expected))
            .unwrap_or(false);
    }
    if let Some(expected) = &task.sha512 {
        return hash_file_sha512(&task.destination)
            .map(|actual| actual.eq_ignore_ascii_case(expected))
            .unwrap_or(false);
    }
    true
}

pub fn hash_file(path: &std::path::Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn hash_file_sha512(path: &std::path::Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha512::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn download_once<F>(
    task: &DownloadTask,
    url: &str,
    cancel: &AtomicBool,
    active_bytes: &AtomicU64,
    report: &F,
) -> Result<u64>
where
    F: Fn(bool),
{
    if let Some(parent) = task.destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let part = task.destination.with_extension(format!(
        "{}.part",
        task.destination
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("bin")
    ));
    let _ = fs::remove_file(&part);

    let response = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "*/*")
        .timeout(Duration::from_secs(90))
        .call();
    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(status, _)) => {
            return Err(YuhinaError::new(
                ErrorCode::Http,
                format!("The download failed ({status})."),
            ));
        }
        Err(ureq::Error::Transport(transport)) => {
            return Err(YuhinaError::new(ErrorCode::Network, transport.to_string()));
        }
    };

    let mut reader = response.into_reader();
    let mut file = File::create(&part)?;
    let mut sha1 = Sha1::new();
    let mut sha512 = Sha512::new();
    let use_sha1 = task.sha1.is_some();
    let use_sha512 = task.sha1.is_none() && task.sha512.is_some();
    let mut received: u64 = 0;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = fs::remove_file(&part);
            return Err(YuhinaError::new(
                ErrorCode::InstallFailed,
                "The installation was cancelled.",
            ));
        }
        let read = reader
            .read(&mut buffer)
            .map_err(|error| YuhinaError::new(ErrorCode::Network, error.to_string()))?;
        if read == 0 {
            break;
        }
        if use_sha1 {
            sha1.update(&buffer[..read]);
        }
        if use_sha512 {
            sha512.update(&buffer[..read]);
        }
        file.write_all(&buffer[..read])?;
        received += read as u64;
        active_bytes.fetch_add(read as u64, Ordering::Relaxed);
        report(false);
    }
    file.sync_all()?;
    drop(file);

    if let Some(size) = task.size
        && size > 0
        && received != size
    {
        let _ = fs::remove_file(&part);
        return Err(YuhinaError::new(
            ErrorCode::Checksum,
            "The downloaded file had an unexpected size.",
        ));
    }
    if let Some(expected) = &task.sha1 {
        let actual = hex::encode(sha1.finalize());
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = fs::remove_file(&part);
            return Err(YuhinaError::new(
                ErrorCode::Checksum,
                "The downloaded file failed its checksum.",
            ));
        }
    } else if let Some(expected) = &task.sha512 {
        let actual = hex::encode(sha512.finalize());
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = fs::remove_file(&part);
            return Err(YuhinaError::new(
                ErrorCode::Checksum,
                "The downloaded file failed its checksum.",
            ));
        }
    }
    fs::rename(&part, &task.destination)?;
    Ok(received)
}
