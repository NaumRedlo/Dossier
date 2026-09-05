pub const MOVE_MS: f64 = 420.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub score: u64,

    pub accuracy: Option<f64>,

    pub mods: String,

    pub avatar: Option<std::path::PathBuf>,

    pub cover: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Leaderboard {
    pub rivals: Vec<Entry>,

    pub player: String,

    pub avatar: Option<std::path::PathBuf>,
    pub cover: Option<std::path::PathBuf>,
}

impl Leaderboard {
    pub fn parse(text: &str, player: &str) -> Self {
        let mut rivals = Vec::new();
        for line in text.lines() {
            let line = line.trim_end_matches(['\r', '\n']);
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(name), Some(score)) = (fields.next(), fields.next()) else {
                continue;
            };
            let Ok(score) = score.trim().parse::<u64>() else {
                continue;
            };

            if name.eq_ignore_ascii_case(player) {
                continue;
            }
            let accuracy = fields.next().and_then(|a| a.trim().parse().ok());
            let mods = fields.next().unwrap_or_default().trim().to_owned();
            let picture = |field: Option<&str>| {
                field
                    .map(str::trim)
                    .filter(|path| !path.is_empty())
                    .map(std::path::PathBuf::from)
            };
            rivals.push(Entry {
                name: name.to_owned(),
                score,
                accuracy,
                mods,
                avatar: picture(fields.next()),
                cover: picture(fields.next()),
            });
        }
        Self {
            rivals,
            player: player.to_owned(),
            avatar: None,
            cover: None,
        }
    }

    #[must_use]
    pub fn with_own_pictures(
        mut self,
        avatar: Option<std::path::PathBuf>,
        cover: Option<std::path::PathBuf>,
    ) -> Self {
        self.avatar = avatar;
        self.cover = cover;
        self
    }

    pub fn is_empty(&self) -> bool {
        self.rivals.is_empty()
    }

    pub fn standings(&self, player_score: u64, limit: usize) -> Vec<Row> {
        let ordered = self.ordered(player_score);
        let mine = ordered
            .iter()
            .position(|(_, is_player)| *is_player)
            .unwrap_or(0);

        let span = limit.max(1).min(ordered.len());

        let from_bottom = (ordered.len() - 1) - mine;
        let mut worst = (ordered.len() - 1) - (from_bottom / span) * span;
        let best = worst.saturating_sub(span - 1);

        worst = worst.max((best + span - 1).min(ordered.len() - 1));
        ordered[best..=worst]
            .iter()
            .enumerate()
            .map(|(offset, (entry, is_player))| Row {
                entry: entry.clone(),
                is_player: *is_player,
                place: best + offset,

                slot: (worst - (best + offset)) as f32,
                from_slot: (worst - (best + offset)) as f32,
                moving: 1.0,
                leaving: false,
                entering: false,
            })
            .rev()
            .collect()
    }

    pub fn standings_at(&self, track: &dyn ScoreAt, time_ms: f64, limit: usize) -> Vec<Row> {
        let now = track.at(time_ms);
        let mut rows = self.standings(now, limit);

        let last_pass = self
            .rivals
            .iter()
            .map(|rival| rival.score)
            .filter(|score| *score < now)
            .map(|score| track.reached(score))
            .fold(f64::NEG_INFINITY, f64::max);
        if !last_pass.is_finite() || time_ms - last_pass > MOVE_MS {
            return rows;
        }
        let progress = (((time_ms - last_pass) / MOVE_MS).clamp(0.0, 1.0)) as f32;
        let before = self.standings(track.at(last_pass - 1.0), limit);
        let was = |row: &Row| {
            before
                .iter()
                .find(|other| {
                    other.is_player == row.is_player && other.entry.name == row.entry.name
                })
                .map(|other| other.slot)
        };
        for row in &mut rows {
            match was(row) {
                Some(slot) => row.from_slot = slot,

                None => {
                    row.from_slot = row.slot;
                    row.entering = true;
                }
            }
            row.moving = progress;
        }

        for old in &before {
            if rows
                .iter()
                .any(|row| row.is_player == old.is_player && row.entry.name == old.entry.name)
            {
                continue;
            }
            rows.push(Row {
                slot: old.slot,
                from_slot: old.slot,
                moving: progress,
                leaving: true,
                entering: false,
                ..old.clone()
            });
        }
        rows
    }

    fn ordered(&self, player_score: u64) -> Vec<(Entry, bool)> {
        let mut rows: Vec<(Entry, bool)> = self
            .rivals
            .iter()
            .cloned()
            .map(|entry| (entry, false))
            .collect();
        rows.push((
            Entry {
                name: self.player.clone(),
                score: player_score,
                accuracy: None,

                mods: String::new(),
                avatar: self.avatar.clone(),
                cover: self.cover.clone(),
            },
            true,
        ));

        rows.sort_by_key(|row| std::cmp::Reverse(row.0.score));
        rows
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub entry: Entry,
    pub is_player: bool,

    pub place: usize,

    pub slot: f32,

    pub from_slot: f32,

    pub moving: f32,

    pub leaving: bool,

    pub entering: bool,
}

pub trait ScoreAt {
    fn at(&self, time_ms: f64) -> u64;

    fn reached(&self, score: u64) -> f64;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board(text: &str) -> Leaderboard {
        Leaderboard::parse(text, "me")
    }

    fn drawn(board: &Leaderboard, score: u64, limit: usize) -> Vec<String> {
        board
            .standings(score, limit)
            .into_iter()
            .map(|row| row.entry.name)
            .collect()
    }

    #[test]
    fn a_row_is_a_name_a_score_and_maybe_an_accuracy() {
        let b = board("mrekk\t12345678\t99.21\tHDDT\nsw1t\t900\n");
        assert_eq!(b.rivals.len(), 2);
        assert_eq!(b.rivals[0].name, "mrekk");
        assert_eq!(b.rivals[0].accuracy, Some(99.21));
        assert_eq!(b.rivals[0].mods, "HDDT");
        assert_eq!(b.rivals[1].accuracy, None, "an unknown accuracy is fine");
        assert_eq!(b.rivals[1].mods, "", "and so are unknown mods");
    }

    #[test]
    fn a_row_can_carry_an_avatar_and_a_cover() {
        let b = board("mrekk\t900\t99.00\tHD\t/tmp/a.png\t/tmp/c.png\n");
        assert_eq!(
            b.rivals[0].avatar.as_deref(),
            Some(std::path::Path::new("/tmp/a.png"))
        );
        assert_eq!(
            b.rivals[0].cover.as_deref(),
            Some(std::path::Path::new("/tmp/c.png"))
        );

        let bare = board("sw1t\t900\t99.00\tHD\t\t\n");
        assert!(bare.rivals[0].avatar.is_none());
    }

    #[test]
    fn a_name_with_spaces_survives() {
        assert_eq!(board("Uika Misumi\t500\n").rivals[0].name, "Uika Misumi");
    }

    #[test]
    fn a_bad_row_is_dropped_and_the_rest_still_draw() {
        let b = board("good\t10\nnonsense\nbad\tnotanumber\nalso good\t20\n");
        assert_eq!(b.rivals.len(), 2);
    }

    #[test]
    fn the_players_own_row_is_never_taken_from_the_file() {
        let b = board("Me\t999999\nsomebody\t10\n");
        assert_eq!(b.rivals.len(), 1);
        assert_eq!(b.rivals[0].name, "somebody");
    }

    #[test]
    fn the_board_is_read_upwards() {
        let b = board("a\t300\nb\t200\nc\t100\n");

        assert_eq!(drawn(&b, 0, 5), ["me", "c", "b", "a"]);

        assert_eq!(drawn(&b, 1000, 5), ["c", "b", "a", "me"]);
    }

    #[test]
    fn the_board_is_a_page_of_the_table_not_a_window_hung_off_the_player() {
        let field: String = (1..=40).map(|i| format!("p{i}\t{}\n", i * 1000)).collect();
        let b = board(&field);

        let bottom = b.standings(1, 5);
        assert_eq!(
            bottom.iter().map(|row| row.place).collect::<Vec<_>>(),
            [40, 39, 38, 37, 36]
        );
        assert!(bottom[0].is_player);
        assert_eq!(bottom[0].slot, 0.0, "at the foot of the page");

        let climbed = b.standings(3_500, 5);
        assert_eq!(
            climbed.iter().map(|row| row.place).collect::<Vec<_>>(),
            [40, 39, 38, 37, 36],
            "the same page"
        );
        let mine = climbed.iter().find(|row| row.is_player).expect("drawn");
        assert_eq!(mine.place, 37);
        assert_eq!(mine.slot, 3.0, "risen three slots up it");

        let top = b.standings(99_999, 5);
        assert_eq!(
            top.iter().map(|row| row.place).collect::<Vec<_>>(),
            [4, 3, 2, 1, 0]
        );
        assert!(top.last().expect("five rows").is_player);
    }

    #[test]
    fn passing_somebody_swaps_the_two_rows() {
        let b = board("a\t9000\nb\t8000\nc\t100\n");
        let before = b.standings_at(&Ramp, 50.0, 5);
        let after = b.standings_at(&Ramp, 100.0 + MOVE_MS * 2.0, 5);
        let slot_of = |rows: &[Row], name: &str| {
            rows.iter()
                .find(|row| row.entry.name == name)
                .map(|row| row.slot)
        };
        assert_eq!(slot_of(&before, "me"), Some(0.0));
        assert_eq!(slot_of(&before, "c"), Some(1.0));
        assert_eq!(slot_of(&after, "me"), Some(1.0), "the player rose");
        assert_eq!(
            slot_of(&after, "c"),
            Some(0.0),
            "and the passed row took their place"
        );
    }

    #[test]
    fn a_place_is_out_of_everybody_not_out_of_the_five_drawn() {
        let field: String = (1..=40).map(|i| format!("p{i}\t{}\n", i * 1000)).collect();
        let rows = board(&field).standings(1, 5);
        assert_eq!(
            rows[0].place, 40,
            "thirty-ninth reads as thirty-nine, not five"
        );
    }

    #[test]
    fn the_player_is_always_on_the_board() {
        let field: String = (1..=40).map(|i| format!("p{i}\t{}\n", i * 1000)).collect();
        let b = board(&field);
        for score in [0, 5_000, 20_000, 39_500, 99_999] {
            assert!(
                b.standings(score, 5).iter().any(|row| row.is_player),
                "at {score}"
            );
        }
    }

    #[test]
    fn slots_are_positions_in_the_window_not_places_in_the_field() {
        let field: String = (1..=40).map(|i| format!("p{i}\t{}\n", i * 1000)).collect();
        let rows = board(&field).standings(1, 5);
        let slots: Vec<f32> = rows.iter().map(|row| row.slot).collect();
        assert_eq!(slots, [0.0, 1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn a_tie_leaves_the_player_behind() {
        let b = board("a\t200\n");
        let rows = b.standings(200, 5);
        assert!(
            rows[0].is_player,
            "the player is drawn first, meaning last place"
        );
        assert_eq!(rows[1].entry.name, "a");
    }

    #[test]
    fn the_board_is_never_longer_than_it_is_allowed_to_be() {
        let field: String = (1..=40).map(|i| format!("p{i}\t{}\n", i * 1000)).collect();
        assert_eq!(board(&field).standings(1, 5).len(), 5);
    }

    struct Ramp;

    impl ScoreAt for Ramp {
        fn at(&self, time_ms: f64) -> u64 {
            time_ms.max(0.0) as u64
        }

        fn reached(&self, score: u64) -> f64 {
            score as f64
        }
    }

    #[test]
    fn a_row_that_just_changed_place_is_still_arriving() {
        let b = board("a\t300\nb\t200\nc\t100\n");
        let moving = b.standings_at(&Ramp, 100.0 + MOVE_MS / 2.0, 5);
        assert!(
            moving
                .iter()
                .any(|row| row.moving < 1.0 && row.from_slot != row.slot),
            "somebody should be mid-move: {moving:?}"
        );
        let settled = b.standings_at(&Ramp, 100.0 + MOVE_MS * 2.0, 5);
        assert!(settled.iter().all(|row| row.moving >= 1.0));
        assert!(settled.iter().all(|row| row.from_slot == row.slot));
    }

    const PAGED: &str = "a\t9000\nb\t8000\nc\t5000\nd\t1000\ne\t25\n";

    #[test]
    fn the_row_that_arrives_grows_rather_than_slides() {
        let b = board(PAGED);
        let rows = b.standings_at(&Ramp, 1000.0 + MOVE_MS / 2.0, 2);
        let arriving: Vec<&Row> = rows.iter().filter(|row| row.entering).collect();
        assert!(
            !arriving.is_empty(),
            "somebody should be arriving: {rows:?}"
        );
        for row in arriving {
            assert_eq!(row.from_slot, row.slot, "and it should not be travelling");
        }
    }

    #[test]
    fn nothing_is_moving_before_the_first_pass() {
        let b = board("a\t300\n");
        let rows = b.standings_at(&Ramp, 10.0, 5);
        assert!(rows.iter().all(|row| row.from_slot == row.slot));
        assert!(rows.iter().all(|row| !row.leaving));
    }

    #[test]
    fn the_page_turns_only_once_the_player_reaches_its_top() {
        let field: String = (1..=40).map(|i| format!("p{i}\t{}\n", i * 1000)).collect();
        let b = board(&field);

        let mut seen = Vec::new();
        for place in 0..12 {
            let score = 500 + place * 1000;
            let rows = b.standings(score, 5);
            let me = rows
                .iter()
                .find(|row| row.is_player)
                .expect("the player is always on the board");
            seen.push(me.slot as usize);
        }
        assert_eq!(
            seen,
            vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 1],
            "the player did not climb a fixed page and turn it at the top"
        );
    }

    #[test]
    fn turning_the_page_shows_five_new_places() {
        let field: String = (1..=40).map(|i| format!("p{i}\t{}\n", i * 1000)).collect();
        let b = board(&field);
        let places = |score: u64| {
            b.standings(score, 5)
                .iter()
                .map(|row| row.place)
                .collect::<Vec<_>>()
        };

        let top = places(4_500);
        let over = places(5_500);
        assert!(
            top.iter().all(|place| !over.contains(place)),
            "the page overlapped: {top:?} then {over:?}"
        );
    }
}
