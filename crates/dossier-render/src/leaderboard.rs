#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub score: u64,

    pub stable: Option<u64>,

    pub standard: Option<u64>,

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

    pub pace: Vec<(f64, f32)>,
    pub strains: Vec<f32>,
    pub span_ms: (f64, f64),
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
                stable: None,
                standard: None,
                accuracy,
                mods,
                avatar: picture(fields.next()),
                cover: picture(fields.next()),
            });
        }
        Self {
            rivals,
            player: player.to_owned(),
            ..Self::default()
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
        let best = mine.saturating_sub(span - 1);
        let worst = best + span - 1;
        ordered[best..=worst]
            .iter()
            .enumerate()
            .map(|(offset, (entry, is_player))| Row {
                entry: entry.clone(),
                is_player: *is_player,
                place: best + offset,
                slot: (worst - (best + offset)) as f32,
            })
            .rev()
            .collect()
    }

    pub fn pp_at(&self, time_ms: f64) -> Option<f32> {
        if self.pace.is_empty() {
            return None;
        }
        let after = self.pace.partition_point(|(at, _)| *at <= time_ms);
        Some(match after {
            0 => 0.0,
            _ if after == self.pace.len() => self.pace[after - 1].1,
            _ => {
                let (from, to) = (self.pace[after - 1], self.pace[after]);
                let share = ((time_ms - from.0) / (to.0 - from.0).max(1.0)) as f32;
                from.1 + (to.1 - from.1) * share.clamp(0.0, 1.0)
            }
        })
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
                stable: None,
                standard: None,
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
    fn the_board_moves_one_place_at_a_time_with_the_player_at_its_foot() {
        let field: String = (1..=40).map(|i| format!("p{i}\t{}\n", i * 1000)).collect();
        let b = board(&field);
        let places = |score: u64| b.standings(score, 5).iter().map(|row| row.place).collect::<Vec<_>>();

        assert_eq!(places(1), [40, 39, 38, 37, 36]);
        assert!(b.standings(1, 5)[0].is_player);
        assert_eq!(places(1_500), [39, 38, 37, 36, 35], "one pass, one place");
        assert_eq!(places(2_500), [38, 37, 36, 35, 34]);

        let near_top = b.standings(38_500, 5);
        let mine = near_top.iter().find(|row| row.is_player).expect("drawn");
        assert_eq!(mine.slot, 2.0, "at the top of the field the player rises up the rows");
        assert_eq!(places(99_999), [4, 3, 2, 1, 0]);
    }

    #[test]
    fn passing_somebody_swaps_the_two_rows() {
        let b = board("a\t9000\nb\t8000\nc\t100\n");
        let slot_of = |rows: &[Row], name: &str| rows.iter().find(|row| row.entry.name == name).map(|row| row.slot);
        let before = b.standings(50, 5);
        let after = b.standings(200, 5);
        assert_eq!(slot_of(&before, "me"), Some(0.0));
        assert_eq!(slot_of(&before, "c"), Some(1.0));
        assert_eq!(slot_of(&after, "me"), Some(1.0), "the player rose");
        assert_eq!(slot_of(&after, "c"), Some(0.0), "and the passed row took their place");
    }

    #[test]
    fn the_pp_counter_follows_the_play_between_its_samples() {
        let mut b = board("a\t100\n");
        assert_eq!(b.pp_at(10.0), None);
        b.pace = vec![(100.0, 10.0), (200.0, 30.0)];
        assert_eq!(b.pp_at(50.0), Some(0.0));
        assert_eq!(b.pp_at(150.0), Some(20.0));
        assert_eq!(b.pp_at(900.0), Some(30.0));
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
}
