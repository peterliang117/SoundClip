use serde::Deserialize;
use std::fs;
use std::io::Read;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::utils;

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("SoundClip/1.0")
        .build()
        .map_err(|e| e.to_string())
}

/// Get the locally installed yt-dlp version string.
pub async fn local_version() -> Result<String, String> {
    let ytdlp = utils::ytdlp_path();
    if !ytdlp.is_file() {
        return Err("yt-dlp not installed".into());
    }

    let output = Command::new(&ytdlp)
        .arg("--version")
        .creation_flags(0x08000000)
        .output()
        .await
        .map_err(|e| format!("Failed to run yt-dlp: {e}"))?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Check the latest yt-dlp release via GitHub API.
pub async fn latest_release() -> Result<(String, String), String> {
    let client = http_client()?;

    let release: GitHubRelease = client
        .get("https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest")
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Parse error: {e}"))?;

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == "yt-dlp.exe")
        .ok_or("yt-dlp.exe asset not found in release")?;

    Ok((release.tag_name, asset.browser_download_url.clone()))
}

/// Download the latest yt-dlp.exe from GitHub and replace the local binary atomically.
pub async fn download_ytdlp(app: &AppHandle, url: &str) -> Result<(), String> {
    let bin_dir = utils::bin_dir();
    fs::create_dir_all(&bin_dir).map_err(|e| format!("Cannot create bin dir: {e}"))?;

    let target = utils::ytdlp_path();
    let tmp = bin_dir.join("yt-dlp.exe.tmp");

    let _ = app.emit("update-log", "Downloading yt-dlp.exe...");

    let client = http_client()?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read error: {e}"))?;

    fs::write(&tmp, &bytes).map_err(|e| format!("Write error: {e}"))?;

    // Atomic replace: remove old, rename tmp.
    if target.is_file() {
        fs::remove_file(&target).map_err(|e| format!("Cannot remove old binary: {e}"))?;
    }
    fs::rename(&tmp, &target).map_err(|e| format!("Rename failed: {e}"))?;

    let _ = app.emit("update-log", "yt-dlp.exe updated successfully.");
    Ok(())
}

/// Run `yt-dlp -U` and stream the output (fallback self-update).
#[allow(dead_code)]
pub async fn self_update(app: AppHandle) -> Result<(), String> {
    let ytdlp = utils::ytdlp_path();
    if !ytdlp.is_file() {
        return Err("yt-dlp not installed".into());
    }

    let mut child = Command::new(&ytdlp)
        .arg("-U")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| format!("Failed to start yt-dlp: {e}"))?;

    if let Some(stdout) = child.stdout.take() {
        let app2 = app.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = app2.emit("update-log", &line);
            }
        });
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!(
            "yt-dlp -U exited with code {}",
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

// ── FFmpeg ────────────────────────────────────────────────────────────

const FFMPEG_ZIP_URL: &str =
    "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip";

/// Get the local ffmpeg version string (if installed in bin dir or on PATH).
pub fn local_ffmpeg_version() -> Option<String> {
    // Prefer the bundled copy.
    let bin = utils::bin_dir().join("ffmpeg.exe");
    let exe = if bin.is_file() {
        bin.to_string_lossy().to_string()
    } else {
        "ffmpeg".to_string()
    };

    let output = std::process::Command::new(&exe)
        .arg("-version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x08000000)
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);
    // First line is like "ffmpeg version N-xxxxx-g... Copyright ..."
    let first_line = text.lines().next()?;
    let version = first_line
        .strip_prefix("ffmpeg version ")?
        .split_whitespace()
        .next()?;
    Some(version.to_string())
}

/// Download ffmpeg from yt-dlp/FFmpeg-Builds, extract ffmpeg.exe + ffprobe.exe
/// into the app bin directory.
pub async fn download_ffmpeg(app: &AppHandle) -> Result<(), String> {
    let bin_dir = utils::bin_dir();
    fs::create_dir_all(&bin_dir).map_err(|e| format!("Cannot create bin dir: {e}"))?;

    let _ = app.emit("update-log", "Downloading ffmpeg (this may take a minute)...");

    let client = http_client()?;

    let response = client
        .get(FFMPEG_ZIP_URL)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read error: {e}"))?;

    let _ = app.emit("update-log", "Extracting ffmpeg...");

    // Extract ffmpeg.exe and ffprobe.exe from the zip.
    // The zip structure is: ffmpeg-master-latest-win64-gpl/bin/ffmpeg.exe
    let cursor = std::io::Cursor::new(&bytes[..]);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| format!("Zip error: {e}"))?;

    let targets = ["ffmpeg.exe", "ffprobe.exe"];

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("Zip entry error: {e}"))?;
        let name = file.name().to_string();

        // Match files ending in /bin/ffmpeg.exe or /bin/ffprobe.exe
        for target in &targets {
            if name.ends_with(&format!("/bin/{target}")) || name == *target {
                let dest = bin_dir.join(target);
                let tmp = bin_dir.join(format!("{target}.tmp"));
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)
                    .map_err(|e| format!("Extract error: {e}"))?;
                fs::write(&tmp, &buf).map_err(|e| format!("Write error: {e}"))?;
                if dest.is_file() {
                    fs::remove_file(&dest)
                        .map_err(|e| format!("Cannot remove old {target}: {e}"))?;
                }
                fs::rename(&tmp, &dest).map_err(|e| format!("Rename error: {e}"))?;
                let _ = app.emit("update-log", &format!("Extracted {target}"));
            }
        }
    }

    // Verify both exist.
    if !bin_dir.join("ffmpeg.exe").is_file() {
        return Err("ffmpeg.exe not found in archive".into());
    }
    if !bin_dir.join("ffprobe.exe").is_file() {
        return Err("ffprobe.exe not found in archive".into());
    }

    let _ = app.emit("update-log", "ffmpeg installed successfully.");
    Ok(())
}
