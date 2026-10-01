# Witness

Witness is the part of Dossier that watches a running osu! client. It reads what
the client holds in memory and tells the application; it writes nothing, injects
nothing and sends no input. A witness sees what happened and says so, and does
not take part.

It is a program of its own (`witness.exe`, crate `crates/dossier-witness`), not
code inside the window: it has to run where the client runs, a crash or an
antivirus's opinion of it must not take the application down, and one small
program can serve the application, overlays and whatever comes after them.

## What it is for

- **Every play in the journal.** stable keeps a replay only when asked or when a
  play enters the local ranking. The client records the frames of the current
  play into the score it holds while the play runs — fails included — so a play
  can be written out as a replay the moment it ends, and rendered later.
- **The chat beside song select.** The map under the cursor is known at once, so
  the application can show how the chat did on it without being asked.
- **The client's own verdicts.** Hits, misses and hit errors as the client
  counted them, to set beside what the engine's simulation says and find the
  first object where they part (ENG-04).
- Later: a live "playing now", watching another Dossier player through the
  engine's own renderer, overlays for streams.

## Where it can work

| | stable | lazer |
|---|---|---|
| Windows | directly | possible, later |
| Linux, Wine | inside the client's prefix | possible, later |
| macOS, Wine | inside the client's prefix only | not possible |

macOS does not let one program read another signed with the hardened runtime
and without the debugging entitlement while System Integrity Protection is on;
osu!lazer and CrossOver's loader are both signed that way (checked 2026-10-01).
Inside one Wine prefix, though, a Windows program reads another through the
prefix's own server, so the same `witness.exe` that runs on Windows runs beside
the client on a Mac and on Linux. That is also how it is developed: the author
has only a Mac, so Windows itself is first tried by others after a release.

## How it finds things

stable is 32-bit managed code; what is read is the runtime's memory, not a file
format, so nothing is at a fixed address.

- **Anchors** are short byte patterns in the compiled code that sits next to the
  address of a static field. The four in `stable.rs` (the base, the screen, the
  play time, the rulesets) are the ones the open memory readers have used for
  years; tosu (LGPL-3.0) and gosumemory document them, and they are taken as
  facts about the client, not as code.
- **Objects** are walked from an anchor by offsets: a string is its length at +4
  and UTF-16 from +8, a list is its array at +4 and its count at +0xC.
- **What a field means** comes from reading the client with ILSpy
  (`docs/stable-client.md`): the decompiled source says that the score holds the
  six hit counts, the mods, the player's name and the list of replay frames, and
  that the frames are added during play. Where exactly the runtime puts each
  field is found on a running client, never guessed from the source.
- Every read is checked for sense (a hash is 32 hex digits, a length is not
  negative, a screen is one of the known ones). A client the patterns do not
  know is reported as such and left alone.

What the bench has shown (2026-10-01, stable 20251102 under CrossOver, offline):

- The four anchors are found once the game itself has loaded; before that the
  code they sit in has not been compiled yet, so a client still at its first
  dialog is simply "not ready", not "unknown".
- The score being played holds its replay frames in a list at +0x34: objects
  with x at +4, y at +8, the keys at +0xC and the time at +0x10. Its first frame
  is the one the client itself inserts (0, 256, −500). The life bar is a list of
  time and health pairs at +0x24, the hit errors a list of integers at +0x38.
- The runtime moves objects while the game runs: the score changed its address
  in the middle of a play. A play is therefore known by its frames — the last
  frame read is still where it was — never by the address of its score.
- A play is over when the screen leaves Play; reaching the results means it was
  passed, and the score shown there is the same object with its last frames.
- The replay written from memory was given to the engine: its judgement of
  that replay matched the client's own counts exactly (84 / 14 / 1 / 0, combo
  188, 89.73%).

`witness --record DIR --player NAME` does this: it follows the client and
writes every play of 120 frames or more as an `.osr` into the folder. The
client's own name is used when it has one; offline it has none, and the name
given is written instead.

The client also has a door of its own: `InterProcessOsu`, a .NET Remoting object
whose `GetBulkClientData` tells the screen, the map's hash and id and the audio
time. It tells the score only in tournament mode, so it cannot replace reading,
but it is a second source for the map under the cursor.

## The bench on a Mac

A CrossOver bottle named `osu-stable` (Windows 10, 64-bit) with the real .NET
Framework 4.8 put in by winetricks, because the client must run on the same
runtime as on Windows for its memory to look the same; Mono would lay it out
differently. winetricks is run through CrossOver's own environment:

    wine --bottle osu-stable --ux-app /bin/sh -c \
      'WINE="$CX_ROOT/bin/wineloader" winetricks -q dotnet48'

Witness is built on the Mac for Windows and run in the bottle:

    cargo build --release -p dossier-witness --target x86_64-pc-windows-gnu
    wine --bottle osu-stable target/x86_64-pc-windows-gnu/release/witness.exe --watch 20

`--process NAME` reads another process, `--regions` tells how its memory is
labelled. The build needs `mingw-w64` and the `x86_64-pc-windows-gnu` target;
the linker is named in `.cargo/config.toml`.

## In the application

Witness runs whenever the application does: it keeps one Witness beside the
client and listens to it. The tile in the settings says where it stands and has
one switch, for keeping every play's replay in the journal, off until the
person turns it on. (Until 2026-10-01 that switch started Witness itself; a
setting saved then is read as the new one.)

- The program travels inside the application: the release builds
  `witness.exe` once, for Windows, and every archive carries it; it is written
  out to `~/.dossier/bin` when first needed. A build made without it says so in
  the tile. `DOSSIER_WITNESS_EXE` names the program at build time,
  `DOSSIER_WITNESS` at run time.
- On Windows it is started directly and waits for the client itself. Elsewhere
  the application looks for a running `osu!.exe` every four seconds and learns
  the prefix from the folder the client works in — a CrossOver bottle is entered
  through CrossOver's own `wine --bottle`, any other prefix with its loader and
  `WINEPREFIX` — so no Wine is started for a client that is not there. Linux is
  written the same way and has not been tried yet.
- Witness speaks on its standard output, one JSON object a line: `waiting`,
  `attached`, `loading`, `state` (the screen and the map under the cursor),
  `playing` once a second, `kept` with the replay itself as hex, `gone`, and
  `alive` so that a dead listener is noticed. Nothing is passed through files,
  so the two sides need no path they both understand.
- With the switch on, a kept play is checked to be a replay and written into `~/.dossier/Witnessed`,
  a source of its own (*Реплеи Witness*) that is added and switched on
  with the tile's switch; the journal notices it the way it notices any new
  replay, and announces it the same way. The first version wrote into
  `~/.dossier/Replays`, which is only read by someone who has Dossier's own
  folder among their sources — the person it was first tried on did not.
- Witness is held on a leash. Inside Wine its output never closes when the
  listener goes, because the prefix's server keeps the pipe open, so a Witness
  whose application had gone stayed for good. The application now touches
  `witness.alive` beside the program every five seconds, and a Witness started
  with `--leash` leaves when that file is twenty seconds old or gone.

## The transmitter

A play Witness saw end is told to the server at once, so the chat's feed does
not wait for the live tracker to ask osu! about it. Witness itself still only
reads and has no network: it says `kept`, and the application, which holds the
person's token, sends the result — `POST /render/me/play` with the map's hash
and id, the replay's hash, the mods, the score, the six counts, the combo and
whether the play was passed. Nothing of the replay itself is sent.

- Only the person's own plays are told. The `kept` line says whether the client
  was showing a replay (`watched`), read from the client's replay-mode flag
  (`[anchor + 0x46]` of a fifth pattern, taken from tosu like the other four);
  a play is told only when that flag was read and said no for the whole play.
  The name in the replay must be the person's or empty (an offline client has
  none), autoplay is refused, and a play that was left is told only from thirty
  judged objects on. **The flag has not been seen on the bench yet** — the
  pattern is optional, and without it nothing is told and everything else works
  as before.
- The server keeps such a play in a table of its own (`witnessed_plays`): what
  an application says about its own person is shown in the feed and in that
  person's recent plays, and never enters the leaderboards, the records or the
  titles — those stay with what osu! itself reports. Accuracy and the grade are
  counted on the server from the counts.
- The row is saved first and answered with; the map's stars, length and combo
  (one cached lookup per map, checked against the hash) and the estimated pp
  come a moment later.
- When the live tracker brings the same play from osu! — same player, same map,
  within a quarter of an hour, same score or same counts — the told one steps
  aside and the confirmed one takes its place. A passed play also nudges the
  tracker, as a saved replay always did.
- What it costs the server: one small request a play, one row, three indexed
  lookups; a player can tell one play in five seconds and six hundred a day,
  and rows older than two weeks are dropped as new ones come. The feed asks one
  more indexed question than before and reuses the plays it had already loaded.

## What is checked by a machine

The reader is written against a `Memory` trait, and the tests build a small
false memory — code with the patterns in it, objects laid out the way the
runtime lays them — and read it back: patterns across the seam of two reads, a
glance at a play, a broken hash refused. No test can start the client; what the
tests cannot say is said by the bench.
