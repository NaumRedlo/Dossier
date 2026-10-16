use std::collections::HashMap;

use crate::library::Entry;
use crate::rating::Rating;

pub const STARS_TOP: f64 = 10.0;
pub const GAP: f32 = 8.0;
pub const ARC_STEP: f32 = 6.0;
pub const ARC_STEPS: usize = 7;
pub const NESTED: f32 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Sort {
    #[default]
    Date,
    Pp,
    Accuracy,
    Stars,
    Artist,
}

impl Sort {
    pub const ALL: [Sort; 5] = [Sort::Date, Sort::Pp, Sort::Accuracy, Sort::Stars, Sort::Artist];

    pub fn key(self) -> &'static str {
        match self {
            Sort::Date => "sort-date",
            Sort::Pp => "sort-pp",
            Sort::Accuracy => "sort-accuracy",
            Sort::Stars => "sort-stars",
            Sort::Artist => "sort-artist",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Group {
    #[default]
    Maps,
    Players,
    Days,
    Flat,
}

impl Group {
    pub const ALL: [Group; 4] = [Group::Maps, Group::Players, Group::Days, Group::Flat];

    pub fn key(self) -> &'static str {
        match self {
            Group::Maps => "group-maps",
            Group::Players => "group-players",
            Group::Days => "group-days",
            Group::Flat => "group-flat",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    pub query: String,
    pub low: f64,
    pub high: f64,
    pub mine: bool,
    pub witness: bool,
    pub clean: bool,
}

impl Default for Filter {
    fn default() -> Self {
        Filter { query: String::new(), low: 0.0, high: STARS_TOP, mine: false, witness: false, clean: false }
    }
}

impl Filter {
    pub fn narrows(&self) -> bool {
        !self.query.trim().is_empty() || self.low > 0.0 || self.high < STARS_TOP || self.mine || self.witness || self.clean
    }
}

pub const HEAD_OPEN: f32 = 80.0;
pub const HEAD_CLOSED: f32 = 64.0;

#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    Head { title: String, under: String, first: usize, count: usize, open: bool, cover: Option<String>, marks: Vec<Option<f64>> },
    Difficulty { version: String, stars: Option<f64>, first: usize },
    Replay { at: usize, chosen: bool },
}

impl Row {
    pub fn height(&self) -> f32 {
        match self {
            Row::Head { open: true, .. } => HEAD_OPEN,
            Row::Head { .. } => HEAD_CLOSED,
            Row::Difficulty { .. } => 36.0,
            Row::Replay { .. } => 60.0,
        }
    }

    pub fn depth(&self) -> f32 {
        match self {
            Row::Head { .. } => 0.0,
            Row::Difficulty { .. } | Row::Replay { .. } => NESTED,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Laid {
    pub rows: Vec<Row>,
    pub order: Vec<usize>,
    pub chosen: Option<usize>,
}

pub struct Context<'a> {
    pub ratings: &'a HashMap<String, Rating>,
    pub me: &'a str,
    pub witnessed: &'a dyn Fn(&Entry) -> bool,
    pub day: &'a dyn Fn(i64) -> String,
}

fn rating<'a>(context: &'a Context<'_>, entry: &Entry) -> Option<&'a Rating> {
    context.ratings.get(&entry.replay_hash)
}

fn song(entry: &Entry) -> (String, String, String) {
    match (&entry.map, &entry.named) {
        (Some(map), _) => (map.artist.clone(), map.title.clone(), map.version.clone()),
        (None, Some(named)) => (named.artist.clone(), named.title.clone(), named.version.clone()),
        (None, None) => (String::new(), String::new(), String::new()),
    }
}

pub fn passes(entry: &Entry, filter: &Filter, context: &Context<'_>) -> bool {
    if !entry.matches(&filter.query) {
        return false;
    }
    if filter.mine && (context.me.is_empty() || !entry.player.eq_ignore_ascii_case(context.me)) {
        return false;
    }
    if filter.witness && !(context.witnessed)(entry) {
        return false;
    }
    if filter.clean && entry.counts[3] > 0 {
        return false;
    }
    if filter.low > 0.0 || filter.high < STARS_TOP {
        let Some(stars) = rating(context, entry).map(|r| r.stars) else {
            return false;
        };
        if stars < filter.low || (filter.high < STARS_TOP && stars > filter.high) {
            return false;
        }
    }
    true
}

fn sorted(entries: &[Entry], mut list: Vec<usize>, sort: Sort, context: &Context<'_>) -> Vec<usize> {
    let number = |value: Option<f64>| value.unwrap_or(f64::NEG_INFINITY);
    list.sort_by(|a, b| {
        let (x, y) = (&entries[*a], &entries[*b]);
        let by = match sort {
            Sort::Date => y.played_at.cmp(&x.played_at),
            Sort::Pp => number(rating(context, y).map(|r| r.pp)).total_cmp(&number(rating(context, x).map(|r| r.pp))),
            Sort::Accuracy => y.accuracy.total_cmp(&x.accuracy),
            Sort::Stars => number(rating(context, y).map(|r| r.stars)).total_cmp(&number(rating(context, x).map(|r| r.stars))),
            Sort::Artist => {
                let (xa, xt, _) = song(x);
                let (ya, yt, _) = song(y);
                (xa.to_lowercase(), xt.to_lowercase()).cmp(&(ya.to_lowercase(), yt.to_lowercase()))
            }
        };
        by.then_with(|| y.played_at.cmp(&x.played_at))
    });
    list
}

fn group_key(entry: &Entry, group: Group, context: &Context<'_>) -> String {
    match group {
        Group::Maps => {
            let (artist, title, _) = song(entry);
            if artist.is_empty() && title.is_empty() { format!("?{}", entry.map_hash) } else { format!("{}\u{1f}{}", artist.to_lowercase(), title.to_lowercase()) }
        }
        Group::Players => entry.player.to_lowercase(),
        Group::Days => (context.day)(entry.played_at),
        Group::Flat => String::new(),
    }
}

pub fn lay(entries: &[Entry], filter: &Filter, sort: Sort, group: Group, chosen: Option<usize>, context: &Context<'_>) -> Laid {
    let passing: Vec<usize> = (0..entries.len()).filter(|at| passes(&entries[*at], filter, context)).collect();
    let list = sorted(entries, passing, sort, context);
    let chosen = chosen.filter(|at| list.contains(at)).or_else(|| list.first().copied());
    let mut keys: Vec<String> = Vec::new();
    let mut buckets: HashMap<String, Vec<usize>> = HashMap::new();
    for at in &list {
        let key = group_key(&entries[*at], group, context);
        if !buckets.contains_key(&key) {
            keys.push(key.clone());
        }
        buckets.entry(key).or_default().push(*at);
    }
    let open_key = chosen.map(|at| group_key(&entries[at], group, context));
    let mut rows = Vec::new();
    let mut order = Vec::new();
    for key in keys {
        let members = &buckets[&key];
        let mut members = members.clone();
        if group == Group::Maps {
            let mut versions: Vec<String> = Vec::new();
            for at in &members {
                let (_, _, version) = song(&entries[*at]);
                if !versions.contains(&version) {
                    versions.push(version);
                }
            }
            members.sort_by_key(|at| versions.iter().position(|v| *v == song(&entries[*at]).2).unwrap_or(usize::MAX));
        }
        order.extend(members.iter().copied());
        let open = open_key.as_deref() == Some(key.as_str());
        let lead = &entries[members[0]];
        if group == Group::Flat {
            rows.extend(members.iter().map(|at| Row::Replay { at: *at, chosen: Some(*at) == chosen }));
            continue;
        }
        let (artist, title, _) = song(lead);
        let (head_title, under, cover) = match group {
            Group::Maps if title.is_empty() => (String::new(), String::new(), Some(lead.map_hash.clone())),
            Group::Maps => (title, artist, Some(lead.map_hash.clone())),
            Group::Players => (lead.player.clone(), String::new(), None),
            Group::Days => (key.clone(), String::new(), Some(lead.map_hash.clone())),
            Group::Flat => unreachable!(),
        };
        let mut marks: Vec<Option<f64>> = Vec::new();
        if group == Group::Maps {
            let mut seen: Vec<String> = Vec::new();
            for at in &members {
                let (_, _, version) = song(&entries[*at]);
                if !seen.contains(&version) {
                    seen.push(version);
                    marks.push(rating(context, &entries[*at]).map(|r| r.stars));
                }
            }
        }
        rows.push(Row::Head { title: head_title, under, first: members[0], count: members.len(), open, cover, marks });
        if !open {
            continue;
        }
        let mut last_version: Option<String> = None;
        for at in &members {
            let entry = &entries[*at];
            if group == Group::Maps {
                let (_, _, version) = song(entry);
                if last_version.as_ref() != Some(&version) {
                    rows.push(Row::Difficulty { version: version.clone(), stars: rating(context, entry).map(|r| r.stars), first: *at });
                    last_version = Some(version);
                }
            }
            rows.push(Row::Replay { at: *at, chosen: Some(*at) == chosen });
        }
    }
    Laid { rows, order, chosen }
}

pub fn insets(rows: &[Row]) -> Vec<f32> {
    let at = rows.iter().position(|row| matches!(row, Row::Replay { chosen: true, .. })).unwrap_or(0);
    rows.iter()
        .enumerate()
        .map(|(i, row)| row.depth() + i.abs_diff(at).min(ARC_STEPS) as f32 * ARC_STEP)
        .collect()
}

pub fn attached(rows: &[Row], at: usize) -> bool {
    if !matches!(rows.get(at), Some(Row::Replay { .. })) {
        return false;
    }
    for row in rows[..at].iter().rev() {
        match row {
            Row::Replay { .. } => continue,
            Row::Difficulty { .. } => return true,
            Row::Head { .. } => return false,
        }
    }
    false
}

pub fn top_of(rows: &[Row], at: usize) -> f32 {
    (0..at).map(|i| rows[i].height() + if attached(rows, i + 1) { 0.0 } else { GAP }).sum()
}

pub fn offset(rows: &[Row], height: f32) -> f32 {
    let Some(at) = rows.iter().position(|row| matches!(row, Row::Replay { chosen: true, .. })) else {
        return 0.0;
    };
    let before = top_of(rows, at);
    (height / 2.0 - before - rows[at].height() / 2.0).round().min(0.0)
}

pub fn step(order: &[usize], chosen: Option<usize>, by: i32) -> Option<usize> {
    if order.is_empty() {
        return None;
    }
    let Some(at) = chosen.and_then(|c| order.iter().position(|o| *o == c)) else {
        return order.first().copied();
    };
    let next = (at as i64 + i64::from(by)).clamp(0, order.len() as i64 - 1) as usize;
    Some(order[next])
}

pub fn jump(laid: &Laid, entries: &[Entry], group: Group, context: &Context<'_>, by: i32) -> Option<usize> {
    let chosen = laid.chosen?;
    let mine = group_key(&entries[chosen], group, context);
    let heads: Vec<usize> = laid.order.iter().copied().fold(Vec::new(), |mut firsts: Vec<usize>, at| {
        let key = group_key(&entries[at], group, context);
        if firsts.last().map(|last| group_key(&entries[*last], group, context)) != Some(key) {
            firsts.push(at);
        }
        firsts
    });
    let here = heads.iter().position(|first| group_key(&entries[*first], group, context) == mine)?;
    let next = (here as i64 + i64::from(by)).clamp(0, heads.len() as i64 - 1) as usize;
    Some(heads[next])
}

pub fn star_at(fraction: f32) -> f64 {
    (f64::from(fraction.clamp(0.0, 1.0)) * STARS_TOP * 10.0).round() / 10.0
}

pub fn nearer_low(value: f64, low: f64, high: f64) -> bool {
    (value - low).abs() <= (value - high).abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{Client, Grade, Map, Outcome};
    use std::path::PathBuf;

    fn entry(player: &str, title: &str, version: &str, played_at: i64, accuracy: f64, miss: u16) -> Entry {
        Entry {
            path: PathBuf::from(format!("/r/{player}-{title}-{version}-{played_at}.osr")),
            kind: crate::sources::Kind::Stable,
            client: Client::Stable,
            player: player.to_owned(),
            replay_hash: format!("{player}{title}{version}{played_at}"),
            map_hash: format!("{title}{version}"),
            mods: Vec::new(),
            combo: 100,
            counts: [100, 2, 0, miss],
            accuracy,
            grade: Grade::A,
            outcome: if miss > 0 { Outcome::Misses(miss) } else { Outcome::FullCombo },
            played_at,
            named: None,
            map: Some(Map { file: PathBuf::from(format!("/m/{title}.osu")), artist: "xi".to_owned(), title: title.to_owned(), version: version.to_owned(), background: None }),
        }
    }

    fn sample() -> Vec<Entry> {
        vec![
            entry("NaumRedlo", "Blue Zenith", "FOUR DIMENSIONS", 50, 95.0, 3),
            entry("Lopuhh", "FREEDOM DiVE", "FOUR DIMENSIONS", 40, 97.0, 0),
            entry("NaumRedlo", "Blue Zenith", "Another", 30, 99.0, 0),
            entry("Guest", "Blue Zenith", "FOUR DIMENSIONS", 20, 98.0, 0),
        ]
    }

    fn with<T>(ratings: &HashMap<String, Rating>, run: impl FnOnce(&Context<'_>) -> T) -> T {
        let witnessed = |entry: &Entry| entry.player == "Guest";
        let day = |at: i64| if at >= 40 { "сегодня".to_owned() } else { "вчера".to_owned() };
        run(&Context { ratings, me: "NaumRedlo", witnessed: &witnessed, day: &day })
    }

    #[test]
    fn maps_group_sets_and_open_only_the_chosen_one_with_difficulties_inside() {
        let entries = sample();
        let laid = with(&HashMap::new(), |c| lay(&entries, &Filter::default(), Sort::Date, Group::Maps, Some(0), c));
        let kinds: Vec<String> = laid.rows.iter().map(|row| match row {
            Row::Head { title, count, open, marks, .. } => format!("head {title} {count} {open} {}", marks.len()),
            Row::Difficulty { version, .. } => format!("diff {version}"),
            Row::Replay { at, chosen } => format!("replay {at} {chosen}"),
        }).collect();
        assert_eq!(kinds, vec![
            "head Blue Zenith 3 true 2", "diff FOUR DIMENSIONS", "replay 0 true", "replay 3 false", "diff Another", "replay 2 false",
            "head FREEDOM DiVE 1 false 1",
        ]);
        assert_eq!(laid.order, vec![0, 3, 2, 1]);
    }

    #[test]
    fn filters_narrow_by_owner_misses_witness_and_stars() {
        let entries = sample();
        let mut ratings = HashMap::new();
        ratings.insert(entries[0].replay_hash.clone(), Rating { stars: 7.5, pp: 400.0, max_combo: 1000, pp_fc: 0.0, pp_ss: 0.0 });
        ratings.insert(entries[1].replay_hash.clone(), Rating { stars: 5.0, pp: 200.0, max_combo: 900, pp_fc: 0.0, pp_ss: 0.0 });
        let order = |filter: Filter| with(&ratings, |c| lay(&entries, &filter, Sort::Date, Group::Flat, None, c).order);
        assert_eq!(order(Filter { mine: true, ..Filter::default() }), vec![0, 2]);
        assert_eq!(order(Filter { clean: true, ..Filter::default() }), vec![1, 2, 3]);
        assert_eq!(order(Filter { witness: true, ..Filter::default() }), vec![3]);
        assert_eq!(order(Filter { low: 6.0, ..Filter::default() }), vec![0]);
        assert_eq!(order(Filter { high: 6.0, ..Filter::default() }), vec![1]);
        assert_eq!(order(Filter { query: "+hd".to_owned(), ..Filter::default() }), Vec::<usize>::new());
    }

    #[test]
    fn sorting_by_pp_puts_unknown_ratings_last_and_groups_follow_their_best() {
        let entries = sample();
        let mut ratings = HashMap::new();
        ratings.insert(entries[1].replay_hash.clone(), Rating { stars: 5.0, pp: 300.0, max_combo: 900, pp_fc: 0.0, pp_ss: 0.0 });
        ratings.insert(entries[3].replay_hash.clone(), Rating { stars: 7.0, pp: 100.0, max_combo: 900, pp_fc: 0.0, pp_ss: 0.0 });
        let laid = with(&ratings, |c| lay(&entries, &Filter::default(), Sort::Pp, Group::Flat, None, c));
        assert_eq!(laid.order, vec![1, 3, 0, 2]);
        let players = with(&ratings, |c| lay(&entries, &Filter::default(), Sort::Pp, Group::Players, Some(3), c));
        let heads: Vec<&str> = players.rows.iter().filter_map(|row| match row { Row::Head { title, .. } => Some(title.as_str()), _ => None }).collect();
        assert_eq!(heads, vec!["Lopuhh", "Guest", "NaumRedlo"]);
    }

    #[test]
    fn a_chosen_replay_that_is_filtered_away_gives_way_to_the_first_one() {
        let entries = sample();
        let laid = with(&HashMap::new(), |c| lay(&entries, &Filter { clean: true, ..Filter::default() }, Sort::Date, Group::Flat, Some(0), c));
        assert_eq!(laid.chosen, Some(1));
    }

    #[test]
    fn the_arc_grows_away_from_the_chosen_row_and_the_chosen_row_sits_in_the_middle() {
        let rows = vec![
            Row::Head { title: "a".into(), under: String::new(), first: 0, count: 2, open: true, cover: None, marks: Vec::new() },
            Row::Replay { at: 0, chosen: false },
            Row::Replay { at: 1, chosen: true },
            Row::Head { title: "b".into(), under: String::new(), first: 2, count: 1, open: false, cover: None, marks: Vec::new() },
        ];
        assert_eq!(insets(&rows), vec![12.0, 30.0, 24.0, 6.0]);
        assert_eq!(offset(&rows, 600.0), 0.0);
        assert_eq!(offset(&rows, 200.0), (100.0 - (80.0 + GAP + 60.0 + GAP) - 30.0).round());
    }

    #[test]
    fn replays_under_a_difficulty_sit_flush_against_it_and_heads_keep_their_gap() {
        let rows = vec![
            Row::Head { title: "a".into(), under: String::new(), first: 0, count: 2, open: true, cover: None, marks: Vec::new() },
            Row::Difficulty { version: "x".into(), stars: None, first: 0 },
            Row::Replay { at: 0, chosen: false },
            Row::Replay { at: 1, chosen: true },
            Row::Head { title: "b".into(), under: String::new(), first: 2, count: 1, open: false, cover: None, marks: Vec::new() },
        ];
        let flush: Vec<bool> = (0..rows.len()).map(|i| attached(&rows, i)).collect();
        assert_eq!(flush, vec![false, false, true, true, false]);
        assert_eq!(top_of(&rows, 3), 80.0 + GAP + 36.0 + 60.0);
    }

    #[test]
    fn stepping_walks_the_whole_order_and_jumping_moves_between_groups() {
        let entries = sample();
        let laid = with(&HashMap::new(), |c| lay(&entries, &Filter::default(), Sort::Date, Group::Maps, Some(0), c));
        assert_eq!(step(&laid.order, Some(0), 1), Some(3));
        assert_eq!(step(&laid.order, Some(1), 1), Some(1));
        assert_eq!(step(&laid.order, None, 1), Some(0));
        assert_eq!(with(&HashMap::new(), |c| jump(&laid, &entries, Group::Maps, c, 1)), Some(1));
        assert_eq!(with(&HashMap::new(), |c| jump(&laid, &entries, Group::Maps, c, -1)), Some(0));
    }

    #[test]
    fn a_click_on_the_star_bar_lands_on_a_tenth_and_moves_the_nearer_edge() {
        assert_eq!(star_at(0.333), 3.3);
        assert_eq!(star_at(1.4), STARS_TOP);
        assert!(nearer_low(2.0, 0.0, 10.0));
        assert!(!nearer_low(8.0, 0.0, 10.0));
    }
}
