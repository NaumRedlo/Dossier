use iced::widget::{button, canvas, column, container, image, mouse_area, row, stack, text, text_input, Space};
use iced::{mouse, Background, Border, Color, ContentFit, Element, Length, Padding, Point, Rectangle, Renderer, Size, Theme, Vector};

use super::{Main, Message, Overlay};
use crate::carousel::{self, Filter, Group, Laid, Row, Sort};
use crate::library::{Entry, Grade};
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    Players,
    Mine,
    Witness,
    Clean,
}

pub const TOP: f32 = 76.0;
pub const SEARCH_H: f32 = 40.0;
pub const SEARCH_GAP: f32 = 12.0;
pub const BAR_H: f32 = 28.0;
pub const SPRING: f32 = 100.0;
pub const DAMPING: f32 = 20.0;
pub const WHEEL_STEP: f32 = 72.0;
pub const SPEED_MOST: f32 = 5000.0;
pub const VERSION_ROOM: f32 = 150.0;
pub const FLING: f32 = 0.22;
pub const BACKING_FOOT: f32 = 10.0;
const BACKING_BOTTOM: f32 = 0.0;
pub const SPRING_STEP: f32 = 1.0 / 240.0;
pub const SQUEEZED: f32 = 0.72;
pub const TOP_FADE: f32 = 64.0;
pub const TOP_FADE_ONSET: f32 = 12.0;
pub const STATS_GAP: f32 = 10.0;
pub const HIT_SIZE: f32 = 17.0;
pub const PANEL_WIDE: f32 = 380.0;
const PP_FC: Color = Color::from_rgb(0.235, 0.549, 0.235);
const PP_SS: Color = Color::from_rgb(0.627, 0.529, 0.039);
pub const ACTIONS_SCALE: f32 = 1.25;
pub const TOP_SLICES: usize = 64;
pub const SPRING_LAG: f32 = 0.1;
pub const HEAD_FLOOR: f32 = 0.3;
use crate::carousel::{HEAD_CLOSED, HEAD_OPEN};
pub const REPLAY_FLOOR: f32 = 0.8;
pub const BANNER: (u32, u32) = (960, 160);
pub const BANNERS_KEPT: usize = 64;
pub const BANNERS_AT_ONCE: usize = 12;
pub const BANNERS_EVERY: std::time::Duration = std::time::Duration::from_millis(300);

pub fn banner_of(path: &std::path::Path) -> Option<iced::widget::image::Handle> {
    let picture = ::image::ImageReader::open(path).ok()?.with_guessed_format().ok()?.decode().ok()?;
    let filled = picture.resize_to_fill(BANNER.0, BANNER.1, ::image::imageops::FilterType::CatmullRom).to_rgba8();
    Some(iced::widget::image::Handle::from_rgba(BANNER.0, BANNER.1, filled.into_raw()))
}
pub const UNFOLD: f32 = 0.18;
const FOLD: f32 = 0.26;
pub const PANEL_OUT: f32 = 16.0;
pub const SKELETON_ROWS: usize = 9;
pub const TRACK_H: f32 = 10.0;
pub const TRACK_KNOB: f32 = 7.0;
pub const SCENE_GAP: f32 = 16.0;
pub const HEAD_H: f32 = 64.0;
pub const STATS_H: f32 = 84.0;
pub const ACTIONS_H: f32 = 32.0;
pub const GROW_MOST: f32 = 1.5;
pub const TOLD_FLOOR: f32 = 0.15;
pub const GROW_WIDTH: f32 = 720.0;
pub const ARC_SPAN: f32 = 72.0;
pub const SIDE: f32 = 32.0;
pub const RANK_SMALL: f32 = 44.0;
pub const RANK_LARGE: f32 = 64.0;
pub const TIDE_SLOW: f32 = 4.8;
pub const MARKS_ONE_BY_ONE: usize = 5;
pub const MARK: f32 = 16.0;
pub const TIDE_LATE: f32 = 6.2;

pub fn grade_colour(grade: Grade) -> Color {
    match grade {
        Grade::Ss => theme::GRADE_SS,
        Grade::S => theme::GRADE_S,
        Grade::A => theme::GRADE_A,
        Grade::B => theme::GRADE_B,
        Grade::C => theme::GRADE_C,
        Grade::D | Grade::F => theme::GRADE_D,
    }
}

fn pulses(_grade: Grade) -> bool {
    true
}

const CARD: Color = theme::GLASS;
const CARD_LIT: Color = theme::GLASS_LIT;
const PICKED: Color = theme::GLASS_PICKED;
const BACKING: Color = Color::from_rgba(0.024, 0.008, 0.016, 0.72);
const SCRIM: Color = theme::GROUND;
const PANEL: Color = Color::from_rgb(0.055, 0.022, 0.031);

fn paint(colour: Color) -> Color {
    Color { a: colour.a * ui::fade(), ..colour }
}

fn tinted(colour: Color, a: f32) -> Color {
    Color { a, ..colour }
}

fn gently(k: f32) -> f32 {
    let k = k.clamp(0.0, 1.0);
    k * k * k * (k * (6.0 * k - 15.0) + 10.0)
}

fn eased(k: f32) -> f32 {
    let k = k.clamp(0.0, 1.0);
    1.0 - (1.0 - k).powi(4)
}

#[derive(Debug, Clone, Copy)]
pub struct SceneLayout {
    pub wide: f32,
    pub tall: f32,
    pub narrow: bool,
    pub grow: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LaidKey {
    filter: Filter,
    sort: Sort,
    group: Group,
    chosen: Option<usize>,
    entries: (usize, usize),
    rated: usize,
    hour: i64,
    me: String,
}

#[derive(Clone)]
pub struct Fold {
    pub head: (String, usize),
    pub rows: Vec<Row>,
    pub at: std::time::Instant,
    pub from: f32,
}

impl Fold {
    pub fn left(&self, now: std::time::Instant) -> f32 {
        self.from * (1.0 - gently(now.saturating_duration_since(self.at).as_secs_f32() / FOLD))
    }
}

pub struct Display {
    pub rows: Vec<Row>,
    pub factors: Vec<f32>,
    pub ghost: Vec<bool>,
}

#[derive(Clone)]
pub struct Hold {
    pub rows: Vec<Row>,
    pub factors: Vec<f32>,
    pub scroll: f32,
    pub head: Option<(String, usize)>,
    pub nested: Vec<Row>,
}

pub fn top_faded<'a>(at: f32, full: f32, reach: f32, place: impl Fn(f32) -> Element<'a, Message>) -> Element<'a, Message> {
    let mut cuts = vec![0.0, full];
    for slice in 0..=TOP_SLICES {
        let local = (slice as f32 * TOP_FADE / TOP_SLICES as f32).round() - at;
        if local > 0.0 && local < full {
            cuts.push(local);
        }
    }
    cuts.sort_by(f32::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    let mut strips = column![].width(Length::Fill);
    for pair in cuts.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let middle = at + (from + to) / 2.0;
        let alpha = 1.0 - reach * (1.0 - eased((middle / TOP_FADE).clamp(0.0, 1.0)));
        let strip: Element<'a, Message> = if alpha < 0.01 {
            Space::new().width(Length::Fill).height(to - from).into()
        } else {
            ui::band(container(place(alpha)).width(Length::Fill).height(full), from, to)
        };
        strips = strips.push(strip);
    }
    strips.into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Head(usize),
    Replay(usize),
}

pub fn focus_of(row: &Row) -> Option<Focus> {
    match row {
        Row::Head { first, .. } => Some(Focus::Head(*first)),
        Row::Replay { at, .. } => Some(Focus::Replay(*at)),
    }
}

pub(super) fn shown_tops(rows: &[Row], factors: &[f32]) -> Vec<f32> {
    let mut top = 0.0;
    (0..rows.len())
        .map(|i| {
            let here = top;
            top += rows[i].height() * factors[i] + carousel::GAP * gap_factor(rows, factors, i + 1);
            here
        })
        .collect()
}

pub(super) fn shown_top(rows: &[Row], factors: &[f32], at: usize) -> f32 {
    shown_tops(rows, factors)[at]
}

fn gap_factor(rows: &[Row], factors: &[f32], at: usize) -> f32 {
    match rows.get(at) {
        Some(Row::Head { .. }) | None => 1.0,
        Some(_) => factors.get(at).copied().unwrap_or(1.0),
    }
}

pub fn shown_offset(rows: &[Row], factors: &[f32], height: f32) -> f32 {
    let Some(at) = rows.iter().position(|row| matches!(row, Row::Replay { chosen: true, .. })) else {
        return 0.0;
    };
    (height / 2.0 - shown_top(rows, factors, at) - rows[at].height() * factors[at] / 2.0).round().min(0.0)
}

pub fn row_top(rows: &[Row], factors: &[f32], focus: Option<usize>) -> Option<f32> {
    let focus = focus?;
    let at = rows
        .iter()
        .position(|row| matches!(row, Row::Replay { at, .. } if *at == focus))
        .or_else(|| rows.iter().position(|row| matches!(row, Row::Head { first, .. } if *first == focus)))?;
    Some(shown_top(rows, factors, at))
}

impl Main {
    fn cover_frame(&self, hash: &str) -> Option<crate::film::Frame> {
        if let Some(frame) = self.cover_frames.borrow().get(hash) {
            return Some(frame.clone());
        }
        let frame = crate::film::Frame::of_handle(crate::film::reel(), self.banners.get(hash).or_else(|| self.thumbs.get(hash))?)?;
        self.cover_frames.borrow_mut().insert(hash.to_owned(), frame.clone());
        Some(frame)
    }

    pub(super) fn carousel_width(&self) -> f32 {
        (self.width * 0.36).clamp(400.0, 560.0)
    }

    pub(super) fn own_name(&self) -> String {
        self.community
            .as_ref()
            .and_then(|catalog| catalog.people.iter().find(|person| person.you))
            .map(|you| you.name.clone())
            .unwrap_or_else(|| self.settings.linked_as.trim_start_matches('@').to_owned())
    }

    pub(super) fn filter(&self) -> Filter {
        Filter { query: self.search.clone(), ..self.filter.clone() }
    }

    pub(crate) fn laid(&self) -> std::rc::Rc<Laid> {
        let key = LaidKey {
            filter: self.filter(),
            sort: self.sort,
            group: self.group,
            chosen: self.chosen,
            entries: self.library.as_ref().map_or((0, 0), |library| (library.entries.as_ptr() as usize, library.entries.len())),
            rated: self.ratings.known.len(),
            hour: self.now_unix / 3600,
            me: self.own_name(),
        };
        if let Some((known, laid)) = self.laid_cache.borrow().as_ref() {
            if *known == key {
                return laid.clone();
            }
        }
        let laid = std::rc::Rc::new(self.lay_fresh());
        *self.laid_cache.borrow_mut() = Some((key, laid.clone()));
        laid
    }

    fn lay_fresh(&self) -> Laid {
        let me = self.own_name();
        let root = crate::sources::witnessed_root();
        let witnessed = |entry: &Entry| entry.path.starts_with(&root);
        let day = |at: i64| self.words.day(at, self.now_unix);
        let context = carousel::Context { ratings: &self.ratings.known, me: &me, witnessed: &witnessed, day: &day };
        carousel::lay(self.entries(), &self.filter(), self.sort, self.group, self.chosen, &context)
    }

    pub(super) fn jump_group(&self, by: i32) -> Option<usize> {
        let laid = self.laid();
        let me = self.own_name();
        let root = crate::sources::witnessed_root();
        let witnessed = |entry: &Entry| entry.path.starts_with(&root);
        let day = |at: i64| self.words.day(at, self.now_unix);
        let context = carousel::Context { ratings: &self.ratings.known, me: &me, witnessed: &witnessed, day: &day };
        carousel::jump(&laid, self.entries(), self.group, &context, by)
    }

    pub(super) fn list_top(&self) -> f32 {
        TOP + (SEARCH_H + SEARCH_GAP) * self.search_amount()
    }

    pub(super) fn list_height(&self) -> f32 {
        (self.height - self.list_top()).max(120.0)
    }

    pub(super) fn display(&self, laid: &Laid) -> Display {
        let unfold = self.unfolded_at(self.now);
        let mut shown = Display { rows: Vec::new(), factors: Vec::new(), ghost: Vec::new() };
        for row in &laid.rows {
            shown.rows.push(row.clone());
            shown.factors.push(match row {
                Row::Head { open: true, .. } => (HEAD_CLOSED + (HEAD_OPEN - HEAD_CLOSED) * unfold) / HEAD_OPEN,
                _ if row.depth() > 0.0 => unfold,
                _ => 1.0,
            });
            shown.ghost.push(false);
            let folding = match row {
                Row::Head { title, first, open: false, .. } => self.folding.iter().find(|fold| &fold.head.0 == title && fold.head.1 == *first).map(|fold| (fold, fold.left(self.now))).filter(|(_, k)| *k > 0.001),
                _ => None,
            };
            if let Some((fold, k)) = folding {
                if let Some(last) = shown.factors.last_mut() {
                    *last = (HEAD_CLOSED + (HEAD_OPEN - HEAD_CLOSED) * k) / HEAD_CLOSED;
                }
                for ghost in &fold.rows {
                    shown.rows.push(match ghost {
                        Row::Replay { at, .. } => Row::Replay { at: *at, chosen: false },
                        other => other.clone(),
                    });
                    shown.factors.push(k);
                    shown.ghost.push(true);
                }
            }
        }
        shown
    }

    pub(super) fn carousel_target(&self) -> f32 {
        let shown = self.display(&self.laid());
        let (rows, factors) = (shown.rows, shown.factors);
        let height = self.list_height();
        match self.free {
            Some(free) => {
                let content = rows.last().map_or(0.0, |last| shown_top(&rows, &factors, rows.len() - 1) + last.height());
                free.clamp((height - content).min(0.0), 0.0)
            }
            None => shown_offset(&rows, &factors, height),
        }
    }

    pub(super) fn near_banners(&self) -> Vec<(String, std::path::PathBuf)> {
        let laid = self.laid();
        let height = self.list_height();
        let shift = self.glide_now(self.carousel_target());
        let entries = self.entries();
        let mut out: Vec<(f32, String, std::path::PathBuf)> = Vec::new();
        let tops = shown_tops(&laid.rows, &vec![1.0; laid.rows.len()]);
        for (index, row) in laid.rows.iter().enumerate() {
            let top = shift + tops[index];
            if top > 2.0 * height {
                break;
            }
            let (first, hash) = match row {
                Row::Head { first, cover: Some(hash), .. } => (first, hash),
                Row::Replay { at, .. } if self.group != Group::Maps => (at, &entries[*at].map_hash),
                _ => continue,
            };
            if top + row.height() < -height {
                continue;
            }
            if let Some(path) = entries.get(*first).and_then(|entry| entry.map.as_ref()).and_then(|map| map.background.clone()) {
                if !out.iter().any(|(_, known, _)| known == hash) {
                    out.push(((top + row.height() / 2.0 - height / 2.0).abs(), hash.clone(), path));
                }
            }
        }
        out.sort_by(|a, b| a.0.total_cmp(&b.0));
        out.into_iter().map(|(_, hash, path)| (hash, path)).collect()
    }

    pub(super) fn focused(&self) -> Option<Focus> {
        let laid = self.laid();
        self.focus
            .filter(|focus| laid.rows.iter().any(|row| focus_of(row) == Some(*focus)))
            .or_else(|| self.chosen.map(Focus::Replay))
    }

    pub(super) fn step_focus(&mut self, by: i32) {
        let laid = self.laid();
        let marks: Vec<Focus> = laid.rows.iter().filter_map(focus_of).collect();
        if marks.is_empty() {
            return;
        }
        let here = self.focused().and_then(|focus| marks.iter().position(|mark| *mark == focus));
        let next = match here {
            Some(at) => (at as i64 + i64::from(by)).clamp(0, marks.len() as i64 - 1) as usize,
            None => 0,
        };
        self.focus = Some(marks[next]);
        let shown = self.display(&laid);
        let Some(at) = shown.rows.iter().enumerate().position(|(i, row)| !shown.ghost[i] && focus_of(row) == Some(marks[next])) else {
            return;
        };
        let top = shown_top(&shown.rows, &shown.factors, at);
        let tall = shown.rows[at].height() * shown.factors[at];
        self.free = Some((self.list_height() / 2.0 - top - tall / 2.0).round());
        self.free = Some(self.carousel_target());
        self.banners_waiting = true;
    }

    pub(super) fn drag_carousel(&mut self, by: f32) {
        let base = self.free.unwrap_or_else(|| self.carousel_target());
        self.free = Some(base + by);
        let held = self.carousel_target();
        self.free = Some(held);
        self.scroll = Some(held);
        self.scroll_speed = 0.0;
        self.banners_waiting = true;
    }

    pub(super) fn drop_carousel(&mut self, speed: f32) {
        let base = self.free.unwrap_or_else(|| self.carousel_target());
        self.free = Some(base + speed.clamp(-SPEED_MOST, SPEED_MOST) * FLING);
        self.free = Some(self.carousel_target());
        self.banners_waiting = true;
    }

    pub(super) fn glide_now(&self, target: f32) -> f32 {
        self.scroll.unwrap_or(target)
    }

    pub(super) fn gliding(&self) -> bool {
        self.scroll_speed.abs() > 1.0 || self.scroll.is_some_and(|scroll| (scroll - self.carousel_target()).abs() > 0.5) || self.unfolding()
    }

    pub(super) fn unfolded_at(&self, now: std::time::Instant) -> f32 {
        self.unfold_at.map_or(1.0, |at| self.unfold_from + (1.0 - self.unfold_from) * gently(now.saturating_duration_since(at).as_secs_f32() / UNFOLD))
    }

    pub(super) fn unfolding(&self) -> bool {
        self.unfold_at.is_some_and(|at| self.now.saturating_duration_since(at).as_secs_f32() < UNFOLD)
            || self.folding.iter().any(|fold| fold.left(self.now) > 0.001)
    }

    pub(super) fn spring_carousel(&mut self, now: std::time::Instant) {
        let last = self.spring_at.replace(now);
        if self.gallery || self.overlay != Overlay::None || self.entries().is_empty() {
            return;
        }
        let target = self.carousel_target();
        let Some(mut scroll) = self.scroll else {
            self.scroll = Some(target);
            return;
        };
        let gap = last.map_or(0.0, |last| now.saturating_duration_since(last).as_secs_f32());
        let mut left = if gap > SPRING_LAG { 1.0 / 60.0 } else { gap };
        while left > 0.0 {
            let step = left.min(SPRING_STEP);
            let pull = SPRING * (target - scroll) - DAMPING * self.scroll_speed;
            self.scroll_speed = (self.scroll_speed + pull * step).clamp(-SPEED_MOST, SPEED_MOST);
            scroll += self.scroll_speed * step;
            left -= step;
        }
        if (target - scroll).abs() < 0.5 && self.scroll_speed.abs() < 4.0 {
            self.scroll = Some(target);
            self.scroll_speed = 0.0;
        } else {
            self.scroll = Some(scroll);
        }
    }

    pub(super) fn glow_seconds(&self) -> f32 {
        if self.gallery {
            return 0.0;
        }
        self.now.saturating_duration_since(self.glow_epoch).as_secs_f32()
    }

    pub(super) fn select_body(&self, k: f32, _s: f32) -> Element<'_, Message> {
        let wide = self.carousel_width();
        let laid = self.laid();
        let list = if self.star_drag { self.skeleton() } else { self.carousel(&laid) };
        let list = container(list).padding(Padding { top: self.list_top(), right: 0.0, bottom: 0.0, left: 0.0 }).width(wide).height(Length::Fill);
        let search_k = self.search_amount();
        let left: Element<'_, Message> = if search_k > 0.001 || self.search_shown() {
            let search = ui::fading(ui::fade() * search_k, || self.filters(&laid));
            let search = ui::grown(search, Point::new(0.5, 0.0), 0.0, 0.97 + 0.03 * search_k);
            let search = container(search).padding(Padding { top: TOP, right: 0.0, bottom: 0.0, left: 0.0 }).width(wide);
            stack![list, search].width(wide).height(Length::Fill).into()
        } else {
            list.into()
        };
        let right = container(self.scene_panel())
            .padding(Padding { top: TOP, right: 40.0, bottom: 28.0, left: 40.0 })
            .width(Length::Fill)
            .height(Length::Fill);
        let shade = canvas(CarouselShade { width: wide, alpha: ui::fade() }).width(Length::Fill).height(Length::Fill);
        let body = row![left, right].width(Length::Fill).height(Length::Fill);
        let _ = k;
        stack![shade, body].width(Length::Fill).height(Length::Fill).into()
    }

    fn search_amount(&self) -> f32 {
        if self.gallery { if self.search_shown() { 1.0 } else { 0.0 } } else { self.search_fade.interpolate(0.0, 1.0, self.now) }
    }

    pub(super) fn search_shown(&self) -> bool {
        self.search_open || self.filters_open || !self.search.is_empty()
    }

    fn filters(&self, laid: &Laid) -> Element<'_, Message> {
        let w = &self.words;
        let search = text_input("", &self.search)
            .id(self.search_id.clone())
            .on_input(Message::Search)
            .font(theme::MONO)
            .size(theme::BODY)
            .padding(Padding { top: 9.0, right: 8.0, bottom: 9.0, left: 0.0 })
            .width(Length::Fill)
            .style(ui::bare_input(ui::fade()));
        let found = text(w.count("replays-count", laid.order.len() as u64)).font(theme::MONO).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED));
        let escape = container(text("Esc").font(theme::MONO).size(11.0).color(ui::faded(FAINT))).padding([1, 6]).style(|_| container::Style {
            border: Border { color: crate::theme::outline(paint(Color::from_rgba(1.0, 1.0, 1.0, 0.085))), width: 1.0, radius: 6.0.into() },
            ..container::Style::default()
        });
        let open = self.filters_open;
        let sliders = button(container(crate::glyphs::glyph(crate::glyphs::Icon::Sliders, 16.0, INK)).center(SEARCH_H - 10.0))
            .padding(0)
            .width(SEARCH_H - 10.0)
            .height(SEARCH_H - 10.0)
            .style(ui::button_faded(toggle_style(open)))
            .on_press(Message::FiltersPanel(!open));
        let line = ui::glass_clear(container(row![
            crate::glyphs::glyph(crate::glyphs::Icon::Search, 15.0, FAINT),
            search,
            found,
            escape,
            sliders,
        ].spacing(10).align_y(iced::Center))
        .padding(Padding { top: 0.0, right: 5.0, bottom: 0.0, left: 15.0 })
        .height(SEARCH_H)
        .center_y(SEARCH_H)
        .width(Length::Fill));
        let panel_k = if self.gallery { if open { 1.0 } else { 0.0 } } else { self.filters_fade.interpolate(0.0, 1.0, self.now) };
        if panel_k <= 0.001 && !open {
            return container(line).padding(Padding { top: 0.0, right: PANEL_OUT, bottom: 0.0, left: PANEL_OUT }).width(Length::Fill).into();
        }
        let filter = &self.filter;
        let bar = canvas(StarBar { low: filter.low, high: filter.high, alpha: ui::fade(), on_at: Message::StarsAt, on_done: || Message::StarsDone }).width(Length::Fill).height(BAR_H);
        let stars = |value: f64| if value >= carousel::STARS_TOP { "∞".to_owned() } else { w.decimal(value, 1) };
        let caption = |key: &str| text(w.t(key)).font(theme::MONO).size(12.0).color(ui::faded(MUTED));
        let stars_row = column![
            row![caption("filter-stars"), ui::grow(), text(format!("{} {} {}", stars(filter.low), "–", stars(filter.high))).font(theme::MONO).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(INK))].align_y(iced::Center),
            bar,
        ]
        .spacing(4);
        let segment = |label: String, on: bool, press: Message| -> Element<'_, Message> {
            button(container(text(label).font(theme::SANS_SEMI).size(theme::BODY).wrapping(text::Wrapping::None)).center_y(theme::workspace::CONTROL))
                .padding([0, 12])
                .style(ui::button_faded(theme::filter_chip(on)))
                .on_press(press)
                .into()
        };
        let sorts = row(Sort::ALL.iter().map(|sort| segment(w.t(sort.key()), *sort == self.sort, Message::SortBy(*sort)))).spacing(6).wrap().vertical_spacing(6);
        let groups = row(Group::ALL.iter().map(|group| segment(w.t(group.key()), *group == self.group, Message::GroupBy(*group)))).spacing(6).wrap().vertical_spacing(6);
        let toggle = |which: Toggle, on: bool, key: &str| segment(w.t(key), on, Message::FilterToggle(which));
        let toggles = row![
            toggle(Toggle::Mine, filter.mine, "filter-mine"),
            toggle(Toggle::Witness, filter.witness, "filter-witness"),
            toggle(Toggle::Clean, filter.clean, "filter-clean"),
        ]
        .spacing(6)
        .wrap()
        .vertical_spacing(6);
        let reset = row![ui::grow(), ui::quiet(w.t("filters-reset"), Some(Message::FiltersReset))];
        let panel = ui::fading(ui::fade() * panel_k, || ui::glass_clear(container(column![
            stars_row,
            column![caption("filter-sort"), sorts].spacing(8),
            column![caption("filter-group"), groups].spacing(8),
            column![caption("filter-only"), toggles].spacing(8),
            reset,
        ].spacing(theme::workspace::GAP))
        .padding(theme::workspace::INSET)
        .width(PANEL_WIDE.min(self.carousel_width() - 2.0 * PANEL_OUT)).id(iced::widget::Id::new("search-filters"))));
        let panel = ui::collapsing(panel, panel_k);
        container(column![line, container(panel).width(Length::Fill).align_x(iced::alignment::Horizontal::Right)].spacing(8.0 * panel_k)).padding(Padding { top: 0.0, right: PANEL_OUT, bottom: 0.0, left: PANEL_OUT }).width(Length::Fill).into()
    }

    fn carousel(&self, laid: &Laid) -> Element<'_, Message> {
        let w = &self.words;
        if laid.rows.is_empty() {
            let mut empty = column![
                text(w.t("search-nothing")).font(theme::SANS_SEMI).size(theme::LEAD).color(ui::faded(INK)),
            ]
            .spacing(12);
            if self.filter().narrows() {
                empty = empty.push(ui::quiet(w.t("filters-reset"), Some(Message::FiltersReset)));
            }
            return container(empty.align_x(iced::alignment::Horizontal::Center)).padding(Padding { top: 0.0, right: 20.0, bottom: 56.0, left: SIDE }).width(Length::Fill).height(Length::Fill).center(Length::Fill).into();
        }
        let entries = self.entries();
        let height = self.list_height();
        let target = self.carousel_target();
        let shift = self.glide_now(target);
        let shown = self.display(laid);
        let rows = &shown.rows;
        let focus = self.focus.and_then(|_| self.focused());
        let mut backs: Vec<(f32, f32, f32)> = Vec::new();
        let mut back: Option<(f32, f32, f32)> = None;
        let mut top = shift;
        let reach = (-shift / TOP_FADE_ONSET).clamp(0.0, 1.0);
        let mut list = column![].width(Length::Fill);
        for (index, row) in rows.iter().enumerate() {
            let unfold = shown.factors[index];
            let ghost = shown.ghost[index];
            let head = matches!(row, Row::Head { .. });
            if index > 0 {
                let gap = carousel::GAP * gap_factor(rows, &shown.factors, index);
                list = list.push(Space::new().width(Length::Fill).height(gap));
                top += gap;
            }
            if top >= height {
                if let Some((from, bottom, right)) = back.take() {
                    let reaches = if row.depth() > 0.0 { height + BACKING_FOOT } else { bottom };
                    backs.push((from, reaches, right));
                }
                break;
            }
            let full = row.height();
            let tall = full * unfold;
            let body = if head { tall } else { full };
            let unfold = if head { 1.0 } else { unfold };
            let seen = top + tall > 0.0 && top < height;
            let whole = !ghost && top >= -2.0 && top + tall <= height + 2.0;
            let centre = top + tall / 2.0;
            let away = ((centre - height / 2.0).abs() / ARC_SPAN).min(carousel::ARC_STEPS as f32);
            let inset = carousel::NESTED.min(row.depth()) + away * carousel::ARC_STEP;
            let inset = if head { inset } else { back.map_or(inset, |(_, _, right)| right) };
            let lower = if top + tall > height { eased(((height - top) / tall.max(1.0)).clamp(0.0, 1.0)) } else { 1.0 };
            let upper = if top < 0.0 { eased(((top + tall) / tall.max(1.0)).clamp(0.0, 1.0)) } else { 1.0 };
            let squeeze = lower.min(upper);
            let toward = if upper < lower { 0.0 } else { 1.0 };
            let at = top;
            top += tall;
            if head {
                if let Some(done) = back.take() {
                    backs.push(done);
                }
                if rows.get(index + 1).is_some_and(|next| next.depth() > 0.0) {
                    back = Some((at, at + tall, inset));
                }
            } else if let Some(open) = back.as_mut() {
                open.1 = at + tall;
            }
            if !seen {
                list = list.push(Space::new().width(Length::Fill).height(tall));
                continue;
            }
            let build = || -> Element<'_, Message> {
                match row {
                    Row::Head { title, under, first, count, open, cover, marks } => self.head_card(title, under, *first, *count, marks, *open, cover.as_deref(), whole, body, !ghost && focus == Some(Focus::Head(*first))),
                    Row::Replay { at, chosen } => self.replay_card(*at, &entries[*at], *chosen, whole, !ghost && focus == Some(Focus::Replay(*at))),
                }
            };
            let place = |alpha: f32| -> Element<'_, Message> {
                let shown = unfold * squeeze * alpha;
                let built = if shown < 1.0 { ui::fading(ui::fade() * shown, build) } else { build() };
                let built: Element<'_, Message> = if squeeze < 1.0 {
                    ui::grown(built, Point::new(0.0, toward), 0.0, SQUEEZED + (1.0 - SQUEEZED) * squeeze).into()
                } else {
                    built
                };
                let placed = container(built).padding(Padding { top: 0.0, right: inset, bottom: 0.0, left: 0.0 }).width(Length::Fill).height(body);
                placed.into()
            };
            let placed = if unfold >= 1.0 && at < TOP_FADE && reach > 0.01 { top_faded(at, body, reach, place) } else { place(1.0) };
            list = list.push(if unfold < 1.0 {
                ui::clipped(container(placed).width(Length::Fill).height(tall).align_y(iced::alignment::Vertical::Bottom))
            } else {
                placed
            });
        }
        if let Some(done) = back.take() {
            backs.push(done);
        }
        let mut trays = iced::widget::Stack::new().width(Length::Fill).height(Length::Fill);
        for (from, bottom, right) in backs {
            let tall = (bottom - from + BACKING_BOTTOM).max(0.0);
            let tray = container(Space::new().width(Length::Fill).height(tall)).width(Length::Fill).height(tall).style(|_| container::Style {
                background: Some(Background::Color(paint(BACKING))),
                border: Border { radius: iced::border::Radius { top_left: 0.0, top_right: 12.0, bottom_right: 12.0, bottom_left: 0.0 }, ..Border::default() },
                ..container::Style::default()
            });
            let tray = container(tray).padding(Padding { top: 0.0, right, bottom: 0.0, left: 0.0 }).width(Length::Fill);
            trays = trays.push(iced::widget::pin(tray).width(Length::Fill).height(Length::Fill).x(-1.0).y(from));
        }
        let moved = iced::widget::pin(list).width(Length::Fill).height(Length::Fill).x(-1.0).y(shift);
        let grabbed = ui::grab(ui::clipped(container(stack![trays, moved]).width(Length::Fill).height(Length::Fill)), Message::CarouselDrag, Message::CarouselDrop);
        let scroll_list = mouse_area(grabbed)
            .on_scroll(|delta| {
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y,
                    mouse::ScrollDelta::Pixels { y, .. } => y / 48.0,
                };
                Message::Wheel(lines)
            });
        scroll_list.into()
    }

    fn skeleton(&self) -> Element<'_, Message> {
        let mut list = column![].spacing(carousel::GAP).width(Length::Fill);
        for index in 0..SKELETON_ROWS {
            let away = (index as f32 - SKELETON_ROWS as f32 / 2.0).abs().min(carousel::ARC_STEPS as f32);
            let fade = 1.0 - index as f32 / (SKELETON_ROWS as f32 * 1.4);
            let line = |wide: f32, tall: f32| container(Space::new().width(wide).height(tall)).style(move |_| container::Style {
                background: Some(Background::Color(paint(Color { a: fade, ..ui::mix(PANEL, Color::WHITE, 0.07) }))),
                border: Border { radius: (tall / 2.0).into(), ..Border::default() },
                ..container::Style::default()
            });
            let card = container(column![line(150.0 + (index % 3) as f32 * 40.0, 12.0), line(90.0, 9.0)].spacing(10))
                .padding(Padding { top: 0.0, right: 20.0, bottom: 0.0, left: SIDE })
                .center_y(64.0)
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Background::Color(paint(Color { a: 0.9 * fade, ..CARD }))),
                    border: Border { color: crate::theme::outline(paint(Color::from_rgba(1.0, 1.0, 1.0, 0.07 * fade))), width: 1.0, radius: iced::border::Radius { top_left: 0.0, top_right: 12.0, bottom_right: 12.0, bottom_left: 0.0 } },
                    ..container::Style::default()
                });
            list = list.push(container(card).padding(Padding { top: 0.0, right: away * carousel::ARC_STEP, bottom: 0.0, left: 0.0 }).width(Length::Fill));
        }
        container(list).width(Length::Fill).height(Length::Fill).into()
    }

    fn head_card<'a>(&'a self, title: &str, under: &str, first: usize, count: usize, marks: &[crate::carousel::Mark], open: bool, cover: Option<&str>, live: bool, height: f32, focused: bool) -> Element<'a, Message> {
        let w = &self.words;
        let shaded = cover.and_then(|hash| self.cover_frame(hash));
        let picture: Element<'_, Message> = match (shaded, cover.and_then(|hash| self.thumbs.get(hash))) {
            (Some(frame), _) => crate::film::show_cover(&frame, ui::fade(), 11.0, CARD, HEAD_FLOOR),
            (None, Some(handle)) => image(handle.clone())
                .content_fit(ContentFit::Cover)
                .width(Length::Fill)
                .height(height)
                .border_radius(iced::border::Radius { top_left: 0.0, top_right: 12.0, bottom_right: 12.0, bottom_left: 0.0 })
                .opacity(ui::fade())
                .into(),
            (None, None) => Space::new().width(Length::Fill).height(height).into(),
        };
        let backdrop = container(picture).padding(Padding { top: 1.0, right: 1.0, bottom: 1.0, left: 0.0 }).width(Length::Fill).height(height);
        let scrim = container(Space::new().width(Length::Fill).height(height)).style(|_| container::Style {
            background: Some(Background::Gradient(iced::Gradient::Linear(
                iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
                    .add_stop(0.0, paint(Color { a: 0.62, ..SCRIM }))
                    .add_stop(0.45, paint(Color { a: 0.42, ..SCRIM }))
                    .add_stop(1.0, paint(Color { a: 0.0, ..SCRIM })),
            ))),
            ..container::Style::default()
        });
        let shown_title = if title.is_empty() { w.t("unknown-map") } else { title.to_owned() };
        let mut words = column![text(shown_title).font(theme::SANS_SEMI).size(theme::LEAD).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(2);
        if !under.is_empty() {
            words = words.push(text(under.to_owned()).font(theme::SANS).size(13.0).wrapping(text::Wrapping::None).color(ui::faded(MUTED)));
        }
        let mut inside = row![].spacing(14).align_y(iced::Center);
        if self.group == Group::Players {
            inside = inside.push(self.player_face(title, 36.0));
        }
        let count_chip = container(text(count.to_string()).font(theme::MONO_BOLD).size(theme::CAPTION).color(ui::faded(INK)))
            .padding([4, 8])
            .style(|_| container::Style {
                background: Some(Background::Color(paint(Color { a: 0.92, ..PANEL }))),
                border: Border { radius: 12.0.into(), ..Border::default() },
                ..container::Style::default()
            });
        let tail: Element<'_, Message> = if marks.is_empty() {
            count_chip.into()
        } else {
            let tint = |stars: Option<f64>| stars.map_or(MUTED, |stars| crate::dossier::star_colour(stars as f32));
            let mut icons = row![].spacing(3).align_y(iced::Center);
            if marks.len() <= MARKS_ONE_BY_ONE {
                for mark in marks {
                    icons = icons.push(canvas(ModeMark { colour: tint(mark.stars), mania: mark.mania, alpha: ui::fade() }).width(MARK).height(MARK));
                }
            } else {
                let top = marks.iter().filter_map(|mark| mark.stars).fold(None, |best: Option<f64>, stars| Some(best.map_or(stars, |best| best.max(stars))));
                let mania = marks.iter().all(|mark| mark.mania);
                icons = icons
                    .push(canvas(ModeMark { colour: tint(top), mania, alpha: ui::fade() }).width(MARK).height(MARK))
                    .push(text(format!("×{}", marks.len())).font(theme::MONO_BOLD).size(11.0).color(ui::faded(INK)));
            }
            container(icons)
                .padding([4, 7])
                .style(|_| container::Style {
                    background: Some(Background::Color(paint(Color { a: 0.92, ..PANEL }))),
                    border: Border { radius: 12.0.into(), ..Border::default() },
                    ..container::Style::default()
                })
                .into()
        };
        inside = inside.push(container(words).width(Length::Fill).clip(true)).push(tail);
        let face = container(inside).padding(Padding { top: 0.0, right: 20.0, bottom: 0.0, left: SIDE }).center_y(height).width(Length::Fill);
        let card = stack![backdrop, scrim, face].width(Length::Fill).height(height);
        button(ui::clipped(card))
            .padding(0)
            .width(Length::Fill)
            .height(height)
            .style(ui::button_faded(card_style(open, false, focused)))
            .on_press_maybe(live.then_some(Message::Choose(first)))
            .into()
    }

    fn replay_card<'a>(&'a self, at: usize, entry: &'a Entry, chosen: bool, live: bool, focused: bool) -> Element<'a, Message> {
        let w = &self.words;
        let height = 60.0;
        let title = match self.group {
            Group::Players => entry.title().map(|title| match &entry.map { Some(map) => format!("{title} [{}]", map.version), None => title }).unwrap_or_else(|| w.t("unknown-map")),
            Group::Maps => entry.player.clone(),
            Group::Flat => entry.title().unwrap_or_else(|| w.t("unknown-map")),
            Group::Days => format!("{}  {}", entry.player, entry.title().unwrap_or_default()),
        };
        let when = match self.group {
            Group::Days => w.clock(entry.played_at),
            _ => format!("{} {}", w.day(entry.played_at, self.now_unix), w.clock(entry.played_at)),
        };
        let when = if self.group == Group::Flat { format!("{} · {}", entry.player, when) } else { when };
        let sub = text(when).font(theme::MONO).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED));
        let title = text(title).font(theme::SANS_SEMI).size(theme::BODY).wrapping(text::Wrapping::None).color(ui::faded(INK));
        let mut name = match self.group {
            Group::Maps => row![title],
            _ => row![container(title).width(Length::Fill).clip(true)],
        }
        .spacing(8)
        .align_y(iced::Center);
        for acronym in entry.mods.iter().filter(|acronym| crate::modicons::shown(acronym)) {
            name = name.push(super::mod_badge_sized(acronym, 16.0));
        }
        let rating = self.ratings.known.get(&entry.replay_hash);
        let pp = rating.map(|r| format!("{}pp", r.pp.round() as i64)).unwrap_or_else(|| "—".to_owned());
        let numbers: Element<'_, Message> = match (self.group, entry.map.as_ref()) {
            (Group::Maps, Some(map)) => {
                let version = ui::fitted(&map.version, theme::SANS_SEMI, theme::CAPTION, VERSION_ROOM);
                let mut said = column![].spacing(4).align_x(iced::alignment::Horizontal::Right);
                if let Some(stars) = rating.map(|r| r.stars) {
                    said = said.push(crate::dossier::star_pill(w.decimal(stars, 2), stars as f32, 12.0));
                }
                said.push(text(version).font(theme::SANS_SEMI).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED))).into()
            }
            _ => column![
                text(pp).font(theme::MONO_BOLD).size(theme::BODY).color(ui::faded(INK)),
                text(w.percent(entry.accuracy)).font(theme::MONO).size(theme::CAPTION).color(ui::faded(MUTED)),
            ]
            .spacing(2)
            .align_x(iced::alignment::Horizontal::Right)
            .into(),
        };
        let rank = self.rank_tile(entry.grade, RANK_SMALL, true);
        let mut inside = row![rank, container(column![name, sub].spacing(2)).width(Length::Fill).clip(true)].spacing(14).align_y(iced::Center);
        if self.group != Group::Maps {
            let colour = rating.map_or(MUTED, |r| crate::dossier::star_colour(r.stars as f32));
            let mark = container(canvas(ModeMark { colour, mania: entry.mania.is_some(), alpha: ui::fade() }).width(MARK).height(MARK)).padding(5).style(|_| container::Style {
                background: Some(Background::Color(paint(Color { a: 0.92, ..PANEL }))),
                border: Border { radius: 14.0.into(), ..Border::default() },
                ..container::Style::default()
            });
            inside = inside.push(mark);
        }
        let inside = if self.group == Group::Flat { inside } else { inside.push(numbers) };
        let content = container(inside).padding(Padding { top: 0.0, right: 20.0, bottom: 0.0, left: 24.0 }).center_y(height - 2.0).width(Length::Fill);
        let face: Element<'_, Message> = match (self.group != Group::Maps).then(|| self.cover_frame(&entry.map_hash)).flatten() {
            Some(frame) => stack![
                container(crate::film::show_cover(&frame, ui::fade(), 9.0, if chosen { PICKED } else { CARD }, REPLAY_FLOOR)).padding(Padding { top: 1.0, right: 1.0, bottom: 1.0, left: 0.0 }).width(Length::Fill).height(height),
                content,
            ]
            .into(),
            None => content.into(),
        };
        button(face)
            .padding(0)
            .width(Length::Fill)
            .height(height)
            .style(ui::button_faded(card_style(chosen, true, focused)))
            .on_press_maybe(live.then_some(Message::Choose(at)))
            .into()
    }

    fn player_face<'a>(&'a self, player: &str, side: f32) -> Element<'a, Message> {
        let fill = ui::faded(Color::from_rgb(0.29, 0.196, 0.212));
        let round = move |_: &Theme| container::Style {
            background: Some(Background::Color(fill)),
            border: Border { radius: (side / 2.0).into(), ..Border::default() },
            ..container::Style::default()
        };
        match self.shared_avatar(player).and_then(|avatar| self.news_pictures.get(avatar)) {
            Some(picture) => container(image(picture.clone()).content_fit(ContentFit::Cover).width(side).height(side).border_radius(side / 2.0).opacity(ui::fade())).center(side).style(round).into(),
            None => {
                let initial = player.chars().find(|c| c.is_alphanumeric()).map(|c| c.to_uppercase().to_string()).unwrap_or_default();
                container(text(initial).font(theme::SANS_SEMI).size(15.0).color(ui::faded(INK))).center(side).style(round).into()
            }
        }
    }

    pub(super) fn rank_tile<'a>(&'a self, grade: Grade, side: f32, live: bool) -> Element<'a, Message> {
        let colour = grade_colour(grade);
        let radius = if side >= RANK_LARGE { 14.0 } else { 10.0 };
        let letter = text(grade.letter()).font(theme::MONO_BOLD).size(side * 0.5).color(ui::faded(colour));
        let base = container(letter).center(side).style(move |_| container::Style {
            background: Some(Background::Color(paint(tinted(colour, 0.14)))),
            border: Border { color: crate::theme::outline(paint(tinted(colour, if side >= RANK_LARGE { 0.4 } else { 0.0 }))), width: 1.0, radius: radius.into() },
            ..container::Style::default()
        });
        if !live || !pulses(grade) {
            return base.into();
        }
        let glow = canvas(RankGlow { grade, side, radius, seconds: self.glow_seconds(), alpha: ui::fade() }).width(side).height(side);
        stack![base, glow].width(side).height(side).into()
    }

    pub(super) fn scene_layout(&self) -> SceneLayout {
        let room = (self.width - self.carousel_width() - 80.0).max(200.0);
        let narrow = room < 640.0;
        let below = HEAD_H + STATS_H + 2.0 * SCENE_GAP + if narrow { ACTIONS_H + STATS_H + STATS_GAP + SCENE_GAP } else { 0.0 };
        let height = (self.height - TOP - 28.0).max(240.0);
        let tall = (room * 9.0 / 16.0).min(height - below).max(120.0);
        let wide = tall * 16.0 / 9.0;
        let grow = ((height - tall) / below).min(wide / GROW_WIDTH).clamp(1.0, GROW_MOST);
        SceneLayout { wide, tall, narrow, grow }
    }

    fn scene_panel(&self) -> Element<'_, Message> {
        let w = &self.words;
        let Some(entry) = self.chosen_entry() else {
            return self.viewer(1.0);
        };
        let SceneLayout { wide, tall, narrow, grow, .. } = self.scene_layout();
        let before = self.live_before.as_ref();
        let behind = |seen: f32| -> Element<'_, Message> {
            match before.filter(|_| seen < 0.999) {
                Some(frame) => crate::film::show_rounded(frame, crate::film::Fit::Cover, ui::fade() * (1.0 - seen), theme::CARD_RADIUS - 1.0),
                None => Space::new().width(Length::Fill).height(Length::Fill).into(),
            }
        };
        let screen: Element<'_, Message> = match self.live.as_ref().filter(|live| live.for_path == entry.path).and_then(|live| live.frame.as_ref().map(|frame| (live, frame))) {
            Some((live, frame)) => {
                let seen = live.fade.interpolate(0.0, 1.0, self.now);
                let now = crate::film::show_rounded(frame, crate::film::Fit::Cover, ui::fade() * seen, theme::CARD_RADIUS - 1.0);
                mouse_area(stack![behind(seen), now]).on_press(Message::TogglePlay).into()
            }
            None if before.is_some() && self.live.as_ref().is_some_and(|live| live.for_path == entry.path) => behind(0.0),
            None => match self.scenes.get(&entry.map_hash) {
                Some(handle) => image(handle.clone()).content_fit(ContentFit::Cover).width(Length::Fill).height(Length::Fill).border_radius(theme::CARD_RADIUS).opacity(ui::fade() * 0.35).into(),
                None => Space::new().width(Length::Fill).height(Length::Fill).into(),
            },
        };
        let screen = container(ui::clipped(screen)).padding(1).width(wide).height(tall).style(|_| container::Style {
            background: Some(Background::Color(paint(PANEL))),
            border: Border { color: crate::theme::outline(paint(Color::from_rgba(1.0, 1.0, 1.0, 0.05))), width: 1.0, radius: theme::CARD_RADIUS.into() },
            ..container::Style::default()
        });
        let rating = self.ratings.known.get(&entry.replay_hash).copied();
        let told = ui::fade() * (TOLD_FLOOR + (1.0 - TOLD_FLOOR) * self.told.interpolate(0.0, 1.0, self.now));
        let (head, stats) = ui::fading(told, || {
            let mut name = row![text(entry.player.clone()).font(theme::SANS_SEMI).size(24.0 * grow).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(10).align_y(iced::Center);
            for acronym in entry.mods.iter().filter(|acronym| crate::modicons::shown(acronym)) {
                name = name.push(super::mod_badge_sized(acronym, 20.0 * grow));
            }
            let song = row![].spacing(8).align_y(iced::Center).push(container(text(entry.song().unwrap_or_else(|| w.t("unknown-map"))).font(theme::SANS).size(theme::BODY * grow).wrapping(text::Wrapping::None).color(ui::faded(MUTED))).width(Length::Fill).clip(true));
            let when = row![
                text(format!("{} {}", w.day(entry.played_at, self.now_unix), w.clock(entry.played_at))).font(theme::MONO).size(theme::CAPTION * grow).color(ui::faded(FAINT)),
                text(self.client_said(entry)).font(theme::MONO).size(theme::CAPTION * grow).color(ui::faded(match entry.client {
                    crate::library::Client::Stable => theme::SHARED,
                    crate::library::Client::Lazer => ACCENT,
                })),
            ]
            .spacing(12);
            let identity = container(column![name, song, when].spacing(2.0 * grow)).width(Length::Fill);
            let head = row![self.rank_tile(entry.grade, RANK_LARGE * grow, self.glowing()), identity].spacing(16).align_y(iced::Center).width(wide);
            (head, self.stats_panel(entry, rating, wide, grow, narrow))
        });
        let actions = ui::scaled(row![self.share_button(entry), self.action(entry)].spacing(8).align_y(iced::Center), ACTIONS_SCALE * grow.min(1.2));
        let mut head = head;
        let gap = SCENE_GAP * grow;
        if narrow {
            let actions = container(actions).width(wide).align_x(iced::alignment::Horizontal::Right);
            return column![screen, head, stats, actions].spacing(gap).width(Length::Fill).into();
        }
        head = head.push(actions);
        column![screen, head, stats].spacing(gap).width(Length::Fill).into()
    }

    fn stats_panel<'a>(&'a self, entry: &'a Entry, rating: Option<crate::rating::Rating>, wide: f32, grow: f32, narrow: bool) -> Element<'a, Message> {
        let w = &self.words;
        let label = move |key: &str| text(w.t(key)).font(theme::SANS).size(theme::CAPTION * grow).color(ui::faded(MUTED));
        let value = move |words: String| text(words).font(theme::MONO_BOLD).size(24.0 * grow).wrapping(text::Wrapping::None).color(ui::faded(INK));
        let max = rating.map(|r| r.max_combo).or_else(|| self.combos.get(&entry.path).copied().flatten());
        let combo_share = max.map_or(0.0, |max| (f32::from(entry.combo) / max.max(1) as f32).clamp(0.0, 1.0));
        let full = max.is_some_and(|max| u32::from(entry.combo) >= max);
        let mut combo_line = row![value(entry.combo.to_string())].spacing(4).align_y(iced::alignment::Vertical::Bottom);
        if let Some(max) = max {
            combo_line = combo_line.push(text(format!("{} {}", w.t("combo-of"), max.to_string())).font(theme::MONO).size(theme::CAPTION * grow).wrapping(text::Wrapping::None).color(ui::faded(FAINT)));
        }
        let combo_bar = canvas(ComboBar { share: combo_share, full, alpha: ui::fade() }).width(Length::Fill).height(3.0);
        let [c300, c100, c50, miss] = entry.counts;
        let hit_size = if entry.mania.is_some() { HIT_SIZE * 0.82 } else { HIT_SIZE } * grow;
        let hit = |label: &str, colour: Color, count: u16| row![
            text(label.to_owned()).font(theme::MONO).size(hit_size).wrapping(text::Wrapping::None).color(ui::faded(colour)),
            text(count.to_string()).font(theme::MONO_BOLD).size(hit_size).wrapping(text::Wrapping::None).color(ui::faded(INK)),
        ].spacing(6);
        let hits = match entry.mania {
            Some(mania) => row![
                column![hit("MAX", theme::GRADE_SS, mania.max), hit("300", theme::HIT_300, c300)].spacing(10.0 * grow),
                column![hit("200", theme::HIT_100, mania.good), hit("100", theme::GRADE_C, c100)].spacing(10.0 * grow),
                column![hit("50", theme::HIT_50, c50), hit("×", ACCENT, miss)].spacing(10.0 * grow),
            ]
            .spacing(12.0 * grow),
            None => row![
                column![hit("300", theme::HIT_300, c300), hit("100", theme::HIT_100, c100)].spacing(10.0 * grow),
                column![hit("50", theme::HIT_50, c50), hit("×", ACCENT, miss)].spacing(10.0 * grow),
            ]
            .spacing(20.0 * grow),
        };
        let pp = rating.map(|r| (r.pp.round() as u64).to_string()).unwrap_or_else(|| "—".to_owned());
        let accuracy_bar = canvas(ComboBar { share: (entry.accuracy as f32 / 100.0).clamp(0.0, 1.0), full: false, alpha: ui::fade() }).width(Length::Fill).height(3.0);
        let tile = move |inside: Element<'a, Message>, share: u16| -> Element<'a, Message> {
            container(inside)
                .padding([12.0 * grow, 12.0 * grow])
                .width(Length::FillPortion(share))
                .height(STATS_H * grow)
                .style(|_| container::Style {
                    background: Some(Background::Color(paint(CARD))),
                    border: Border { color: crate::theme::outline(paint(Color::from_rgba(1.0, 1.0, 1.0, 0.05))), width: 1.0, radius: theme::CARD_RADIUS.into() },
                    ..container::Style::default()
                })
                .into()
        };
        let spread = |top: Element<'a, Message>, middle: Element<'a, Message>, bottom: Element<'a, Message>| -> Element<'a, Message> {
            column![top, Space::new().height(Length::Fill), middle, Space::new().height(Length::Fill), bottom].height(Length::Fill).into()
        };
        let hits_share = if entry.mania.is_some() { 4 } else { 3 };
        let accuracy = tile(spread(label("stat-accuracy").into(), value(w.percent(entry.accuracy)).into(), accuracy_bar.into()), 3);
        let badge = move |words: String, tint: Color| -> Element<'a, Message> {
            let k = ui::fade();
            let scale = grow.min(1.12);
            container(text(words).font(theme::MONO_BOLD).size(11.0 * scale).wrapping(text::Wrapping::None).color(Color { a: k, ..Color::WHITE }))
                .padding([1.0 * scale, 6.0 * scale])
                .style(move |_| container::Style {
                    background: Some(Background::Color(Color { a: k, ..tint })),
                    border: Border { radius: (5.0 * grow).into(), ..Border::default() },
                    ..container::Style::default()
                })
                .into()
        };
        let [_, _, _, missed] = entry.counts;
        let mut extra = column![].spacing(3.0 * grow);
        if let Some(rating) = rating.filter(|rating| rating.complete()) {
            let worth = |pp: f64| (pp.round() as u64).to_string();
            extra = extra.push(badge(if missed == 0 { "FC".to_owned() } else { format!("FC {}", worth(rating.pp_fc)) }, PP_FC));
            extra = extra.push(badge(if entry.accuracy >= 100.0 { "SS".to_owned() } else { format!("SS {}", worth(rating.pp_ss)) }, PP_SS));
        }
        let pp_line: Element<'a, Message> = row![value(pp), extra].spacing(10.0 * grow).align_y(iced::Center).into();
        let pp = tile(spread(label("stat-pp").into(), pp_line, Space::new().height(3.0).into()), if narrow { 2 } else { 3 });
        let combo = tile(spread(label("stat-combo").into(), combo_line.into(), combo_bar.into()), 3);
        let hits = tile(container(hits).center_x(Length::Fill).center_y(Length::Fill).into(), hits_share);
        if narrow {
            return column![row![accuracy, combo].spacing(STATS_GAP * grow), row![pp, hits].spacing(STATS_GAP * grow)].spacing(STATS_GAP * grow).width(wide).into();
        }
        row![accuracy, pp, combo, hits].spacing(STATS_GAP * grow).width(wide).into()
    }

    fn share_button<'a>(&'a self, entry: &'a Entry) -> Element<'a, Message> {
        let _ = entry;
        ui::quiet(self.words.t("share"), Some(Message::ShareMenu(!self.share_open)))
    }

    pub(super) fn share_layer_menu(&self) -> Element<'_, Message> {
        let Some(entry) = self.chosen_entry().filter(|_| self.share_open && self.overlay == Overlay::None) else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let w = &self.words;
        let video = self.store.videos.iter().find(|video| !video.replay_hash.is_empty() && video.replay_hash == entry.replay_hash);
        let mut list = column![].spacing(2);
        let item = |label: String, message: Option<Message>| {
            button(container(text(label).font(theme::SANS).size(13.0).color(ui::faded(if message.is_some() { INK } else { FAINT }))).center_y(32.0))
                .padding([0, 12])
                .width(Length::Fill)
                .style(ui::button_faded(option_style(false)))
                .on_press_maybe(message)
        };
        list = list.push(item(w.t("share-chat"), video.map(|video| Message::ShareVideo(video.path.clone()))));
        list = list.push(item(w.t("share-player"), video.map(|video| Message::SendToPlayer(video.path.clone()))));
        list = list.push(item(w.t("show-in-folder"), Some(Message::OpenFolder)));
        if video.is_none() {
            list = list.push(container(text(w.t("share-needs-video")).font(theme::SANS).size(theme::CAPTION).color(ui::faded(FAINT))).padding([6, 12]));
        }
        let menu = container(list).padding(4).width(240.0).style(|_| container::Style {
            background: Some(Background::Color(Color { a: ui::fade(), ..Color::from_rgb(0.09, 0.027, 0.043) })),
            border: Border { color: crate::theme::outline(paint(Color::from_rgba(1.0, 1.0, 1.0, 0.07))), width: 1.0, radius: 10.0.into() },
            shadow: iced::Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.45 * ui::fade()), offset: Vector::new(0.0, 16.0), blur_radius: 40.0 },
            ..container::Style::default()
        });
        let veil = mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::ShareMenu(false));
        let placed = container(menu).width(Length::Fill).height(Length::Fill).align_x(iced::alignment::Horizontal::Right).align_y(iced::alignment::Vertical::Bottom).padding(Padding { top: 0.0, right: 40.0, bottom: 28.0 + 66.0 + 16.0 + 64.0, left: 0.0 });
        stack![veil, placed].width(Length::Fill).height(Length::Fill).into()
    }

    pub(super) fn client_said(&self, entry: &Entry) -> String {
        match entry.path.starts_with(crate::sources::witnessed_root()) {
            true => format!("{} (Witness)", entry.client.tag()),
            false => entry.client.tag().to_owned(),
        }
    }

    pub(super) fn glowing(&self) -> bool {
        self.overlay == Overlay::None && !self.minimized && !self.gallery && self.chosen_entry().is_some_and(|entry| pulses(entry.grade))
    }
}

fn card_style(on: bool, replay: bool, focused: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let lit = focused || matches!(status, button::Status::Hovered | button::Status::Pressed);
        let (fill, edge) = match (on, replay) {
            (true, true) => (PICKED, Color { a: 0.7, ..ACCENT }),
            _ => (if lit { CARD_LIT } else { CARD }, Color::TRANSPARENT),
        };
        let (edge, width) = if focused { (ACCENT, 2.0) } else { (edge, 1.0) };
        button::Style {
            background: Some(Background::Color(fill)),
            text_color: INK,
            border: Border { color: edge, width, radius: iced::border::Radius { top_left: 0.0, top_right: if replay { 10.0 } else { 12.0 }, bottom_right: if replay { 10.0 } else { 12.0 }, bottom_left: 0.0 } },
            ..button::Style::default()
        }
    }
}

fn option_style(on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(Background::Color(if on { PICKED } else if lit { CARD_LIT } else { Color::TRANSPARENT })),
            text_color: INK,
            border: Border { radius: 7.0.into(), ..Border::default() },
            ..button::Style::default()
        }
    }
}

fn toggle_style(on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let mut style = theme::workspace::choice(on)(theme, status);
        style.border.radius = ((SEARCH_H - 10.0) / 2.0).into();
        style
    }
}

struct CarouselShade {
    width: f32,
    alpha: f32,
}

impl<M> canvas::Program<M> for CarouselShade {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let edge = (self.width / bounds.width.max(1.0)).clamp(0.0, 1.0);
        let gradient = canvas::gradient::Linear::new(Point::ORIGIN, Point::new(bounds.width, 0.0))
            .add_stop(0.0, Color { a: 0.62 * self.alpha, ..theme::GROUND })
            .add_stop(edge, Color { a: 0.48 * self.alpha, ..theme::GROUND })
            .add_stop(1.0, Color { a: 0.4 * self.alpha, ..theme::GROUND });
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), canvas::Fill::from(gradient));
        vec![frame.into_geometry()]
    }
}

pub(crate) struct StarBar<M> {
    pub low: f64,
    pub high: f64,
    pub alpha: f32,
    pub on_at: fn(f32, bool) -> M,
    pub on_done: fn() -> M,
}

const SPECTRUM: [(f32, (u8, u8, u8)); 10] = [
    (0.0, (79, 192, 255)),
    (0.14, (79, 255, 213)),
    (0.24, (124, 255, 79)),
    (0.34, (246, 240, 92)),
    (0.46, (255, 128, 104)),
    (0.56, (255, 78, 111)),
    (0.66, (198, 69, 184)),
    (0.76, (101, 99, 222)),
    (0.88, (24, 21, 142)),
    (1.0, (0, 0, 0)),
];

impl<M> canvas::Program<M> for StarBar<M> {
    type State = bool;

    fn update(&self, held: &mut bool, event: &iced::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<M>> {
        let fraction = |position: Point| (position.x - bounds.x - TRACK_KNOB - 2.0) / (bounds.width - 2.0 * (TRACK_KNOB + 2.0)).max(1.0);
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position_in(bounds).map(|p| Point::new(p.x + bounds.x, p.y))?;
                *held = true;
                Some(canvas::Action::publish((self.on_at)(fraction(position), true)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) if *held => Some(canvas::Action::publish((self.on_at)(fraction(*position), false)).and_capture()),
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if *held => {
                *held = false;
                Some(canvas::Action::publish((self.on_done)()).and_capture())
            }
            _ => None,
        }
    }

    fn draw(&self, _: &bool, renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let size = bounds.size();
        let knob_room = TRACK_KNOB + 2.0;
        let span = (size.width - 2.0 * knob_room).max(1.0);
        let top = (size.height - TRACK_H) / 2.0;
        let track = canvas::Path::rounded_rectangle(Point::new(knob_room, top), Size::new(span, TRACK_H), (TRACK_H / 2.0).into());
        let mut gradient = canvas::gradient::Linear::new(Point::new(knob_room, 0.0), Point::new(knob_room + span, 0.0));
        for (at, (r, g, b)) in SPECTRUM {
            gradient = gradient.add_stop(at, Color { a: self.alpha, ..Color::from_rgb8(r, g, b) });
        }
        frame.fill(&track, canvas::Fill::from(gradient));
        let low = knob_room + (self.low / carousel::STARS_TOP) as f32 * span;
        let high = knob_room + (self.high / carousel::STARS_TOP) as f32 * span;
        let dim = Color { a: 0.8 * self.alpha, ..theme::GROUND };
        if self.low > 0.0 {
            frame.with_clip(Rectangle::new(Point::new(0.0, 0.0), Size::new(low, size.height)), |inside| inside.fill(&track, dim));
        }
        if self.high < carousel::STARS_TOP {
            frame.with_clip(Rectangle::new(Point::new(high, 0.0), Size::new(size.width - high, size.height)), |inside| inside.fill(&track, dim));
        }
        let rim = Color { a: 0.9 * self.alpha, ..theme::GROUND };
        let handle = Color { a: self.alpha, ..Color::WHITE };
        for x in [low, high] {
            let knob = canvas::Path::circle(Point::new(x, size.height / 2.0), TRACK_KNOB);
            frame.fill(&knob, handle);
            frame.stroke(&knob, canvas::Stroke::default().with_width(2.5).with_color(rim));
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(&self, _: &bool, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if cursor.is_over(bounds) { mouse::Interaction::Pointer } else { mouse::Interaction::default() }
    }
}

struct ComboBar {
    share: f32,
    full: bool,
    alpha: f32,
}

impl<M> canvas::Program<M> for ComboBar {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let size = bounds.size();
        frame.fill(&canvas::Path::rounded_rectangle(Point::ORIGIN, size, 1.5.into()), Color::from_rgba(1.0, 1.0, 1.0, 0.1 * self.alpha));
        let fill = if self.full { theme::GRADE_S } else { INK };
        frame.fill(&canvas::Path::rounded_rectangle(Point::ORIGIN, Size::new(size.width * self.share, size.height), 1.5.into()), Color { a: self.alpha, ..fill });
        vec![frame.into_geometry()]
    }
}

struct RankGlow {
    grade: Grade,
    side: f32,
    radius: f32,
    seconds: f32,
    alpha: f32,
}

impl<M> canvas::Program<M> for RankGlow {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let side = self.side;
        let colour = grade_colour(self.grade);
        let light = ui::mix(colour, Color::WHITE, 0.45);
        let tile = canvas::Path::rounded_rectangle(Point::ORIGIN, Size::new(side, side), self.radius.into());
        let swell = |period: f32| 0.5 - 0.5 * (self.seconds * std::f32::consts::TAU / period).cos();
        for (level, tint, strength, middle) in [
            (0.28 + 0.30 * swell(TIDE_SLOW), colour, 0.34, 0.12),
            (0.46 - 0.24 * swell(TIDE_LATE), light, 0.18, 0.0),
        ] {
            let top = side * (1.0 - level);
            let gradient = canvas::gradient::Linear::new(Point::new(0.0, side), Point::new(0.0, top))
                .add_stop(0.0, Color { a: strength * self.alpha, ..tint })
                .add_stop(0.6, Color { a: middle * self.alpha, ..tint })
                .add_stop(1.0, Color { a: 0.0, ..tint });
            frame.fill(&tile, canvas::Fill::from(gradient));
        }
        vec![frame.into_geometry()]
    }
}

struct ModeMark {
    colour: Color,
    mania: bool,
    alpha: f32,
}

impl<M> canvas::Program<M> for ModeMark {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let side = bounds.width.min(bounds.height);
        let centre = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let ink = Color { a: self.alpha, ..self.colour };
        let ring = side / 2.0;
        let stroke = (ring * 0.16).max(1.6);
        frame.stroke(&canvas::Path::circle(centre, ring - stroke / 2.0), canvas::Stroke::default().with_width(stroke).with_color(ink));
        if self.mania {
            let wide = ring * 0.21;
            for (dx, half) in [(0.0, ring * 0.63), (-ring * 0.33, ring * 0.33), (ring * 0.33, ring * 0.33)] {
                let bar = canvas::Path::rounded_rectangle(Point::new(centre.x + dx - wide / 2.0, centre.y - half), iced::Size::new(wide, half * 2.0), (wide / 2.0).into());
                frame.fill(&bar, ink);
            }
        } else {
            frame.fill(&canvas::Path::circle(centre, ring * 0.5), ink);
        }
        vec![frame.into_geometry()]
    }
}
