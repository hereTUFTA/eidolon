# Changelog

All notable changes to EIDOLON are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [1.0.0] — 2026-05-10

### Initial Release

**Core Protocol**
- L.I.N.E. (Luma-Isolated Network Encoding) architecture for encoding arbitrary binary data into standard `.mp4` video streams
- Luma-channel-only encoding (Y-plane) with 8×8 pixel blocks — survives CDN chroma subsampling
- Spatial anchor blocks at frame corners for CDN-resilient grid alignment
- 1920×1080 resolution, 6fps, 25,300 usable bits per frame

**Encryption**
- AES-256-GCM end-to-end payload encryption
- SHA-256 key derivation from user-supplied passphrase
- Authentication tag validation on decode — detects tampering or wrong key

**Error Correction**
- Reed-Solomon FEC (GF 2⁸), 8 data / 4 parity shard configuration
- FNV-1a per-shard integrity hashing
- Tolerates up to 4 destroyed shards out of 12

**Compression**
- Zstandard level 3 pre-encryption compression
- Optional — configurable per session

**Frame Synchronization**
- 32-bit deterministic frame index injected into each frame
- Protects against CDN frame reordering and drop artifacts

**Encoding Pipeline**
- Streaming FFmpeg pipe — frames generated and piped in real time, no intermediate files written to disk
- Hardware acceleration: CPU (libx264), NVENC (Nvidia), AMF (AMD)
- Rayon-based multi-core parallel frame matrix generation

**Decoding Pipeline**
- FFmpeg-based frame extraction with nearest-neighbor rescaling
- URL-based decode via yt-dlp integration (clipboard → download → extract)
- Supports local `.mp4`, `.mkv`, `.webm` inputs
- Folder payloads automatically unpacked from embedded TAR archive

**Acoustic Track**
- Optional FSK modem audio track muxed into output video
- Cosmetic anti-spam measure for CDN silent-video filters
- No cryptographic or data-carrying function

**Interface**
- Ratatui TUI — four tabs: ENCODE, DECODE, SETTINGS, SYSTEM LOGS
- Persistent config (`eidolon_config.txt`) — key and all settings saved automatically
- Real-time progress overlay during encode/decode
- Output file size and elapsed time reported on completion
- Warning displayed when encryption key is empty

**Platform Support**
- Windows: self-contained installer (Inno Setup), no admin rights required, ffmpeg + yt-dlp bundled
- Linux (Arch / Debian / Ubuntu / Fedora): shell installer, yt-dlp auto-fetched if absent

---

*EIDOLON // hereTUFTA // 2026*