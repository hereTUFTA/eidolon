<div align="center">

```
███████╗██╗██████╗  ██████╗ ██╗      ██████╗ ███╗   ██╗
██╔════╝██║██╔══██╗██╔═══██╗██║     ██╔═══██╗████╗  ██║
█████╗  ██║██║  ██║██║   ██║██║     ██║   ██║██╔██╗ ██║
██╔══╝  ██║██║  ██║██║   ██║██║     ██║   ██║██║╚██╗██║
███████╗██║██████╔╝╚██████╔╝███████╗╚██████╔╝██║ ╚████║
╚══════╝╚═╝╚═════╝  ╚═════╝ ╚══════╝ ╚═════╝ ╚═╝  ╚═══╝
```

**L.I.N.E. ARCHITECTURE // LUMA-ISOLATED NETWORK ENCODING**

[![Language](https://img.shields.io/badge/Language-Rust-CE422B.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-4A4A4A.svg?style=flat-square)](#installation)
[![License](https://img.shields.io/badge/License-MIT-2A2A2A.svg?style=flat-square)](LICENSE)
[![Release](https://img.shields.io/github/v/release/hereTUFTA/eidolon?style=flat-square&color=3A3A3A)](https://github.com/hereTUFTA/eidolon/releases)

*Encrypt. Encode. Distribute. Leave no trace.*

</div>

---

## Overview

**EIDOLON** is a terminal-based cryptographic archiving tool that encodes arbitrary binary data — files or entire directories — into standard `.mp4` video streams using the **L.I.N.E. (Luma-Isolated Network Encoding)** protocol.

The encoded video is indistinguishable from ordinary content to automated systems. It can be uploaded to any public video CDN — YouTube, Vimeo, VK — and later retrieved and decoded back to its exact original form. The payload is encrypted end-to-end with AES-256-GCM before encoding begins. Without the correct key, the data is cryptographically inaccessible.

**Use cases:**
- Encrypted long-term file archival using CDN infrastructure as a storage backend
- Distributing encrypted data payloads through conventional video hosting platforms
- Preserving files in a format resilient to automated content filtering systems

---

## How It Works

Modern video compression (H.264, VP9, AV1) is destructive by design. CDNs apply chroma subsampling (4:2:0), aggressively discarding color information to reduce bandwidth. EIDOLON bypasses this entirely through three mechanisms:

### 1 — Luma Isolation
All data is encoded exclusively into the **Y-channel** (luma/brightness) of the video signal — never color. Each bit is represented as an 8×8 pixel block of pure black (`0`) or pure white (`255`). Luma information survives CDN recompression far better than chroma.

### 2 — Spatial Anchoring
Fixed high-contrast anchor blocks are placed at the four corners of each frame. These provide absolute reference points for the decoder, ensuring the data grid remains mathematically aligned even after CDN rescaling or quality degradation.

### 3 — Forward Error Correction
Before encoding, the payload is passed through **Reed-Solomon (GF 2⁸) FEC** with a 8-data / 4-parity shard configuration. If CDN compression destroys a portion of the data, the missing bytes are mathematically reconstructed during decoding. Individual shard integrity is verified using **FNV-1a hashing**.

### Full Encoding Pipeline

```
Input File / Directory
        │
        ▼
[TAR packing if directory]
        │
        ▼
[Zstandard compression]  ← optional
        │
        ▼
[AES-256-GCM encryption]
        │
        ▼
[Reed-Solomon FEC]        ← optional
        │
        ▼
[L.I.N.E. frame encoding → raw grayscale frames]
        │
        ▼
[FFmpeg muxing → .mp4 output]
```

The decoder runs this pipeline in reverse. The input video is never modified — it is streamed frame-by-frame through FFmpeg, decoded in memory, and the original file is reconstructed without writing any intermediate artifacts to disk.

---

## Interface

EIDOLON runs as a terminal UI application built with [Ratatui](https://ratatui.rs/). Navigation uses keyboard shortcuts only. Four tabs are accessible via number keys `1–4` or `Tab` / arrow keys.

### `[1] ENCODE` — Target Injection

Encodes a file or directory into a video stream.

| Key | Action |
|-----|--------|
| `Space` | Select a **file** to encode |
| `F` | Select a **directory** to encode (auto-packed into `.tar`) |

After selecting the input, a save dialog prompts for the output `.mp4` path. Encoding begins immediately. A progress overlay displays until completion.

---

### `[2] DECODE` — Substratum Extraction

Extracts and decrypts a previously encoded payload from a video stream.

| Key | Action |
|-----|--------|
| `Space` | Select a **local `.mp4` file** for extraction |
| `U` | Read a **URL from clipboard** and download + extract via `yt-dlp` |

For URL-based extraction, copy a CDN video link to your clipboard before pressing `U`. EIDOLON will invoke `yt-dlp` to download the stream, pipe it through the extraction pipeline, and write the decrypted output to a directory you select. The temporary download is deleted after extraction.

Supported local formats: `.mp4`, `.mkv`, `.webm`

---

### `[3] SETTINGS` — Protocol Parameters

Navigate settings with `↑` / `↓` arrow keys. Toggle or modify with `Space` / `Enter`.

| Setting | Description |
|---------|-------------|
| **Access Key** | AES-256-GCM symmetric encryption key. The same key must be used for both encoding and decoding. An empty key disables encryption. |
| **Engine Core** | Video encoding backend. Cycles between `CPU (libx264)`, `NVENC (Nvidia)`, and `AMF (AMD)`. Hardware acceleration bypasses CPU bottlenecks for large payloads. |
| **Matrix Threading** | Enables Rayon-based multi-core parallelism for pixel matrix generation. Recommended: enabled. |
| **Parity Injection** | Enables Reed-Solomon FEC. Adds ~50% overhead to the output video size but provides strong resilience against CDN compression artifacts. Recommended for CDN uploads. |
| **Frame Synchronization** | Embeds a 32-bit deterministic index into each frame. Protects against data corruption from CDN frame drops or reordering. Recommended: enabled. |
| **Entropy Compression** | Applies Zstandard (level 3) compression before encryption. Reduces payload size and therefore output video length. Recommended: enabled. |
| **Acoustic Modem Track** | Synthesizes an FSK (Frequency-Shift Keying) audio track derived from the payload data and muxes it into the output video. Does not carry recoverable data. Designed to prevent CDN platforms from flagging silent-video uploads as spam. |

Settings are persisted to `~/.config/eidolon_config.txt` (Linux) or `%APPDATA%\eidolon_config.txt` (Windows) automatically on change.

---

### `[4] SYSTEM LOGS` — Terminal Diagnostics

Displays the full operation log for the current session. Shows all pipeline steps, progress messages, and error output. The last four log entries are also visible in the status bar at the bottom of every tab.

---

## Installation

### Windows

Download `EIDOLON_Setup_v1.0.0.exe` from the [Releases](https://github.com/hereTUFTA/eidolon/releases) page.

Run the installer. Administrator privileges are **not required** — the application installs into `%LOCALAPPDATA%\Programs\EIDOLON`. `ffmpeg.exe` and `yt-dlp.exe` are bundled and require no separate installation.

---

### Linux (Arch / Debian / Ubuntu / Fedora)

Download `EIDOLON_Linux_Setup_v1.0.0.zip` from the [Releases](https://github.com/hereTUFTA/eidolon/releases) page and extract it.

**Required dependencies** (install via your package manager before running):

```bash
# Arch
sudo pacman -S ffmpeg yt-dlp

# Debian / Ubuntu
sudo apt install ffmpeg yt-dlp

# Fedora
sudo dnf install ffmpeg yt-dlp
```

**Install:**

```bash
cd EIDOLON_Linux_Release
chmod +x install.sh
./install.sh
```

The installer deploys the binary to `~/.local/bin/eidolon`, creates a `.desktop` entry for application launchers, and installs the icon. No root access required.

**Uninstall:**

```bash
chmod +x uninstall.sh
./uninstall.sh
```

Removes all files including configuration and temporary artifacts.

---

## Building from Source

Requires [Rust](https://rustup.rs/) toolchain (edition 2024).

```bash
git clone https://github.com/hereTUFTA/eidolon.git
cd eidolon
cargo build --release
```

The compiled binary is written to `target/release/EIDOLON` (Linux) or `target/release/EIDOLON.exe` (Windows).

**Windows note:** Place `ffmpeg.exe` in the project root before building for proper bundling by the build script.

**Runtime dependencies:** `ffmpeg` must be available in `PATH` or placed in the same directory as the EIDOLON binary. `yt-dlp` is required only for URL-based decoding.

---

## Security Model

- **Encryption:** AES-256-GCM with a SHA-256 derived key. Authentication tag validation guarantees payload integrity — corrupted or tampered data will fail decryption.
- **Key storage:** The access key is stored in plaintext in the config file. Treat it accordingly.
- **No backdoor:** There is no master key or recovery mechanism. A lost key means the payload is permanently inaccessible.
- **CDN threat model:** The CDN host is treated as a fully untrusted, surveilled environment. Security depends entirely on AES-256-GCM and the strength of the user-supplied key.
- **Audio track:** The FSK acoustic track carries no recoverable data and provides no cryptographic function.
- **FEC limits:** Reed-Solomon FEC tolerates up to 4 destroyed shards out of 12. Extreme CDN downscaling (e.g., forced 144p) may result in permanent data loss.

Report cryptographic vulnerabilities via a GitHub Issue tagged `[SECURITY]`.

---

## Technical Specifications

| Parameter | Value |
|-----------|-------|
| Frame resolution | 1920 × 1080 |
| Block size | 8 × 8 px |
| Usable blocks per frame | 220 × 115 = 25,300 |
| Bits per frame (no sync) | 25,300 |
| Bits per frame (with sync) | 25,268 (32 bits reserved for frame index) |
| Frame rate | 6 fps |
| FEC configuration | 8 data / 4 parity shards (GF 2⁸) |
| Encryption | AES-256-GCM |
| Key derivation | SHA-256 |
| Compression | Zstandard level 3 |
| Maximum payload | 1,000 MB |
| Audio sample rate | 44,100 Hz, 16-bit mono |

---

## License

MIT — see [LICENSE](LICENSE).

---

<div align="center">
<sub>EIDOLON // hereTUFTA // 2026</sub>
</div>
