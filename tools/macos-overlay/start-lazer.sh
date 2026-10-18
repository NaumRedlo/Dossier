#!/bin/bash
set -euo pipefail
# Uses the installed, unchanged client. No launchctl/global environment changes.
client='/Applications/osu!lazer.app/Contents/MacOS/osu!'
if [[ ! -x "$client" ]]; then
    echo 'Не найден установленный osu!lazer в /Applications/osu!lazer.app.' >&2
    exit 1
fi
if /usr/bin/pgrep -x 'osu!' >/dev/null; then
    echo 'Сначала закрой osu!lazer обычным способом, затем повтори запуск.' >&2
    exit 1
fi
exec env OSU_WEBSOCKET_SERVER=1 "$client"
