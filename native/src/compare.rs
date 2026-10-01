use crate::community::{Board, Person};
use crate::lang::Words;

pub const MOST: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    Board(Board),
    Rank,
    Level,
    Ss,
    S,
    Titles,
    Streak,
}

impl Line {
    pub const ALL: [Line; 12] = [
        Line::Board(Board::Pp),
        Line::Rank,
        Line::Board(Board::Accuracy),
        Line::Board(Board::Plays),
        Line::Board(Board::Hours),
        Line::Board(Board::Score),
        Line::Board(Board::HitsPerPlay),
        Line::Level,
        Line::Ss,
        Line::S,
        Line::Titles,
        Line::Streak,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Line::Board(board) => board.key(),
            Line::Rank => "global-rank",
            Line::Level => "compare-level",
            Line::Ss => "compare-ss",
            Line::S => "compare-s",
            Line::Titles => "compare-titles",
            Line::Streak => "streak",
        }
    }

    pub fn value(self, person: &Person) -> f64 {
        match self {
            Line::Board(board) => board.value(person),
            Line::Rank => f64::from(person.rank),
            Line::Level => f64::from(person.level),
            Line::Ss => f64::from(person.ss),
            Line::S => f64::from(person.s),
            Line::Titles => person.titles.iter().filter(|code| code.as_str() != "registered").count() as f64,
            Line::Streak => f64::from(person.streak),
        }
    }

    fn lower_wins(self) -> bool {
        self == Line::Rank
    }

    pub fn shown(self, words: &Words, value: f64) -> String {
        let group = |value: f64| words.lang().group(value.max(0.0).round() as u64);
        match self {
            Line::Rank | Line::Level if value <= 0.0 => "—".to_owned(),
            Line::Rank => format!("#{}", group(value)),
            Line::Board(Board::Accuracy) => words.percent(value),
            Line::Board(Board::HitsPerPlay) => group(value),
            Line::Board(Board::Hours) => format!("{} {}", group(value), words.t("hours-short")),
            _ => group(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub line: Line,
    pub cells: Vec<String>,
    pub best: Vec<bool>,
    pub share: Vec<f32>,
}

pub fn table(words: &Words, people: &[&Person]) -> Vec<Row> {
    Line::ALL
        .into_iter()
        .map(|line| {
            let values: Vec<f64> = people.iter().map(|person| line.value(person)).collect();
            let counted: Vec<f64> = values.iter().copied().filter(|value| *value > 0.0).collect();
            let top = if line.lower_wins() { counted.iter().copied().fold(f64::INFINITY, f64::min) } else { counted.iter().copied().fold(0.0, f64::max) };
            let contested = people.len() > 1 && counted.len() > 1 && counted.iter().any(|value| *value != top);
            let best = values.iter().map(|value| contested && *value > 0.0 && *value == top).collect();
            let share = values
                .iter()
                .map(|value| match (*value > 0.0, line.lower_wins()) {
                    (false, _) => 0.0,
                    (true, true) => (top / value) as f32,
                    (true, false) if top > 0.0 => (value / top) as f32,
                    _ => 0.0,
                })
                .collect();
            Row { line, cells: values.iter().map(|value| line.shown(words, *value)).collect(), best, share }
        })
        .collect()
}

pub fn chosen<'a>(pool: &'a [Person], ids: &[i64]) -> Vec<&'a Person> {
    ids.iter().filter_map(|id| pool.iter().find(|person| person.id == *id)).collect()
}

pub fn offered<'a>(pool: &'a [Person], ids: &[i64], query: &str) -> Vec<&'a Person> {
    let wanted = query.trim().to_lowercase();
    if wanted.is_empty() || ids.len() >= MOST {
        return Vec::new();
    }
    let mut found: Vec<&Person> = pool.iter().filter(|person| !ids.contains(&person.id) && person.name.to_lowercase().contains(&wanted)).collect();
    found.sort_by(|a, b| b.pp.cmp(&a.pp));
    found.truncate(6);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(id: i64, name: &str, pp: u32, rank: u32, accuracy: f32) -> Person {
        Person { id, name: name.to_owned(), pp, rank, accuracy, plays: 1_000, titles: vec!["registered".into(), "wysi".into()], ..Person::default() }
    }

    #[test]
    fn the_best_of_each_line_is_marked_and_a_rank_wins_by_being_lower() {
        let words = Words::new(crate::lang::Lang::En);
        let (naum, koto) = (person(1, "NaumRedlo", 9_870, 11_402, 97.84), person(2, "kotofey", 12_480, 3_402, 98.61));
        let rows = table(&words, &[&naum, &koto]);
        let pp = rows.iter().find(|row| row.line == Line::Board(Board::Pp)).unwrap();
        assert_eq!(pp.best, [false, true]);
        assert!((pp.share[0] - 9_870.0 / 12_480.0).abs() < 1e-4 && pp.share[1] == 1.0);
        let rank = rows.iter().find(|row| row.line == Line::Rank).unwrap();
        assert_eq!(rank.best, [false, true], "the higher rank number won");
        assert_eq!(rank.cells[1], "#3,402");
        let plays = rows.iter().find(|row| row.line == Line::Board(Board::Plays)).unwrap();
        assert_eq!(plays.best, [false, false], "a tie was given to someone");
        let titles = rows.iter().find(|row| row.line == Line::Titles).unwrap();
        assert_eq!(titles.cells, ["1", "1"], "the registration title was counted");
        assert!(table(&words, &[&naum]).iter().all(|row| row.best == [false]), "one player alone beat nobody");
    }

    #[test]
    fn up_to_four_are_offered_by_name_leaving_out_the_chosen() {
        let pool = vec![person(1, "NaumRedlo", 9_870, 1, 97.0), person(2, "kotofey", 12_480, 1, 97.0), person(3, "Koto2", 100, 1, 97.0)];
        assert_eq!(offered(&pool, &[1], "KOTO").iter().map(|person| person.id).collect::<Vec<_>>(), [2, 3]);
        assert!(offered(&pool, &[1], "  ").is_empty());
        assert!(offered(&pool, &[1, 2, 3, 4], "n").is_empty(), "a fifth was offered");
        assert_eq!(chosen(&pool, &[3, 9, 1]).iter().map(|person| person.id).collect::<Vec<_>>(), [3, 1]);
    }
}
