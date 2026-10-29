use super::*;
use iced::widget::column;

impl State {
    pub(super) fn observe_training(&mut self, found: desk::Job) -> Vec<Effect> {
        if self.training_seen.as_deref() == Some(&found.job) { return Vec::new(); }
        if self.training.as_ref().and_then(|active| active.status.as_ref()).is_some_and(|old| old.job == found.job && old.revision > found.revision) { return Vec::new(); }
        if !matches!(found.state.as_str(), "running" | "done" | "nothing_new" | "failed" | "interrupted") { return Vec::new(); }
        if self.training.as_ref().and_then(|active| active.status.as_ref()).is_some_and(|old| {
            old.job != found.job && old.started_at.as_ref().zip(found.started_at.as_ref()).is_some_and(|(old, new)| old >= new)
        }) { return Vec::new(); }
        if found.state == "running" {
            if self.training.as_ref().and_then(|active| active.job.as_ref()) != Some(&found.job) {
                let mut active = Training::new(self.desk.versions.len());
                active.job = Some(found.job.clone());
                self.training = Some(active);
                self.training_selected = true;
            }
            self.training_seen = None;
            let active = self.training.as_mut().unwrap();
            active.status = Some(found);
            active.disconnected = false;
            active.received = std::time::Instant::now();
            self.training_failed = None;
            self.training_done = None;
            self.training_lost = false;
            self.nothing_new = false;
            return Vec::new();
        }
        self.training_done = None;
        self.training_failed = None;
        self.training_lost = false;
        self.nothing_new = false;
        self.training_seen = Some(found.job);
        self.training = None;
        self.training_selected = false;
        match found.state.as_str() {
            "done" => {
                self.training_done = found.version.clone();
                self.version = found.version;
                self.loading = true;
                vec![Effect::Load]
            }
            "nothing_new" => { self.nothing_new = true; Vec::new() }
            "failed" => { self.training_failed = Some(found.error.unwrap_or_default()); Vec::new() }
            "interrupted" => { self.training_lost = true; Vec::new() }
            _ => Vec::new(),
        }
    }
}

fn fraction(active: &Training) -> f32 {
    active.status.as_ref().filter(|job| job.total > 0).map(|job| (job.completed as f64 / job.total as f64).clamp(0.0, 1.0) as f32).unwrap_or(0.0)
}

fn phase(w: &Words, active: &Training) -> String {
    let key = match active.status.as_ref().map(|job| job.phase.as_str()) {
        Some("gathering") => "desk-training-gathering",
        Some("validation") => "desk-training-validating",
        Some("fitting") => "desk-training-fitting",
        Some("saving") => "desk-training-saving",
        _ => "desk-training-queued",
    };
    w.t(key)
}

pub(super) fn card<'a>(w: &Words, active: &Training, selected: bool, k: f32) -> Element<'a, Message> {
    let amount = active.status.as_ref().filter(|job| job.total > 0).map(|_| format!("{}%", (fraction(active) * 100.0).floor() as u32)).unwrap_or_else(|| "—".into());
    let face = container(column![
        row![strong(w.t("desk-training-new"), 16.0 * k, INK), Space::new().width(Length::Fill), strong(amount, 14.0 * k, INK)].spacing(8),
        words(if active.disconnected { w.t("desk-training-waiting-link") } else { phase(w, active) }, 13.0 * k, MUTED),
    ].spacing(6)).padding([12.0 * k, 14.0 * k]).width(Length::Fill).height(84.0 * k);
    let progress = iced::widget::canvas(ui::Bar { fraction: fraction(active), alpha: 1.0 }).width(Length::Fill).height(Length::Fill);
    container(button(stack![progress, face].width(Length::Fill).height(84.0 * k))
        .padding(0).style(quiet(selected, theme::CONTROL_RADIUS)).on_press(Message::TrainingFocus)).id("desk-training-card").width(Length::Fill)
        .into()
}

pub(super) fn detail<'a>(w: &Words, active: &Training, k: f32) -> Element<'a, Message> {
    let elapsed = active.status.as_ref().map(|job| job.elapsed_seconds + if active.disconnected { 0 } else { active.received.elapsed().as_secs() }).unwrap_or(active.since.elapsed().as_secs());
    let mut content = column![
        strong(w.t("desk-training-new"), 26.0 * k, INK),
        strong(phase(w, active), 18.0 * k, INK),
        words(w.with("desk-training-elapsed", &[("time", format!("{}:{:02}", elapsed / 60, elapsed % 60))]), 14.0 * k, MUTED),
    ].spacing(16);
    if let Some(job) = &active.status {
        if job.total > 0 {
            content = content.push(bar(fraction(active), ACCENT)).push(words(w.with("desk-training-steps", &[("done", job.completed.to_string()), ("total", job.total.to_string())]), 14.0 * k, MUTED));
        }
        if job.maps > 0 {
            content = content.push(words(w.with("desk-training-maps", &[("count", job.maps.to_string())]), 14.0 * k, MUTED));
        }
        if job.checks_total > 0 {
            content = content.push(words(w.with("desk-training-checks", &[("done", job.checks_done.to_string()), ("total", job.checks_total.to_string())]), 14.0 * k, MUTED));
        }
        if job.phase == "validation" {
            if let Some(fold) = &job.fold {
                let source = if fold == "queue" { w.t("desk-tab-queue") } else { fold.clone() };
                content = content.push(words(w.with("desk-training-fold", &[("source", source)]), 14.0 * k, MUTED));
            }
        }
    }
    content = content.push(words(w.t(if active.disconnected { "desk-training-reconnecting" } else { "desk-training-server" }), 14.0 * k, if active.disconnected { UNSURE } else { MUTED }))
        .push(words(w.t("desk-training-shadow"), 13.0 * k, FAINT));
    container(scrollable(content).direction(ui::hidden_bar())).id("desk-training-detail").padding(24).width(Length::Fill).height(Length::Fill).style(workspace::surface).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(state: &str, completed: u64, revision: u64) -> desk::Job {
        desk::Job {
            job: "current".into(), state: state.into(), phase: "fitting".into(),
            total: 100, completed, revision, started_at: Some("2026-10-29T12:00:00".into()),
            version: (state == "done").then(|| "kamerton-42".into()), ..Default::default()
        }
    }

    fn presence(job: Option<desk::Job>) -> Message {
        Message::Presence(Ok(desk::PresenceUpdate { training_known: true, training: job, ..Default::default() }))
    }

    #[test]
    fn reopening_recovers_progress_and_ignores_old_responses() {
        let w = Words::new(crate::lang::Lang::Ru);
        let mut state = State::default();
        state.update(presence(Some(job("running", 35, 5))), &w);
        assert!(state.training_selected);
        assert_eq!(fraction(state.training.as_ref().unwrap()), 0.35);
        state.update(Message::Presence(Err("offline".into())), &w);
        let active = state.training.as_ref().unwrap();
        assert!(active.disconnected && fraction(active) == 0.35);
        state.update(presence(Some(job("running", 60, 8))), &w);
        state.update(Message::TrainingState("current".into(), Ok(job("running", 40, 6))), &w);
        let active = state.training.as_ref().unwrap();
        assert!(!active.disconnected && fraction(active) == 0.6);
        assert_eq!(state.update(presence(Some(job("done", 100, 10))), &w), [Effect::Load]);
        assert!(state.training.is_none());
        assert_eq!(state.training_done.as_deref(), Some("kamerton-42"));
        assert!(state.update(presence(Some(job("done", 100, 10))), &w).is_empty());
        state.update(presence(Some(job("running", 60, 8))), &w);
        assert!(state.training.is_none(), "a delayed response cannot revive a completed job");
    }

    #[test]
    fn old_jobs_cannot_replace_a_newer_training() {
        let w = Words::new(crate::lang::Lang::Ru);
        let mut state = State::default();
        state.update(presence(Some(job("running", 35, 5))), &w);
        let mut old = job("done", 100, 50);
        old.job = "older".into();
        old.started_at = Some("2026-10-29T11:00:00".into());
        state.update(presence(Some(old)), &w);
        assert_eq!(state.training.as_ref().unwrap().job.as_deref(), Some("current"));
        assert!(state.training_done.is_none());
    }

    #[test]
    fn lost_start_response_recovers_without_starting_a_second_job() {
        let w = Words::new(crate::lang::Lang::Ru);
        let mut state = State::default();
        state.update(Message::Train, &w);
        state.update(Message::TrainStarted(Err("connection reset".into())), &w);
        assert!(state.training.as_ref().unwrap().disconnected);
        assert!(state.update(Message::Train, &w).is_empty());
        state.update(presence(Some(job("running", 35, 5))), &w);
        assert_eq!(state.training.as_ref().unwrap().job.as_deref(), Some("current"));
        assert!(!state.training.as_ref().unwrap().disconnected);
        state.update(Message::TrainingState("current".into(), Err("UNKNOWN_JOB".into())), &w);
        assert!(state.training.is_none() && state.training_lost);
    }

    #[test]
    fn a_request_that_never_started_can_be_retried_after_reconnecting() {
        let w = Words::new(crate::lang::Lang::Ru);
        let mut state = State::default();
        state.update(Message::Train, &w);
        state.update(Message::TrainStarted(Err("offline".into())), &w);
        state.update(presence(None), &w);
        assert!(state.training.is_none() && state.training_failed.is_some());
        assert!(!state.update(Message::Train, &w).is_empty());
    }

    #[test]
    fn a_server_refusal_is_shown_without_waiting_for_reconnection() {
        let w = Words::new(crate::lang::Lang::Ru);
        let mut state = State::default();
        state.update(Message::Train, &w);
        state.update(Message::TrainRefused("FORBIDDEN".into()), &w);
        assert!(!state.posting && state.training.is_none());
        assert_eq!(state.training_failed.as_deref(), Some("FORBIDDEN"));
    }

    #[test]
    fn selecting_a_saved_version_keeps_progress_without_switching_back() {
        let w = Words::new(crate::lang::Lang::Ru);
        let mut state = State::default();
        state.update(presence(Some(job("running", 35, 5))), &w);
        state.update(Message::Version("kamerton-1".into()), &w);
        state.update(presence(Some(job("running", 40, 6))), &w);
        assert!(!state.training_selected && state.training.is_some());
        state.update(Message::TrainingFocus, &w);
        assert!(state.training_selected);
    }
}
