# Security Policy // EIDOLON L.I.N.E. Protocol

## Supported Versions
Only the latest release (`v1.0.x`) receives security patches.

## Cryptographic Guarantees
EIDOLON utilizes `AES-256-GCM` for payload encryption and `Sha256` for key hashing. 
The host Video CDN (YouTube, VK, etc.) is considered an **Untrusted Environment**. The protocol assumes the CDN operates under full surveillance. 
Security relies entirely on the strength of your User Password. 

## Known Vectors & Limitations
- **Key Loss:** Data is unrecoverable without the exact key. There is no backdoor or master key.
- **Audio Track:** The acoustic modem track is synthesized using FSK (Frequency-Shift Keying). It does **NOT** contain recoverable data and is designed solely for CDN spam-filter evasion.
- **Extreme Compression:** While Reed-Solomon FEC provides high fault tolerance, aggressive 144p downscaling by a CDN may result in permanent data loss.

## Reporting a Vulnerability
If you discover a cryptographic flaw in the `core.rs` encryption pipeline, please open an Issue with the `[SECURITY]` tag in the repository.