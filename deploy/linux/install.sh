#!/bin/bash
clear
echo -e "\033[1;37m"
echo "========================================================="
echo "          E I D O L O N  //  L.I.N.E. ARCHITECTURE"
echo "========================================================="
echo -e "\033[0m"
echo -e "\033[1;36m[KERNEL]\033[0m Initializing Substratum Deployment..."
echo ""

# --- Dependency: FFmpeg ---
if ! command -v ffmpeg &> /dev/null; then
    echo -e "\033[1;33m[!] WARNING: FFmpeg engine not detected.\033[0m"
    echo "    EIDOLON requires FFmpeg for L.I.N.E. Protocol multiplexing."
    echo "    Install via your package manager:"
    echo -e "\033[1;36m    Arch:    sudo pacman -S ffmpeg"
    echo -e "    Debian:  sudo apt install ffmpeg"
    echo -e "    Fedora:  sudo dnf install ffmpeg\033[0m"
    echo ""
else
    echo -e "\033[1;32m[+] FFmpeg engine verified.\033[0m"
fi

# --- Dependency: yt-dlp ---
if command -v yt-dlp &> /dev/null; then
    echo -e "\033[1;32m[+] yt-dlp binary verified.\033[0m"
else
    echo -e "\033[1;36m[NET]\033[0m yt-dlp not found. Attempting automatic deployment..."

    YTDLP_URL="https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"
    YTDLP_DEST="$HOME/.local/bin/yt-dlp"
    YTDLP_OK=0

    if command -v curl &> /dev/null; then
        curl -fsSL "$YTDLP_URL" -o "$YTDLP_DEST" && YTDLP_OK=1
    elif command -v wget &> /dev/null; then
        wget -q "$YTDLP_URL" -O "$YTDLP_DEST" && YTDLP_OK=1
    fi

    if [ "$YTDLP_OK" -eq 1 ]; then
        chmod +x "$YTDLP_DEST"
        echo -e "\033[1;32m[+] yt-dlp deployed to ~/.local/bin/yt-dlp\033[0m"
    else
        echo -e "\033[1;33m[!] Auto-download failed (no curl or wget, or no network access).\033[0m"
        echo "    yt-dlp is required ONLY for URL-based stream interception."
        echo "    Install manually via your package manager:"
        echo -e "\033[1;36m    Arch:    sudo pacman -S yt-dlp"
        echo -e "    Debian:  sudo apt install yt-dlp"
        echo -e "    Fedora:  sudo dnf install yt-dlp\033[0m"
        echo "    Or: pip install yt-dlp"
    fi
    echo ""
fi

echo -e "\033[1;36m[FS]\033[0m Generating local hierarchy..."
mkdir -p ~/.local/bin ~/.local/share/applications ~/.local/share/icons/eidolon

echo -e "\033[1;36m[SYS]\033[0m Deploying core binary..."
cp ./EIDOLON ~/.local/bin/eidolon
chmod +x ~/.local/bin/eidolon

echo -e "\033[1;36m[UI]\033[0m Linking visual assets..."
cp ./icon.png ~/.local/share/icons/eidolon/icon.png

cat <<EOF > ~/.local/share/applications/eidolon.desktop
[Desktop Entry]
Name=EIDOLON
Comment=L.I.N.E. Protocol // The Wired Substratum
Exec=eidolon
Icon=$HOME/.local/share/icons/eidolon/icon.png
Terminal=true
Type=Application
Categories=Utility;Security;
EOF

chmod +x ~/.local/share/applications/eidolon.desktop

echo ""
echo -e "\033[1;32m=========================================================\033[0m"
echo -e "\033[1;32m[+] EIDOLON DEPLOYMENT COMPLETE. ACCESS GRANTED.\033[0m"
echo -e "\033[1;32m=========================================================\033[0m"