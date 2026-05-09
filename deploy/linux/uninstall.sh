#!/bin/bash
clear
echo -e "\033[1;31m"
echo "========================================================="
echo "      E I D O L O N  //  P U R G E  P R O T O C O L"
echo "========================================================="
echo -e "\033[0m"
echo -e "\033[1;36m[KERNEL]\033[0m Initiating System Purge..."

echo -e "\033[1;36m[FS]\033[0m Erasing core binary..."
rm -f ~/.local/bin/eidolon

echo -e "\033[1;36m[UI]\033[0m Destroying visual assets and desktop links..."
rm -rf ~/.local/share/icons/eidolon
rm -f ~/.local/share/applications/eidolon.desktop

echo -e "\033[1;36m[SYS]\033[0m Wiping persistent configuration and temporary artifacts..."
rm -f ~/.config/eidolon_config.txt
rm -rf /tmp/eidolon_*

echo ""
echo -e "\033[1;32m=========================================================\033[0m"
echo -e "\033[1;32m[+] EIDOLON ERADICATED. ZERO TRACE REMAINING.\033[0m"
echo -e "\033[1;32m=========================================================\033[0m"