# The duel system of Dossier and the 1984 bot

## Current behaviour: 7 November 2026

The model learns from OWC, user collections and balance reviewers' decisions. The published version assesses new maps from mirrors separately from the catalogue. A nine-slot pool takes seven maps from the catalogue and up to two that Kamerton found among new ranked maps, only in slots where its auto admission is open; if the found maps do not fit the corridor, the generator takes one, then none, and builds the pool from the catalogue alone. New maps are not admitted to the catalogue automatically and do not become training labels. Training maps, maps already in the catalogue and quarantined maps are excluded from the new source.

A new map needs a confident estimate (probability at least 0.80 and a lead of at least 0.20), Ranked status, osu!standard, a checked file and an exact star rating with the slot's mods. The search is limited to 35 seconds and 24 analysed maps per attempt; analyses are kept for later duels. When the catalogue cannot fill a slot's corridor either, the duel is cancelled with an explanation and both players are free to search again. The published model's quality and the mirrors' availability affect whether a pool can be built.

Once both players have the pool, the draft starts at once. Maps can be downloaded with a button; there is no separate wait for downloads. A ban or a pick is made with one click (since 6 November 2026), and a meeting screen comes before every duel. Readiness on one version of the screen is accepted from each player independently, but a request from the previous round is refused. Setting up the room is limited to 60 seconds and confirming the start to 30 seconds. Without an answer the duel is cancelled.

A player without history starts at 5.5★ with high uncertainty, matching the catalogue's 5–10★ range. Missing history is not taken as evidence of a difference in strength when looking for an opponent. Limits on rating closeness and on repeated rated meetings stay.

The design document of the first version

Version 1.0 · 23 October 2026 · Owner and acceptor: the developer of Dossier

The document defines how one-on-one osu!standard duels are implemented for Dossier users. The first version's goal: practise tournament skills, collect checkable information about pool quality and grow the generator on match results and feedback. The version includes direct challenges that do not change rating, a rated queue with Glicko-2, mandatory calibration, automatic running of official osu!stable rooms and recovery after failures.

Every mandatory product condition came from the owner. The specific timers, matching formulas, initial analyser thresholds and technical procedures below are design decisions made at his request. They are given in full so that the implementation does not depend on implicit guesses. Their values are to be checked in a pilot and are kept in versioned configuration. No accuracy threshold stated here claims quality already reached.

## 1 How to use this document

Sections 2–6 define the rules and the player's path. Sections 7–12 describe maps, the analyser, the portfolio, matching and rating. Sections 13–19 set the architecture, storage, external integrations, result checks and recovery. Sections 20–24 hold balance management, observability, the order of implementation and acceptance criteria. Section 25 lists sources.

In this document "series" means one BO5 duel to three wins; the interface says "duel". A "round" is playing one picked map. A round may have several attempts because of a tie, an interruption or a broken condition. A "pool" holds nine different map difficulties. A "slot" sets the skill tested and the mandatory mods. A "map" in the catalogue is identified by beatmap_id, not beatmapset_id: different difficulties of one set are different maps.

BO5 means best of five. NM is No Mod, HD Hidden, HR Hard Rock, DT Double Time, NF No Fail, FM Free Mod, TB Tiebreaker. In practice NM in Dossier always includes the mandatory NF. SR is Star Rating, the star difficulty with specific mods. AR sets when objects appear, OD affects hit windows, CS sets object size, BPM is the tempo. The API is osu!'s programming interface; IRC is used to run rooms through Bancho.

Glicko-2 keeps a rating r, an uncertainty RD and a volatility sigma. The skill portfolio is a separate internal estimate. High uncertainty in any value is kept as data; the absence of observations does not mean weak play.

## 2 Scope of the first version

Dossier users with a confirmed linked osu! account take part. There are no limits by rank, experience or account age. One osu! account is one active playing identity in Dossier. A user can be in only one queue, accepted duel or running series at a time. The app must stay connected to the server throughout a duel.

Only osu!standard, official multiplayer and the stable client are supported. lazer results do not count. Metadata available through the API and the client's install check are used to check compatibility, but the client's claim about its version is not proof. If the sources cannot reliably tell the conditions apart, the mode is not considered checked until an integration test.

Features of the first version:

- a direct challenge with a warning about a large difference in strength;
- an automatic queue, five mandatory calibration series and Glicko-2;
- nine skill slots and pool generation after the match is accepted;
- two bans per player, alternating picks and a mandatory TB at 2:2;
- automatic room creation or the bot joining the players' room as referee;
- checking conditions, results, lost connections and restoring state;
- history, statistics, a hidden skill portfolio, feedback and tools for balance reviewers;
- a queue for resources with a place and a wait estimate, a Telegram notice when the room opens;
- an extra unrated TB after a finished series by mutual consent.

Not included: a game server of our own, other osu! modes, lazer support in duels, appeals and investigation of suspicious results, automatic punishment for cheating, reading memory with Witness (superseded by section 27), seasons and retroactive mass changes to ratings. A technical check of a result's correctness and an administrative fix after a failure are part of the version and are not a dispute system.

The ban on practising concerns playing maps already revealed before their counted start. It does not forbid having known a map before. The rule cannot be fully proved through the available sources; the interface states it and the system records only observable violations. Memory reading is not used to tighten control.

## 3 Participation modes and calibration

A direct challenge is available before, during and after calibration. It keeps history, results and confirmed skill observations but does not change Glicko-2 or the calibration counter. When the estimates differ strongly, both players see a warning before accepting. Declining has no consequences.

The rated queue pairs players automatically. After a pair is offered, both confirm. Accepting creates a fixed link between the two and starts building the pool. A user does not choose the first calibration map or the starting difficulty range.

Calibration is five finished rated BO5 series. Series cancelled for technical reasons, technical wins without a natural end of the BO5, the extra TB and direct challenges do not count. Rating is computed during calibration too; outwardly the progress and a provisional status are shown. After the fifth series the status changes to "calibration finished" once the matching rating period is processed. RD is not reset: five series do not guarantee an exact estimate.

A calibrating player may meet another calibrating player or one already rated. Matching uses both rating with RD and the confirmed estimate of playable difficulty. A strong new player is not paired with a beginner just because both got the same starting rating.

## 4 Series rules and mods

The first to win three rounds wins. The final ScoreV2 of the specific multiplayer game from the official source is compared. The score is not recomputed from accuracy, combo, the replay or PP. Any built-in effects of allowed mods count as the game applies them.

| Slot | Mandatory mods | Player's choice |
|---|---|---|
| NM1, NM2, NM3, NM4 | NF | None |
| HD2 | NF and HD | None |
| HR2 | NF and HR | None |
| DT1, DT2 | NF and DT | None |
| TB | NF | NM, HD, HR or HD and HR |

DT speed is the standard 1.5. NC is not allowed in the first version: equivalence is not assumed. EZ, FL, HT, Relax, Autopilot, SpunOut and every mod outside the allow list are forbidden. ScoreV2 is checked as a scoring mode separately from the game's mod set; if a source also encodes it as a bit, the adapter normalises the representation and keeps the original data.

NF is mandatory for both on every map, TB included. Dossier adds no multipliers of its own for HD, HR or NF. A missing NF makes the attempt invalid whatever the result. The passed field alone does not replace checking that the game ended, who took part and that both results exist.

There are no warm-ups, free breaks or an overall time limit on a series. Technical waits are limited. A hang timer cannot interrupt a long map that is running properly.

## 5 Revealing the pool and the order of picks

The pool is created after a challenge or a queue offer is accepted. Until a room is assigned the pool is kept hidden on the server. This lets generation run ahead and avoids unequal familiarisation time when resources run short. When the room is ready and the apps are connected, the server reveals one snapshot of the pool to both at once.

Preparation starts after the snapshot reaches both clients; delivery must be confirmed within 15 seconds. Then each gets a common 120 second window. This is time for looking and downloading, not for practice. If delivery is not confirmed, the reveal does not count as synchronous and the series waits for the connection. The server keeps the actual moments of delivery: someone who has seen the pool is never given new maps for a better line-up.

Sides A and B are chosen with equal chance by the server's cryptographic generator when the series is created and are logged. Bans go A, B, A, B. Only ordinary maps still available can be banned. TB is protected. The four remaining maps are picked in the order B, A, B, A; the side that started banning does not get the first pick. The order does not depend on who won the previous map.

Each picked map is marked used for later picks at once. A replay belongs to the same round and does not use a new map. At 3:0 or 3:1 the unplayed ordinary maps stay in history as unused. At 2:2 TB is picked automatically, and refusing to play it counts as refusing to continue the series.

A ban or a pick has 30 seconds. When the timer first runs out, the server makes a reproducible random choice among the legal options using the stored seed and the turn number; the interface explains this rule in advance. Two missed actions in a row by one player put the series into a 120 second wait for presence. Before the first round a no-show cancels the series without rating; after the start the technical refusal rules of section 6 apply. The random automatic choice does not look at the opponent's weaknesses.

Before every map, including the first one after the bans, there are up to 90 seconds for readiness and downloading. When both readiness answers arrive, a 5 second countdown starts. A player leaving, losing Dossier or changing settings cancels the countdown. TB mods are each player's own choice in osu! among the allowed ones (NM, HD, HR, HDHR); Dossier has no choice for them, and whether they are allowed is checked against the official match.

## 6 Timers and exceptional situations

All deadlines are kept in UTC on the server and shown in local time in the app. While the process runs, monotonic clocks are used; after a restart the absolute deadlines are restored. A duplicated timer event does not repeat its action.

| Event | First version value | Consequence |
|---|---|---|
| Answer to a direct challenge | 120 seconds | The invitation expires |
| Confirming a queue pair | 30 seconds | Whoever accepted returns with their earlier waiting time |
| Choosing a wider range | 60 seconds | Without two consents the offer is cancelled |
| Both entering the ready room | 10 minutes | Cancelled without rating, the room closed |
| Preparation after the reveal | 120 seconds | Moves on to bans |
| A ban or a pick | 30 seconds | An automatic legal choice |
| Readiness for a round | 90 seconds | A wait for presence of up to 120 seconds |
| Return after a disconnect | 120 seconds | Technical end or cancellation |
| Infrastructure recovery | 10 minutes | Technical cancellation; the series does not count |
| Waiting for the API after a round | Up to 10 minutes | Technical cancellation without evidence |
| Consent to an extra TB | 60 seconds | Ends without an extra game |

An immediate abort means sending the command right after a disconnect is reliably detected, without waiting for the map to end. The system cannot detect a network break earlier than an available source reports it. Bancho's event of leaving the game room is grounds for aborting. An IRC user merely leaving the chat without leaving the game slot is not treated as a player disconnecting.

Dossier sends a heartbeat every 5 seconds. Closing the app explicitly triggers a stop at once; no heartbeat for 20 seconds counts as lost presence. In an active round the bot sends an abort, confirms the stop, saves the attempt as ABORTED and waits for the return. If a confirmed end of the round actually arrived before the disconnect, the order of events from section 17 applies first, not an unconditional cancellation of a game already finished.

After the return, players and settings are checked, then the same map is replayed with the same mod choice. A player's first two confirmed disconnects in a series allow recovery. A third one, or no return within 120 seconds after the first round started, means a technical win for the opponent if the infrastructure is fine. This design limit prevents endless restarts. Before the first round the series is cancelled. Losing both players at once, or being unable to tell which side failed, leads to a technical cancellation rather than naming someone at fault.

A technical win is shown apart from the game score: for example "1:1, technical win A". No won maps are added to it. It counts in Glicko-2 once at least one properly started round has begun, but it does not finish a calibration series. Giving up voluntarily after the first round has started has the same consequences. Before the start a refusal cancels the series without rating. A repeated no-show before the start within 24 hours bans searching for 10 minutes; this limits behaviour, not skill level.

With equal final ScoreV2 both results are kept, no point is given and the map is replayed. Two replays after the first tie are allowed. A third tie in a row in one round cancels the series with the reason `unresolved_tie` and without rating: replacing the map would change the agreed pool. Tied attempts stay in the practice history, but the portfolio and calibration of a cancelled series are not updated.

With a wrong map, mods or line-up the attempt is voided. The bot fixes the first technical setup error and repeats the map. The same unavoidable error again stops the series as `control_failure` without rating. No penalties for an assumed deliberate violation are given automatically.

A server failure, losing control of the bot or a long API outage is not a player's loss. A cancellation keeps the log and the results received but does not change rating, wins and losses, calibration progress or the portfolio. Derived values are therefore written only after the whole series is finalised.

After a BO5 ends naturally, the players are offered an extra TB if it was not already played as the decider. Two consents within 60 seconds create a separate practice game. The main result and rating are already fixed. The room is held only for this short choice and the TB itself; on refusal it closes at once. This is an explicit exception to closing the room right after the result, needed for an agreed extra game. Its failure does not cancel the main series.

## 7 The skill slot dictionary

The number in a tournament label is the map's number inside its mod group. There is no skill standard binding on all tournaments. The organisers of the Non-Gimmick Tournament, for example, describe NM1 as aim/consistency, NM2 as streams, NM3 as alternating, NM4 as technical, HD2 as reading, HR2 as precision, DT1 as aim and DT2 as speed. Their rules separately note how some other slots differ [S6]. So Dossier fixes a dictionary of its own, version 1, built on this practice; importing a number from OWC does not replace analysing the map.

Checking other primary rules confirms the variety: Sedi's Scuffed Amateur Tourney puts flow aim and alternating into NM3, and finger control and tech into NM4 [S13]. In 6 Digit United States Cup 3, DT2 is described through lower-tempo mechanics and DT3 through tapping [S14]. So the claim "DT2 is always one particular BPM or kind of stream" is not used. In Dossier the boundaries are fixed explicitly so that model fixes do not change a slot's meaning.

| Slot | Main purpose in Dossier | Not enough on its own |
|---|---|---|
| NM1 | Steady aim on jumps and object sequences | Only a high aim SR |
| NM2 | Flow aim, streams and holding even taps | One short stream |
| NM3 | Alternating and control of mid-tempo sequences | Only BPM or the actual finger layout |
| NM4 | Technical aim, sliders and hard rhythm transitions | Just a high overall difficulty |
| HD2 | Reading with Hidden at a relatively long approach window | Only HD turned on |
| HR2 | Precise aim on small objects with Hard Rock | Only a high OD |
| DT1 | Steady aim and reading at DT speed | Any map with DT |
| DT2 | Tap speed, bursts and stamina with DT | Only a high final BPM |
| TB | A combination of several skills and keeping quality | The hardest map in the collection |

The definitions below are Dossier's contract for labelling and tests, not a claim about a universal tournament standard.

### 7 1 NM1

The main load comes from sequences of noticeable cursor movement, repeated jumps, changing angles and keeping quality over the map. Streams and sliders are allowed as a secondary load. A map does not fit if its difficulty comes almost entirely from the speed of long streams, low AR or one short abnormal stretch. The analyser shows the share of aim load, the distribution of distances in object radii and the number of independent hard stretches.

### 7 2 NM2

The defining skill: even taps on sequences and continuous aim along them. Run lengths, distances between objects, tempo, repetition of load and short recovery gaps count. At low difficulty runs may be short but must systematically define the map's pattern. A single stream among ordinary jumps does not make a map NM2. A high BPM without sequences is not enough either.

### 7 3 NM3

The defining skill: control of alternating taps and aim on sequences that differ from NM2's dominant long streams. Rhythmic intervals, short groups, turns and alternating patterns are analysed. A map can create conditions where alternating is convenient, but an .osu file does not prove which fingers a person plays with. The assignment describes what the map asks for and does not prescribe an input technique.

### 7 4 NM4

The defining skill: handling technical transitions. Slider speeds and curvature, direction changes, mixes of sliders and circles, rhythm changes, uneven intervals and the need for exact transitions between patterns matter. One big slider, or a high slider share without hard transitions, is no reason for the slot. A map with hard reading but simple motion is not NM4 automatically.

### 7 5 HD2

The slot tests reading disappearing objects, memory of sequences and handling visual overlap and density with Hidden. Version 1 requires a base AR of at most 8.5; this is Dossier's design filter, not a tournament law. A provable reading load is also needed: overlaps, object order, dense approach windows or ambiguous transitions. When maps run short the filter is never dropped silently. Motion difficulty is matched to the players' level so that reading stays the main distinguishing skill.

### 7 6 HR2

The slot tests spatial precision with HR. Design filter: an effective CS after HR of at least 6.0. Object radii, normalised distances, angles and the share of hard precise transitions are computed. A high OD means a narrow timing window but does not confirm precision aim by itself. A map of fast streams with small circles must pass a comparative check so that speed does not stand in for HR2's purpose.

### 7 7 DT1 and DT2

DT1 asks for steady aim and reading at speed. DT2 asks above all for repeated tap speed, bursts and/or holding the tempo. Dossier joins speed and stamina inside DT2 because the pool has no DT3. For DT2 the card names the subtype "short fast runs" or "long load". Generation takes the subtype into account so that the same players do not always get only one variant. DT1 is never assigned only for a high AR, nor DT2 only for BPM.

### 7 8 TB

TB must contain several independent stretches with different demands, stay comparable in difficulty to the pool and not put all the deciding load on one narrow skill. There is no hard length limit. Maps where no skill exceeds 60 percent of the normalised load and at least three measurable directions are present are preferred; the threshold is checked on labelled examples. NF is mandatory, the choice of NM/HD/HR/HDHR is free. The analyser computes all four variants to show how the load differs, but does not promise that the mods are equally effective for every player.

## 8 The map catalogue and importing examples

Duel candidates are taken from Ranked osu!standard maps. Approved, Loved, Qualified, Pending, WIP and Graveyard maps are not allowed into a first version pool. The training corpus allows maps of any status from the OWC 2021–2025 pools [S1–S5]. The corpus and the candidate catalogue are stored apart: using a map as an example does not admit it to duels.

An import keeps the year, the stage, the original slot, beatmap_id, beatmapset_id, the source link, the mods, the .osu hash, the time fetched and the status at import time. Repeated appearances of a map do not become independent training examples. With different original slots the whole set of observations is kept along with a conflict mark. Such an example is not labelled automatically by majority vote.

Examples of NM5, HD1, HR1, DT3, FM and other slots Dossier does not have are kept for negative comparisons and for studying boundaries, not renamed into the nearest allowed slot. Moving an OWC label into Dossier's dictionary goes through analysis and a checkable mapping table for a specific corpus version.

A beatmap_id has at most one active slot assignment in the whole catalogue. A map assigned NM1 does not become DT1 when DT is added. Different difficulties of one beatmapset_id may have different assignments. Until a map first enters a published pool, the analyser may compare hypotheses. On the first assignment to the catalogue the slot and the decision's version are fixed. After that automatic reclassification is forbidden.

The model does not change a slot it already assigned confidently. The slot decision belongs to a person: a reviewer confirms the assignment, moves the map to the right slot, rejects it as fitting no slot or sends it to quarantine. The owner allowed moving on 24 October 2026: a move is recorded as a person's decision, and the model learns the right slot rather than a refusal. A rejected map does not move to another slot. Pools built before a move do not change.

Assignment states: UNASSIGNED, AUTO_ASSIGNED, HUMAN_APPROVED, REJECTED, QUARANTINED. AUTO_ASSIGNED allows confidence levels and a review queue. HUMAN_APPROVED records the reviewer, the date, a note and the analyser version. An analyser update can quarantine a map with an explanation but cannot quietly undo a person's decision.

If an .osu file's contents change, a new revision with a new hash is created. beatmap_id and the fixed slot stay, and the revision's admission is voided until analysed. A pool already revealed with a changed map gets no silent replacement: before the first start the series is cancelled with the reason MAP_REVISION_CHANGED; after the start the whole series is cancelled for technical reasons. This keeps the meaning of the bans already made.

Input files are checked for size, a valid format, negative and non-numeric values, object order, valid timing points and geometry. A parse error leads to quarantine. Background loading does not crawl the API in bulk: allowed datasets, targeted requests, a cache and the source's limits are used [S8].

## 9 A transparent map analyser

Analysis runs on the server or in a separate worker process on the same VPS, without blocking the IRC loop. Client-side analysis may be used for the interface, but the published metrics and the slot decision come from the server. A result is reproducible from the file hash, the parser version, the computing library's version, the mods, the feature version and the model.

SR and game parameters come from a checked library with a pinned version and explicitly chosen stable semantics; the candidate for a Rust component was rosu-pp [S11]. Before it is accepted, the library is compared against reference maps. Its aim/speed or PP computation is not taken as a ready classifier of tournament slots. (In the code, stars with mods come from the osu! API's `beatmaps/{id}/attributes`, see section 26.)

### 9 1 Computed features

Time is normalised to the real game tempo: with DT, time intervals are divided by 1.5. HR geometry, effective AR/OD/CS and stacking are computed by fixed stable rules. A slider is not reduced to a line between its start and end: its shape, repeats, duration and the move to the next object count.

For every object and every pair of neighbouring objects the time interval, the distance between targets, the distance in object radii, the turning angle, the movement speed, the transition type and the rhythmic ratio to the local beat length are computed. For sliders: length, duration, average and peak speed, turns, repeats and the move after the end. Spinners count towards duration and recovery but create no fictitious aim load.

Streams are found as chains of circles with similar intervals between objects: deviation from the chain's median at most 12 percent, at least three objects. Separate histograms of lengths 3–5, 6–15, 16–31 and 32 or more are kept. This is an initial rule for finding chain candidates, not a universal definition of a stream. The result is compared with the rhythmic grid and distances; an accidental equality of intervals does not finish the classification.

Analysis windows: 2, 10 and 30 seconds with a half-window step. For each feature group the median, the 90th and 99th percentiles, the peak stretch, the share of time under load and recovery lengths are kept. The end of a short map is handled by a partial window with its real length. This tells a single burst of difficulty from a constant load.

Reading is estimated from the number of objects visible at once, visibility time, overlapping areas, crossing paths, repeated coordinates and order changes. These are approximations of reading demands; they do not measure a particular person's perception. The technical load is described by the distributions of angles, slider curvature, speed changes and rhythmic unevenness.

A map's vector has eight directions: jump aim, flow aim, tapping speed, tapping stamina, rhythm control, reading, precision and slider control. Values are normalised over labelled examples of comparable difficulty. Normalisation bounds and original units are kept so that the explanation can be rebuilt. Averaging features over the whole map without a time profile is forbidden.

### 9 2 The assignment model

The first working variant: a regularised multiclass logistic regression over normalised features, with mandatory slot filters and a separate OTHER class. It can show the features' contributions. A more complex model is allowed only after comparison on the same held-out sample while keeping explainability. A large language model is never the source of a final label.

Each feature is normalised by the training sample's median and interquartile range; a zero range is replaced by 1. Then z is clamped to −5…5. Class probabilities come from a softmax of linear scores. Slots ruled out by mandatory filters are excluded before choosing. Players' tags are separate features with limited weight; the mapper's name can never decide the slot by itself.

Data is split in groups by beatmapset_id and repeated appearances of a map. The main time test: train on 2021–2023, tune on 2024, final estimate on 2025 with overlaps removed. An independent hand-picked sample of Ranked maps outside OWC, across difficulty ranges, is also mandatory. Otherwise the model may learn the style of elite pools without fitting most users.

Confidence is calibrated on a separate validation sample, for example with temperature scaling. Until probability calibration is confirmed, the interface shows a "feature agreement score", not a percentage chance of the right assignment. A person's approval is shown separately and does not turn the number into 100 percent.

Initial bounds: a confident assignment needs p at least 0.80 and a lead over the second allowed class of at least 0.20; an uncertain one needs p from 0.60 and a lead from 0.10. Below these bounds a map is not used and goes to labelling. The OTHER class and a detected out-of-distribution case rule out admission. Uncertain maps are allowed in rated and direct duels with a warning; the first version allows at most two such maps per pool and only a confident or person-confirmed TB.

Out of distribution is recorded when more than 20 percent of significant features lie outside the 1st–99th percentiles of the training range for that level, or mandatory features are missing. The number is an initial heuristic. Automatically lowering requirements to fill an empty slot is forbidden.

### 9 3 Explanations for players and administrators

A slot card shows its purpose, the mods, the SR with them, a short reason for the choice, the confidence and a mark of a person's check. For example, an NM2 explanation is built from real features: the share of objects in chains, chain lengths, tempo, time of continuous load. Numbers are never filled into a text template without computing them.

An administrator also sees a time graph of the eight directions, problem stretches, raw values, alternative hypotheses before fixing, the label's source, versions and the feedback history. A comparison with similar checked maps gives their ids and feature distance. Participants' personal weaknesses are not revealed in it.

## 10 The skill portfolio and starting difficulty

The portfolio keeps observations and their interpretation apart. For each map: the source, time, user, mods, play values, map version, confirmation quality and slot assignment. For each slot and direction: an estimate of playable difficulty, the uncertainty, the number of independent maps, age and where the data came from. The portfolio is not published to other players; its owner sees their own details, and an administrator's access to service data is logged.

Official match results and confirmed submitted osu! results take priority. Local .osr and scores.db help find maps of interest and ask for confirmation but do not raise the trusted strength estimate or change rating. Unconfirmed local observations may be shown to the user separately without taking part in rated matching. A player name in a replay is not a link to an identity.

The first load uses available best results, recent plays, the history of attempts already synced, mods and the profile. Rank and PP are weak general signs: they do not replace measuring HD reading or HR precision. Other people's profiles are not mined in bulk. If recent plays are unavailable or contain no play of the needed slot, that is recorded as an absence of observations.

### 10 1 The initial reproducible computation

For a confirmed play a quality value is introduced: y = 0.65 × accuracy + 0.25 × combo_ratio + 0.10 × (1 − miss_ratio). All shares lie between 0 and 1; combo_ratio is capped at one and uses the maximum combo of exactly this map revision. This value is not used to decide a round's winner.

miss_ratio is the number of misses on main objects divided by the number of such objects; slider breaks are not added a second time. When the denominator is unavailable or the statistics are incomplete, the observation does not take part in the initial formula. For a stable sample of fewer than ten independent maps only one current observation per map is allowed; the 10 percent cap on relative weight applies once ten maps are reached.

For a map of the slot's type with difficulty s, an observation of playable difficulty is a = s + 2 × (y − 0.85), clamped to s − 1.7…s + 0.3. The coefficients are Dossier's initial heuristic and not a physical measurement of skill. An observation's weight is the source weight times 2 to the power of (−age in days / 90). A full confirmed result has source weight 1; a result with incomplete metadata does not take part. At most one observation a day is taken per map, and the total weight of one beatmap_id is capped at 10 percent of the slot's weight.

A slot's theta estimate is the weighted 60th percentile of a. The uncertainty u = max(0.25, 1.2 / sqrt(n_eff)), where n_eff = (sum of weights)² / sum of squared weights. With fewer than three independent maps, u is at least 0.8. This is a technical uncertainty range in SR units, not a statistically calibrated confidence interval. Without observations the slot borrows the overall estimate with u = 1.2 and an explicit UNKNOWN_SLOT mark.

The portfolio's eight directions get observations weighted by the matching direction's share in the map. At least 30 percent of the load must belong to a direction for a play to update it. For TB only the directions really present in that map are used; one TB does not count as nine independent checks.

### 10 2 A profile without results

First the system checks the profile and the existing cache, the available confirmed results and mods. Then, with only general data, it applies a starting difficulty model trained on users with known history: features log(1 + PP), log(1 + playcount), overall accuracy and available result distributions; missing features have separate masks. The model is trained only on an agreed internal sample, does not promise an exact estimate and gives a wide uncertainty.

With no reliable data at all an individual skill cannot be computed. The document set a general conservative prior of theta = 2.5 SR, u = 1.5, adapting automatically after the first series; the code now starts at 5.5★ with u = 1.5 to match the 5–10★ catalogue (see the current behaviour above). This is a design value for launching, not the player's choice and not an established fact about their strength. When no allowed maps of the needed slots exist, the system offers an agreed higher range or says that no suitable pool exists yet. A user is never cut off by rank, but the system does not hand out an unsuitable map just to start formally.

During a BO5 the pool and the matching profile are fixed. New results update the portfolio only after the series ends, so the system does not change difficulty to the current score. After a cancelled series the observations stay in a diagnostic archive without changing the portfolio. The extra TB updates the portfolio in a separate transaction after its own check.

## 11 Matching opponents and pool difficulty

### 11 1 Choosing a pair

The search runs every 5 seconds over present users. The player waiting longest is considered first, then the allowed candidates. Waiting time is not reset when an opponent declines a found pair. Leaving ends the user's own request. When Dossier is lost, the request is paused for 20 seconds, then removed without a penalty.

The first comparison uses p = 1 / (1 + 10^((r_B − r_A) / 400)). This is a symmetric closeness measure for the queue, not an exact Glicko-2 outcome probability. Playable slot difficulty and RD are compared as well. A high RD does not automatically allow any difference in skills.

For the first 60 seconds p from 0.40 to 0.60 is allowed and a difference in overall theta of at most 0.6 SR. From 60 to 180 seconds p from 0.35 to 0.65 and up to 0.8 SR. After 180 seconds the bounds are 0.30–0.70 and 1.0 SR. For unknown theta the distance is not taken as zero: other users with an unknown estimate or a close general prior get priority, and a warning about missing data is shown.

Candidates are ranked by the sum 2 × |p − 0.5| + 0.5 × min(|theta_A − theta_B|, 2) + 0.2 × repeat_opponent. repeat_opponent is 1 if the pair already played in the last 60 minutes, otherwise 0. On a tie the earlier request wins. The automatic queue does not arrange a pair's third rated meeting within 24 hours; a direct challenge stays available. This limit reduces repetition in a small sample without investigating anyone's intent.

Beyond the last range the queue does not widen silently. The user keeps waiting or sends a direct challenge. In a direct challenge the warning about a large difference shows when p is outside 0.20–0.80 or the overall theta differs by more than 1.5 SR. With too little data, uncertainty is shown instead of an exact statement. Both players always get the same map conditions; there are no handicaps or individual speeds.

### 11 2 Choosing difficulty inside a slot

For each slot theta_A,s, theta_B,s and their uncertainties are computed. The initial target difficulty is t_s = 0.7 × min(theta_A,s, theta_B,s) + 0.3 × max(theta_A,s, theta_B,s). When either player's u exceeds 0.8, t_s is lowered by 0.25 SR. The formula leans slightly towards the weaker participant without trying to erase a real difference in strength. The initial candidate corridor is t_s ± 0.4 SR.

Equal SR does not mean equal load across skills. So each slot uses its own theta, and for mod maps SR is computed with the mandatory mods. TB has a base target SR equal to the median t_s of the ordinary slots, with a preferred corridor of ±0.4 SR. Its four allowed mod combinations are also checked, and extreme jumps in load are flagged.

Candidates are ranked by h_i(m) = sigmoid((theta_i,s − SR_m) / max(0.35, u_i,s)), where sigmoid(x) = 1 / (1 + exp(−x)). This is a heuristic accessibility index while there is no empirical probability calibration. It must not be called a probability of winning, passing or reaching a given accuracy. A value near 0.5 means closeness to the estimated level.

A map's load penalty is |h_A − 0.6| + |h_B − 0.6|. A marked asymmetry of strengths is penalised as well: |advantage(m) − baseline_pair|, where advantage(m) = h_A − h_B and baseline_pair is the median of this difference over all available slot candidates. So the generator does not turn every map into a test of one player's single weakness and does not promise equal chances to a clearly unequal pair.

The portfolio affects the line-up symmetrically: swapping A and B does not change the pool's quality score. In the rated queue hidden personal compensation through separate win rules is forbidden. The formula's version and the input snapshots are kept before the reveal.

### 11 3 Familiarity and repeats

Familiarity history includes confirmed plays, earlier duels, a map's known presence in a revealed pool and the auxiliary fact of having it locally. Having a file means the map is available, not that it was practised; the absence of a known play does not prove unfamiliarity.

A player's repeat penalty: max(2^(−days_since_play / 14), 0.5 × 2^(−days_since_reveal / 7)). Without such an event the matching part is zero, and the unknown is kept separately. For a pair the maximum of the two penalties and the difference in familiarity count. Maps played by either player in the last 7 days are excluded at first; this soft filter is lifted when candidates run short.

A beatmap_id cannot repeat in one pool. Preferably at most one difficulty of one beatmapset_id and at most two maps by one mapper. The last two limits are soft. The mapper's name is used for variety, not as a stand-in for measured skills.

### 11 4 The generation algorithm

1. Fix the participants, portfolios, rules, seed, catalogue and analyser versions.
2. For each of the nine slots, select Ranked maps with the needed fixed assignment, an available revision, the right mode and an allowed confidence.
3. Apply the difficulty corridor and the initial repeat limits. Check that at least one candidate stays in every slot.
4. Compute a candidate's cost: 3 × load penalty + 2 × asymmetry + 2 × repeat penalty + 1 × familiarity difference + 2 × (1 − confidence) + 0.5 × peak stretch penalty. All parts are normalised to 0–1; an unknown confidence without a person's admission rules a map out.
5. Take up to 100 best candidates per slot. Build the pool with a beam search of width 50 partial solutions, adding slots from the scarcest. Check uniqueness and variety limits at every step.
6. Among up to 20 best full solutions pick one reproducibly by seed with weights exp(−(cost − min_cost) / 0.25). This avoids always handing out one minimal solution.
7. Run the final check of the nine slots, fixed assignments, mods, revisions and explanations. Save the snapshot and hash_manifest. Only then allow the reveal.

A pool's cost is the average cost of its nine maps plus penalties for one mapper's concentration and repeated sets. The log keeps the candidates, the reasons they were dropped and the relaxation stage chosen. The whole database is not searched at the moment of a challenge: features and candidate indexes are ready in advance.

Cost normalisation is fixed: the load penalty is divided by 2; the asymmetry is divided by 2; both are clamped to 0–1. The familiarity difference is the absolute difference of the individual familiarity penalties. The peak penalty is clamp((peak_2s / max(p90_10s, 0.01) − 2) / 3, 0, 1), with load measured on one analyser scale. For a person's admission without a calibrated probability the uncertainty penalty is zero with a HUMAN_OVERRIDE mark; the interface does not show it as a 100 percent probability. Each map by a mapper beyond two adds 0.1 to the pool's cost, each extra difficulty of one set adds 0.2. These additions apply only once the matching relaxation stage is allowed.

### 11 5 Stages when candidates run short

Stage 0 uses all the initial conditions. Stage 1 allows a map played in the last 7 days, keeping the recency penalty. Stage 2 allows more than two maps by one mapper and several difficulties of one set with an extra penalty. Stage 3 widens the difficulty corridor to ±0.7 SR while keeping the target level. If the load goes beyond the range shown at first, both confirm the updated range before the pool is revealed.

Stage 4 offers both a specific other range computed from the nearest full set of slots. The new bounds and the reason are shown; two consents are needed within 60 seconds. Without consent the series is cancelled before play and rating does not change. The range must not be raised automatically through many small widenings.

Never relaxed: Ranked, osu!standard, unique beatmap_id within the pool, the fixed slot, the mod allow list, no quarantine, map integrity, the minimum classification quality and checking the result's source. If a full pool cannot be built, POOL_UNAVAILABLE is returned with the scarce slots listed. Repeats help only where history is the limit; they do not create a skill missing from the catalogue.

## 12 Glicko 2 rating

The rated unit is the outcome of the whole series, not of a single map. A win is 1, a loss 0. 3:0 and 3:2 get no different coefficients. Direct challenges, cancellations and the extra TB are not rated. Technical wins are included only under the conditions of section 6 and carry their own reason code.

Initial values r = 1500, RD = 350, sigma = 0.06, tau = 0.5. The common rating period is one UTC hour. All results with verified_finalized_at inside an hour are processed together after it ends; participants and opponents are taken as they were at the start of the period. The system shows when the next update is. The initial values and the hourly window are design settings to be evaluated on pilot data. The canonical algorithm works in periods; the parameters and a worked example are given by Glicko-2's author [S10].

A series confirmed late falls into the period of its actual finalisation, not backdated into an hour already closed. The server does not mix one player's sequentially updated rating with the other's old one. The map's, client's or replay's time does not set the period.

### 12 1 Computation

Conversion: mu = (r − 1500) / 173.7178; phi = RD / 173.7178. For each opponent j: g_j = 1 / sqrt(1 + 3 × phi_j² / pi²); E_j = 1 / (1 + exp(−g_j × (mu − mu_j))). Then v = 1 / sum(g_j² × E_j × (1 − E_j)); delta = v × sum(g_j × (s_j − E_j)).

For volatility: a = ln(sigma²). The function f(x) = exp(x) × (delta² − phi² − v − exp(x)) / (2 × (phi² + v + exp(x))²) − (x − a) / tau². The root is found with the canonical procedure of [S10] to a precision of 0.000001, guarded against non-numeric results. sigma_new = exp(root / 2).

The iteration step is written as follows. Start with A = a. If delta² > phi² + v, take B = ln(delta² − phi² − v); otherwise find the first integer k from 1 for which f(a − k × tau) is not below zero, and take B = a − k × tau. Compute f_A and f_B. While |B − A| > 0.000001: C = A + (A − B) × f_A / (f_B − f_A); f_C = f(C). If f_C × f_B ≤ 0, set A = B and f_A = f_B, otherwise halve f_A. Then B = C, f_B = f_C. After convergence root = A. Implementation guards: at most 1000 steps, checks for division by zero and finiteness of every number; a violation does not fall back to the old sigma but stops fixing the period.

Update: phi_star = sqrt(phi² + sigma_new²); phi_new = 1 / sqrt(1 / phi_star² + 1 / v); mu_new = mu + phi_new² × sum(g_j × (s_j − E_j)). Back: r_new = 173.7178 × mu_new + 1500; RD_new = 173.7178 × phi_new. Rounding is applied only in the interface.

With no games in a period mu and sigma do not change, phi_new = sqrt(phi² + sigma²). For several missed periods the increase is computed at once from their count. The design's upper bound on RD is 350; this is an extra Dossier rule recorded in the rating version. For a new account periods are counted from the creation of the rating record, not from the osu! registration date.

### 12 2 Processing and checks

The period task locks the rating_period row, loads all unused outcomes, computes updates from the snapshot and writes the rating, the transition log and the applied marks in one transaction. A unique key on rating_version, duel_id and user_id rules out double crediting. If one result's computation fails a check, the period is not fixed partially; the task fails and is repeated after the fix.

The reference test [S10]: start 1500/200/0.06, opponents 1400/30, 1550/100 and 1700/300, outcomes 1/0/0, tau 0.5. Expected about r 1464.06, RD 151.52, sigma 0.059996. Also checked: no games, identical players, large differences, several games with one opponent, reordering inputs and running a period twice.

A test run has its own rating version: glicko2-v1-test. The outcome history is kept so that a recount is technically possible, but the first version never automatically changes an already published history. Correcting a technically wrong result is recorded as a separate administrative event and stops publishing the affected computation until the owner decides; it does not erase the original data.

## 13 Architecture

The main store is PostgreSQL. The Python service 1984 uses SQLAlchemy 2 async and asyncpg. SQLite and the old botdata.db are not a source for new duels. SQLite tests, where used, do not replace PostgreSQL checks of transactions, locks and unique indexes.

The Rust app Dossier handles the authorised interface, presence, accepting a challenge, viewing the pool, bans, picks and readiness. It does not name the winner or write rating directly. Witness was not connected at first (superseded by section 27). The existing .osr and scores.db readers keep an auxiliary role only.

The Python side is split into modules: DuelService, MatchmakingService, PoolService, CatalogService, ProfileService, RatingService, RoomCoordinator, BanchoGateway, OsuMatchVerifier, NotificationService and BalanceAdminService. The names mark responsibilities, not a requirement to split everything into separate network services at once.

The first launch allows one main process with async tasks and a separate map analysis process on the same VPS. Durable job queues, events, locks and the outbox live in PostgreSQL; Redis is not required. Heavy .osu analysis never runs inside the IRC loop or a game action's transaction.

The main series flow: Dossier → DuelService → stored event and outbox → RoomCoordinator → BanchoGateway → the official room. Back: Bancho → incoming log → duel state; osu! API → OsuMatchVerifier → confirmed attempt → series result → portfolio and the rating period queue. Telegram gets only notification jobs after the event is committed to the database.

### 13 1 Consistency principles

Every action carries operation_id, actor_id, duel_id and expected_version. In a transaction the server locks the duel row, checks the role and version, applies an allowed transition, increments the version and writes the event together with the external action's job. Repeating an operation_id returns the original answer. A stale version gives a conflict and the current snapshot.

A command to an external service is never run inside an open transaction. The outbox worker delivers it after commit. A failure between sending and confirmation is possible; so instead of repeating the whole operation blindly, a reconciliation of the room's actual state runs. Creating a room, starting and aborting have their own scenarios for an uncertain outcome.

One coordinator owns a room through a lease with an epoch. After the owner changes, old jobs with the previous epoch are dropped before sending. The lease is renewed every 5 seconds and expires after 20. A lease expiring does not by itself prove that the room closed and the creation slot is free.

## 14 Data model

All osu! ids are stored as integers wide enough. Local entities use UUIDs. Times are in UTC. The system handles no money or payments. Percentages and ratings are stored without premature rounding.

| Entity | Main fields and purpose |
|---|---|
| duel | id, mode, state, state_version, rules_version, seed, created_at, finalized_at, outcome_reason |
| duel_participant | duel_id, user_id, osu_user_id, side, accepted_at, connection_state, calibration_snapshot |
| queue_entry | user_id, mode, enqueued_at, heartbeat_at, status, profile_snapshot_id |
| room_binding | duel_id, room_id, match_id, origin, creator_id, capabilities, lease_epoch, status |
| room_capacity | bot_account_id, capacity, reserved_count, reconciliation_state |
| pool_snapshot | duel_id, manifest_hash, catalog_version, analyzer_version, profile_versions, relaxation_level |
| pool_entry | pool_id, slot, beatmap_id, revision_hash, mods, metrics_snapshot, confidence, review_state |
| draft_action | duel_id, turn_index, actor_id, type, slot, automatic, operation_id |
| round | duel_id, ordinal, slot, state, selected_by, winner_id, tie_count |
| round_attempt | round_id, attempt_no, game_id, expected_mods, start_at, end_at, validity, invalid_reason |
| official_evidence | match_id, event_id, game_id, payload_hash, payload, received_at, source_version |
| round_score | attempt_id, osu_user_id, score_v2, mods, accuracy, max_combo, statistics, evidence_id |
| duel_event | duel_id, sequence, type, payload, actor, occurred_at, received_at |
| pending_command | id, duel_id, room_epoch, kind, payload, status, attempts, next_attempt_at |
| scheduled_deadline | duel_id, state_version, kind, expires_at, processed_at |
| map_revision | beatmap_id, hash, format_version, status, downloaded_at, object_count |
| map_assignment | beatmap_id, immutable_slot, assignment_version, status, reviewer_id |
| map_analysis | revision_hash, mods, engine_version, feature_version, model_version, features, predictions |
| training_example | corpus_version, source, year, stage, original_slot, beatmap_id, normalized_label, conflict |
| skill_observation | user_id, duel_id, evidence_id, slot, value, weight, verified, profile_version |
| skill_profile | user_id, version, slot_estimates, direction_estimates, uncertainty, updated_at |
| rating | user_id, version, rating, rd, sigma, period_id, calibration_count |
| rating_result | duel_id, user_id, opponent_id, result, period_id, processed_at |
| rating_period | id, version, starts_at, ends_at, state, input_hash, completed_at |
| rating_change | period_id, user_id, before_values, after_values, input_hash |
| pool_feedback | duel_id, user_id, slot, fit_label, difficulty_label, fairness_label, comment, version |
| balance_review | beatmap_id, revision_hash, requested_at, reason, decision, reviewer_id, decided_at |
| notification_outbox | event_id, user_id, channel, payload, delivery_state, attempts |

The official_evidence table keeps the original official answer and its hash; a changed answer is kept as a separate revision. Passwords, OAuth tokens, the IRC password and the Telegram token never go into a payload. A room's room_id for joining and its match_id for history may be represented differently; the adapter does not assume they are equal without checking.

Mandatory constraints: a unique active user in a queue or duel; a unique active room_binding per room; unique (pool_id, slot) and (pool_id, beatmap_id); one slot assignment per beatmap_id; unique (duel_id, turn_index), (round_id, attempt_no), the official game_id for a counted attempt, (attempt_id, osu_user_id), (duel_id, sequence), operation_id and the rating credit keys. The active user constraint is a separate participant_lease table or a partial index, not a check in the application only.

The history of matches, pools, confirmed results, rules and rating transitions is kept for the account's lifetime and within the accepted data deletion policy. The raw technical IRC log is kept 30 days, errors and the administrative audit 180 days, and minimal confirmations of game results stay with the history. These are design periods, not a public promise to keep personal data without the possibility of deletion.

## 15 States and transitions

| Duel state | Entering event and action | Next state |
|---|---|---|
| INVITED | A direct challenge is created | ACCEPTED, DECLINED, EXPIRED |
| MATCH_OFFERED | A queue pair is found | ACCEPTED or the accepter returns to the queue |
| ACCEPTED | Both confirmed | GENERATING_POOL |
| GENERATING_POOL | The generator's inputs are fixed | WAITING_ROOM, RANGE_OFFERED, CANCELLED |
| RANGE_OFFERED | Another range is needed | GENERATING_POOL with two consents, otherwise CANCELLED |
| WAITING_ROOM | The pool is ready and hidden | ROOM_SETUP or CANCELLED |
| ROOM_SETUP | Creating or checking the external room | WAITING_PLAYERS or RECOVERING |
| WAITING_PLAYERS | Waiting for entry and presence | PREPARING or CANCELLED |
| PREPARING | The pool is revealed synchronously | DRAFTING after 120 seconds |
| DRAFTING | Four bans in turn | SELECTING |
| SELECTING | A legal pick of the next map | ROUND_SETUP |
| ROUND_SETUP | Setting up and checking the room | WAITING_READY, PAUSED or CANCELLED |
| WAITING_READY | Both ready, presence confirmed | STARTING or PAUSED |
| STARTING | Countdown and the start command | PLAYING or PAUSED |
| PLAYING | The map is running | VERIFYING, ABORTING or RECOVERING |
| ABORTING | The abort was sent | PAUSED once the stop is confirmed |
| VERIFYING | Checking the official result | SELECTING, ROUND_SETUP, FINALIZING or CANCELLED |
| PAUSED | A reason to stop and a return point exist | ROUND_SETUP, the previous safe state or a terminal outcome |
| RECOVERING | Infrastructure lost | The restored safe state or CANCELLED |
| FINALIZING | A natural or technical outcome is saved | COMPLETED |
| COMPLETED | The main series is fixed | Does not change; the extra TB is a separate entity |
| CANCELLED | Technical cancellation, not counted | Does not change |
| DECLINED, EXPIRED | The challenge did not happen | Does not change |

ROUND_SETUP and SELECTING repeat as the series goes on. At 2:2 SELECTING picks TB automatically. After a tie VERIFYING returns to ROUND_SETUP for the same map with attempt_no incremented. After a broken condition the attempt becomes INVALID; the round and the series score do not change.

The extra TB has its own states OFFERED, SETUP, PLAYING, VERIFYING, COMPLETED, CANCELLED. It refers to the finished series but does not move it back to PLAYING. Releasing the room is tied to the end of the child game or the offer expiring.

Every pause keeps its reason, the previous state, a deadline and the attempt's context. Recovery never returns to STARTING automatically: first it checks whether the game already began on the osu! server. An action from an unsuitable state is refused with the INVALID_STATE code and the current snapshot.

## 16 Rooms and the Bancho integration

### 16 1 Rooms created by the bot

BanchoGateway implements the IRC client, reconnecting, PING/PONG, limiting the rate of outgoing messages, subscribing to rooms and parsing events. The official documentation names irc.ppy.sh and a separate IRC password [S9]. The design assumed an account agreed with osu!: a separate bot account cannot simply be registered as an ordinary second account and goes through osu!'s procedure [S12]. A separate account turned out not to be allowed, so the bot runs Bancho under the owner's own account (see section 26).

The limit on created rooms is configurable, starting at 4. Creation uses an atomic reservation in PostgreSQL. When a command times out, the reservation stays uncertain until reconciliation proves whether the room was created. A unique name holds a short duel id for recovery. !mp make must never be sent again just because the app got no answer.

Room setup covers the name, password, head-to-head, ScoreV2, osu!standard, the map, mods, players and no extra playing users. The management commands are documented by osu! [S7]. Actual values and the adapter's answers are checked by integration tests. Size and lock are used after their behaviour is checked; a spectator's right must not make them a third participant in a round.

Before the start the bot checks !mp settings and Bancho's known state, the players by osu_user_id, the current map and mods. For Freemod each player's effective set is checked, not only the room's general flag. A line from an ordinary user that looks like a BanchoBot message is not a control event. The nick/source is checked in the protocol's context; renamed players are matched by their fixed ids.

IRC events are used for quick reactions and the API for the final check. "The player finished" does not mean "both results are confirmed". The end of a round names no winner until the official game is checked.

### 16 2 The room queue

The room queue is separate from the opponent search. It holds pairs that already accepted a match, with a ready hidden pool. The place is FIFO by the moment both accepted; broken or unconfirmed pairs are skipped for a while with an explanation. A freed room is reserved for the next available pair in a transaction.

The wait estimate is built from the remaining games of current series, their maps' lengths with mods and the median time of actions. A P50–P90 range over recent finished series is shown, and with too little history the text "not enough data for an estimate yet". An exact time must not be promised, since a BO5 has no overall time limit.

The ROOM_READY event creates one notice in Dossier and one in Telegram for each participant with an existing confirmed chat link. The notice holds the link and the entry deadline. Telegram being unavailable does not block the match; the app shows the delivery status. Repeated sends create no new game events. The bot never writes to users who did not link their Telegram to the service.

If neither enters within 10 minutes, the series is cancelled and the room closed. A room counts as freed only after its closing is confirmed or its absence is reliably reconciled. An uncertain closing room counts towards the limit; the user queue gets a message about a technical delay.

### 16 3 A players' room with referee rights

When the limit is taken, a pair may provide a link to their own official room. Both participants confirm the binding in Dossier. The server accepts only valid osu! links, extracts the id and calls the fixed API itself; an arbitrary URL is never loaded. The room must not be bound to another active duel.

The room's creator adds the bot as referee. The bot joins the room's channel and checks the rights to read, change settings, start, abort and close. Before admitting the room a safe settings check with a confirmed answer runs. The ability to abort is checked in a separate integration test, and the binding confirms the role still holds. Without a checked set of capabilities the match stays in ROOM_SETUP and does not become rated.

After the check an external room runs under the same rules, including the immediate abort on a detected disconnect, mandatory Dossier presence and the API check. A "read the result only" mode is not part of the first version. If the creator changes settings, the bot restores them before the start; if referee rights are revoked, the series stops and is cancelled without rating after the recovery deadline.

An external room does not use the counter of rooms created by the bot but does use the shared IRC, API and worker resources. The design limit on rooms managed at once of any origin is 8: up to four created by the bot and free places for external ones. This limit is enabled only after a load test; until then managed_room_limit is 4 and the interface does not promise a way around a busy shared capacity. So the fallback solves the creation limit but does not promise endless parallelism. The separate created_room_limit and managed_room_limit are visible in operation.

When binding, the participants agree that the bot changes game settings and closes the room after the series or a technical timeout. If closing an external room is technically impossible, the service drops its own binding after checking that no game is active and tells the creator to close it by hand; the bot's created room limit does not change because of it.

## 17 Checking results and trust boundaries

### 17 1 Sources

The main evidence of a score is the game in the history of the specific official match through GET /matches/{id}. The event history is paginated; the event_id sequence and a separate re-read of an unfinished game_id are used [S8]. The last cursor does not mean that a game already read could not have been updated.

General best or recent scores, map results outside the match, screenshots, .osr and scores.db do not replace the multiplayer result. Equal map, time and score are not enough to attribute an outside attempt to a duel. Local data cannot confirm a win even with both players' consent.

### 17 2 Binding an attempt

Before the start the match_id, slot, beatmap_id, revision, both osu_user_ids, the effective mods, the round and attempt numbers, the rules version, the expected start time and the last event_id are saved. After the start the attempt gets its game_id from the official history and the live events. One game_id cannot belong to two attempts or duels.

The verifier checks the room, the game_id, the map, osu!standard, head-to-head, ScoreV2, a start time after the command was sent and a line-up of the two expected players. Then it checks that the game ended, that both results exist, each player's effective mods and that no known abort or broken condition happened.

A revision hash is not assumed to be available in every API answer. Before the start the available control data of the map is compared; the server's copy of the .osu and the catalogue keep the hash. If the API does not confirm a field, the verifier does not mark it "checked by the API". A specific source's limit is documented in the integration test report. A critical condition that cannot be proved disables rated admission for that path.

The API may show a game with incomplete results. Such data is in the PENDING_EVIDENCE state. A score of zero is not a missing result. Two finished zero results count as a tie if the other conditions are proved. A player who left before the end does not get an artificial zero instead of an interrupted attempt.

### 17 3 Order of events and delays

Events keep the source time, the receiving time and their sequence. If an official finished result precedes a confirmed exit of a participant, the round stands and the pause applies before the next one. If a disconnect is confirmed during play, the attempt counts as interrupted even with late partial scores. With a contradictory order the verifier does not guess a winner: it asks for the history again and holds VERIFYING.

Receiving the same result many times does not change the series score. Two different payloads of one official event are kept as versions and compared. When already finalised data differs, rating is never changed automatically and quietly.

API requests are combined per room. The first version's budget: up to 30 requests a minute for the whole duel module, with finished rounds first and one limiter shared with the existing sync. After the end event a request is made; with incomplete data it repeats after 5, 15, 30 seconds and then every 60 seconds up to a total of 10 minutes. These intervals are a design budget, not a stated osu! limit. A 429 answer respects Retry-After; other temporary errors use backoff and jitter. Players' profiles are not polled constantly.

### 17 4 Manual finalisation after a technical failure

Only the owner or an assigned technical administrator can run manual_finalize. The operation needs a specific match_id and game_id, available official evidence, a stated reason and an action log. It repeats the same checks of players, map and result but lets a person tie a correctly played game to a stuck attempt.

If no official result can be had, an administrator can cancel the series but cannot enter an arbitrary score for rating. Players' consent and a local replay do not lift this limit. A balance administrator does not get the right of manual finalisation by default.

## 18 Recovery and cleaning up resources

When the process starts it first blocks creating new rooms, then loads active duels, leases, commands with an uncertain outcome and deadlines. The coordinator restores the IRC connection, joins known rooms and asks for their state and API history. Only after reconciliation are new resource allocations enabled.

The recovery deadline is the start of a known fault plus 10 minutes. If the process vanished without an event, the time of the service's last saved heartbeat plus 10 minutes is used. Restarts do not extend the deadline. Deadlines already passed move the matching duels to technical cancellation after the diagnostic state is saved.

Recovery scenarios:

- The room exists with no active game: check players, map, mods and presence, then return to safely preparing the needed round.
- The game ended during the failure: fetch the full official result and check the binding. Count only a proven correct attempt, then continue the series.
- The game is still running: on regaining control abort it and schedule a repeat; the server cannot prove Dossier's presence over the whole unknown interval.
- The room is closed: if the state and both players are restored within the deadline, create a replacement with the same pool and game score, binding the new match_id. Without resources, wait only for the rest of the recovery deadline.
- A create command may have gone through: find the room by the saved answer/id and the unique name; while the outcome is unknown, do not create a duplicate or free the reservation.
- The database already holds the result: do not return the series to play. Deliver undelivered notices, finish the missing cleanup and wait for the single application of rating.

The system keeps the reasons for cancellations and cleanup attempts. A stuck room is found by a missing expected transition after the current phase's deadline, not by the series' overall length. For PLAYING the latest moment for a technical check is the start plus the map's real length with mods plus 120 seconds. If the official data says the game is still going, a reconciliation runs rather than an unconditional result.

Closing a room with !mp close runs from a cleanup job with another reconciliation. If IRC is unavailable, the record stays CLOSING_UNKNOWN and counts towards the limit. The operations screen shows such rooms and how long they wait. A closed room does not delete the series' history.

## 19 Contracts between Dossier and the server

The routes below were the new module's design interface. In the code they live under `/render/duels/` (see section 26). Every changing request uses Dossier's authorisation, an idempotency key and expected_version. The participant's identity comes from the session, not from an arbitrary user_id in the body.

| Operation | Purpose and key inputs |
|---|---|
| POST /duels/invitations | opponent_id; create a direct challenge |
| POST /duels/{id}/accept | Accept a challenge or a pair offer |
| POST /duels/{id}/decline | Decline before the start |
| POST /duels/queue | Enter the rated queue |
| DELETE /duels/queue | Leave the search |
| GET /duels/{id} | A snapshot of state, version, deadlines, score and allowed actions |
| POST /duels/{id}/pool-ack | Confirm delivery of a specific manifest_hash |
| POST /duels/{id}/draft-actions | turn_index, action, slot |
| POST /duels/{id}/ready | round_id, chosen TB mods, manifest_hash |
| POST /duels/{id}/external-room | An official room link, participants' confirmation |
| POST /duels/{id}/range-accept | The offered range's version |
| POST /duels/{id}/surrender | An explicit confirmation of giving up |
| POST /duels/{id}/bonus-tb | Consent to an extra TB |
| POST /duels/{id}/feedback | Rating the pool, slots and difficulty |
| GET /duels/history | Paginated public history |
| GET /duels/profile/me | One's own portfolio and uncertainty |
| POST /admin/duels/{id}/finalize | Official evidence and the reason for a technical finalisation |
| POST /admin/maps/{id}/review | APPROVE, REJECT or QUARANTINE and a reason |

The WebSocket update stream carries event_id, duel_id, state_version, type, payload and server_time. After a break the client asks for the events after its cursor or a full snapshot. Missed events are never rebuilt by the interface's guess. The heartbeat is confirmed by the server, and the user sees a lost connection before trying an action.

Standard errors: INVALID_STATE, VERSION_CONFLICT, NOT_PARTICIPANT, ALREADY_BUSY, NOT_YOUR_TURN, SLOT_UNAVAILABLE, POOL_UNAVAILABLE, MAP_REVISION_CHANGED, ROOM_CAPACITY_WAIT, ROOM_CONTROL_REQUIRED, EVIDENCE_PENDING, INCOMPATIBLE_CLIENT, SERVICE_RECOVERING. An answer holds an explanation in the user's language, whether the action can be repeated, and the current state. An error never reveals the opponent's portfolio or integration secrets.

The interface covers the challenge and queue paths, the screen waiting for resources, the pool with explanations, bans, the current round, timers, recovery, the result and feedback. A result shown before the official check is marked "waiting for confirmation". Buttons of unavailable actions are hidden or disabled with a reason; the server checks everything again on its own.

## 20 Feedback and balance administration

A player can rate the whole pool and a single slot during or after the match. The ratings are separate: whether the map fits its purpose; too easy, right or too hard; whether familiarity felt unfair; whether there was a technical problem. A comment is optional. One current rating per participant per slot replaces the previous one, keeping the edit history. A public rating does not reveal a player's hidden features.

The loser's feedback does not automatically weigh less, and the winner's does not become truth. What matters is repetition across independent pairs, a confirmed play and separating difficulty from fitting the slot. Feedback on a banned or unplayed map counts as a preview opinion with a smaller weight, not as a play result.

For quick ranking a smoothed share of negative feedback is used: q_bad = (2 + sum of negative weights) / (4 + sum of all weights). Feedback on a played map weighs 1, on a preview 0.25; one user's contribution per map is capped at 1 per 30 days. Feedback adds up to 0.5 × max(0, q_bad − 0.5) to a candidate's cost but does not change the fixed slot or the mandatory filters. So feedback affects the next generations without instant retraining on a single message.

An uncertain assignment automatically creates a balance_review task when the pool is revealed. A map also goes to review after three independent negative ratings of slot fit with q_bad at least 0.60. If after five independent played ratings q_bad reaches 0.75, the map is excluded until reviewed. The threshold does not replace an administrator's decision.

The administrator sees the analysis evidence and the feedback, then confirms, rejects or quarantines the map. Moving it to another slot follows section 8. The administrator's decision joins the reference labelling with its reason. Deleting an assignment or getting around a block through a new model version is not allowed.

Retraining runs on a schedule at most once a day when there are at least 50 new independent checked labels; otherwise the current model stays. Negative feedback without a confirmed label is a signal for picking what to review, not automatic truth. A new model goes through the same held-out sample, then a shadow computation without affecting games. The owner or an assigned model administrator publishes a version; a rollback returns the previous version for new pools without changing revealed ones.

Roles: PLAYER, BALANCE_REVIEWER, TECHNICAL_OPERATOR, MODEL_PUBLISHER, OWNER. The owner assigns specific people. BALANCE_REVIEWER does not change rating, TECHNICAL_OPERATOR does not rename slots, MODEL_PUBLISHER gets no access to tokens. Every decision keeps actor_id, the grounds, the time and the previous version.

## 21 Observability and operating limits

Measured: search time, the wait for a room, generation time, reasons for a missing pool, maps per slot and level, the share of uncertain assignments, repeats, feedback, broken conditions, result check time, recoveries, closing and the rating period's delay.

For the analyser: precision and recall per slot, the confusion matrix, the OTHER share, quality per SR range, confidence calibration error and the out-of-distribution share. Overall accuracy does not replace the weakest slot's result. For the generator: concentration of maps and mappers, the familiarity difference and how wins split by first pick.

For rating: prediction error, the RD distribution, new players' results and repeated pairs. Glicko-2 is not considered validated just because a library passes the worked example. The usefulness of the hourly period at the real frequency of games is evaluated too.

Initial technical goals: p95 generation from a ready catalogue under 10 seconds; p95 internal processing of a game event under 500 milliseconds; sending an abort after a processed disconnect event within 1 second while IRC is available; no double credit in any recovery test. These are service goals, not guarantees of Bancho's or the API's speed.

Map analysis has its own CPU and memory limits. One map at a time is analysed on a small VPS until load is measured; workers are added only if the coordinator's responsiveness does not suffer. The hard limit on managed rooms is never computed automatically from the core count. Accepting the fallback includes eight rooms managed at once, four of them created by players, and checking the IRC budget, the API budget, memory and recovery scenarios. If the VPS cannot carry the load, external rooms stay but the available places are limited to the measured capacity.

An administrator may watch through Dossier and osu!'s allowed means, without adding a third playing participant. The first version has no forms for cheating reports or disputes, but service errors and balance tasks stay available.

## 22 Order of implementation

Stage 1: create PostgreSQL migrations, new entities and transactional transitions, isolate the old SQLite tables. Implement the log, outbox, idempotency, deadlines and one state snapshot. Check competing accepts and locks without connecting to osu!.

Stage 2: implement BanchoGateway and RoomCoordinator. On controlled test games prove creation, entry, ScoreV2 and NF settings, Freemod, the start, abort, closing, losing rights and recovery. Separately check a player-created room with referee rights. Until these checks pass, the rated queue stays off.

Stage 3: connect reading match history, pagination, binding game_id, checking mods and the final recording of results. Check that no score comes from a local file. Set the budget together with 1984's existing sync.

Stage 4: import the OWC corpus and prepare hand labelling of Ranked candidates. Implement server analysis, explanations, fixed slots, quarantine and the initial generator. A small hand-confirmed catalogue is acceptable for the first closed test; it does not replace the final version's requirement of automatic analysis.

Stage 5: build the interface of a full direct challenge, timers, the room queue, the Telegram notice, recovery and the extra TB. Run BO5 tests with a known pool, then with the generator.

Stage 6: implement the portfolio, the automatic cold start, the opponent queue, five calibration series and the rating period. Pass numeric and transactional tests before rating is turned on.

Stage 7: connect feedback, the balance desk, feedback's limited effect on ranking, training, the shadow computation and model publishing. Measure the coverage of difficulty levels and slot quality. A poorly supported range is shown explicitly.

Stage 8: run the owner's acceptance, a load run with four rooms, network disconnects and restarts in every phase. Record the release configuration, the model and catalogue versions, the recovery instructions and the rollback procedure. These stages are the development order of one first version, not a move of agreed features into an indefinite future.

## 23 Acceptance criteria

### 23 1 Rules and interface

R01: a direct challenge before calibration finishes, is kept in history and changes neither rating nor its counter. R02: five naturally finished rated BO5s take a player out of calibration once the period is applied. R03: two parallel accepts do not create two active duels for one user.

R04: a pool holds exactly the nine given slots and nine different beatmap_ids. R05: four bans in the order A/B/A/B leave four maps; the picks B/A/B/A replay after a restart. R06: TB cannot be banned, it is mandatory at 2:2, and the extra TB does not change the series' outcome. R07: both get one manifest and 120 seconds of preparation; the hidden queue never reveals maps to one earlier than the other.

R08: a forbidden mod, a missing NF, a wrong map or a third playing participant prevent the start; if a violation still happened, no point is given. R09: the interface shows the reason for a stop, the time left and the available actions. R10: the room queue gives the place, and the wait estimate is not shown as an exact guarantee.

### 23 2 Maps and classification

M01: the OWC 2021–2025 import is reproducible from its manifest, removes repeated training observations and keeps conflicting labels. M02: a non-Ranked example never enters the game catalogue through training. M03: a map assigned NM1 cannot become DT1 through the API, a new model or a new file revision. Only a reviewer can move it, with a *Move* decision, and that decision goes into the log. A newly published model reconsiders only uncertain automatic assignments.

M04: every slot has at least 30 independent hand-labelled check maps and separate negative examples of nearby slots. After splitting by map sets no test map appears in training. M05: the precision of positive admissions is at least 0.90 for every slot on the check sample; macro-F1 at least 0.80. The sample size and a confidence interval are published with the numbers. A small number of examples is never hidden by rounding percentages.

M06: for confident automatic assignments the measured error share is at most 0.10; if the threshold is not reached, that automatic admission is turned off and hand-confirmed maps stay available. M07: every included map has an explanation with computed values, versions and a correct confidence mark. M08: abnormal files, missing features and out-of-distribution cases never lead to forcibly filling a slot.

M09: launching for a difficulty range needs at least five different admitted candidates per slot within the working corridor and a full pool without breaking hard conditions. The same range is checked with real mods. Levels where the conditions are not met stay available to ask for, but the system honestly returns that there is no pool; promising support for every level without a catalogue is not allowed.

M10: 1000 generations on a fixed matrix of pairs create no forbidden maps, repeats within a pool or wrong slots. Swapping the players does not change the pool's cost. The share of successful generation across supported ranges is at least 95 percent, counting the allowed lifting of the history filter. A quality failure needs a fix of the catalogue or the algorithm, not a hidden lifting of limits.

### 23 3 Results and recovery

V01: only the official game of the right match_id/game_id can change the score. A forged replay, a renamed .osr, a local scores.db and a matching outside score fail the check. V02: a missing result never turns into zero. V03: pagination never skips a needed round; an unfinished event is checked again after an update.

V04: delivering one event ten times increases neither score nor rating. V05: a disconnect during a round triggers an abort after detection; if the abort is not confirmed, no new attempt starts. V06: finishing before leaving and leaving before finishing give different correct outcomes on recorded event sequences.

V07: a restart in GENERATING_POOL, ROOM_SETUP, STARTING, PLAYING, VERIFYING, FINALIZING and while closing a room leads to a predefined state without duplicate rooms or rating. V08: a series is cancelled after the 10 minute recovery expires without updating the portfolio, statistics or calibration. V09: another restart does not extend this deadline.

V10: four simultaneous reservations are created, the fifth waits; a !mp make timeout does not free an uncertain slot. V11: an external room without referee rights is not admitted; after control is confirmed it is admitted on the common terms. V12: revoking rights stops control and does not let an unchecked game count.

### 23 4 Rating and feedback

G01: the Glicko-2 worked example passes with r and RD off by less than 0.02. G02: the order of series within a period does not affect the result. G03: running a period again does not change values already written. G04: both sides of a series use the same period snapshot. G05: direct challenges, cancellations and the bonus TB create no rating_result.

F01: feedback is available only to a participant of that series and cannot multiply endlessly. F02: feedback about difficulty never becomes a negative slot label. F03: a person's rejection immediately excludes the map from new pools without changing revealed ones. F04: a new model without checks and publishing does not change real duels. F05: rolling a model back keeps past pools reproducible.

The owner runs the acceptance by a protocol that records the versions of the app, server, database, model and rules. The acceptance document records the real test results and the limits found. Until these criteria are met the system is under development, not proven ready for rated operation.

## 24 Parameters of the first version

| Parameter | Initial value | Where it applies |
|---|---|---|
| series_wins_required | 3 | The BO5 outcome |
| pool_size | 9 | The generator |
| bans_per_player | 2 | Bans |
| preparation_seconds | 120 | Familiarisation |
| draft_action_seconds | 30 | A ban or a pick |
| ready_seconds | 90 | Preparing a round |
| room_join_seconds | 600 | Entering the room |
| reconnect_seconds | 120 | A player's return |
| recovery_seconds | 600 | Infrastructure |
| evidence_wait_seconds | 600 | The API check |
| bonus_tb_seconds | 60 | Consent to an extra TB |
| app_heartbeat_seconds | 5 | Dossier's presence |
| app_absence_seconds | 20 | Detecting a lost app |
| created_room_limit | 4 | Rooms created by the bot |
| managed_room_limit | 8 after a load test, 4 until then | Total managed load |
| calibration_series | 5 | Rating calibration |
| rating_period_seconds | 3600 | The common UTC period |
| initial_rating / rd / sigma | 1500 / 350 / 0.06 | A new rating |
| glicko_tau | 0.5 | Volatility |
| confident_probability / margin | 0.80 / 0.20 | Classification admission |
| uncertain_probability / margin | 0.60 / 0.10 | Admission with a warning |
| uncertain_maps_per_pool | 2 | Limiting classification risk |
| hd2_max_base_ar | 8.5 | Assigning HD2 |
| hr2_min_effective_cs | 6.0 | Assigning HR2 |
| initial_sr_window | 0.4 | The candidate corridor |
| expanded_sr_window | 0.7 | The shortage stage |
| recent_map_days | 7 | The initial repeat filter |
| max_disconnect_recoveries | 2 | Repeats after losing a player |
| max_tie_replays | 2 | Ties on one map |
| api_duels_requests_per_minute | 30 | The module's budget within the shared limits |

The live values are in `PARAMETERS` in the bot's `services/duels/rules.py`. Changing a parameter creates a new rules_version. An active series keeps using the old snapshot unless the change is caused by an immediate need for a technical stop. Filters, timers or rating conditions must never change for one side during a match. The analyser version, the rating version and the rules version are different and kept separately.

## 25 Sources and grounds

The sources were checked on 23 October 2026. Tournament pages are used for where pools come from and for examples of rules; they do not prove the quality of an analyser not yet trained. Dossier's numeric thresholds and architecture are defined by this project. The Dossier and 1984 repositories were not read directly while preparing the document; information about the code at the time was provided by the owner after checking it.

- S1. [OWC 2021 and its published pools](https://osu.ppy.sh/wiki/en/Tournaments/OWC/2021).
- S2. [OWC 2022 and its published pools](https://osu.ppy.sh/wiki/en/Tournaments/OWC/2022).
- S3. [OWC 2023 and its published pools](https://osu.ppy.sh/wiki/en/Tournaments/OWC/2023).
- S4. [OWC 2024 and its published pools](https://osu.ppy.sh/wiki/en/Tournaments/OWC/2024).
- S5. [OWC 2025 and its published pools](https://osu.ppy.sh/wiki/en/Tournaments/OWC/2025).
- S6. [The Non-Gimmick Tournament organiser's rules and slot descriptions](https://osu.ppy.sh/community/forums/topics/1765322). A primary source for one tournament's conventions, not an official universal standard.
- S7. [Official tournament room management commands](https://osu.ppy.sh/wiki/en/osu!_tournament_client/osu!tourney/Tournament_management_commands).
- S8. [Official osu! API v2 documentation](https://osu.ppy.sh/docs/index.html#get-match). Match history belongs to stable multiplayer; the rooms paths of the other multiplayer model do not stand in for it automatically.
- S9. [Official IRC documentation](https://osu.ppy.sh/wiki/en/Community/Internet_Relay_Chat).
- S10. [Mark E Glickman: the Glicko-2 algorithm and worked example](https://glicko.net/glicko/glicko2.pdf).
- S11. [rosu-pp, the original difficulty and performance computation project](https://github.com/MaxOhn/rosu-pp). A candidate for implementation with a mandatory pinned version and a check against stable.
- S12. [Rules for getting a bot account](https://osu.ppy.sh/wiki/en/Bot_account).
- S13. [Sedi's Scuffed Amateur Tourney rules](https://osu.ppy.sh/community/forums/topics/1512165). The organiser's primary description of skill slots.
- S14. [6 Digit United States Cup 3 rules](https://osu.ppy.sh/community/forums/topics/2151355). An example of assigning DT2 and DT3 differently.

When written, the OWC corpus still had to be downloaded, parsed and labelled by Dossier's contract. The document did not claim that the five tournaments' maps had been analysed from .osu or that the model had reached the target metrics. Its result was a full specification of implementation and checks, not a trained generator.

## 26 State of the code on 7 November 2026

The owner accepted the document as it is. Below is how the current code of Dossier and 1984 differs from what the document assumes; it is a work list, not an amendment to the rules.

Done:

- Database: PostgreSQL through SQLAlchemy 2 async and asyncpg on the server (the bot moved to PostgreSQL on 8 October 2026).
- Bancho: the IRC client `services/duels/bancho.py` opens and sets up rooms with `!mp` commands, takes someone else's room by link and aborts a round. osu! does not allow a separate account, so the bot runs Bancho under the owner's account, NaumRedlo.
- Results: `services/duels/verifier.py` reads `GET /matches/{id}`, checks the map, mods and players and keeps the evidence.
- Updates: a WebSocket in the bot (`services/duels/live.py`) and in the app (`native/src/duels.rs`), the app's heartbeat every 5 seconds.
- Routes: the section 19 operations live under `/render/duels/` in `services/duels/http.py`. Outcome reasons are stored in lowercase (`unresolved_tie`, `control_failure`, `pool_unavailable` and others).
- Stars with mods come from the osu! API's `beatmaps/{id}/attributes`, and `.osu` files only from mirrors.
- Catalogue and Kamerton: the OWC 2021–2025 import, the reviewers' queue, the extra slot, training checked on held-out years. Kamerton admits maps on its own only in slots where at least 10 of its confident answers were checked and at least 90% were right (M06); in the other slots only maps confirmed by people are played.
- The generator does not build a pool if any slot's working corridor has fewer than five different admitted maps (M09).
- A pool is built as 7+2: seven maps from the collected catalogue and two that Kamerton found among new ranked maps. If the found maps do not fit the corridor, the generator takes one, then none.
- A round whose end the server did not hear (a restart, a broken IRC connection) goes to an API check after the map's length and 10 minutes; a map stop not confirmed within 10 minutes cancels the series without rating (V08). Deadlines are stored in the database with absolute times, so a restart does not extend them (V09).
- A ban or a pick is one click (since 6 November 2026), and a meeting screen comes before every duel.

Not done:

- No real series has been played on the server yet.
- No M04 report: many slots have fewer than 30 hand-checked maps, and accuracy confidence intervals are not published yet (M05).
- No test of 1000 generations over a matrix of real pairs (M10) and no restart check in every state (V07); there is a test of 1000 pools over a synthetic catalogue.
- Duels run only on stable, and Dossier is required for the whole series; Glicko-2 and calibration are on.

## 27 Witness in duels

The owner's decision of 23 October 2026 replaces the phrases "Witness is not connected" and "reading memory with Witness" in sections 2, 13 and 10: Witness takes part in duels. osu! support considered reading the game's memory undesirable; the owner accepted this risk for his players, and Witness still works only with its switch on.

- Purpose: checking a round's progress in real time and watching the game through the server and Dossier's engine. Witness passes the server the participant's game state: the map, mods, the time in the map, score, combo, hits and signs of leaving the map or closing the game.
- A round's final score still comes only from the official match (section 17). Witness data names no winner and does not change rating.
- A difference between Witness and the official result (another map, other mods, a noticeably different score) is written to the series' log with both values and flagged for the technical operator; there are no automatic penalties.
- Disconnects: Witness data stopping during a round while Dossier's heartbeat is alive, leaving the map or closing the game according to Witness are reliable events for the abort of section 6, on a par with Bancho's events.
- Watching: the server relays live Witness frames to the series' participants and spectators; Dossier's engine rebuilds the round from them. Frames are kept no longer than the IRC log (30 days).
- Streaming: with Witness on, the participant's app sends the server frames of their game (cursor position, presses, time) that Witness reads from the current play's record. The server hands them to spectators, and Dossier's engine at the spectator's draws the field from the same map off a mirror with the same mods, a few seconds behind for a buffer. Hits on the spectator's screen are counted by the engine; the match result stays official.
- A player with Witness off can still play a duel: the result is checked through the official match, but live watching and early disconnect detection are unavailable for them, which the opponent is told before accepting.
