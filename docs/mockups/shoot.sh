#!/usr/bin/env bash
# Screenshot a static mockup with headless Chrome (no server): ./shoot.sh a-clay-spotlight [width] [height]
set -e
name="$1"; w="${2:-1600}"; h="${3:-1000}"
dir="$(cd "$(dirname "$0")" && pwd -W 2>/dev/null || pwd)"
"/c/Program Files/Google/Chrome/Application/chrome.exe" --headless=new --disable-gpu --hide-scrollbars \
  --force-device-scale-factor=1 --virtual-time-budget=10000 --window-size="$w,$h" \
  --screenshot="$dir/$name.png" "file:///$dir/$name.html" 2>&1 | tail -1
