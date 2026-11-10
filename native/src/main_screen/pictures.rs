use super::*;

impl Main {
    pub(super) fn news_pictures_task(&mut self) -> Task<Message> {
        let mut wanted: Vec<(String, (u32, u32))> = Vec::new();
        let stories = self.news.stories.iter().take(4).filter_map(|story| story.image.clone()).map(|url| (url, (192, 108)));
        let posts = self.news.posts.iter().take(12).filter_map(|post| post.cover().map(str::to_owned)).map(|url| (url, (128, 128)));
        let candidates: Vec<_> = stories.chain(posts).collect();
        self.kept_pictures.news = candidates.iter().map(|(url, _)| url.clone()).collect();
        for (url, size) in candidates {
            if self.ask_picture(&url, Instant::now()) {
                wanted.push((url, size));
            }
        }
        if wanted.is_empty() {
            return Task::none();
        }
        let mut lanes: Vec<Vec<(String, (u32, u32))>> = vec![Vec::new(); PICTURE_LANES];
        for (at, one) in wanted.into_iter().enumerate() {
            lanes[at % PICTURE_LANES].push(one);
        }
        Task::batch(lanes.into_iter().filter(|lane| !lane.is_empty()).map(|lane| {
            ui::streamed(move |push| {
                for (url, (w, h)) in lane {
                    let handle = crate::news::picture(&url).and_then(|bytes| covered_bytes(&bytes, w, h));
                    if !push(Message::NewsPicture(url, handle)) {
                        return;
                    }
                }
            })
        }))
    }

    pub(super) fn ask_picture(&mut self, url: &str, now: Instant) -> bool {
        if url.is_empty() || self.news_pictures.contains_key(url) { return false; }
        if let Some(failed) = self.news_picture_failures.get(url) {
            if now.saturating_duration_since(*failed) < Duration::from_secs(30) { return false; }
            self.news_picture_failures.remove(url);
            self.news_asked.remove(url);
        }
        self.news_asked.insert(url.to_owned())
    }

    pub(super) fn cache_news_picture(&mut self, url: String, handle: image::Handle) {
        let old_bytes = self.news_pictures.get(&url).map_or(0, image_bytes);
        let new_bytes = image_bytes(&handle);
        if old_bytes == new_bytes && self.news_pictures.contains_key(&url) {
            self.news_pictures.insert(url.clone(), handle);
            self.news_asked.remove(&url);
            return;
        }
        if !self.news_pictures.contains_key(&url) {
            self.news_picture_order.push_back(url.clone());
        }
        self.news_picture_bytes = self.news_picture_bytes.saturating_sub(old_bytes).saturating_add(new_bytes);
        self.news_pictures.insert(url.clone(), handle);
        self.news_asked.remove(&url);

        let mut skipped = 0;
        while self.news_picture_bytes > NEWS_PICTURE_CACHE_BYTES && skipped < self.news_picture_order.len() {
            let Some(oldest) = self.news_picture_order.pop_front() else { break; };
            if oldest == url || self.kept_pictures.holds(&oldest) || self.pictures_seen.borrow().lately(&oldest, Instant::now(), PICTURE_HELD_FOR) {
                self.news_picture_order.push_back(oldest);
                skipped += 1;
                continue;
            }
            if let Some(removed) = self.news_pictures.remove(&oldest) {
                self.news_picture_bytes = self.news_picture_bytes.saturating_sub(image_bytes(&removed));
                self.news_asked.remove(&oldest);
                self.news_frosts.remove(&oldest);
                self.moving_pictures.remove(&oldest);
                self.moving_order.retain(|known| known != &oldest);
            }
            skipped = 0;
        }
    }

    pub(super) fn pictures_task(&mut self, wanted: Vec<(String, u32)>) -> Task<Message> {
        for (url, side) in &wanted {
            self.picture_sides.entry(url.clone()).or_insert(*side);
        }
        let wanted: Vec<(String, u32)> = wanted.into_iter().filter(|(url, _)| self.ask_picture(url, Instant::now())).collect();
        if wanted.is_empty() {
            return Task::none();
        }
        ui::streamed(move |push| {
            for (url, side) in wanted {
                let bytes = crate::news::picture(crate::community::fetched(&url));
                if let Some(frost) = bytes.as_deref().filter(|_| side == crate::community::COVER).and_then(crate::community_screen::Frost::of) {
                    if !push(Message::NewsFrost(url.clone(), frost)) {
                        return;
                    }
                }
                let handle = bytes.and_then(|bytes| match side {
                    crate::community::BACKDROP => backdrop_bytes(&bytes, side),
                    crate::community::POSTER => poster_bytes(&bytes),
                    side if side <= 256 => covered_bytes(&bytes, side, side),
                    side => fitted_bytes(&bytes, side),
                });
                if !push(Message::NewsPicture(url, handle)) {
                    return;
                }
            }
        })
    }

    pub(super) fn community_pictures_task(&mut self) -> Task<Message> {
        let Some(catalog) = self.community.as_ref() else {
            return Task::none();
        };
        let mut known: Vec<(String, u32)> = catalog.pictures_with(usize::MAX);
        for dossier in self.people_dossiers.values() {
            if !dossier.person.avatar.is_empty() { known.push((dossier.person.avatar.clone(), 256)); }
            if !dossier.person.cover.is_empty() { known.push((dossier.person.cover.clone(), crate::community::COVER)); }
        }
        if let Some(card) = self.shown_card.clone().or_else(|| catalog.card_of()) {
            known.extend(card.pictures());
        }
        let mut front: Vec<(String, u32)> = catalog.front_pictures(self.feed_rows);
        let mut sharing: Vec<&str> = self.entries().iter().filter(|entry| crate::mixed::is_shared(&entry.path)).map(|entry| entry.player.as_str()).collect();
        sharing.sort_unstable();
        sharing.dedup();
        front.extend(sharing.into_iter().filter_map(|player| self.shared_avatar(player)).map(|avatar| (avatar.to_owned(), 128)));
        for (url, side) in known.into_iter().chain(front.iter().cloned()) {
            self.picture_sides.entry(url).or_insert(side);
        }
        self.kept_pictures.community = front.iter().map(|(url, _)| url.clone()).collect();
        let flag = self.flag_task();
        Task::batch([self.fetch_pictures(front), flag])
    }

    pub fn shelf(&self) -> crate::community_screen::Shelf<'_> {
        if self.gallery {
            return crate::community_screen::Shelf::plain(&self.news_pictures);
        }
        crate::community_screen::Shelf::watched(&self.news_pictures, crate::community_screen::Watch { seen: &self.pictures_seen, known: &self.picture_sides, asked: &self.news_asked, now: Instant::now() })
    }

    pub(super) fn missed_pictures_task(&mut self) -> Option<Task<Message>> {
        let missed: Vec<String> = {
            let mut seen = self.pictures_seen.borrow_mut();
            if seen.at.len() > PICTURES_SEEN_MOST {
                seen.forget_before(Instant::now(), PICTURE_HELD_FOR);
            }
            if seen.missed.is_empty() {
                return None;
            }
            seen.missed.drain().collect()
        };
        let wanted: Vec<(String, u32)> = missed.into_iter().filter_map(|url| self.picture_sides.get(&url).map(|side| (url, *side))).collect();
        Some(self.fetch_pictures(wanted))
    }

    pub(super) fn missed_heroes_task(&mut self) -> Option<Task<Message>> {
        if self.gallery {
            return None;
        }
        let missed = self.desk.heroes_missed();
        if missed.is_empty() {
            return None;
        }
        Some(Task::batch(missed.into_iter().map(|set| {
            ui::in_thread(move || {
                let url = format!("https://assets.ppy.sh/beatmaps/{set}/covers/cover@2x.jpg");
                let handle = crate::news::picture(&url).and_then(|bytes| decoded_from(&bytes, POOL_COVER.0, Some(POOL_COVER)));
                Message::Community(crate::community_screen::Message::Desk(crate::desk_screen::Message::Hero(set, handle)))
            })
        })))
    }

    pub(super) fn fetch_pictures(&mut self, wanted: Vec<(String, u32)>) -> Task<Message> {
        let wanted: Vec<(String, u32)> = wanted.into_iter().filter(|(url, _)| self.ask_picture(url, Instant::now())).collect();
        if wanted.is_empty() {
            return Task::none();
        }
        let mut wanted = wanted;
        wanted.sort_by_key(|(_, side)| match *side {
            side if side <= 256 => 0,
            crate::community::POSTER => 1,
            crate::community::BACKDROP => 3,
            _ => 2,
        });
        let mut lanes: Vec<Vec<(String, u32)>> = vec![Vec::new(); PICTURE_LANES];
        for (at, one) in wanted.into_iter().enumerate() {
            lanes[at % PICTURE_LANES].push(one);
        }
        let pictures = lanes.into_iter().filter(|lane| !lane.is_empty()).map(|lane| {
            ui::streamed(move |push| {
                for (url, side) in lane {
                    let bytes = crate::news::picture(crate::community::fetched(&url));
                    if let Some(frost) = bytes.as_deref().filter(|_| side == crate::community::COVER).and_then(crate::community_screen::Frost::of) {
                        if !push(Message::NewsFrost(url.clone(), frost)) {
                            return;
                        }
                    }
                    if let Some(moving) = bytes.as_deref().filter(|_| side <= 256).and_then(|bytes| moving_bytes(bytes, side)) {
                        if !push(Message::NewsMoving(url.clone(), moving)) {
                            return;
                        }
                    }
                    let handle = bytes.and_then(|bytes| match side {
                        crate::community::BACKDROP => backdrop_bytes(&bytes, side),
                        crate::community::POSTER => poster_bytes(&bytes),
                        side if side <= 256 => covered_bytes(&bytes, side, side),
                        side => fitted_bytes(&bytes, side),
                    });
                    if !push(Message::NewsPicture(url, handle)) {
                        return;
                    }
                }
            })
        });
        Task::batch(pictures)
    }

    pub(super) fn wide_pictures_task(&mut self, urls: Vec<String>) -> Task<Message> {
        self.kept_pictures.wide = urls.iter().map(|url| crate::community_screen::wide(url)).collect();
        let wanted: Vec<String> = urls.into_iter().filter(|url| self.ask_picture(&crate::community_screen::wide(url), Instant::now())).collect();
        if wanted.is_empty() {
            return Task::none();
        }
        ui::streamed(move |push| {
            for url in wanted {
                let handle = crate::news::picture(&url).and_then(|bytes| fitted_bytes(&bytes, 1400));
                if !push(Message::NewsPicture(crate::community_screen::wide(&url), handle)) {
                    return;
                }
            }
        })
    }
}
