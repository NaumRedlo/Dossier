use iced::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
    Mythic,
    Secret,
}

impl Rarity {
    pub const ALL: [Rarity; 7] = [Rarity::Common, Rarity::Uncommon, Rarity::Rare, Rarity::Epic, Rarity::Legendary, Rarity::Mythic, Rarity::Secret];

    pub fn colour(self) -> Color {
        let (r, g, b) = match self {
            Rarity::Common => (158, 158, 158),
            Rarity::Uncommon => (76, 175, 80),
            Rarity::Rare => (66, 165, 245),
            Rarity::Epic => (171, 71, 188),
            Rarity::Legendary => (255, 179, 0),
            Rarity::Mythic => (229, 57, 53),
            Rarity::Secret => (130, 96, 170),
        };
        Color::from_rgb8(r, g, b)
    }

    pub fn key(self) -> &'static str {
        match self {
            Rarity::Common => "rarity-common",
            Rarity::Uncommon => "rarity-uncommon",
            Rarity::Rare => "rarity-rare",
            Rarity::Epic => "rarity-epic",
            Rarity::Legendary => "rarity-legendary",
            Rarity::Mythic => "rarity-mythic",
            Rarity::Secret => "rarity-secret",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Title {
    pub code: &'static str,
    pub rarity: Rarity,
    pub name: [&'static str; 2],
    pub about: [&'static str; 2],
}

impl Title {
    pub fn name(&self, lang: crate::lang::Lang) -> &'static str {
        self.name[usize::from(lang == crate::lang::Lang::Ru)]
    }

    pub fn about(&self, lang: crate::lang::Lang) -> &'static str {
        self.about[usize::from(lang == crate::lang::Lang::Ru)]
    }
}

const fn title(code: &'static str, rarity: Rarity, en: &'static str, ru: &'static str, about_en: &'static str, about_ru: &'static str) -> Title {
    Title { code, rarity, name: [en, ru], about: [about_en, about_ru] }
}

pub const TITLES: &[Title] = &[
    title("registered", Rarity::Common, "It's Nice to Meet You", "Приятно познакомиться", "Sign up with the bot.", "Зарегистрируйся в боте."),
    title("graveyard", Rarity::Common, "Necrotourist", "Некротурист", "Play a map with Graveyard status.", "Сыграй карту со статусом Graveyard."),
    title("account_2y", Rarity::Common, "Citizen of Record", "Учтённый гражданин", "Have an account older than 2 years.", "Владей аккаунтом старше 2 лет."),
    title("s_50", Rarity::Common, "Serial Performer", "Серийный исполнитель", "Earn 50 S-ranks.", "Получи 50 S-рангов."),
    title("wysi", Rarity::Uncommon, "WYSI", "WYSI", "Get a combo containing the number 727.", "Набери комбо, содержащее число 727."),
    title("broken_record", Rarity::Uncommon, "On repeat!", "На повторе!", "Play one map 20 times.", "Сыграй одну карту 20 раз."),
    title("masks_5", Rarity::Uncommon, "Wardrobe of Masks", "Ведущий маскарада", "Play maps with 5 different mods.", "Сыграй карты с 5 разными модами."),
    title("ss_100", Rarity::Uncommon, "Five Collector", "Отличник", "Earn 100 SS ranks.", "Получи 100 рангов SS."),
    title("dejavu", Rarity::Rare, "Déjà Vu", "Дежавю", "Get the same score on two different maps.", "Набери одинаковый счёт на двух разных картах."),
    title("archaeologist", Rarity::Rare, "Archaeologist", "Археолог", "Pass a map ranked 12 years ago or earlier.", "Пройди карту, ранкнутую 12 лет назад или ранее."),
    title("combo_2000", Rarity::Rare, "Hardy", "Выносливый", "Get a 2000 combo or above on one score.", "Набери 2000 комбо и больше за одну игру."),
    title("heavy_hand", Rarity::Rare, "Heavy Hand", "Крепкая рука", "FC a map from 5* with AR 10.3 and above.", "Сделай FC карты от 5* с AR 10.3 и выше."),
    title("fc_bpm_210", Rarity::Epic, "Rapid Fire", "Скорострел", "FC a map from 240 BPM.", "Сделай FC карты от 240 BPM."),
    title("ss_hdfl_5", Rarity::Epic, "Tunnel Vision", "Туннельное зрение", "Get an SS on a map from 5* with HDFL.", "Получи SS на карте от 5* с HDFL."),
    title("sr_10", Rarity::Epic, "Double Digit Threat", "Двузначная угроза", "Pass a map of 10* or harder.", "Пройди карту сложностью 10* или выше."),
    title("archivist", Rarity::Legendary, "Archivist", "Архивариус", "Hold the highest ranked score in the chat.", "Держи лучший счёт ранговых очков в чате."),
    title("streak_30d", Rarity::Legendary, "Sleepless Watch", "Бессонная вахта", "Stay active 30 days in a row.", "Оставайся активным 30 дней подряд."),
    title("hdhr_fc7", Rarity::Legendary, "Double Sentence", "Двойной приговор", "FC a map from 7* with HDHR.", "Сделай FC карты от 7* с HDHR."),
    title("ss_8star", Rarity::Mythic, "The Machine", "Киборг", "Get an SS on a map from 8.5*.", "Получи SS на карте от 8.5*."),
    title("ss_streak_10", Rarity::Mythic, "Idealist", "Идеалист", "Get 10 SS ranks in a row.", "Получи 10 рангов SS подряд."),
    title("doublethink", Rarity::Secret, "Doublethink", "Двоемыслие", "SS an EZ map up to 2* and pass a map from 7*.", "Получи SS на карте с EZ до 2* и пройди карту от 7*."),
    title("choke_95", Rarity::Secret, "Not This Time", "Попытка не пытка", "Break a full combo in the last 5% at 99% accuracy or above.", "Сорви комбо в последних 5% при точности 99% и выше."),
];

pub fn title_of(code: &str) -> Option<&'static Title> {
    TITLES.iter().find(|title| title.code == code)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Board {
    Pp,
    Accuracy,
    Plays,
    Hours,
    Score,
    HitsPerPlay,
}

impl Board {
    pub const ALL: [Board; 6] = [Board::Pp, Board::Accuracy, Board::Plays, Board::Hours, Board::Score, Board::HitsPerPlay];

    pub fn key(self) -> &'static str {
        match self {
            Board::Pp => "board-pp",
            Board::Accuracy => "board-accuracy",
            Board::Plays => "board-plays",
            Board::Hours => "board-hours",
            Board::Score => "board-score",
            Board::HitsPerPlay => "board-hits",
        }
    }

    pub fn value(self, person: &Person) -> f64 {
        match self {
            Board::Pp => f64::from(person.pp),
            Board::Accuracy => f64::from(person.accuracy),
            Board::Plays => f64::from(person.plays),
            Board::Hours => f64::from(person.hours),
            Board::Score => person.score as f64,
            Board::HitsPerPlay => f64::from(person.hits_per_play),
        }
    }

    pub fn index(self) -> usize {
        Board::ALL.iter().position(|board| *board == self).unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Play {
    pub map: usize,
    pub pp: f32,
    pub accuracy: f32,
    pub mods: Vec<&'static str>,
    pub grade: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    pub name: String,
    pub country: &'static str,
    pub pp: u32,
    pub rank: u32,
    pub accuracy: f32,
    pub plays: u32,
    pub hours: u32,
    pub score: u64,
    pub hits_per_play: f32,
    pub title: Option<&'static str>,
    pub titles: Vec<&'static str>,
    pub streak: u32,
    pub moved: [i32; 6],
    pub top: Vec<Play>,
}

impl Person {
    pub fn initial(&self) -> String {
        self.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()
    }

    pub fn shown_title(&self) -> Option<&'static Title> {
        self.title.and_then(title_of)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapRef {
    pub hash: String,
    pub line: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Render { accuracy: f32, mods: Vec<&'static str> },
    TopPlay { pp: f32, place: u32, mods: Vec<&'static str> },
    Title(&'static str),
    Climb { board: Board, from: u32, to: u32 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Happening {
    pub who: usize,
    pub kind: Kind,
    pub map: Option<usize>,
    pub at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LivePlay {
    pub who: usize,
    pub map: usize,
    pub accuracy: f32,
    pub pp: f32,
    pub mods: Vec<&'static str>,
    pub grade: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Friend {
    pub name: String,
    pub country: &'static str,
    pub pp: u32,
    pub rank: u32,
    pub online: bool,
    pub seen_minutes: u32,
}

impl Friend {
    pub fn initial(&self) -> String {
        self.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Catalog {
    pub group: String,
    pub week: u32,
    pub people: Vec<Person>,
    pub maps: Vec<MapRef>,
    pub feed: Vec<Happening>,
    pub live: Vec<LivePlay>,
    pub friends: Vec<Friend>,
}

impl Catalog {
    pub fn ranked(&self, board: Board) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.people.len()).collect();
        order.sort_by(|a, b| board.value(&self.people[*b]).total_cmp(&board.value(&self.people[*a])));
        order
    }

    pub fn holders(&self, code: &str) -> Vec<usize> {
        self.people.iter().enumerate().filter(|(_, person)| person.titles.contains(&code)).map(|(at, _)| at).collect()
    }

    pub fn map(&self, at: Option<usize>) -> Option<&MapRef> {
        at.and_then(|at| self.maps.get(at))
    }

    pub fn staged(maps: Vec<MapRef>, you: &str, now: i64) -> Catalog {
        let maps = if maps.is_empty() { vec![MapRef { hash: String::new(), line: "Y&Co. — Daisuke [moph's Expert]".to_owned() }] } else { maps };
        let spot = |n: usize| n % maps.len();
        let play = |map: usize, pp: f32, accuracy: f32, mods: &[&'static str], grade: &'static str| Play { map: spot(map), pp, accuracy, mods: mods.to_vec(), grade };
        let person = |name: &str, country: &'static str, pp: u32, rank: u32, accuracy: f32, plays: u32, hours: u32, score: u64, hits_per_play: f32| Person {
            name: name.to_owned(),
            country,
            pp,
            rank,
            accuracy,
            plays,
            hours,
            score,
            hits_per_play,
            title: None,
            titles: vec!["registered"],
            streak: 0,
            moved: [0; 6],
            top: Vec::new(),
        };
        let you = if you.trim().is_empty() { "NaumRedlo" } else { you.trim_start_matches('@') };
        let mut people = vec![
            person(you, "RU", 9_870, 11_402, 97.84, 38_120, 1_204, 61_880_412_330, 612.4),
            person("kotofey", "RU", 12_480, 3_402, 98.61, 52_904, 1_880, 98_120_553_870, 704.9),
            person("Mirrorwave", "KZ", 11_215, 5_127, 96.92, 44_701, 1_512, 83_411_020_117, 655.2),
            person("ssnowy", "BY", 8_402, 18_966, 98.95, 21_330, 702, 40_227_985_004, 588.0),
            person("d1ce", "RU", 7_655, 26_310, 95.71, 61_008, 2_104, 71_906_338_201, 431.6),
            person("Aurelian", "UA", 6_120, 44_871, 97.12, 17_455, 540, 22_004_117_530, 520.3),
            person("tarakan_3000", "RU", 4_890, 78_224, 93.48, 29_870, 966, 18_550_402_876, 377.1),
            person("lumen", "RU", 3_402, 131_560, 96.40, 9_805, 311, 7_120_993_445, 402.8),
        ];
        let equip = |person: &mut Person, title: Option<&'static str>, titles: &[&'static str], streak: u32, moved: [i32; 6]| {
            person.title = title;
            person.titles.extend_from_slice(titles);
            person.streak = streak;
            person.moved = moved;
        };
        equip(&mut people[0], Some("wysi"), &["wysi", "masks_5", "graveyard", "s_50", "combo_2000", "doublethink"], 12, [1, 0, 0, -1, 0, 2]);
        equip(&mut people[1], Some("hdhr_fc7"), &["hdhr_fc7", "archivist", "ss_hdfl_5", "fc_bpm_210", "ss_100", "account_2y", "s_50", "heavy_hand"], 41, [0, 1, 0, 0, 0, -1]);
        equip(&mut people[2], Some("fc_bpm_210"), &["fc_bpm_210", "combo_2000", "broken_record", "account_2y", "s_50"], 6, [0, -1, 1, 0, 0, 0]);
        equip(&mut people[3], Some("ss_streak_10"), &["ss_streak_10", "ss_100", "archaeologist", "s_50"], 30, [2, 0, -2, 0, 1, 0]);
        equip(&mut people[4], Some("streak_30d"), &["streak_30d", "broken_record", "graveyard", "account_2y"], 33, [-2, 0, 1, 1, -1, 0]);
        equip(&mut people[5], None, &["masks_5", "dejavu"], 2, [0, 0, 0, 0, 0, 1]);
        equip(&mut people[6], Some("broken_record"), &["broken_record", "graveyard", "choke_95"], 0, [-1, 0, -1, 0, 0, 0]);
        equip(&mut people[7], Some("registered"), &[], 4, [0, 0, 1, -1, 0, -2]);
        let tops: [&[Play]; 8] = [
            &[play(0, 412.6, 98.34, &["HD"], "S"), play(1, 398.1, 97.02, &["HD", "DT"], "A"), play(2, 377.9, 99.10, &[], "S")],
            &[play(3, 611.2, 99.21, &["HD", "HR"], "S"), play(0, 588.7, 98.44, &["DT"], "S"), play(4, 571.0, 97.66, &["HD", "DT"], "A")],
            &[play(1, 540.3, 97.80, &["HD", "DT"], "S"), play(5, 522.8, 96.31, &["DT"], "A"), play(2, 501.4, 98.96, &["HD"], "S")],
            &[play(2, 402.2, 100.0, &["HD"], "SS"), play(6, 377.5, 99.87, &[], "SS"), play(3, 360.0, 99.40, &["HR"], "S")],
            &[play(4, 356.9, 95.12, &["DT"], "A"), play(0, 341.3, 96.40, &["HD", "DT"], "A"), play(1, 330.8, 94.72, &[], "B")],
            &[play(5, 288.0, 97.90, &["HD"], "S"), play(3, 271.4, 96.55, &[], "A"), play(6, 255.2, 98.01, &["HD"], "S")],
            &[play(6, 231.7, 92.30, &["DT"], "B"), play(4, 219.9, 93.85, &[], "A"), play(0, 201.6, 91.02, &["HD"], "B")],
            &[play(1, 164.0, 96.12, &[], "A"), play(2, 151.8, 97.40, &["HD"], "S"), play(5, 140.5, 95.03, &[], "A")],
        ];
        for (person, top) in people.iter_mut().zip(tops) {
            person.top = top.to_vec();
        }
        let minutes = |m: i64| now - m * 60;
        let feed = vec![
            Happening { who: 1, kind: Kind::Render { accuracy: 99.21, mods: vec!["HD", "HR"] }, map: Some(spot(3)), at: minutes(18) },
            Happening { who: 0, kind: Kind::TopPlay { pp: 412.6, place: 1, mods: vec!["HD"] }, map: Some(spot(0)), at: minutes(52) },
            Happening { who: 3, kind: Kind::Title("ss_streak_10"), map: None, at: minutes(95) },
            Happening { who: 2, kind: Kind::Climb { board: Board::Pp, from: 4, to: 2 }, map: None, at: minutes(240) },
            Happening { who: 0, kind: Kind::Render { accuracy: 98.34, mods: vec!["HD"] }, map: Some(spot(0)), at: minutes(300) },
            Happening { who: 4, kind: Kind::Title("streak_30d"), map: None, at: minutes(26 * 60) },
            Happening { who: 2, kind: Kind::TopPlay { pp: 540.3, place: 1, mods: vec!["HD", "DT"] }, map: Some(spot(1)), at: minutes(27 * 60 + 12) },
            Happening { who: 6, kind: Kind::Render { accuracy: 92.30, mods: vec!["DT"] }, map: Some(spot(6)), at: minutes(30 * 60) },
            Happening { who: 3, kind: Kind::Climb { board: Board::Accuracy, from: 2, to: 1 }, map: None, at: minutes(50 * 60) },
            Happening { who: 1, kind: Kind::Title("archivist"), map: None, at: minutes(52 * 60) },
        ];
        let live_play = |who: usize, map: usize, accuracy: f32, pp: f32, mods: &[&'static str], grade: &'static str| LivePlay { who, map: spot(map), accuracy, pp, mods: mods.to_vec(), grade };
        let live = vec![
            live_play(4, 2, 94.12, 188.0, &["DT"], "A"),
            live_play(7, 5, 97.80, 64.2, &[], "S"),
            live_play(2, 1, 98.02, 402.7, &["HD"], "S"),
            live_play(6, 3, 91.44, 120.5, &[], "B"),
            live_play(0, 0, 96.71, 311.9, &["HD", "DT"], "A"),
            live_play(1, 4, 99.40, 530.0, &["HD", "HR"], "S"),
            live_play(3, 6, 100.0, 288.4, &["HD"], "SS"),
            live_play(5, 2, 95.55, 142.1, &["DT"], "A"),
            live_play(4, 1, 92.87, 205.3, &[], "B"),
            live_play(2, 0, 97.13, 377.6, &["HD", "DT"], "S"),
            live_play(0, 3, 98.61, 280.8, &["HD"], "S"),
            live_play(7, 6, 93.30, 51.4, &[], "B"),
            live_play(1, 5, 98.88, 498.2, &["DT"], "S"),
            live_play(6, 4, 89.90, 97.3, &["HD"], "C"),
        ];
        let friend = |name: &str, country: &'static str, pp: u32, rank: u32, online: bool, seen_minutes: u32| Friend { name: name.to_owned(), country, pp, rank, online, seen_minutes };
        let friends = vec![
            friend("quietstorm", "PL", 13_902, 1_988, true, 0),
            friend("Riv3r", "DE", 10_440, 7_310, true, 0),
            friend("moonlit", "JP", 9_120, 12_840, false, 35),
            friend("tsubasa_", "RU", 8_015, 20_117, true, 0),
            friend("Kest", "US", 7_402, 27_604, false, 180),
            friend("pixelfox", "FI", 5_660, 51_230, false, 2_880),
            friend("hanabi", "KR", 4_210, 88_902, false, 60),
        ];
        Catalog { group: "osu! RU".to_owned(), week: 39, people, maps, feed, live, friends }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_title_the_catalogue_names_is_known() {
        let catalog = Catalog::staged(Vec::new(), "", 0);
        for person in &catalog.people {
            for code in person.titles.iter().chain(person.title.iter()) {
                assert!(title_of(code).is_some(), "{} holds an unknown title {code}", person.name);
            }
            if let Some(worn) = person.title {
                assert!(person.titles.contains(&worn), "{} wears a title they do not hold", person.name);
            }
        }
        for happening in &catalog.feed {
            if let Kind::Title(code) = happening.kind {
                assert!(catalog.people[happening.who].titles.contains(&code));
            }
        }
    }

    #[test]
    fn a_board_ranks_from_the_top() {
        let catalog = Catalog::staged(Vec::new(), "", 0);
        let by_pp = catalog.ranked(Board::Pp);
        assert_eq!(catalog.people[by_pp[0]].name, "kotofey");
        let values: Vec<f64> = by_pp.iter().map(|at| Board::Pp.value(&catalog.people[*at])).collect();
        assert!(values.windows(2).all(|pair| pair[0] >= pair[1]));
    }

    #[test]
    fn every_live_play_is_by_someone_on_some_map() {
        let catalog = Catalog::staged(Vec::new(), "", 0);
        assert!(catalog.live.len() > 6);
        assert!(catalog.live.iter().all(|play| play.who < catalog.people.len() && play.map < catalog.maps.len()));
    }

    #[test]
    fn the_person_opening_the_catalogue_is_in_it() {
        let catalog = Catalog::staged(Vec::new(), "@someone", 0);
        assert_eq!(catalog.people[0].name, "someone");
    }
}
