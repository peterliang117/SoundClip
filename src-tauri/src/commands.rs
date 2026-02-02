use serde::Serialize;
use tauri::State;

use crate::downloader::{self, ProcessHandle};
use crate::settings::Settings;
use crate::updater;
use crate::utils;

// ── Dependencies ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DepsStatus {
    pub ytdlp: bool,
    pub ffmpeg: bool,
}

#[tauri::command]
pub fn check_dependencies() -> DepsStatus {
    DepsStatus {
        ytdlp: utils::is_ytdlp_installed(),
        ffmpeg: utils::is_ffmpeg_installed(),
    }
}

// ── Settings ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_settings() -> Settings {
    Settings::load()
}

#[tauri::command]
pub fn save_settings(settings: Settings) -> Result<(), String> {
    settings.save()
}

// ── Download ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn start_download(
    app: tauri::AppHandle,
    handle: State<'_, ProcessHandle>,
    url: String,
    audio_format: String,
    playlist: bool,
    save_path: String,
) -> Result<(), String> {
    downloader::run(app, handle.inner().clone(), url, audio_format, playlist, save_path).await
}

#[tauri::command]
pub async fn cancel_download(handle: State<'_, ProcessHandle>) -> Result<(), String> {
    downloader::cancel(handle.inner().clone()).await
}

// ── Updater ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct VersionInfo {
    pub local: Option<String>,
    pub latest: Option<String>,
    pub download_url: Option<String>,
    pub update_available: bool,
}

#[tauri::command]
pub async fn check_ytdlp_update() -> Result<VersionInfo, String> {
    let local = updater::local_version().await.ok();
    let (latest_tag, download_url) = updater::latest_release().await?;

    let update_available = match &local {
        Some(v) => v != &latest_tag,
        None => true,
    };

    Ok(VersionInfo {
        local,
        latest: Some(latest_tag),
        download_url: Some(download_url),
        update_available,
    })
}

#[tauri::command]
pub async fn update_ytdlp(app: tauri::AppHandle, download_url: String) -> Result<(), String> {
    updater::download_ytdlp(&app, &download_url).await
}

// ── FFmpeg ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct FfmpegInfo {
    pub installed: bool,
    pub version: Option<String>,
}

#[tauri::command]
pub fn check_ffmpeg() -> FfmpegInfo {
    let version = updater::local_ffmpeg_version();
    FfmpegInfo {
        installed: version.is_some() || utils::is_ffmpeg_installed(),
        version,
    }
}

#[tauri::command]
pub async fn install_ffmpeg(app: tauri::AppHandle) -> Result<(), String> {
    updater::download_ffmpeg(&app).await
}
