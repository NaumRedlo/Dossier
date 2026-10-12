# Native macOS HUD prototype

This experimental host displays the current map and screen from the **native
macOS osu!lazer** runtime log. Ordinary mode contains no sample statistics.
It does not read lazer memory, save replays, send scores or load code into the game.
Live PP, UR, accuracy and current combo are not available from this source.
The host also connects to the stock client's optional WebSocket server for
structured map metadata (see below).

The reader follows `~/Library/Application Support/osu/logs/*.runtime.log`,
selecting the log whose filename timestamp matches the running game's launch
(with two seconds of clock rounding tolerance and up to 120 seconds startup delay).
It resets when the process changes, reads only complete lines, handles truncation,
and bounds catch-up to the last 2 MiB. Unsupported or missing events are shown as
waiting/unknown, not replaced by demo values. Logs are polled at most four times
per second while the game is visible. No log content is sent to a server.
A custom logs directory can be supplied to the renderer with `DOSSIER_LAZER_LOGS`.
Log messages are not a stable telemetry API; upstream changes may need an adapter
update. Historical selected-map metadata may remain while browsing menus.

From the repository root, with Rust and Xcode Command Line Tools installed:

```sh
./tools/macos-overlay/run.sh
```

The script builds a proper `Dossier HUD Demo.app` bundle and launches it through
Launch Services. Build progress and the diagnostic log path are printed in the
terminal. It may take longer on the first build while Swift fills its module cache.

Start lazer and bring its window to the foreground. The host identifies the
installed client's bundle ID `sh.ppy.osu.lazer`, follows its largest visible
normal window, and hides when another app becomes active or the window is
minimized. It reads window bounds/PID only, not window titles or screenshots.
It does not request Accessibility or Screen Recording permissions.

The first menu item reports whether the host is waiting for lazer, cannot find a
visible game window, or has rendered the plates. Diagnostics are also written to
`tools/macos-overlay/target/hud.log` when launched through the script.

Ordinary mode uses **Dossier HUD** in the menu bar. Run with `--demo` to explicitly
enable sample scenes over lazer. In demo mode use **HUD Demo** to choose `menu`, `select`, `play`, `rest`,
`results`, `hold` or `sent`, or quit. The scenarios do not follow the real game.
To inspect the visuals without starting lazer:

```sh
./tools/macos-overlay/run.sh --preview
```

The preview option opens a dedicated resizable test window and activates it.
Ordinary overlay mode never activates the host or takes keyboard/mouse focus.

## Implementation and checks

AppKit owns a transparent, nonactivating, click-through NSPanel. The native host
tracks window geometry at 20 Hz and maps CoreGraphics screen coordinates to AppKit,
including displays positioned left of or above the primary display. It selects
the intersecting display's Retina scale and asks the existing Rust HUD renderer
for sprites at physical-pixel resolution. PNG transport converts premultiplied
pixels to straight alpha before encoding. Only one render request can be pending;
EOF, invalid responses or a two-second renderer stall close the prototype.

Demo mode uses the existing sample scenes and persistent animation state.
Ordinary mode sends small status messages and renders a native AppKit card.
The PNG/pipe bridge is for this prototype; CPU cost and latency must be measured
before treating it as a production transport.

```sh
cargo test --manifest-path tools/overlay-host/Cargo.toml --bin hud-preview
"tools/macos-overlay/target/Dossier HUD Demo.app/Contents/MacOS/macos-overlay" --self-test
"tools/macos-overlay/target/Dossier HUD Demo.app/Contents/MacOS/macos-overlay" --probe
```

`--probe` reports whether a running lazer process and usable window bounds can be
found without displaying an overlay. It does not activate the game.

Full-screen Spaces use AppKit's `canJoinAllSpaces` and `fullScreenAuxiliary` window
behaviors, plus `canJoinAllApplications` on macOS 13 and later so the panel may
join another application's fullscreen space. Actual fullscreen visibility, Stage Manager, multiple monitors and
click-through during gameplay still require checking on the user's display setup.
This host is independent of the Windows/asdf transport. Full gameplay telemetry still needs a separate source. The runtime-log adapter
only reports selected maps and screen transitions. Exclusive fullscreen is not
confirmed supported: AppKit Space flags do not guarantee visibility above that
mode. The host no longer reorders its window on every frame; this is not proof
that fullscreen warnings or visibility are fixed.

## Stock lazer WebSocket integration

Verified by inspecting the locally installed **2026.1005.1.0** assemblies:
`osu.Desktop.Program` enables `OsuWebSocketProvider` only when
`OSU_WEBSOCKET_SERVER=1`. The provider listens on localhost port **49727**.
No custom build, patch, injected component or change to macOS security is used.

1. Quit lazer normally. The launcher refuses to run while an `osu!` process exists.
2. From the repository root, run `./tools/macos-overlay/start-lazer.sh`.
3. In another terminal, run `./tools/macos-overlay/run.sh`.
4. Select a different map or change mods after the overlay connects. This client
   broadcasts changes; it does **not** send an initial state when connecting.

The launcher sets the variable only for that game process. It does not change
login items, global environment, application files or game settings.

The shipped `BeatmapStateWebSocketMessage` supplies artist, title, difficulty,
mods, adjusted star rating/BPM, map maximum combo and maximum PP. These values
are labelled as **map limits**, not current-play measurements. Zero maximum PP
or combo means unavailable for this adapter. Ruleset/mod settings are calculated
by lazer, not reconstructed from stable's mod bitmask.

This version does not publish current combo, accuracy, hit offsets or live PP
through its two registered data sources (`BeatmapStateWebSocketDataSource` and
`UserActivityWebSocketDataSource`). Neither exact UR nor live PP can be derived
from these map metadata alone. The existing stable reader cannot substitute for
this missing source on stock macOS lazer.

The connection is asynchronous, read-only, limited to localhost and 64 KiB per
message, with ping checks and a three-second reconnect delay. Data is cleared
on process change, connection failure or an invalid replacement map event.
A hung connection is discarded after a three-second ping deadline. If the API
is unavailable, the card keeps using the runtime-log map and screen. The server
is best-effort and lacks sequence IDs; upstream event ordering is not guaranteed.
No cloud/server request is involved.

Checks:

```sh
./tools/macos-overlay/run.sh --build-only
"tools/macos-overlay/target/Dossier HUD Demo.app/Contents/MacOS/macos-overlay" --self-test
python3 tools/macos-overlay/test-socket.py
```

The local socket test sends a fixture map then closes the connection and verifies
that the native consumer receives and clears it. This is not a claim that live
combo/accuracy or fullscreen work. The stock client must be restarted with its
API enabled before a real-client API check can be performed.
