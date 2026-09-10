#!/usr/bin/env bash
# Open a site as its own window in the first Chromium-family browser installed: WebHID and WebUSB
# only exist there, so the keyboard configurators need one.
#     webapp.sh https://launcher.keychron.com
set -euo pipefail
url="${1:?url}"
for b in vivaldi-stable vivaldi chromium google-chrome-stable google-chrome brave; do
    if command -v "$b" >/dev/null; then exec "$b" --ozone-platform-hint=auto "--app=$url"; fi
done
notify-send "No browser for $url" "A Chromium-family browser is needed: Vivaldi, Chromium, Chrome or Brave."
exit 1
