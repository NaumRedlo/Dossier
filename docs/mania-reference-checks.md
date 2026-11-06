# Checking the remaining mania differences in osu!stable

A full match is not confirmed yet. The corpus check of 2026-10-16 gave 161/204
matches of the six judgement counts and 196/204 matches of the maximum combo.
The diagnostic ScoreV1 formula matches 36/50 stable NoMod results; it is a
separate check and not yet part of the engine's score.

## How to get an independent reference

1. Use the original `.osr` from `Documents/Dossier Corpus` and the matching
   file from `Mania Beatmaps`. Their MD5s are listed below. Do not resave the
   map in the editor.
2. Write down the installed stable version and the FPS limit mode. The version
   recorded in the replay is listed below too; another client version may
   behave differently. Lazer Classic does not replace this stable check.
3. Play the replay from start to end without seeking. Write down the live
   score, combo and judgements at the end of gameplay. The results screen may
   show stored values, so a screenshot of it alone is not enough.
4. For the two cases with a second press, record a short video of the given
   stretch with judgements, combo and the key overlay visible. If the client's
   own logs, when enabled, hold per-note results, keep them. No injector or
   modified client is needed for this check.
5. For the combo check, play the same replay twice without changing settings.
   A difference between the two plays is kept as an observation, not resolved
   by picking the more convenient result.

Every list gives judgements in the order MAX, 300, 200, 100, 50, Miss.
Object indexes in our traces start at zero, column numbers at one.
The "model" values below come from our model, not from the stable client.

## 1. EVERYTHING BLACK [HYPER]: score without long notes

- Replay: `39aecf6b7e0ee50176f392cc9c064519`, version `20250603`.
- Map: `70dd5255aff9cc50ffa0b3e7e64a9c1c`.
- Player: Zedoreks; the file name holds `EVERYTHING BLACK` and `2025-06-09_01-02`.
- Recorded score: **934345**; model: **929346**.
- Judgements match: `[584,273,41,3,0,0]`; combo matches: **901**.

This is the first priority: the map has no holds and no misses. If a live
stable playback reproduces 934345, we need intermediate score values,
especially around 72–80 seconds, where our model registers three 100s.
Reordering simultaneous events does not change its final 929346.

## 2. Navsegda [Insane]: score and automatic misses

- Replay: `63ac67c97e3c97eeb4bdd61e8e1b5b5a`, version `20251022`.
- Map: `20683887d0372c2ce8c8ca13e8b2a924`.
- The file name holds `Navsegda` and `2025-10-28_01-36`.
- Recorded score: **909228**; model: **908341**.
- Judgements match: `[729,491,34,1,2,9]`; combo matches: **415**.

No holds. Watch 28–29 seconds and 99–106 seconds, where automatic misses
happen. Even with free reordering of results that share a time, our formula
ranges over 908123–908341.

## 3. DEUX EX MAXHINA [NOVICE]: a release before the hold starts

- Replay: `5dc9d27044b9e5b8a7cc5324ae3bfa15`, version `20250815`.
- Map: `42299ca7a516f8658541a132a54f675e`.
- The file name holds `DEUX EX MAXHINA` and `2025-09-18_23-27`.
- Recorded: `[787,442,17,1,0,0]`; model: `[787,442,16,1,1,0]`.
- Score: **981268 / 980841**; maximum combo: **1453 / 1457**.

Candidate: object 1244, column 1, hold 114917–115617 ms. Press at 114899,
release at 114913, second press at 114926, final release at 115662 ms. The
model gives a 50. The recorded result has one more 200 and one less 50, but
that does not prove it is this object that differs. We need roughly
1:54–1:56 with the judgement visible.

## 4. Dotabata Animation [Insane]: a very early press and a return

- Replay: `c4e406a1241552a855edcbe6a10168aa`, version `20251022`.
- Map: `21ed3f44e7507b21676ada576ff7737b`.
- The file name holds `Dotabata Animation` and `2025-10-29_03-03`.
- Recorded: `[1313,484,47,6,1,4]`; model: `[1313,484,46,6,2,4]`.
- Score: **957475 / 955311**; combo matches: **1910**.

Candidate: object 1731, column 3, hold 168231–168808 ms. Press at 168075,
release at 168089, second press at 168238, final release at 168804 ms. The
model keeps the first press's −156 ms error and gives a 50. We need roughly
2:47–2:49. Turning the assumed 50 into a 200 does not close the whole score
difference.

## 5. King For A Day [Normal]: one combo point

- Replay: `d3b6e5d328fcaff69132f69ba864df61`, version `20250815`.
- Map: `3d692d9097270220c65651441866399d`.
- The file name holds `King For A Day`, `[Normal]`, `2025-09-19_00-09`.
- Judgements match: `[1244,516,34,0,0,0]`.
- Combo: **1959 / 1960**; score: **975222 / 975223**.

We need two full playbacks with the combo counter visible. There are no holds
here where exactly 100 ms at the edge would explain an extra tick. The hold
ends in column 4 near 15.757 and 180.244 seconds are worth watching on their
own: the model ends them automatically after an overhold. These are places to
watch, not proven sources of the extra combo.

## Already ruled out

- An unknown map version: every corpus map was checked against the replays' MD5.
- Fixing ScoreV1 by one reordering of simultaneous judgements: all 14
  remaining NoMod results lie outside the computed bounds of such a search.
- Dropping the last tick everywhere, or counting ticks from the note's start:
  these make combo matches worse on the rest of the corpus.
- Simply removing the early release penalty: it does not give the needed
  judgements in both disputed cases at once.

The 41 lazer differences are each consistent with losing a fraction of a
millisecond on export, but the timing variants found are not recovered data.
Details and the check commands: [mania-corpus-status.md](mania-corpus-status.md).
