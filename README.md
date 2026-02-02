# SoundClip

A minimal desktop app for downloading YouTube audio. Built with Tauri v2 + React + Rust.

## Features

- Paste a YouTube URL and download audio in your chosen format
- Format selection: best / mp3 / m4a / opus / flac / wav / aac / alac / vorbis
- Single track or full playlist download
- Custom save folder with persistent settings
- One-click install and update for both **yt-dlp** and **ffmpeg**
- Real-time progress bar and streaming log output
- Hardened yt-dlp commands: safe filename templates, `--windows-filenames`, process tree cancellation

## How It Works

SoundClip spawns [yt-dlp](https://github.com/yt-dlp/yt-dlp) as a child process with flags like:

```
yt-dlp -x --audio-format mp3 --no-playlist \
  -P "home:C:\Users\you\Music" \
  -o "%(title).200s [%(id)s].%(ext)s" \
  --windows-filenames --ffmpeg-location <bin_dir> \
  --newline --no-colors <URL>
```

All binaries are managed in `%LOCALAPPDATA%\SoundClip\bin\`:

| Binary | Source | Install method |
|--------|--------|----------------|
| `yt-dlp.exe` | [yt-dlp/yt-dlp](https://github.com/yt-dlp/yt-dlp/releases) | GitHub Releases API, atomic download |
| `ffmpeg.exe` | [yt-dlp/FFmpeg-Builds](https://github.com/yt-dlp/FFmpeg-Builds/releases) | ZIP download + extraction |
| `ffprobe.exe` | Same as above | Extracted alongside ffmpeg |

## Prerequisites

- **Windows 10/11** (x64)
- **Node.js** >= 18
- **Rust** >= 1.70
- **WebView2** (pre-installed on Windows 10 21H2+ and Windows 11)

## Development

```bash
# Install dependencies
npm install

# Run in dev mode (hot reload)
npx tauri dev

# Production build
npx tauri build
```

Build output:
- `src-tauri/target/release/soundclip.exe` (standalone binary)
- `src-tauri/target/release/bundle/msi/SoundClip_1.0.0_x64_en-US.msi`
- `src-tauri/target/release/bundle/nsis/SoundClip_1.0.0_x64-setup.exe`

## First Run

1. Launch the app
2. Click **Setup / Update** to automatically download yt-dlp and ffmpeg
3. Paste a YouTube URL, pick a format and save folder, click **Download**

## Project Structure

```
SoundClip/
├── src/                    # React frontend
│   ├── App.tsx             # Main UI component
│   ├── App.css             # Styles (dark theme)
│   └── main.tsx            # Entry point
├── src-tauri/src/          # Rust backend
│   ├── commands.rs         # Tauri command handlers
│   ├── downloader.rs       # yt-dlp process management
│   ├── updater.rs          # GitHub API update + ffmpeg installer
│   ├── settings.rs         # JSON persistence
│   ├── utils.rs            # Path helpers, ffmpeg check, process tree kill
│   ├── lib.rs              # Tauri plugin/command registration
│   └── main.rs             # Entry point
├── src-tauri/tauri.conf.json
├── package.json
├── tsconfig.json
└── vite.config.ts
```

## Settings

Stored at `%LOCALAPPDATA%\SoundClip\settings.json`:

```json
{
  "save_path": "C:\\Users\\you\\Music",
  "audio_format": "mp3",
  "playlist_mode": false
}
```

## License

MIT
