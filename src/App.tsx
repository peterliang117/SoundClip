import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

interface DepsStatus {
  ytdlp: boolean;
  ffmpeg: boolean;
}

interface Settings {
  save_path: string;
  audio_format: string;
  playlist_mode: boolean;
}

interface VersionInfo {
  local: string | null;
  latest: string | null;
  download_url: string | null;
  update_available: boolean;
}

interface FfmpegInfo {
  installed: boolean;
  version: string | null;
}

const FORMATS = [
  "best",
  "mp3",
  "m4a",
  "opus",
  "flac",
  "wav",
  "aac",
  "alac",
  "vorbis",
];

export default function App() {
  const [url, setUrl] = useState("");
  const [format, setFormat] = useState("best");
  const [playlist, setPlaylist] = useState(false);
  const [savePath, setSavePath] = useState("");
  const [logs, setLogs] = useState<string[]>([]);
  const [progress, setProgress] = useState(0);
  const [status, setStatus] = useState("Ready");
  const [downloading, setDownloading] = useState(false);
  const [deps, setDeps] = useState<DepsStatus>({ ytdlp: true, ffmpeg: true });
  const [updating, setUpdating] = useState(false);
  const [showFfmpegPrompt, setShowFfmpegPrompt] = useState(false);
  const logRef = useRef<HTMLDivElement>(null);

  const refreshDeps = () => invoke<DepsStatus>("check_dependencies").then(setDeps);

  // Load settings + check deps on mount.
  useEffect(() => {
    invoke<Settings>("get_settings").then((s) => {
      setSavePath(s.save_path);
      setFormat(s.audio_format);
      setPlaylist(s.playlist_mode);
    });
    refreshDeps();
  }, []);

  // Listen to backend events.
  useEffect(() => {
    const unlisteners = [
      listen<string>("download-log", (e) => {
        setLogs((prev) => [...prev, e.payload]);
      }),
      listen<number>("download-progress", (e) => {
        setProgress(e.payload);
      }),
      listen<string>("download-complete", (e) => {
        setDownloading(false);
        if (e.payload === "success") {
          setStatus("Download complete");
          setProgress(100);
        } else {
          setStatus(`Download failed (${e.payload})`);
        }
      }),
      listen<string>("update-log", (e) => {
        setLogs((prev) => [...prev, e.payload]);
      }),
    ];
    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()));
    };
  }, []);

  // Auto-scroll logs.
  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [logs]);

  const saveCurrentSettings = (overrides?: Partial<Settings>) => {
    const s: Settings = {
      save_path: overrides?.save_path ?? savePath,
      audio_format: overrides?.audio_format ?? format,
      playlist_mode: overrides?.playlist_mode ?? playlist,
    };
    invoke("save_settings", { settings: s });
  };

  const pickFolder = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (selected) {
      const p = selected as string;
      setSavePath(p);
      saveCurrentSettings({ save_path: p });
    }
  };

  const handleDownload = async () => {
    if (!url.trim()) {
      setStatus("Enter a URL first");
      return;
    }
    setLogs([]);
    setProgress(0);
    setDownloading(true);
    setStatus("Downloading...");
    saveCurrentSettings();

    try {
      await invoke("start_download", {
        url: url.trim(),
        audioFormat: format,
        playlist,
        savePath,
      });
    } catch (e) {
      setStatus(`Error: ${e}`);
      setDownloading(false);
    }
  };

  const handleCancel = async () => {
    try {
      await invoke("cancel_download");
      setStatus("Cancelled");
      setDownloading(false);
    } catch (e) {
      setStatus(`Cancel error: ${e}`);
    }
  };

  const handleCheckUpdate = async () => {
    setUpdating(true);
    setLogs([]);
    setStatus("Checking for updates...");

    try {
      // 1. Check yt-dlp
      const ytInfo = await invoke<VersionInfo>("check_ytdlp_update");
      if (!ytInfo.local) {
        setStatus("yt-dlp not installed — downloading...");
        if (ytInfo.download_url) {
          await invoke("update_ytdlp", { downloadUrl: ytInfo.download_url });
        }
      } else if (ytInfo.update_available) {
        setStatus(`Updating yt-dlp: ${ytInfo.local} -> ${ytInfo.latest}...`);
        if (ytInfo.download_url) {
          await invoke("update_ytdlp", { downloadUrl: ytInfo.download_url });
        }
      } else {
        setLogs((prev) => [...prev, `yt-dlp ${ytInfo.local} is up to date`]);
      }

      // 2. Check ffmpeg
      const ffInfo = await invoke<FfmpegInfo>("check_ffmpeg");
      if (!ffInfo.installed) {
        // Prompt user for confirmation before downloading ffmpeg
        setShowFfmpegPrompt(true);
        setStatus("ffmpeg not found. Install it?");
      } else {
        setLogs((prev) => [
          ...prev,
          `ffmpeg ${ffInfo.version ?? "(found)"} is installed`,
        ]);
        setStatus("Everything is up to date");
      }

      await refreshDeps();
    } catch (e) {
      setStatus(`Update check failed: ${e}`);
    } finally {
      if (!showFfmpegPrompt) {
        setUpdating(false);
      }
    }
  };

  const handleInstallFfmpeg = async () => {
    setShowFfmpegPrompt(false);
    setStatus("Installing ffmpeg...");
    try {
      await invoke("install_ffmpeg");
      setStatus("ffmpeg installed successfully");
      await refreshDeps();
    } catch (e) {
      setStatus(`ffmpeg install failed: ${e}`);
    } finally {
      setUpdating(false);
    }
  };

  const handleDeclineFfmpeg = () => {
    setShowFfmpegPrompt(false);
    setUpdating(false);
    setStatus("ffmpeg installation skipped");
  };

  return (
    <div className="app">
      <h1>SoundClip</h1>

      {!deps.ytdlp && (
        <div className="warning">
          yt-dlp not found. Click "Check Update" to download it.
        </div>
      )}
      {!deps.ffmpeg && !showFfmpegPrompt && (
        <div className="warning mild">
          ffmpeg not found. Audio conversion may fail for some formats. Click
          "Check Update" to install it.
        </div>
      )}

      {showFfmpegPrompt && (
        <div className="ffmpeg-prompt">
          <p>
            ffmpeg is required for audio conversion but was not found. Download
            and install it now? (~150 MB from yt-dlp/FFmpeg-Builds)
          </p>
          <div className="prompt-actions">
            <button className="primary" onClick={handleInstallFfmpeg}>
              Install ffmpeg
            </button>
            <button onClick={handleDeclineFfmpeg}>Skip</button>
          </div>
        </div>
      )}

      <div className="field">
        <label>YouTube URL</label>
        <input
          type="text"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="https://youtube.com/watch?v=..."
          disabled={downloading}
        />
      </div>

      <div className="row">
        <div className="field">
          <label>Format</label>
          <select
            value={format}
            onChange={(e) => {
              setFormat(e.target.value);
              saveCurrentSettings({ audio_format: e.target.value });
            }}
            disabled={downloading}
          >
            {FORMATS.map((f) => (
              <option key={f} value={f}>
                {f}
              </option>
            ))}
          </select>
        </div>

        <div className="field checkbox-field">
          <label>
            <input
              type="checkbox"
              checked={playlist}
              onChange={(e) => {
                setPlaylist(e.target.checked);
                saveCurrentSettings({ playlist_mode: e.target.checked });
              }}
              disabled={downloading}
            />
            Download full playlist
          </label>
        </div>
      </div>

      <div className="field">
        <label>Save to</label>
        <div className="path-row">
          <input type="text" value={savePath} readOnly />
          <button onClick={pickFolder} disabled={downloading}>
            Browse
          </button>
        </div>
      </div>

      <div className="actions">
        {!downloading ? (
          <button
            className="primary"
            onClick={handleDownload}
            disabled={!deps.ytdlp}
          >
            Download
          </button>
        ) : (
          <button className="danger" onClick={handleCancel}>
            Cancel
          </button>
        )}
        <button
          onClick={handleCheckUpdate}
          disabled={downloading || updating}
        >
          {!deps.ytdlp || !deps.ffmpeg ? "Setup / Update" : "Check Update"}
        </button>
      </div>

      <div className="progress-bar">
        <div className="progress-fill" style={{ width: `${progress}%` }} />
      </div>

      <div className="log" ref={logRef}>
        {logs.map((line, i) => (
          <div key={i} className="log-line">
            {line}
          </div>
        ))}
      </div>

      <div className="status-bar">{status}</div>
    </div>
  );
}
