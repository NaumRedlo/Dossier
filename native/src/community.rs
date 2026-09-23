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

    pub fn of(said: &str) -> Rarity {
        match said {
            "uncommon" => Rarity::Uncommon,
            "rare" => Rarity::Rare,
            "epic" => Rarity::Epic,
            "legendary" => Rarity::Legendary,
            "mythic" => Rarity::Mythic,
            "secret" => Rarity::Secret,
            _ => Rarity::Common,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Title {
    pub code: String,
    pub rarity: Rarity,
    pub name: [String; 2],
    pub about: [String; 2],
}

impl Title {
    pub fn name(&self, lang: crate::lang::Lang) -> &str {
        &self.name[usize::from(lang == crate::lang::Lang::Ru)]
    }

    pub fn about(&self, lang: crate::lang::Lang) -> &str {
        &self.about[usize::from(lang == crate::lang::Lang::Ru)]
    }
}

const STAGED_TITLES: &[(&str, Rarity, &str, &str, &str, &str)] = &[
    ("registered", Rarity::Common, "It's Nice to Meet You", "Приятно познакомиться", "Sign up with the bot.", "Зарегистрируйся в боте."),
    ("graveyard", Rarity::Common, "Necrotourist", "Некротурист", "Play a map with Graveyard status.", "Сыграй карту со статусом Graveyard."),
    ("account_2y", Rarity::Common, "Citizen of Record", "Учтённый гражданин", "Have an account older than 2 years.", "Владей аккаунтом старше 2 лет."),
    ("s_50", Rarity::Common, "Serial Performer", "Серийный исполнитель", "Earn 50 S-ranks.", "Получи 50 S-рангов."),
    ("wysi", Rarity::Uncommon, "WYSI", "WYSI", "Get a combo containing the number 727.", "Набери комбо, содержащее число 727."),
    ("broken_record", Rarity::Uncommon, "On repeat!", "На повторе!", "Play one map 20 times.", "Сыграй одну карту 20 раз."),
    ("masks_5", Rarity::Uncommon, "Wardrobe of Masks", "Ведущий маскарада", "Play maps with 5 different mods.", "Сыграй карты с 5 разными модами."),
    ("ss_100", Rarity::Uncommon, "Five Collector", "Отличник", "Earn 100 SS ranks.", "Получи 100 рангов SS."),
    ("dejavu", Rarity::Rare, "Déjà Vu", "Дежавю", "Get the same score on two different maps.", "Набери одинаковый счёт на двух разных картах."),
    ("archaeologist", Rarity::Rare, "Archaeologist", "Археолог", "Pass a map ranked 12 years ago or earlier.", "Пройди карту, ранкнутую 12 лет назад или ранее."),
    ("combo_2000", Rarity::Rare, "Hardy", "Выносливый", "Get a 2000 combo or above on one score.", "Набери 2000 комбо и больше за одну игру."),
    ("heavy_hand", Rarity::Rare, "Heavy Hand", "Крепкая рука", "FC a map from 5* with AR 10.3 and above.", "Сделай FC карты от 5* с AR 10.3 и выше."),
    ("fc_bpm_210", Rarity::Epic, "Rapid Fire", "Скорострел", "FC a map from 240 BPM.", "Сделай FC карты от 240 BPM."),
    ("ss_hdfl_5", Rarity::Epic, "Tunnel Vision", "Туннельное зрение", "Get an SS on a map from 5* with HDFL.", "Получи SS на карте от 5* с HDFL."),
    ("sr_10", Rarity::Epic, "Double Digit Threat", "Двузначная угроза", "Pass a map of 10* or harder.", "Пройди карту сложностью 10* или выше."),
    ("archivist", Rarity::Legendary, "Archivist", "Архивариус", "Hold the highest ranked score in the chat.", "Держи лучший счёт ранговых очков в чате."),
    ("streak_30d", Rarity::Legendary, "Sleepless Watch", "Бессонная вахта", "Stay active 30 days in a row.", "Оставайся активным 30 дней подряд."),
    ("hdhr_fc7", Rarity::Legendary, "Double Sentence", "Двойной приговор", "FC a map from 7* with HDHR.", "Сделай FC карты от 7* с HDHR."),
    ("ss_8star", Rarity::Mythic, "The Machine", "Киборг", "Get an SS on a map from 8.5*.", "Получи SS на карте от 8.5*."),
    ("ss_streak_10", Rarity::Mythic, "Idealist", "Идеалист", "Get 10 SS ranks in a row.", "Получи 10 рангов SS подряд."),
    ("doublethink", Rarity::Secret, "Doublethink", "Двоемыслие", "SS an EZ map up to 2* and pass a map from 7*.", "Получи SS на карте с EZ до 2* и пройди карту от 7*."),
    ("choke_95", Rarity::Secret, "Not This Time", "Попытка не пытка", "Break a full combo in the last 5% at 99% accuracy or above.", "Сорви комбо в последних 5% при точности 99% и выше."),
];

pub fn staged_titles() -> Vec<Title> {
    STAGED_TITLES
        .iter()
        .map(|(code, rarity, en, ru, about_en, about_ru)| Title {
            code: (*code).to_owned(),
            rarity: *rarity,
            name: [(*en).to_owned(), (*ru).to_owned()],
            about: [(*about_en).to_owned(), (*about_ru).to_owned()],
        })
        .collect()
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

    pub fn of(said: &str) -> Option<Board> {
        match said {
            "pp" => Some(Board::Pp),
            "accuracy" => Some(Board::Accuracy),
            "play_count" => Some(Board::Plays),
            "play_time" => Some(Board::Hours),
            "ranked_score" => Some(Board::Score),
            "hits_per_play" => Some(Board::HitsPerPlay),
            _ => None,
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
    pub mods: Vec<String>,
    pub grade: String,
    pub at: i64,
    pub full_combo: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Person {
    pub id: i64,
    pub name: String,
    pub country: String,
    pub pp: u32,
    pub rank: u32,
    pub accuracy: f32,
    pub plays: u32,
    pub hours: u32,
    pub score: u64,
    pub hits_per_play: f32,
    pub level: u32,
    pub ss: u32,
    pub s: u32,
    pub joined: i64,
    pub supporter: bool,
    pub title: Option<String>,
    pub titles: Vec<String>,
    pub streak: u32,
    pub streak_best: u32,
    pub avatar: String,
    pub cover: String,
    pub moved: [i32; 6],
    pub gained: [f64; 6],
    pub top: Vec<Play>,
    pub you: bool,
}

impl Person {
    pub fn initial(&self) -> String {
        self.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapRef {
    pub hash: String,
    pub line: String,
    pub set: Option<u64>,
    pub stars: Option<f32>,
}

impl MapRef {
    pub fn local(hash: String, line: String) -> MapRef {
        MapRef { hash, line, set: None, stars: None }
    }

    pub fn card(&self) -> Option<String> {
        self.set.map(|set| format!("https://assets.ppy.sh/beatmaps/{set}/covers/card.jpg"))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Render { accuracy: f32, mods: Vec<String> },
    TopPlay { pp: f32, place: u32, mods: Vec<String> },
    Title(String),
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
    pub mods: Vec<String>,
    pub grade: String,
    pub at: i64,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Friend {
    pub name: String,
    pub country: String,
    pub pp: u32,
    pub rank: u32,
    pub online: bool,
    pub seen: Option<i64>,
    pub avatar: String,
}

impl Friend {
    pub fn initial(&self) -> String {
        self.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()
    }

    pub fn minutes_away(&self, now: i64) -> u32 {
        match (self.online, self.seen) {
            (true, _) => 0,
            (false, Some(seen)) => ((now - seen).max(60) / 60) as u32,
            (false, None) => u32::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Me {
    pub person: Person,
    pub recent: Vec<Play>,
    pub duels: [u32; 2],
    pub points: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Friends {
    Staged,
    Waiting,
    Need(String),
    Failed,
    Ready,
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
    pub friends_state: Friends,
    pub titles: Vec<Title>,
    pub me: Option<Me>,
    pub staged: bool,
}

impl Catalog {
    pub fn ranked(&self, board: Board) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.people.len()).collect();
        order.sort_by(|a, b| board.value(&self.people[*b]).total_cmp(&board.value(&self.people[*a])));
        order
    }

    pub fn holders(&self, code: &str) -> Vec<usize> {
        self.people.iter().enumerate().filter(|(_, person)| person.titles.iter().any(|held| held == code)).map(|(at, _)| at).collect()
    }

    pub fn map(&self, at: Option<usize>) -> Option<&MapRef> {
        at.and_then(|at| self.maps.get(at))
    }

    pub fn title_of(&self, code: &str) -> Option<&Title> {
        self.titles.iter().find(|title| title.code == code)
    }

    pub fn shown_title(&self, person: &Person) -> Option<&Title> {
        person.title.as_deref().and_then(|code| self.title_of(code))
    }

    pub fn you(&self) -> Option<&Person> {
        self.me.as_ref().map(|me| &me.person).or_else(|| self.people.iter().find(|person| person.you))
    }

    pub fn staged(maps: Vec<MapRef>, you: &str, now: i64) -> Catalog {
        let maps = if maps.is_empty() { vec![MapRef::local(String::new(), "Y&Co. — Daisuke [moph's Expert]".to_owned())] } else { maps };
        let spot = |n: usize| n % maps.len();
        let owned = |list: &[&str]| list.iter().map(|m| (*m).to_owned()).collect::<Vec<String>>();
        let play = |map: usize, pp: f32, accuracy: f32, mods: &[&str], grade: &str| Play { map: spot(map), pp, accuracy, mods: owned(mods), grade: grade.to_owned(), at: 0, full_combo: false };
        let person = |name: &str, country: &str, pp: u32, rank: u32, accuracy: f32, plays: u32, hours: u32, score: u64, hits_per_play: f32| Person {
            name: name.to_owned(),
            country: country.to_owned(),
            pp,
            rank,
            accuracy,
            plays,
            hours,
            score,
            hits_per_play,
            titles: vec!["registered".to_owned()],
            ..Person::default()
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
        people[0].you = true;
        people[0].level = 101;
        people[0].ss = 214;
        people[0].s = 1_388;
        people[0].streak_best = 27;
        let equip = |person: &mut Person, title: Option<&str>, titles: &[&str], streak: u32, moved: [i32; 6]| {
            person.title = title.map(str::to_owned);
            person.titles.extend(titles.iter().map(|code| (*code).to_owned()));
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
        let tops: [Vec<Play>; 8] = [
            vec![play(0, 412.6, 98.34, &["HD"], "S"), play(1, 398.1, 97.02, &["HD", "DT"], "A"), play(2, 377.9, 99.10, &[], "S")],
            vec![play(3, 611.2, 99.21, &["HD", "HR"], "S"), play(0, 588.7, 98.44, &["DT"], "S"), play(4, 571.0, 97.66, &["HD", "DT"], "A")],
            vec![play(1, 540.3, 97.80, &["HD", "DT"], "S"), play(5, 522.8, 96.31, &["DT"], "A"), play(2, 501.4, 98.96, &["HD"], "S")],
            vec![play(2, 402.2, 100.0, &["HD"], "SS"), play(6, 377.5, 99.87, &[], "SS"), play(3, 360.0, 99.40, &["HR"], "S")],
            vec![play(4, 356.9, 95.12, &["DT"], "A"), play(0, 341.3, 96.40, &["HD", "DT"], "A"), play(1, 330.8, 94.72, &[], "B")],
            vec![play(5, 288.0, 97.90, &["HD"], "S"), play(3, 271.4, 96.55, &[], "A"), play(6, 255.2, 98.01, &["HD"], "S")],
            vec![play(6, 231.7, 92.30, &["DT"], "B"), play(4, 219.9, 93.85, &[], "A"), play(0, 201.6, 91.02, &["HD"], "B")],
            vec![play(1, 164.0, 96.12, &[], "A"), play(2, 151.8, 97.40, &["HD"], "S"), play(5, 140.5, 95.03, &[], "A")],
        ];
        for (person, top) in people.iter_mut().zip(tops) {
            person.top = top;
        }
        let minutes = |m: i64| now - m * 60;
        let feed = vec![
            Happening { who: 1, kind: Kind::Render { accuracy: 99.21, mods: owned(&["HD", "HR"]) }, map: Some(spot(3)), at: minutes(18) },
            Happening { who: 0, kind: Kind::TopPlay { pp: 412.6, place: 1, mods: owned(&["HD"]) }, map: Some(spot(0)), at: minutes(52) },
            Happening { who: 3, kind: Kind::Title("ss_streak_10".to_owned()), map: None, at: minutes(95) },
            Happening { who: 2, kind: Kind::Climb { board: Board::Pp, from: 4, to: 2 }, map: None, at: minutes(240) },
            Happening { who: 0, kind: Kind::Render { accuracy: 98.34, mods: owned(&["HD"]) }, map: Some(spot(0)), at: minutes(300) },
            Happening { who: 4, kind: Kind::Title("streak_30d".to_owned()), map: None, at: minutes(26 * 60) },
            Happening { who: 2, kind: Kind::TopPlay { pp: 540.3, place: 1, mods: owned(&["HD", "DT"]) }, map: Some(spot(1)), at: minutes(27 * 60 + 12) },
            Happening { who: 6, kind: Kind::Render { accuracy: 92.30, mods: owned(&["DT"]) }, map: Some(spot(6)), at: minutes(30 * 60) },
            Happening { who: 3, kind: Kind::Climb { board: Board::Accuracy, from: 2, to: 1 }, map: None, at: minutes(50 * 60) },
            Happening { who: 1, kind: Kind::Title("archivist".to_owned()), map: None, at: minutes(52 * 60) },
        ];
        let live_play = |who: usize, map: usize, accuracy: f32, pp: f32, mods: &[&str], grade: &str| LivePlay { who, map: spot(map), accuracy, pp, mods: owned(mods), grade: grade.to_owned(), at: now, passed: true };
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
        let friend = |name: &str, country: &str, pp: u32, rank: u32, online: bool, away: i64| Friend {
            name: name.to_owned(),
            country: country.to_owned(),
            pp,
            rank,
            online,
            seen: (!online).then_some(now - away * 60),
            avatar: String::new(),
        };
        let friends = vec![
            friend("quietstorm", "PL", 13_902, 1_988, true, 0),
            friend("Riv3r", "DE", 10_440, 7_310, true, 0),
            friend("moonlit", "JP", 9_120, 12_840, false, 35),
            friend("tsubasa_", "RU", 8_015, 20_117, true, 0),
            friend("Kest", "US", 7_402, 27_604, false, 180),
            friend("pixelfox", "FI", 5_660, 51_230, false, 2_880),
            friend("hanabi", "KR", 4_210, 88_902, false, 60),
        ];
        Catalog {
            group: "osu! RU".to_owned(),
            week: 39,
            people,
            maps,
            feed,
            live,
            friends,
            friends_state: Friends::Staged,
            titles: staged_titles(),
            me: None,
            staged: true,
        }
    }

    pub fn from_wire(said: wire::Community) -> Catalog {
        let mut maps: Vec<MapRef> = Vec::new();
        let mut play_of = |play: &wire::Play| -> Play {
            let line = play.map.line();
            let at = match maps.iter().position(|kept| kept.set == play.map.set && kept.line == line) {
                Some(at) => at,
                None => {
                    maps.push(MapRef { hash: String::new(), line, set: play.map.set, stars: play.map.stars });
                    maps.len() - 1
                }
            };
            Play { map: at, pp: play.pp, accuracy: play.accuracy, mods: play.mods.clone(), grade: play.grade.clone(), at: play.at.unwrap_or(0), full_combo: play.full_combo }
        };
        let person_of = |said: &wire::Person, play_of: &mut dyn FnMut(&wire::Play) -> Play| Person {
            id: said.id,
            name: said.name.clone(),
            country: said.country.clone(),
            pp: said.pp,
            rank: said.rank,
            accuracy: said.accuracy,
            plays: said.plays,
            hours: said.hours,
            score: said.score,
            hits_per_play: said.hits_per_play,
            level: said.level,
            ss: said.ss,
            s: said.s,
            joined: said.joined.unwrap_or(0),
            supporter: said.supporter,
            title: said.title.clone().filter(|code| !code.is_empty()),
            titles: said.titles.clone(),
            streak: said.streak,
            streak_best: said.streak_best,
            avatar: said.avatar.clone(),
            cover: said.cover.clone(),
            moved: std::array::from_fn(|at| said.moved.get(at).copied().unwrap_or(0)),
            gained: std::array::from_fn(|at| said.gained.get(at).copied().unwrap_or(0.0)),
            top: said.top.iter().map(|play| play_of(play)).collect(),
            you: said.you,
        };
        let people: Vec<Person> = said.people.iter().map(|person| person_of(person, &mut play_of)).collect();
        let index = |id: i64| people.iter().position(|person| person.id == id);
        let mut live: Vec<LivePlay> = said
            .live
            .iter()
            .filter_map(|play| {
                let who = index(play.who)?;
                let made = play_of(&play.play);
                Some(LivePlay { who, map: made.map, accuracy: made.accuracy, pp: made.pp, mods: made.mods, grade: made.grade, at: made.at, passed: play.passed })
            })
            .collect();
        live.sort_by_key(|play| play.at);
        let feed: Vec<Happening> = said
            .happened
            .iter()
            .filter_map(|happened| {
                let who = index(happened.who)?;
                let at = happened.at.unwrap_or(0);
                match happened.kind.as_str() {
                    "top_play" => {
                        let play = happened.play.as_ref().map(&mut play_of)?;
                        Some(Happening { who, kind: Kind::TopPlay { pp: play.pp, place: happened.place.unwrap_or(0), mods: play.mods }, map: Some(play.map), at })
                    }
                    "title" => Some(Happening { who, kind: Kind::Title(happened.title.clone()?), map: None, at }),
                    "climb" => Some(Happening {
                        who,
                        kind: Kind::Climb { board: Board::of(happened.board.as_deref().unwrap_or("pp"))?, from: happened.from.unwrap_or(0), to: happened.to.unwrap_or(0) },
                        map: None,
                        at,
                    }),
                    _ => None,
                }
            })
            .collect();
        let me = said.me.as_ref().map(|me| Me {
            person: person_of(&me.person, &mut play_of),
            recent: me.recent.iter().map(|play| play_of(&play.play)).collect(),
            duels: [me.duels.first().copied().unwrap_or(0), me.duels.get(1).copied().unwrap_or(0)],
            points: me.points,
        });
        let titles = said
            .titles
            .iter()
            .map(|title| Title {
                code: title.code.clone(),
                rarity: Rarity::of(&title.rarity),
                name: [title.name.clone(), if title.name_ru.is_empty() { title.name.clone() } else { title.name_ru.clone() }],
                about: [title.about.clone(), if title.about_ru.is_empty() { title.about.clone() } else { title.about_ru.clone() }],
            })
            .collect();
        Catalog {
            group: said.group,
            week: said.week,
            people,
            maps,
            feed,
            live,
            friends: Vec::new(),
            friends_state: Friends::Waiting,
            titles,
            me,
            staged: false,
        }
    }

    pub fn take_friends(&mut self, listed: Vec<wire::Friend>) {
        self.friends = listed
            .into_iter()
            .map(|friend| Friend { name: friend.name, country: friend.country, pp: friend.pp, rank: friend.rank, online: friend.online, seen: friend.seen, avatar: friend.avatar })
            .collect();
        self.friends_state = Friends::Ready;
    }

    pub fn card_of(&self) -> Option<wire::Card> {
        let you = self.you()?;
        let top_scores = you
            .top
            .iter()
            .map(|play| {
                let map = self.maps.get(play.map);
                wire::Score {
                    rank: play.grade.clone(),
                    title: map.map(|map| map.line.clone()).unwrap_or_default(),
                    pp: f64::from(play.pp),
                    accuracy: f64::from(play.accuracy),
                    mods: play.mods.join(","),
                    beatmapset_id: map.and_then(|map| map.set).unwrap_or(0) as f64,
                    hash: map.map(|map| map.hash.clone()).unwrap_or_default(),
                    ..wire::Score::default()
                }
            })
            .collect();
        let mut card = wire::Card {
            username: you.name.clone(),
            pp: f64::from(you.pp),
            global_rank: f64::from(you.rank),
            country: you.country.clone(),
            accuracy: f64::from(you.accuracy),
            play_count: f64::from(you.plays),
            play_seconds: f64::from(you.hours) * 3600.0,
            ranked_score: you.score as f64,
            total_hits: f64::from(you.hits_per_play) * f64::from(you.plays),
            total_score: you.score as f64,
            level: f64::from(you.level),
            grade_counts: wire::Grades { ss: f64::from(you.ss), s: f64::from(you.s), ..wire::Grades::default() },
            is_supporter: you.supporter,
            avatar_url: you.avatar.clone(),
            cover_url: you.cover.clone(),
            top_scores,
            title_code: you.title.clone().unwrap_or_default(),
            ..wire::Card::default()
        };
        if self.staged {
            card.handle = format!("@{}", you.name.to_lowercase());
            card.country_rank = 412.0;
            card.level_progress = 45.0;
            card.maximum_combo = 3_421.0;
            card.replays_watched = 234.0;
            card.total_score = you.score as f64 * 1.62;
            card.grade_counts = wire::Grades { a: 1_120.0, s: 1_388.0, sh: 402.0, ss: 214.0, ssh: 57.0 };
            card.total_maps = 3_181.0;
            card.join_date = "2016-03-11T00:00:00+00:00".to_owned();
            card.last_visit = String::new();
            card.is_online = true;
            card.rank_history = (0..90).map(|day| {
                let t = f64::from(day) / 89.0;
                let wobble = (f64::from(day) * 0.61).sin() * 140.0 + (f64::from(day) * 0.23).cos() * 90.0;
                (15_400.0 - (15_400.0 - f64::from(you.rank)) * t.powf(1.3) + wobble * (1.0 - t)).round()
            }).collect();
            if let Some(last) = card.rank_history.last_mut() {
                *last = f64::from(you.rank);
            }
        }
        Some(card)
    }

    pub fn pictures(&self) -> Vec<(String, u32)> {
        let mut wanted: Vec<(String, u32)> = Vec::new();
        let mut want = |url: &str, side: u32| {
            if !url.is_empty() && !wanted.iter().any(|(kept, _)| kept == url) {
                wanted.push((url.to_owned(), side));
            }
        };
        if let Some(me) = &self.me {
            want(&me.person.avatar, 256);
            want(&me.person.cover, 1400);
        }
        for person in &self.people {
            want(&person.avatar, 128);
        }
        for friend in &self.friends {
            want(&friend.avatar, 128);
        }
        for map in &self.maps {
            if let Some(card) = map.card() {
                want(&card, 400);
            }
        }
        wanted
    }
}

pub mod wire {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Map {
        #[serde(default)]
        pub beatmap: Option<u64>,
        #[serde(default)]
        pub set: Option<u64>,
        #[serde(default)]
        pub artist: String,
        #[serde(default)]
        pub title: String,
        #[serde(default)]
        pub version: String,
        #[serde(default)]
        pub creator: String,
        #[serde(default)]
        pub stars: Option<f32>,
    }

    impl Map {
        pub fn line(&self) -> String {
            match (self.artist.is_empty(), self.version.is_empty()) {
                (false, false) => format!("{} — {} [{}]", self.artist, self.title, self.version),
                (false, true) => format!("{} — {}", self.artist, self.title),
                (true, false) => format!("{} [{}]", self.title, self.version),
                (true, true) => self.title.clone(),
            }
        }
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Play {
        #[serde(default)]
        pub map: Map,
        #[serde(default)]
        pub pp: f32,
        #[serde(default)]
        pub accuracy: f32,
        #[serde(default)]
        pub mods: Vec<String>,
        #[serde(default)]
        pub grade: String,
        #[serde(default)]
        pub combo: Option<u32>,
        #[serde(default)]
        pub max_combo: Option<u32>,
        #[serde(default)]
        pub full_combo: bool,
        #[serde(default)]
        pub at: Option<i64>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Person {
        #[serde(default)]
        pub id: i64,
        #[serde(default)]
        pub osu_id: Option<i64>,
        #[serde(default)]
        pub name: String,
        #[serde(default)]
        pub country: String,
        #[serde(default)]
        pub pp: u32,
        #[serde(default)]
        pub rank: u32,
        #[serde(default)]
        pub accuracy: f32,
        #[serde(default)]
        pub plays: u32,
        #[serde(default)]
        pub hours: u32,
        #[serde(default)]
        pub score: u64,
        #[serde(default)]
        pub hits_per_play: f32,
        #[serde(default)]
        pub level: u32,
        #[serde(default)]
        pub ss: u32,
        #[serde(default)]
        pub s: u32,
        #[serde(default)]
        pub joined: Option<i64>,
        #[serde(default)]
        pub supporter: bool,
        #[serde(default)]
        pub title: Option<String>,
        #[serde(default)]
        pub titles: Vec<String>,
        #[serde(default)]
        pub streak: u32,
        #[serde(default)]
        pub streak_best: u32,
        #[serde(default)]
        pub avatar: String,
        #[serde(default)]
        pub cover: String,
        #[serde(default)]
        pub moved: Vec<i32>,
        #[serde(default)]
        pub gained: Vec<f64>,
        #[serde(default)]
        pub top: Vec<Play>,
        #[serde(default)]
        pub you: bool,
    }

    fn passed() -> bool {
        true
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Live {
        pub who: i64,
        #[serde(default = "passed")]
        pub passed: bool,
        #[serde(flatten)]
        pub play: Play,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Happened {
        pub who: i64,
        pub kind: String,
        #[serde(default)]
        pub at: Option<i64>,
        #[serde(default)]
        pub place: Option<u32>,
        #[serde(default)]
        pub play: Option<Play>,
        #[serde(default)]
        pub title: Option<String>,
        #[serde(default)]
        pub board: Option<String>,
        #[serde(default)]
        pub from: Option<u32>,
        #[serde(default)]
        pub to: Option<u32>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Title {
        pub code: String,
        #[serde(default)]
        pub rarity: String,
        #[serde(default)]
        pub name: String,
        #[serde(default)]
        pub name_ru: String,
        #[serde(default)]
        pub about: String,
        #[serde(default)]
        pub about_ru: String,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Recent {
        #[serde(default = "passed")]
        pub passed: bool,
        #[serde(flatten)]
        pub play: Play,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Me {
        #[serde(flatten)]
        pub person: Person,
        #[serde(default)]
        pub recent: Vec<Recent>,
        #[serde(default)]
        pub duels: Vec<u32>,
        #[serde(default)]
        pub points: u32,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Community {
        #[serde(default)]
        pub chat: Option<i64>,
        #[serde(default)]
        pub group: String,
        #[serde(default)]
        pub week: u32,
        #[serde(default)]
        pub people: Vec<Person>,
        #[serde(default)]
        pub live: Vec<Live>,
        #[serde(default)]
        pub happened: Vec<Happened>,
        #[serde(default)]
        pub titles: Vec<Title>,
        #[serde(default)]
        pub me: Option<Me>,
        #[serde(default)]
        pub at: Option<i64>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Friend {
        #[serde(default)]
        pub osu_id: Option<i64>,
        #[serde(default)]
        pub name: String,
        #[serde(default)]
        pub country: String,
        #[serde(default)]
        pub pp: u32,
        #[serde(default)]
        pub rank: u32,
        #[serde(default)]
        pub online: bool,
        #[serde(default)]
        pub seen: Option<i64>,
        #[serde(default)]
        pub avatar: String,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Friends {
        #[serde(default)]
        pub friends: Vec<Friend>,
    }

    fn number<'de, D: serde::Deserializer<'de>>(said: D) -> Result<f64, D::Error> {
        let value = Option::<serde_json::Value>::deserialize(said)?;
        Ok(match value {
            Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(0.0),
            Some(serde_json::Value::String(s)) => s.trim().parse().unwrap_or(0.0),
            Some(serde_json::Value::Bool(b)) => f64::from(u8::from(b)),
            _ => 0.0,
        })
    }

    fn words<'de, D: serde::Deserializer<'de>>(said: D) -> Result<String, D::Error> {
        let value = Option::<serde_json::Value>::deserialize(said)?;
        Ok(match value {
            Some(serde_json::Value::String(s)) => s,
            Some(serde_json::Value::Number(n)) => n.to_string(),
            _ => String::new(),
        })
    }

    fn numbers<'de, D: serde::Deserializer<'de>>(said: D) -> Result<Vec<f64>, D::Error> {
        let value = Option::<Vec<serde_json::Value>>::deserialize(said)?;
        Ok(value.unwrap_or_default().into_iter().filter_map(|item| item.as_f64()).filter(|n| *n > 0.0).collect())
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Grades {
        #[serde(default, deserialize_with = "number")]
        pub a: f64,
        #[serde(default, deserialize_with = "number")]
        pub s: f64,
        #[serde(default, deserialize_with = "number")]
        pub sh: f64,
        #[serde(default, deserialize_with = "number")]
        pub ss: f64,
        #[serde(default, deserialize_with = "number")]
        pub ssh: f64,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Score {
        #[serde(default, deserialize_with = "words")]
        pub rank: String,
        #[serde(default, deserialize_with = "words")]
        pub artist: String,
        #[serde(default, deserialize_with = "words")]
        pub title: String,
        #[serde(default, deserialize_with = "words")]
        pub version: String,
        #[serde(default, deserialize_with = "number")]
        pub pp: f64,
        #[serde(default, deserialize_with = "number")]
        pub accuracy: f64,
        #[serde(default, deserialize_with = "number")]
        pub max_combo: f64,
        #[serde(default, deserialize_with = "words")]
        pub mods: String,
        #[serde(default, deserialize_with = "number")]
        pub beatmap_id: f64,
        #[serde(default, deserialize_with = "number")]
        pub beatmapset_id: f64,
        #[serde(default, deserialize_with = "words")]
        pub creator: String,
        #[serde(skip)]
        pub hash: String,
    }

    impl Score {
        pub fn cover(&self) -> Option<String> {
            (self.beatmapset_id > 0.0).then(|| format!("https://assets.ppy.sh/beatmaps/{}/covers/cover.jpg", self.beatmapset_id as u64))
        }

        pub fn page(&self) -> Option<String> {
            (self.beatmap_id > 0.0).then(|| format!("https://osu.ppy.sh/b/{}", self.beatmap_id as u64))
        }
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Card {
        #[serde(default, deserialize_with = "words")]
        pub username: String,
        #[serde(default, deserialize_with = "words")]
        pub handle: String,
        #[serde(default, deserialize_with = "number")]
        pub osu_id: f64,
        #[serde(default, deserialize_with = "number")]
        pub pp: f64,
        #[serde(default, deserialize_with = "number")]
        pub global_rank: f64,
        #[serde(default, deserialize_with = "words")]
        pub country: String,
        #[serde(default, deserialize_with = "words")]
        pub country_name: String,
        #[serde(default, deserialize_with = "number")]
        pub country_rank: f64,
        #[serde(default, deserialize_with = "number")]
        pub accuracy: f64,
        #[serde(default, deserialize_with = "number")]
        pub play_count: f64,
        #[serde(default, deserialize_with = "number")]
        pub play_seconds: f64,
        #[serde(default, deserialize_with = "number")]
        pub ranked_score: f64,
        #[serde(default, deserialize_with = "number")]
        pub total_hits: f64,
        #[serde(default, deserialize_with = "number")]
        pub total_score: f64,
        #[serde(default, deserialize_with = "number")]
        pub level: f64,
        #[serde(default, deserialize_with = "number")]
        pub level_progress: f64,
        #[serde(default, deserialize_with = "number")]
        pub maximum_combo: f64,
        #[serde(default, deserialize_with = "number")]
        pub replays_watched: f64,
        #[serde(default)]
        pub grade_counts: Grades,
        #[serde(default, deserialize_with = "number")]
        pub total_maps: f64,
        #[serde(default)]
        pub is_online: bool,
        #[serde(default)]
        pub is_supporter: bool,
        #[serde(default, deserialize_with = "words")]
        pub join_date: String,
        #[serde(default, deserialize_with = "words")]
        pub last_visit: String,
        #[serde(default, deserialize_with = "words")]
        pub avatar_url: String,
        #[serde(default, deserialize_with = "words")]
        pub cover_url: String,
        #[serde(default, deserialize_with = "numbers")]
        pub rank_history: Vec<f64>,
        #[serde(default)]
        pub top_scores: Vec<Score>,
        #[serde(default, deserialize_with = "words")]
        pub title_code: String,
    }

    pub fn card_path() -> std::path::PathBuf {
        crate::sources::own_root().join("card.json")
    }

    pub fn load_card() -> Option<Card> {
        std::fs::read(card_path()).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }

    pub fn save_card(said: &Card) {
        if let Ok(bytes) = serde_json::to_vec(said) {
            let _ = std::fs::create_dir_all(crate::sources::own_root());
            let _ = std::fs::write(card_path(), bytes);
        }
    }

    pub fn cache_path() -> std::path::PathBuf {
        crate::sources::own_root().join("community.json")
    }

    pub fn load() -> Option<Community> {
        std::fs::read(cache_path()).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }

    pub fn save(said: &Community) {
        if let Ok(bytes) = serde_json::to_vec(said) {
            let _ = std::fs::create_dir_all(crate::sources::own_root());
            let _ = std::fs::write(cache_path(), bytes);
        }
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
                assert!(catalog.title_of(code).is_some(), "{} holds an unknown title {code}", person.name);
            }
            if let Some(worn) = &person.title {
                assert!(person.titles.contains(worn), "{} wears a title they do not hold", person.name);
            }
        }
        for happening in &catalog.feed {
            if let Kind::Title(code) = &happening.kind {
                assert!(catalog.people[happening.who].titles.contains(code));
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
        assert_eq!(catalog.you().map(|you| you.name.as_str()), Some("someone"));
    }

    const ANSWER: &str = r#"{
        "chat": -1001, "group": "Osu Squad", "week": 39, "at": 1790157600,
        "people": [
            {"id": 8, "name": "kotofey", "country": "RU", "pp": 12480, "rank": 3402, "accuracy": 98.61, "titles": ["hdhr_fc7"], "title": "hdhr_fc7",
             "avatar": "https://a.ppy.sh/80", "moved": [0, 1, 0, 0, 0, 0], "gained": [12.5, 0, 0, 0, 0, 0],
             "top": [{"map": {"beatmap": 3, "set": 4, "artist": "xi", "title": "FREEDOM DiVE", "version": "FOUR DIMENSIONS", "stars": 7.2}, "pp": 611.2, "accuracy": 99.21, "mods": ["HD", "HR"], "grade": "SS", "at": 1790000000}]},
            {"id": 7, "name": "NaumRedlo", "country": "RU", "pp": 9870, "rank": 11402, "you": true, "titles": ["wysi"], "title": "wysi"}
        ],
        "live": [
            {"who": 8, "passed": true, "map": {"set": 9, "artist": "Phoneboy", "title": "Nevermind", "version": "Insane"}, "pp": 530.0, "accuracy": 99.4, "mods": ["HD", "HR"], "grade": "S", "at": 1790157500},
            {"who": 7, "map": {"set": 4, "artist": "xi", "title": "FREEDOM DiVE", "version": "FOUR DIMENSIONS"}, "pp": 312.0, "accuracy": 96.7, "mods": [], "grade": "A", "at": 1790157000},
            {"who": 99, "map": {}, "pp": 1.0, "accuracy": 1.0, "grade": "D", "at": 1}
        ],
        "happened": [
            {"who": 7, "kind": "title", "title": "wysi", "at": 1790150000},
            {"who": 7, "kind": "climb", "board": "pp", "from": 3, "to": 2, "at": 1790100000},
            {"who": 8, "kind": "top_play", "place": 1, "at": 1790000000, "play": {"map": {"set": 4, "artist": "xi", "title": "FREEDOM DiVE", "version": "FOUR DIMENSIONS"}, "pp": 611.2, "accuracy": 99.21, "mods": ["HD", "HR"], "grade": "SS"}},
            {"who": 7, "kind": "something new", "at": 1}
        ],
        "titles": [{"code": "wysi", "rarity": "uncommon", "name": "WYSI", "name_ru": "WYSI", "about": "727", "about_ru": "727"},
                   {"code": "hdhr_fc7", "rarity": "legendary", "name": "Double Sentence", "name_ru": "Двойной приговор", "about": "a", "about_ru": "б"}],
        "me": {"id": 7, "name": "NaumRedlo", "pp": 9870, "you": true, "cover": "https://assets.ppy.sh/user-profile-covers/7/c.jpg",
               "recent": [{"passed": false, "map": {"set": 9, "artist": "Phoneboy", "title": "Nevermind", "version": "Insane"}, "pp": 0, "accuracy": 80.0, "grade": "F", "at": 1790157400}],
               "duels": [3, 1], "points": 120}
    }"#;

    #[test]
    fn the_bots_answer_becomes_a_catalogue() {
        let said: wire::Community = serde_json::from_str(ANSWER).expect("the answer reads");
        let catalog = Catalog::from_wire(said);
        assert!(!catalog.staged);
        assert_eq!(catalog.group, "Osu Squad");
        assert_eq!(catalog.people.len(), 2);
        assert_eq!(catalog.you().map(|you| you.name.as_str()), Some("NaumRedlo"));
        assert_eq!(catalog.live.len(), 2, "a play by someone unknown is left out");
        assert!(catalog.live[0].at < catalog.live[1].at, "the pool runs oldest first, the newest last");
        assert_eq!(catalog.people[catalog.live[1].who].name, "kotofey");
        assert_eq!(catalog.maps.iter().filter(|map| map.line == "xi — FREEDOM DiVE [FOUR DIMENSIONS]").count(), 1, "one map, one entry");
        assert_eq!(catalog.feed.len(), 3);
        assert!(matches!(catalog.feed[1].kind, Kind::Climb { board: Board::Pp, from: 3, to: 2 }));
        assert_eq!(catalog.title_of("hdhr_fc7").map(|title| title.name(crate::lang::Lang::Ru)), Some("Двойной приговор"));
        let me = catalog.me.as_ref().expect("the person");
        assert_eq!(me.duels, [3, 1]);
        assert_eq!(me.recent.len(), 1);
        let wanted: Vec<String> = catalog.pictures().into_iter().map(|(url, _)| url).collect();
        assert!(wanted.contains(&"https://a.ppy.sh/80".to_owned()));
        assert!(wanted.contains(&"https://assets.ppy.sh/beatmaps/4/covers/card.jpg".to_owned()));
    }

    #[test]
    fn friends_arrive_into_the_catalogue() {
        let said: wire::Friends = serde_json::from_str(r#"{"friends": [{"name": "Riv3r", "country": "DE", "pp": 10440, "rank": 7310, "online": true}]}"#).unwrap();
        let mut catalog = Catalog::from_wire(wire::Community::default());
        assert_eq!(catalog.friends_state, Friends::Waiting);
        catalog.take_friends(said.friends);
        assert_eq!(catalog.friends_state, Friends::Ready);
        assert_eq!(catalog.friends[0].minutes_away(0), 0);
    }
}
