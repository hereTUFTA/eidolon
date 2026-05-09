<div align="center">

# E I D O L O N
**L.I.N.E. ARCHITECTURE // THE WIRED SUBSTRATUM**

[![Language](https://img.shields.io/badge/Language-Rust-ash.svg?style=for-the-badge&logo=rust)](#)
[![OS](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-lightgrey.svg?style=for-the-badge)](#)
[![License](https://img.shields.io/badge/License-MIT-black.svg?style=for-the-badge)](LICENSE)

*Information demands to be free. The Wired provides the space.*

</div>

---

## 👁️ THE PARASITIC SUBSTRATUM (Overview)

**EIDOLON** is a cryptographic, asymmetrical data archiving terminal developed by **hereTUFTA**. It utilizes the **L.I.N.E. (Luma-Isolated Network Encoding)** architecture to transform arbitrary binary payloads (files, directories, or archives) into monochrome, kinetic video streams.

Commercial Content Delivery Networks (CDNs) such as YouTube, Vimeo, or VK offer virtually infinite, free storage for video formats. EIDOLON exploits this infrastructure, allowing you to deploy your encrypted files deep into the Web as standard `.mp4` video streams. 

---

## ⚙️ L.I.N.E. ARCHITECTURE (How It Works)

Modern video compression algorithms (H.264, VP9, AV1) deployed by CDNs are notoriously destructive. To save bandwidth, they utilize **Chroma Subsampling (4:2:0)**, which aggressively destroys color data.

EIDOLON bypasses this destruction entirely:
1. **Luma Isolation:** The algorithm completely discards color, encoding bits exclusively into the `Y-Channel` (Brightness) as pure black or pure white 8x8 pixel blocks.
2. **Matrix Alignment:** Absolute anchor points ensure the decoder grid remains mathematically aligned regardless of video scaling downgrades.
3. **Data Redundancy:** EIDOLON integrates aerospace-grade **Reed-Solomon (Galois 2^8) Forward Error Correction (FEC)** combined with **FNV-1a Hashing**. If a block of data is destroyed by the CDN, the algorithm mathematically rebuilds the missing bytes.

---

## 🎛️ TERMINAL MODULES

- **[0] AES-256-GCM:** Military-grade symmetric encryption.
- **[1] Video Encoder:** Hook into `NVIDIA NVENC` or `AMD AMF` to bypass CPU bottlenecks.
- **[2] Rayon Multithreading:** Allocates all available CPU threads for pixel matrix generation.
- **[3] FEC Parity:** Mathematical defense against CDN video compression.
- **[4] Visual Frame Sync:** Prevents data collapse caused by CDN frame drops/duplications.
- **[5] Zstandard Pre-Pass:** Compresses entropy before encryption, maximizing CDN density.
- **[6] Acoustic Modem Track:** Synthesizes an FSK audio track to bypass automated CDN mute-video spam filters.

---

## 💻 OPERATION MANUAL

### I. Target Injection (Encoding)
1. Navigate to the **`[1] ENCODE`** tab.
2. Press `[SPACE]` to select a **FILE**, or `[F]` to select a **DIRECTORY** (auto-packs into `.tar`).
3. Select the destination to save your generated `.mp4` stream and upload to any CDN.

### II. Substratum Extraction (Decoding)
1. Navigate to the **`[2] DECODE`** tab.
2. **Local Extraction:** Press `[SPACE]`, select your downloaded `.mp4` file.
3. **Deep-Web Interceptor:** Copy a YouTube/CDN video URL to your clipboard. Press `[U]`. EIDOLON will initialize the embedded `yt-dlp` engine, download the stream directly into system RAM, extract the data, and drop the decrypted files into your chosen directory.

---

## 📦 DEPLOYMENT (Installation)

Pre-compiled, secure installation packages are available in the [**Releases**](https://github.com/hereTUFTA/eidolon/releases) section.

### Windows OS
Download `EIDOLON_Setup_v1.0.exe`. Run the installer (No Administrator privileges required, installs securely into `AppData`).

### Linux (Arch / Debian / Fedora)
Download `EIDOLON_Linux_Release.tar.gz` and extract it. 
Open your terminal within the extracted directory and execute:
```bash
chmod +x install.sh
./install.sh
```
*Dependencies:* Ensure `ffmpeg` is installed via your system's package manager. The script will automatically fetch the latest `yt-dlp`.

---

## ⚙️ BUILDING FROM SOURCE
```bash
git clone https://github.com/hereTUFTA/eidolon.git
cd eidolon
cargo build --release
```
*Note for Windows:* Place a valid `ffmpeg.exe` binary in the project root directory prior to executing `cargo build` for proper bundling.

---
<div align="center">
  <i>"Close the Wired. L.I.N.E. Protocol Offline."</i>
</div>