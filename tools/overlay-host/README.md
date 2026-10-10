# Experimental Windows overlay host

This separate workspace implements transport for Witness's tile, PP and unstable
rate plates inside stable's graphics surface. It is not enabled by Dossier settings and is not a
release component. Witness itself remains a read-only telemetry process; this
explicit companion loads an asdf overlay DLL into the selected game process.

Build on Windows with Rust 1.92 or newer and the MSVC toolchain:

```text
cargo build --manifest-path tools/overlay-host/Cargo.toml --locked --release --target x86_64-pc-windows-msvc
```

Supply x86 and x64 asdf overlay DLLs built from upstream revision
`c057c2a4b469d8079dbce39f41fdfeedf5060f86`, the same revision pinned by this host.
DLLs are not bundled or downloaded at runtime. The x86 DLL is needed for stable;
the host can run as x64. With stable running, use its actual PID in `cmd.exe`:

```bat
witness.exe --overlay | dossier-overlay-host.exe 1234 C:\overlay\asdf-overlay-x86.dll C:\overlay\asdf-overlay-x64.dll
```

The host waits for a fresh stable snapshot matching the chosen PID before loading
the DLL. It selects the graphics adapter by the surface's LUID, honors its mutex
mode, composites the available premultiplied RGBA plates into a transparent BGRA
texture cropped to their bounds, and uploads at up to 20 updates per second.
Each surface retains its animation state; unchanged static scenes skip both
rasterization and GPU upload. Resizing forces a redraw. The composite allocation
is limited to 64 MiB; an oversized scene is hidden rather than retaining old pixels.
There is no input interception,
replay recording, or bot sending in this prototype. PP and UR require the relevant
telemetry; absent values are not invented. A restarted game with another
PID requires restarting the command.

## Dossier context

With this version of Dossier running and Witness attached, the application exports
`witness-context.json` in its `.dossier` folder. Add that path as the optional fourth
argument (in `cmd.exe`):

```bat
witness.exe --overlay | dossier-overlay-host.exe 1234 asdf-overlay-x86.dll asdf-overlay-x64.dll "%USERPROFILE%\.dossier\witness-context.json"
```

The first producer supplies the UI language and the first matching non-collection
pool from Dossier's loaded pool list, matching by beatmap MD5. Open Pools in Dossier
to load that list before selecting a map. Only NM slot measurements supply stars;
modified difficulty is not assumed to match the game's current selection.

The file is replaced atomically every five seconds, contains no credentials, and
is read by the host at most once per second. Context is scoped to the attached PID;
map cards also require the matching MD5. Files over 64 KiB, malformed input,
unsupported versions, future timestamps and context at least 15 seconds old are
ignored. Expiring context forces a redraw even when telemetry has not changed.
There is no network access from the graphics host.

The transport schema also accepts a day summary, but Dossier does not populate it
yet: the currently available data is insufficient for a complete daily summary.
Saved replay outcomes and bot-send acknowledgments are not transported yet.

Telemetry older than two seconds hides the tile. EOF, malformed input or Ctrl+C
ends the host, with a best-effort request to hide its textures first. IPC requests
have time limits. The upstream GPU texture lock can still block indefinitely;
the separate process isolates that failure from Witness and Dossier. Disconnect
does not unload an already injected DLL; restart the game to remove it completely.

## Validation gate before integration

The host has passed cross-compilation checking for Windows MSVC and portable
tests. Presentation has not been verified on a Windows GPU. Check the tile in
stable menus, gameplay, results, windowed/fullscreen modes, after Alt+Tab and
resize. Verify PP/UR, folding in breaks, and transparency between the plates.
Stop Witness and verify all plates disappear within two seconds; close
the game and verify the host exits. Compare frame times with and without the
host and test on hybrid-GPU systems before automatic startup.

The tested CrossOver 26.3 bottle lacks the needed shared-texture path (see
`docs/witness-hud.md`); it cannot establish that Windows presentation works.
Lazer telemetry and deployment are separate later steps.
