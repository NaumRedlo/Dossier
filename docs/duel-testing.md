# Closed duel testing

The generator builds nine working slots: NM1–NM4, HD2, HR2, DT1, DT2 and TB.
FM1–FM3 stay catalogue labels only: free mod with HD, HR or HD+HR by OWC rules.
FM1 tests aim like NM1, HD1 and HR1; FM2 tests accuracy and reading like HR2
and HD2; FM3 tests technique like NM4, HD3 and HR3. The catalogue has no FM4,
EZ or FL slots. The model assigns categories, and the generator picks maps from
the admitted catalogue for the players' level. Maps a person confirmed are
admitted without a new model.

After the server repository is updated, the catalogue can be checked without
creating a match or a room:

```sh
venv/bin/python -m scripts.duel_probe --first 5.5 --second 5.8 --human-only
```

The levels in this check are synthetic and the same for every skill of each
player. The command reads the catalogue, writes no results and does not talk to
Bancho. Its answer holds the status, the chosen maps and the missing slots. It
does not check the IRC connection, opening a room or the players' real profiles.

When a pool does not come together, check first that there are admitted ranked
maps, assignments for every working slot and analyses with the needed mods. For
a closed test, maps checked by people are preferred; TB is never taken from
uncertain assignments. An empty slot must not be filled with a random map of
another category just to start.

Then two testers with linked osu! accounts create an unrated challenge in the
*Duels* section. They check accepting, building the pool, fetching the maps,
the draft and the room, then the result and cancelling the match. Whether rooms
open depends on the server's coordinator and its Bancho connection; local
generator tests do not prove they work on the VPS.

A pool is built only when every slot's corridor holds at least five different
admitted maps. Maps Kamerton assigned on its own, and maps it found among new
ranked ones, enter a pool only in slots where at least 10 of its confident
answers were checked and at least 90% of them were right. A pool rests on the
collected catalogue: seven maps from it and two that Kamerton found itself. If
the found maps do not fit the corridor, one or none is taken.

After a server update that changes the features, the map analyses must be
recomputed (`python scripts/duel_corpus.py reanalyse`): the features are now
`features-v3` with rhythm, and the server will not load models trained on older
features. Kamerton then has to be trained again and published; until then only
maps confirmed by people are played. The open slots are shown in the
*Auto admission* column on the *Model* tab.

As of 29 October the interface and server changes were checked locally. The VPS
was not updated from this task and no real match was played.
