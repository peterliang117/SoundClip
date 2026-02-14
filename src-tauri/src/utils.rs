use std::path::PathBuf;
use std::process::Command;

/// Returns `%LOCALAPPDATA%\SoundClip` (e.g. `C:\Users\<user>\AppData\Local\SoundClip`).
pub fn app_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("SoundClip")
}

/// Returns `%LOCALAPPDATA%\SoundClip\bin`.
pub fn bin_dir() -> PathBuf {
    app_data_dir().join("bin")
}

/// Full path to the yt-dlp binary.
pub fn ytdlp_path() -> PathBuf {
    bin_dir().join("yt-dlp.exe")
}

/// Check whether yt-dlp.exe exists in the expected location.
pub fn is_ytdlp_installed() -> bool {
    ytdlp_path().is_file()
}

/// Check whether ffmpeg is reachable — either in the bin dir or on PATH.
pub fn is_ffmpeg_installed() -> bool {
    let bin = bin_dir().join("ffmpeg.exe");
    if bin.is_file() {
        return true;
    }
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Return the first available JS runtime for yt-dlp's YouTube extractor.
/// Priority: node > deno > bun.
pub fn ytdlp_js_runtime() -> Option<String> {
    for runtime in ["node", "deno", "bun"] {
        if Command::new(runtime)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Some(runtime.to_string());
        }
    }
    None
}

/// Kill an entire process tree on Windows using `taskkill /T /F /PID`.
pub fn kill_process_tree(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Parse a yt-dlp progress line like `[download]  45.2% of ~10MiB ...` and
/// return the percentage as a float, or `None` if the line is not a progress line.
pub fn parse_progress(line: &str) -> Option<f64> {
    let re = regex::Regex::new(r"\[download\]\s+([\d.]+)%").ok()?;
    let caps = re.captures(line)?;
    caps.get(1)?.as_str().parse::<f64>().ok()
}
