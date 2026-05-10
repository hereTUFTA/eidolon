# Security Policy // EIDOLON L.I.N.E. Protocol

## Supported Versions
Only the latest release (`v1.1.x`) receives security patches.

## Cryptographic Guarantees
EIDOLON utilizes `AES-256-GCM` for payload encryption and `Sha256` for key hashing.
The host Video CDN (YouTube, VK, etc.) is considered an **Untrusted Environment**. The protocol assumes the CDN operates under full surveillance.
Security relies entirely on the strength of your user-supplied password.

## Known Vectors & Limitations
- **Key Loss:** Data is unrecoverable without the exact key. There is no backdoor or master key.
- **Empty Key:** If the Access Key field is left empty, the payload is stored without encryption. Any party with access to the video and the EIDOLON decoder can extract the data.
- **Audio Track:** The FSK acoustic modem track is a cosmetic addition with no cryptographic function. It does not carry encoded payload data and provides no additional security or data redundancy. Its sole purpose is to avoid CDN silent-video spam filters. Removing it has no effect on encode/decode correctness.
- **Extreme Compression:** While Reed-Solomon FEC provides high fault tolerance, aggressive 144p downscaling by a CDN may result in permanent data loss beyond the FEC recovery threshold.

## Reporting a Vulnerability
If you discover a cryptographic flaw in the `core.rs` encryption pipeline, please open an Issue with the `[SECURITY]` tag in the repository.
