use super::{
    judge::{Combo, OrderedEvent, ResultKind},
    stable::{TraceEvent, TraceKind},
    Timeline,
};

const TICK_MS: f64 = 100.0;

#[derive(Debug, Clone)]
struct Interval {
    start: f64,
    end: f64,
    before: u64,
}
impl Interval {
    fn ticks_at(&self, time: f64) -> u64 {
        ((time.min(self.end) - self.start).max(0.0) / TICK_MS).floor() as u64
    }
}

#[derive(Default, Debug, Clone)]
struct Lane {
    intervals: Vec<Interval>,
    total: u64,
}
impl Lane {
    fn ticks_at(&self, time: f64) -> u64 {
        let end = self.intervals.partition_point(|i| i.start <= time);
        end.checked_sub(1).map_or(0, |index| {
            let i = &self.intervals[index];
            i.before + i.ticks_at(time)
        })
    }
}

struct Action {
    time: f64,
    frame: Option<usize>,
    object: usize,
    add: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct Checkpoint {
    time: f64,
    added: u64,
    reset_at: u64,
    maximum: u64,
}

#[derive(Debug, Clone)]
pub(super) struct StableCombo {
    lanes: Vec<Lane>,
    checkpoints: Vec<Checkpoint>,
}
impl StableCombo {
    fn ticks_at(&self, time: f64) -> u64 {
        self.lanes.iter().map(|lane| lane.ticks_at(time)).sum()
    }

    pub(super) fn at(&self, time: f64) -> Combo {
        if time.is_nan() {
            return Combo::default();
        }
        let end = self.checkpoints.partition_point(|c| c.time <= time);
        let c = end
            .checked_sub(1)
            .map_or(Checkpoint::default(), |i| self.checkpoints[i]);
        let current = self.ticks_at(time) + c.added - c.reset_at;
        Combo {
            current: current as u32,
            maximum: c.maximum.max(current) as u32,
        }
    }
}

pub(super) struct Builder {
    ends: Vec<Option<f64>>,
    lanes: Vec<Lane>,
    active: Vec<Option<(usize, f64, f64)>>,
    actions: Vec<Action>,
    valid: bool,
}
impl Builder {
    pub(super) fn new(timeline: &Timeline) -> Self {
        let mut ends = vec![None; timeline.lanes.iter().map(|l| l.notes.len()).sum()];
        for note in timeline.lanes.iter().flat_map(|l| &l.notes) {
            if note.hold {
                ends[note.object_index] = Some(note.end_ms);
            }
        }
        Self {
            ends,
            lanes: vec![Lane::default(); timeline.columns() as usize],
            active: vec![None; timeline.columns() as usize],
            actions: Vec::new(),
            valid: true,
        }
    }

    fn close(&mut self, column: usize, time: f64) {
        let Some((_, start, end)) = self.active[column].take() else {
            return;
        };
        let end = time.min(end).max(start);
        let lane = &mut self.lanes[column];
        let interval = Interval {
            start,
            end,
            before: lane.total,
        };
        let ticks = interval.ticks_at(end);
        if ticks > u32::MAX as u64 || lane.total + ticks > u32::MAX as u64 {
            self.valid = false;
            return;
        }
        lane.total += ticks;
        lane.intervals.push(interval);
    }

    pub(super) fn observe(&mut self, e: TraceEvent) {
        let Some(end) = self.ends[e.object_index] else {
            return;
        };
        let column = e.column as usize;
        match e.kind {
            TraceKind::HeadPressed | TraceKind::Regrabbed => {
                self.close(column, e.time_ms);
                self.active[column] = Some((e.object_index, e.time_ms, end));
            }
            _ if self.active[column].is_some_and(|(object, _, _)| object == e.object_index) => {
                self.close(column, e.time_ms);
            }
            _ => {}
        }
        if matches!(
            e.kind,
            TraceKind::BodyReleased
                | TraceKind::ReleasedBeforeHead
                | TraceKind::HeadExpired
                | TraceKind::HeadForcedMiss
        ) || (e.kind == TraceKind::HeadPressed && e.broken)
        {
            self.actions.push(Action {
                time: e.time_ms,
                frame: e.frame_index,
                object: e.object_index,
                add: false,
            });
        }
    }

    pub(super) fn finish(mut self, events: &[OrderedEvent]) -> Option<StableCombo> {
        let total: u64 = self.lanes.iter().map(|l| l.total).sum();
        if !self.valid
            || self.active.iter().any(Option::is_some)
            || total + events.len() as u64 > u32::MAX as u64
        {
            return None;
        }
        self.actions.extend(events.iter().map(|e| Action {
            time: e.event.time_ms,
            frame: e.input_order,
            object: e.event.object_index,
            add: e.event.result != ResultKind::Miss,
        }));
        self.actions.sort_by(|a, b| {
            a.time
                .total_cmp(&b.time)
                .then(a.frame.cmp(&b.frame))
                .then(a.object.cmp(&b.object))
        });
        let mut combo = StableCombo {
            lanes: self.lanes,
            checkpoints: Vec::with_capacity(self.actions.len()),
        };
        let mut c = Checkpoint::default();
        for action in self.actions {
            c.time = action.time;
            let earned = combo.ticks_at(c.time) + c.added;
            c.maximum = c.maximum.max(earned - c.reset_at);
            if action.add {
                c.added += 1;
                c.maximum = c.maximum.max(earned + 1 - c.reset_at);
            } else {
                c.reset_at = earned;
            }
            combo.checkpoints.push(c);
        }
        Some(combo)
    }
}
