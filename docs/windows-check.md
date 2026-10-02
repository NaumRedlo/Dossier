# Checking Witness on Windows

The author has only a Mac, so what Witness does on Windows itself is first tried
by others after a release. This is what to try and what to send back. It takes
about ten minutes and nothing in it changes the client or the account.

## What you need

- Windows 10 or 11 and the osu!stable client (the normal installation).
- The Dossier prerelease named in the message you were sent, installed and run
  once, with the device paired to the bot (settings, the bot tile).
- The program Witness itself: Dossier writes it out when it first needs it, to
  `%USERPROFILE%\.dossier\bin\witness.exe`.

## Part one: the report

The report is a plain text the program prints about what it sees in the client.
It reads only what is in the client's memory and two lines of its configuration
(the build and the name), never anything else in that file.

1. Start osu!stable and wait for the main menu.
2. Open a terminal (Win+R, `cmd`) and run:

       "%USERPROFILE%\.dossier\bin\witness.exe" --report 120

3. For the two minutes it runs: open song select and move over a few maps, play
   one map for about thirty seconds and let it fail (or press Esc and leave it),
   then watch any replay for a few seconds and leave it.
4. Copy everything the program printed and send it back.

What the lines mean:

- `the client runs from`: the folder of osu!.exe. It must be the real folder.
- `the client's build is`: from the client's configuration; `not told` means the
  file was not found.
- `signature ...: N found in code`: each of the five must be at least 1 once the
  client has shown its menu. A 0 is the one the client's code has moved from.
- `[ 12 s] screen ...`: one line each time something changed. During a play the
  health must change with the play, and the replay flag must say
  `not watching` while you play and `watching a replay` while you watch one.
- the last four lines sum it up; `health: a play was seen and the health bar
  could not be read` is the one that says the bar's place in memory differs.

## Part two: Dossier itself

With the client still open and Dossier running:

1. Settings, the Witness tile: it must say the client is connected, and show its
   version ("Client version: ...").
2. Go to song select in the client and keep the cursor on a map for a second. A
   card with the chat's scores on that map must appear at the bottom left of
   Dossier's window and go away when you leave song select. The tile's switch
   "Show the chat at song select" turns it off.
3. Play a map to its end. Within a few seconds the result must appear in the
   chat's feed with a note that it was seen by Witness.
4. Turn on "Save a replay of every play to the journal", fail a map and then
   find the replay in the journal. Its player must be your name.

## What to send back

The text of the report, and for each step of part two whether it worked. If
something did not, what was on the screen. Screenshots help more than words.
