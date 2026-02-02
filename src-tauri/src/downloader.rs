use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::utils;

/// Shared handle so we can cancel the running process from another command.
pub type ProcessHandle = Arc<Mutex<Option<tokio::process::Child>>>;

pub fn new_process_handle() -> ProcessHandle {
    Arc::new(Mutex::new(None))
}

/// Spawn yt-dlp and stream its output to the frontend via Tauri events.
pub async fn run(
    app: AppHandle,
    handle: ProcessHandle,
    url: String,
    audio_format: String,
    playlist: bool,
    save_path: String,
) -> Result<(), String> {
    let ytdlp = utils::ytdlp_path();
    if !ytdlp.is_file() {
        return Err("yt-dlp.exe not found. Use Check Update to download it.".into());
    }

    let bin_dir = utils::bin_dir();

    let mut args: Vec<String> = vec![
        "-x".into(),
        "-P".into(),
        format!("home:{save_path}"),
        "-o".into(),
        "%(title).200s [%(id)s].%(ext)s".into(),
        "--windows-filenames".into(),
        "--newline".into(),
        "--no-colors".into(),
        format!("--ffmpeg-location={}", bin_dir.to_string_lossy()),
    ];

    if audio_format != "best" {
        args.push("--audio-format".into());
        args.push(audio_format);
    }

    if playlist {
        args.push("--yes-playlist".into());
    } else {
        args.push("--no-playlist".into());
    }

    args.push(url);

    let mut child = Command::new(&ytdlp)
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|e| format!("Failed to start yt-dlp: {e}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or("Failed to capture yt-dlp stdout")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("Failed to capture yt-dlp stderr")?;

    // Store the child so cancel can reach it.
    {
        let mut guard = handle.lock().await;
        *guard = Some(child);
    }

    // Stream stdout.
    let app2 = app.clone();
    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Some(pct) = utils::parse_progress(&line) {
                let _ = app2.emit("download-progress", pct);
            }
            let _ = app2.emit("download-log", &line);
        }
    });

    let app3 = app.clone();
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = app3.emit("download-log", &line);
        }
    });

    // Wait for both stream tasks.
    let _ = tokio::join!(stdout_task, stderr_task);

    // Wait for the process to finish.
    let status = {
        let mut guard = handle.lock().await;
        if let Some(ref mut child) = *guard {
            child.wait().await.map_err(|e| e.to_string())?
        } else {
            return Err("Process was cancelled".into());
        }
    };

    // Clear the handle.
    {
        let mut guard = handle.lock().await;
        *guard = None;
    }

    if status.success() {
        let _ = app.emit("download-complete", "success");
        Ok(())
    } else {
        let code = status.code().unwrap_or(-1);
        let _ = app.emit("download-complete", format!("failed:{code}"));
        Err(format!("yt-dlp exited with code {code}"))
    }
}

/// Kill the running yt-dlp process tree.
pub async fn cancel(handle: ProcessHandle) -> Result<(), String> {
    let mut guard = handle.lock().await;
    if let Some(ref child) = *guard {
        if let Some(pid) = child.id() {
            utils::kill_process_tree(pid);
        }
    }
    *guard = None;
    Ok(())
}
