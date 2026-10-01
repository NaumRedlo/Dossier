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

## What is checked by a machine

The reader is written against a `Memory` trait, and the tests build a small
false memory — code with the patterns in it, objects laid out the way the
runtime lays them — and read it back: patterns across the seam of two reads, a
glance at a play, a broken hash refused. No test can start the client; what the
tests cannot say is said by the bench.
