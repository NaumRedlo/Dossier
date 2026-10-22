use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dossier-pools-screen-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn map(title: &str, artist: &str, version: &str) -> Map {
    Map { file: PathBuf::from(format!("/songs/{title}.osu")), artist: artist.to_owned(), title: title.to_owned(), version: version.to_owned(), background: Some(PathBuf::from(format!("/songs/{title}.jpg"))) }
}

fn state_with_songs(name: &str) -> State {
    let mut state = State::new(scratch(name));
    let mut songs = HashMap::new();
    for (hash, title, artist, version) in [("a1", "Glass Orchard", "Nova Tide", "Garden"), ("b2", "Salt and Static", "Marrow", "Another"), ("c3", "Ninth Window", "Kite and Ash", "Hard"), ("d4", "Glass Harbour", "Nova Tide", "Expert")] {
        songs.insert(hash.to_owned(), map(title, artist, version));
    }
    state.songs = Some(Arc::new(songs));
    state.loaded = true;
    state
}

fn measure(stars: f64) -> Measure {
    Measure { stars, bpm: 180.0, length_ms: 120_000, ar: 9.0, od: 8.0, cs: 4.0, hp: 5.0, max_combo: 700, aim: 2.0, speed: 2.0, reading: 0.5, stamina: 1.0 }
}

fn open_new(state: &mut State) -> String {
    state.update(Message::New, 1_790_000_000);
    state.update(Message::UseFrame(Frame::Duel), 1_790_000_000);
    state.update(Message::Begin(Start::Empty), 1_790_000_000);
    state.editor_mut().unwrap().grouped = false;
    state.editing().expect("an editor").id.clone()
}

#[test]
fn regression_failed_saves_keep_edits_and_retry_the_latest_version() {
    let mut state = state_with_songs("save-retry");
    let id = open_new(&mut state);
    let before = pools::load_all(&state.dir)[0].clone();
    let staging = state.dir.join(format!("{id}.pool.part"));
    std::fs::create_dir(&staging).unwrap();
    state.update(Message::Rename("Changed".into()), 101);
    assert!(!state.flush_saves(true));
    assert!(state.save_errors.contains_key(&id));
    assert_eq!(pools::load_all(&state.dir)[0].name, before.name);
    state.update(Message::Back, 102);
    assert!(matches!(state.screen, Screen::Editor(_)));
    state.update(Message::Rename("Latest".into()), 103);
    std::fs::remove_dir(&staging).unwrap();
    state.update(Message::RetrySaves, 104);
    assert!(!state.has_unsaved() && state.save_errors.is_empty());
    assert_eq!(pools::load_all(&state.dir)[0].name, "Latest");
    let _ = std::fs::remove_dir_all(&state.dir);
}

#[test]
fn regression_typing_is_coalesced_and_leaving_flushes_it() {
    let mut state = state_with_songs("save-debounce");
    open_new(&mut state);
    let before = pools::load_all(&state.dir)[0].clone();
    for name in ["N", "Ne", "New name"] { state.update(Message::Rename(name.into()), 101); }
    state.update(Message::Note(0, "Latest note".into()), 102);
    state.update(Message::FlushSaves, 103);
    assert_eq!(pools::load_all(&state.dir)[0].name, before.name);
    assert!(state.has_unsaved());
    state.update(Message::Back, 104);
    let kept = pools::load_all(&state.dir).remove(0);
    assert_eq!(kept.name, "New name");
    assert_eq!(kept.slots[0].note, "Latest note");
    assert!(!state.has_unsaved());
    let _ = std::fs::remove_dir_all(&state.dir);
}

#[test]
fn a_new_pool_starts_with_four_free_slots_and_grows_without_a_preset() {
    let mut state = state_with_songs("free-default");
    state.update(Message::New, 100);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.grouped));
    assert_eq!(state.editing().unwrap().frame, Frame::Free);
    assert_eq!(state.editing().unwrap().slots.len(), 4);
    assert!(state.editing().unwrap().slots.iter().all(|slot| slot.is_empty() && slot.mods == Mod::Nm));
    state.update(Message::Put("a1".into()), 101);
    state.update(Message::Put("b2".into()), 102);
    assert_eq!(state.editing().unwrap().slots.len(), 4);
    state.update(Message::Clear(0), 103);
    assert_eq!(state.editing().unwrap().slots.len(), 3);
    assert_eq!(pools::load_all(&state.dir)[0].slots[0].hash.as_deref(), Some("b2"));
}

#[test]
fn adding_a_map_uses_a_free_slot_and_keeps_its_category() {
    let mut state = state_with_songs("add-placeholder");
    state.update(Message::New, 100);
    state.update(Message::AddPanel(true), 101);
    assert_eq!(state.editing().unwrap().slots.len(), 4);
    let Screen::Editor(editor) = &state.screen else { panic!("editor") };
    assert_eq!(editor.selected, Some(0));
    assert_eq!(editor.panel, Panel::Add);
    state.update(Message::SetMod(0, Mod::Hr), 102);
    state.update(Message::AddPanel(false), 103);
    state.update(Message::AddPanel(true), 104);
    assert_eq!(state.editing().unwrap().slots.len(), 4);
    state.update(Message::Put("a1".into()), 105);
    assert_eq!(state.editing().unwrap().slots[0].mods, Mod::Hr);
    assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("a1"));
    state.update(Message::AddPanel(true), 106);
    assert_eq!(state.editing().unwrap().slots.len(), 4);
    assert!(state.editing().unwrap().slots[1].is_empty());
    assert_eq!(pools::load_all(&state.dir)[0].slots, state.editing().unwrap().slots);
}

#[test]
fn adding_and_pasting_keep_the_last_category_but_explicit_slots_keep_their_own() {
    let mut state = state_with_songs("remember-category");
    state.update(Message::New, 100);
    state.update(Message::AddPanel(true), 101);
    state.update(Message::SetMod(0, Mod::Hd), 102);
    state.update(Message::Put("a1".into()), 103);
    state.update(Message::AddPanel(true), 104);
    assert_eq!(state.editing().unwrap().slots[1].mods, Mod::Hd);
    state.update(Message::Put("b2".into()), 105);
    state.update(Message::Put("c3".into()), 106);
    assert_eq!(state.editing().unwrap().slots[2].mods, Mod::Hd);
    state.update(Message::Select(Some(3)), 107);
    state.update(Message::Put("d4".into()), 108);
    assert_eq!(state.editing().unwrap().slots[3].mods, Mod::Nm);
    state.update(Message::Pasted(LINK.into()), 109);
    state.update(Message::AddMod(Mod::Hr), 110);
    state.update(Message::Resolved(state.resolve_request, Ok(found(&["e5"], Some(0)))), 111);
    state.place_map("e5", map("Fifth", "Artist", "Hard"), Some(state.default_place()), 112);
    assert_eq!(state.editing().unwrap().slots[4].mods, Mod::Hr);
    state.update(Message::AddPanel(true), 113);
    assert_eq!(state.editing().unwrap().slots[5].mods, Mod::Hr);
}

#[test]
fn authors_and_custom_colours_are_saved_and_follow_the_next_map() {
    let mut state = state_with_songs("custom-category-authors");
    state.update(Message::New, 100);
    state.update(Message::Authors("Alice, Bob, alice".into()), 101);
    state.update(Message::AddPanel(true), 102);
    state.update(Message::Category(0, "Aim".into()), 103);
    state.update(Message::Colour(0, "#123456".into()), 104);
    state.update(Message::Put("a1".into()), 105);
    state.update(Message::AddPanel(true), 106);
    state.update(Message::Put("b2".into()), 107);
    let pool = state.editing().unwrap();
    assert_eq!(pool.authors, ["Alice", "Bob"]);
    assert_eq!(pool.slots[1].category, "Aim");
    assert_eq!(pool.slots[1].colour, Some([0x12, 0x34, 0x56]));
    state.update(Message::Colour(1, "#abcdef".into()), 108);
    state.update(Message::Colour(1, "#xx".into()), 109);
    let saved = pools::load_all(&state.dir).remove(0);
    assert_eq!(saved.authors, ["Alice", "Bob"]);
    assert!(saved.slots[..2].iter().all(|slot| slot.colour == Some([0xab, 0xcd, 0xef])));
    let id = saved.id;
    state.update(Message::Back, 110);
    state.update(Message::Open(id), 111);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.authors == "Alice Bob" && editor.grouped));
}

#[test]
fn author_spaces_and_category_creation_preserve_named_profiles_and_definitions() {
    let mut state = state_with_songs("category-creation");
    state.update(Message::New, 100);
    state.update(Message::Authors("Alice \"Mapper One\" Bob alice".into()), 101);
    assert_eq!(state.editing().unwrap().authors, ["Alice", "Mapper One", "Bob"]);
    state.update(Message::CreateCategory(true), 102);
    state.update(Message::CategoryDraft("aim 123".into()), 103);
    state.update(Message::Colour(0, "#abcdef".into()), 104);
    assert!(state.editing().unwrap().slots[0].category.is_empty());
    state.update(Message::SaveCategory(0), 105);
    assert_eq!(state.editing().unwrap().slots[0].category, "AI");
    assert_eq!(state.editing().unwrap().slots[0].colour, Some([171, 205, 239]));
    state.update(Message::SetMod(0, Mod::Hr), 106);
    assert_eq!(state.editing().unwrap().categories["AI"], [171, 205, 239]);
    state.update(Message::Category(1, "AI".into()), 107);
    assert_eq!(state.editing().unwrap().slots[1].colour, Some([171, 205, 239]));
    let saved = pools::load_all(&state.dir).remove(0);
    assert_eq!(saved.categories["AI"], [171, 205, 239]);
    let shared = pool_share::from_file(&pool_share::to_file(&saved), 108).unwrap();
    assert_eq!(shared.categories, saved.categories);
}

#[test]
fn collection_publication_keeps_its_kind_and_failed_saves_keep_the_draft() {
    let mut state = state_with_songs("collection-publication");
    state.collections = Some(vec![Collection { name: "Favorites".into(), hashes: vec!["a1".into(), "b2".into()] }]);
    state.update(Message::ImportCollection(0), 100);
    let id = state.editing().unwrap().id.clone();
    assert!(state.editing().unwrap().collection);
    assert!(state.update(Message::Publish, 101).is_empty(), "nobody is signed in yet");
    state.update(Message::OwnCompiler(id.clone(), "NaumRedlo".into()), 101);
    let actions = state.update(Message::Publish, 101);
    assert!(matches!(actions.as_slice(), [Effect::Publish(pool)] if pool.collection && pool.filled() == 2));
    assert!(state.update(Message::Publish, 102).is_empty());
    state.update(Message::Published(id.clone(), Err("503".into())), 103);
    assert!(!state.publishing);
    assert_eq!(state.editing().unwrap().filled(), 2);
    assert_eq!(state.catalogue_error.as_deref(), Some("503"));
    let shared = pool_share::from_file(&pool_share::to_file(state.editing().unwrap()), 104).unwrap();
    assert!(shared.collection);
}

#[test]
fn categories_start_in_tournament_order_and_manual_order_survives_sharing() {
    let mut state = state_with_songs("tournament-order");
    state.update(Message::New, 100);
    state.list[0].slots = Mod::ALL.iter().map(|mods| Slot::empty(*mods)).collect();
    assert_eq!(category_order(&state.list[0]), ["NM", "HD", "HR", "DT", "FM", "TB"]);
    state.update(Message::MoveCategory("DT".into(), false), 101);
    assert_eq!(category_order(&state.list[0]), ["NM", "HD", "DT", "HR", "FM", "TB"]);
    let saved = pool_share::from_file(&pool_share::to_file(&state.list[0]), 102).unwrap();
    assert_eq!(category_order(&saved), category_order(&state.list[0]));
}

#[test]
fn author_names_apply_only_after_confirmation() {
    let mut state = state_with_songs("confirmed-credits");
    state.update(Message::New, 100);
    state.update(Message::AuthorsDraft("Alice Bob".into()), 101);
    assert!(state.editing().unwrap().authors.is_empty());
    let effects = state.update(Message::ApplyAuthors, 102);
    assert_eq!(state.editing().unwrap().authors, ["Alice", "Bob"]);
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Faces(names) if names == &["Alice", "Bob"])));
}

#[test]
fn the_publisher_is_whoever_is_signed_in_and_is_asked_for_again_each_time_the_pool_is_opened() {
    let mut state = state_with_songs("pinned-publisher");
    let effects = state.update(Message::New, 100);
    let id = state.editing().unwrap().id.clone();
    assert!(effects.iter().any(|effect| matches!(effect, Effect::OwnCompiler(asked) if asked == &id)));
    let effects = state.update(Message::OwnCompiler(id.clone(), "Mapper One".into()), 101);
    assert_eq!(state.editing().unwrap().compiler, "Mapper One");
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Faces(names) if names == &["Mapper One"])));
    assert!(state.update(Message::OwnCompiler(id.clone(), String::new()), 102).is_empty());
    assert_eq!(state.editing().unwrap().compiler, "Mapper One");
    assert!(state.update(Message::OwnCompiler(id.clone(), "Mapper One".into()), 103).is_empty());
    state.update(Message::Select(None), 104);
    state.update(Message::Put("a1".into()), 104);
    state.update(Message::Back, 104);
    let effects = state.update(Message::Open(id.clone()), 105);
    assert!(effects.iter().any(|effect| matches!(effect, Effect::OwnCompiler(asked) if asked == &id)));
    state.update(Message::OwnCompiler(id.clone(), "Another Account".into()), 106);
    assert_eq!(state.editing().unwrap().compiler, "Another Account");
    let shared = pool_share::from_file(&pool_share::to_file(state.editing().unwrap()), 107).unwrap();
    assert_eq!(shared.compiler, "Another Account");
}

#[test]
fn a_collection_names_only_its_publisher_and_nobody_can_retype_that_name() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let shown = |collection: bool| {
        let mut state = pool_with_maps(if collection { "collection-credits" } else { "pool-credits" }, &["a1", "b2"]);
        state.editor_mut().unwrap().panel = Panel::Closed;
        let at = state.editing_at().unwrap();
        state.list[at].collection = collection;
        state.list[at].authors = vec!["Alice".into()];
        state.list[at].compiler = "Builder".into();
        state.editor_mut().unwrap().editing_authors = false;
        state
    };
    let collection = shown(true);
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1200.0, 900.0), view(&collection, &words, &thumbs, 1200.0, 1.0, &Clocks::settled()));
    assert!(screen.find(words.t("pool-mappoolers")).is_err());
    assert!(screen.find("Alice").is_err());
    assert!(screen.find(words.t("pool-publisher")).is_ok());
    screen.click("Builder").unwrap();
    assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Author(name)] if name == "Builder"));
    let pool = shown(false);
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1200.0, 900.0), view(&pool, &words, &thumbs, 1200.0, 1.0, &Clocks::settled()));
    assert!(screen.find(words.t("pool-mappoolers")).is_ok());
    assert!(screen.find("Alice").is_ok());
    assert!(screen.find("Builder").is_err(), "a pool names its publisher on the publication sheet, not in the head");
    let mut signed_out = shown(true);
    let at = signed_out.editing_at().unwrap();
    signed_out.list[at].compiler.clear();
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1200.0, 900.0), view(&signed_out, &words, &thumbs, 1200.0, 1.0, &Clocks::settled()));
    assert!(screen.find(words.t("pool-publisher")).is_err());
}

#[test]
fn a_space_confirms_names_and_the_shelf_requests_their_avatars() {
    let mut state = state_with_songs("space-credits");
    state.update(Message::New, 100);
    let effects = state.update(Message::AuthorsDraft("Alice ".into()), 101);
    assert_eq!(state.editing().unwrap().authors, ["Alice"]);
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Faces(names) if names == &["Alice"])));
    state.update(Message::EditAuthors, 102);
    state.update(Message::AuthorsDraft("Alice \"Mapper ".into()), 103);
    assert_eq!(state.editing().unwrap().authors, ["Alice"]);
    state.update(Message::AuthorsDraft("Alice \"Mapper One\" ".into()), 104);
    assert_eq!(state.editing().unwrap().authors, ["Alice", "Mapper One"]);
    let id = state.editing().unwrap().id.clone();
    state.update(Message::OwnCompiler(id, "Builder".into()), 105);
    assert_eq!(state.editing().unwrap().compiler, "Builder");
    let effects = state.open();
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Faces(names) if names.contains(&"Alice".into()) && names.contains(&"Builder".into()))));
}

fn shown(state: &State) -> &Editor {
    match &state.screen {
        Screen::Editor(editor) => editor,
        _ => panic!("the editor is not open"),
    }
}

fn picture(state: &State, words: &Words, size: iced::Size, pointer: iced::Point, clocks: &Clocks, name: &str) -> ::image::RgbaImage {
    let thumbs = HashMap::new();
    let mut screen = iced_test::Simulator::with_size(crate::settings(), size, view(state, words, &thumbs, size.width, 1.0, clocks));
    screen.point_at(pointer);
    let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved { position: pointer })]);
    let shot = screen.snapshot(&crate::theme::theme()).unwrap();
    let file = crate::gallery::write_snapshot(&shot, &scratch(&format!("picture-{name}")).join(name)).unwrap();
    ::image::open(file).unwrap().to_rgba8()
}

#[test]
fn tiles_behind_an_open_panel_do_not_answer_the_pointer_and_answer_it_again_once_it_is_closed() {
    let words = Words::new(crate::lang::Lang::En);
    let size = iced::Size::new(1900.0, 900.0);
    let mut state = pool_with_maps("veil-hover", &["a1", "b2", "c3"]);
    state.update(Message::Select(Some(0)), 1_790_000_100);
    let thumbs = HashMap::new();
    let mut screen = iced_test::Simulator::with_size(crate::settings(), size, view(&state, &words, &thumbs, size.width, 1.0, &Clocks::settled()));
    let found = screen.find("Glass Orchard").unwrap().visible_bounds().unwrap();
    let tile = iced::Point::new(found.x + 4.0, found.center().y);
    assert!((tile.x - size.width / 2.0).abs() > 320.0, "the tile has to lie beside the panel, it is at {tile:?}");
    screen.point_at(tile);
    let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)), iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left))]);
    assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Select(None)]));
    let away = iced::Point::new(6.0, 6.0);
    let open = Clocks::settled();
    assert!(picture(&state, &words, size, tile, &open, "open-on-tile") == picture(&state, &words, size, away, &open, "open-away"));
    state.update(Message::Select(None), 1_790_000_101);
    assert!(picture(&state, &words, size, tile, &open, "closed-on-tile") != picture(&state, &words, size, away, &open, "closed-away"));
}

#[test]
fn a_map_is_searched_right_in_its_slot_and_the_four_sources_hang_under_it_on_one_line() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut state = state_with_songs("centred-panel");
    open_new(&mut state);
    state.update(Message::Select(Some(0)), 1_790_000_001);
    assert_eq!(shown(&state).panel, Panel::Add);
    for width in [980.0f32, 1400.0, 1920.0] {
        let size = iced::Size::new(width, 900.0);
        let mut screen = iced_test::Simulator::with_size(crate::settings(), size, view(&state, &words, &thumbs, width, 1.0, &Clocks::settled()));
        let field = screen.find(words.t("pool-search-hint")).unwrap().visible_bounds().unwrap();
        let tabs: Vec<iced::Rectangle> = ["pool-tab-search", "pool-tab-collections", "pool-tab-suggest"].iter().map(|key| screen.find(words.t(key)).unwrap().visible_bounds().unwrap()).collect();
        assert!(tabs.iter().all(|tab| (tab.center().y - tabs[0].center().y).abs() < 0.5), "{width}: {tabs:?}");
        assert!(tabs.windows(2).all(|pair| pair[0].x + pair[0].width <= pair[1].x), "{width}: {tabs:?}");
        assert!(tabs[0].y > field.y + field.height && tabs[0].y - field.y < 90.0, "{width}: the sources hang right under the slot, {field:?} {tabs:?}");
    }
}

#[test]
fn a_closed_panel_stays_while_it_fades_takes_no_clicks_and_is_gone_when_the_fade_is_over() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let size = iced::Size::new(1400.0, 900.0);
    let mut state = pool_with_maps("leaving-panel", &["a1", "b2"]);
    state.update(Message::Select(Some(1)), 1_790_000_100);
    assert!(shown(&state).gone.is_none());
    let buttons: Vec<iced::Point> = {
        let mut live = iced_test::Simulator::with_size(crate::settings(), size, view(&state, &words, &thumbs, size.width, 1.0, &Clocks::settled()));
        ["pool-remove", "pool-replace"].iter().map(|key| live.find(words.t(key)).unwrap().visible_bounds().unwrap().center()).collect()
    };
    state.update(Message::Select(None), 1_790_000_101);
    let gone = shown(&state).gone.as_deref().expect("what was shown is remembered");
    assert_eq!((gone.panel, gone.selected), (Panel::Slot, Some(1)));
    let at = |gone: f32| Clocks { gone, ..Clocks::settled() };
    let mut fading = iced_test::Simulator::with_size(crate::settings(), size, view(&state, &words, &thumbs, size.width, 1.0, &at(0.1)));
    assert!(fading.find(words.t("pool-remove")).is_err());
    for button in buttons {
        fading.point_at(button);
        let _ = fading.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)), iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left))]);
    }
    assert_eq!(fading.into_messages().count(), 0);
    let mut tile = iced_test::Simulator::with_size(crate::settings(), size, view(&state, &words, &thumbs, size.width, 1.0, &at(0.1)));
    tile.click("Glass Orchard").unwrap();
    assert!(matches!(tile.into_messages().collect::<Vec<_>>().as_slice(), [Message::Select(Some(0))]));
    let early = picture(&state, &words, size, iced::Point::new(6.0, 6.0), &at(0.02), "leaving-early");
    let late = picture(&state, &words, size, iced::Point::new(6.0, 6.0), &at(0.25), "leaving-late");
    let done = picture(&state, &words, size, iced::Point::new(6.0, 6.0), &at(ui::APPEAR), "leaving-done");
    let differs = |a: &::image::RgbaImage, b: &::image::RgbaImage| a.pixels().zip(b.pixels()).filter(|(x, y)| x != y).count();
    assert!(differs(&early, &done) > differs(&late, &done) && differs(&late, &done) > 0);
}

#[test]
fn a_closing_panel_leaves_no_dark_slab_behind_while_it_fades_or_after() {
    let words = Words::new(crate::lang::Lang::En);
    let size = iced::Size::new(1400.0, 900.0);
    let away = iced::Point::new(6.0, 6.0);
    let at = |gone: f32| Clocks { gone, ..Clocks::settled() };
    let far = |a: &::image::RgbaImage, b: &::image::RgbaImage, by: i32| a.pixels().zip(b.pixels()).filter(|(x, y)| (0..3).any(|c| (x.0[c] as i32 - y.0[c] as i32).abs() > by)).count();
    for (name, slot) in [("slab-slot", Some(1usize))] {
        let mut state = pool_with_maps(name, &["a1", "b2"]);
        if let Some(slot) = slot {
            state.update(Message::Select(Some(slot)), 1_790_000_100);
        }
        assert_eq!(state.panel_key(), "panel");
        let open = picture(&state, &words, size, away, &Clocks::settled(), &format!("{name}-open"));
        state.update(Message::Select(None), 1_790_000_101);
        let rest = picture(&state, &words, size, away, &Clocks::settled(), &format!("{name}-rest"));
        let start = picture(&state, &words, size, away, &at(0.0), &format!("{name}-start"));
        let nearly = picture(&state, &words, size, away, &at(ui::APPEAR * 0.97), &format!("{name}-nearly"));
        let over = picture(&state, &words, size, away, &at(ui::APPEAR), &format!("{name}-over"));
        assert!(far(&open, &rest, 12) > 20_000, "{name}: an open panel covers a good part of the page");
        let scale = open.width() as f32 / size.width;
        let inside = |image: &::image::RgbaImage| ::image::imageops::crop_imm(image, (420.0 * scale) as u32, (300.0 * scale) as u32, (560.0 * scale) as u32, (300.0 * scale) as u32).to_image();
        assert!(far(&inside(&start), &inside(&open), 12) < 800, "{name}: the fade starts from what was open, {} pixels moved", far(&inside(&start), &inside(&open), 12));
        assert!(far(&inside(&open), &inside(&rest), 12) > 2_000, "{name}: the panel covers that part of the page");
        assert_eq!(far(&nearly, &rest, 12), 0, "{name}: nothing of the panel is left at the end of the fade");
        assert!(over == rest, "{name}: after the fade the page is as if the panel had never been");
    }
}

fn shelf_with(name: &str, makers: &[&str]) -> (State, String) {
    let mut state = pool_with_maps(name, &["a1", "b2", "c3"]);
    state.update(Message::Rename("Spring duel".into()), 1_790_000_200);
    let at = state.editing_at().unwrap();
    state.list[at].authors = makers.iter().map(|name| (*name).to_owned()).collect();
    let id = state.list[at].id.clone();
    state.update(Message::Back, 1_790_000_201);
    assert!(matches!(state.screen, Screen::Shelf));
    (state, id)
}

#[test]
fn the_latest_draft_is_a_wide_card_with_its_facts_and_the_others_are_tiles() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let (mut state, id) = shelf_with("showcase-mine", &["Alice", "Bob"]);
    let at = state.list.iter().position(|pool| pool.id == id).unwrap();
    state.list[at].slots[0].measure = Some(measure(4.52));
    state.list[at].slots[1].measure = Some(measure(6.35));
    state.list[at].changed_at = 1_790_000_000;
    let mut other = Pool::new(Frame::Free, "Second pool", 1_780_000_000);
    other.slots = vec![Slot::empty(Mod::Nm)];
    other.slots[0].hash = Some("a".repeat(32));
    let other_id = other.id.clone();
    state.list.push(other);
    let clocks = Clocks { today: 1_790_000_000, ..Clocks::settled() };
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1400.0, 900.0), view(&state, &words, &thumbs, 1400.0, 1.0, &clocks));
    for label in ["shelf-mine", "shelf-saved", "catalogue-published", "catalogue-collections", "shelf-changed"] {
        assert!(screen.find(words.t(label)).is_ok(), "{label}");
    }
    assert!(screen.find(words.t("shelf-latest")).is_err());
    assert!(screen.find("Spring duel").is_ok() && screen.find("Second pool").is_ok());
    assert!(screen.find("Alice, Bob").is_ok());
    assert!(screen.find("4.52").is_ok() && screen.find("6.35").is_ok());
    assert!(screen.find(words.t("shelf-no-makers")).is_ok(), "the tile says that nobody is listed");
    screen.click("Spring duel").unwrap();
    assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Open(open)] if *open == id));
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1400.0, 900.0), view(&state, &words, &thumbs, 1400.0, 1.0, &clocks));
    screen.click("Second pool").unwrap();
    assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Open(open)] if *open == other_id));
}

#[test]
fn the_shelf_asks_for_the_covers_of_what_its_tab_shows() {
    let (mut state, _) = shelf_with("showcase-covers", &[]);
    state.songs = Some(Arc::new(HashMap::new()));
    let own: Vec<String> = state.list[0].slots.iter().filter_map(|slot| slot.hash.clone()).collect();
    assert!(!own.is_empty());
    assert_eq!(state.covers().into_iter().map(|(hash, _)| hash).collect::<Vec<_>>(), own);
    state.shelf = Shelf::Saved;
    assert!(state.covers().is_empty(), "a draft is not a saved pool");
    let theirs = pool_share::to_file(&state.list[0]);
    state.publications.push(crate::bot::Publication { code: "XQIS43GB".into(), id: "theirs".into(), kind: "pool".into(), local_id: "x".into(), revision: 3, name: "Theirs".into(), content: serde_json::from_slice(&theirs).unwrap(), mine: false });
    state.shelf = Shelf::Published;
    assert_eq!(state.covers().into_iter().map(|(hash, _)| hash).collect::<Vec<_>>(), own);
}

#[test]
fn a_published_pool_shows_its_code_and_is_saved_once_into_its_own_tab() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let (mut state, id) = shelf_with("showcase-published", &["Alice"]);
    let mut pool = state.list.iter().find(|pool| pool.id == id).unwrap().clone();
    pool.compiler = "Builder".into();
    let content: serde_json::Value = serde_json::from_slice(&pool_share::to_file(&pool)).unwrap();
    state.publications.push(crate::bot::Publication { code: "XQIS43GB".into(), id: "theirs".into(), kind: "pool".into(), local_id: "x".into(), revision: 3, name: "Someone's pool".into(), content: content.clone(), mine: false });
    state.publications.push(crate::bot::Publication { code: "K7M2PD4T".into(), id: "ours".into(), kind: "pool".into(), local_id: id.clone(), revision: 2, name: "My own pool".into(), content, mine: true });
    state.update(Message::Shelf(Shelf::Published), 1_790_000_000);
    state.publications.truncate(2);
    assert_eq!(state.shelf, Shelf::Published);
    {
    let mut screen = look(&state, &words, &thumbs);
    for said in ["Someone's pool", "My own pool", "XQIS43GB", "K7M2PD4T", "#3", "#2", "Builder"] {
        assert!(screen.find(said).is_ok(), "{said}");
    }
    assert!(screen.find(words.t("shelf-published-by")).is_ok());
    screen.click("My own pool").unwrap();
    assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::OpenPublication(open)] if open == "ours"), "one's own publication opens by its card, no button is needed");
    let mut screen = look(&state, &words, &thumbs);
    screen.click(words.t("shelf-save")).unwrap();
    assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::SavePublication(save)] if save == "theirs"));
    }
    let before = state.list.len();
    state.update(Message::SavePublication("theirs".into()), 1_790_000_001);
    state.update(Message::SavePublication("theirs".into()), 1_790_000_002);
    state.update(Message::SavePublication("ours".into()), 1_790_000_003);
    assert_eq!(state.list.len(), before + 1, "saved once, and one's own publication is not saved as a copy");
    let saved = state.list.iter().find(|pool| pool.saved.is_some()).unwrap().clone();
    assert_eq!(saved.saved, Some(pools::Saved { id: "theirs".into(), code: "XQIS43GB".into(), revision: 3, publisher: "Builder".into(), collected: false }));
    assert_eq!(saved.published_revision, 0);
    assert_eq!(pools::load_all(&state.dir).iter().filter(|pool| pool.saved.is_some()).count(), 1, "the mark survives on disk");
    {
        let mut screen = look(&state, &words, &thumbs);
        assert!(screen.find(words.t("shelf-saved-mark")).is_ok() && screen.find(words.t("shelf-save")).is_err());
    }
    state.update(Message::Shelf(Shelf::Saved), 1_790_000_004);
    state.songs = None;
    {
        let mut screen = look(&state, &words, &thumbs);
        assert!(screen.find("XQIS43GB").is_err() && screen.find("#3").is_err(), "a card carries neither the code nor the issue number");
        assert!(screen.find(words.t("shelf-checking")).is_ok(), "before the game's maps are read nothing is claimed about them");
    }
    state.songs = Some(Arc::new(HashMap::new()));
    {
        let mut screen = look(&state, &words, &thumbs);
        let wanted = match saved_count(&state) { 0 => words.t("shelf-all-here"), missing => words.n("shelf-missing", missing) };
        assert!(screen.find(wanted).is_ok());
    }
    state.update(Message::Shelf(Shelf::Mine), 1_790_000_005);
    let mut screen = look(&state, &words, &thumbs);
    assert!(screen.find("XQIS43GB").is_err(), "a saved pool is not among the drafts");
}

fn look<'a>(state: &'a State, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> iced_test::Simulator<'a, Message> {
    iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1400.0, 900.0), view(state, words, thumbs, 1400.0, 1.0, &Clocks::settled()))
}

#[test]
fn deleting_a_game_collection_removes_the_database_entry_before_the_draft() {
    let mut state = state_with_songs("delete-game-origin");
    let game = scratch("delete-game-database");
    std::fs::create_dir_all(&game).unwrap();
    let original = Collection { name: "Tournament".into(), hashes: vec!["a".repeat(32)] };
    let kept = Collection { name: "Favorites".into(), hashes: vec!["b".repeat(32)] };
    std::fs::write(game.join("collection.db"), crate::pool_collections::compose(20250101_i32.to_le_bytes(), &[original.clone(), kept.clone()])).unwrap();
    state.update(Message::Collections(vec![original, kept.clone()]), 100);
    state.update(Message::ImportCollection(0), 101);
    let id = state.editing().unwrap().id.clone();
    state.update(Message::Rename("Renamed draft".into()), 102);
    state.update(Message::AskDelete(true), 103);
    let effects = state.update(Message::DeletePool, 104);
    let name = effects.iter().find_map(|effect| match effect {
        Effect::Uncollect(pool, name) if *pool == id => Some(name),
        _ => None,
    }).expect("deleting a game tile must remove its game copy");
    assert_eq!(name, "Tournament");
    assert!(state.editing().is_some());
    let removed = crate::pool_collections::remove(&game, name).unwrap();
    let effects = state.update(Message::Uncollected(id.clone(), Ok(removed)), 105);
    assert!(effects.iter().any(|effect| matches!(effect, Effect::ReadCollections)));
    let remaining = crate::pool_collections::parse(&std::fs::read(game.join("collection.db")).unwrap()).unwrap();
    state.update(Message::Collections(remaining), 106);
    assert_eq!(state.collections.as_deref(), Some([kept].as_slice()));
    assert!(state.list.iter().all(|pool| pool.id != id));
    assert!(!state.dir.join(format!("{id}.pool")).exists());
    std::fs::remove_dir_all(game).unwrap();
}

#[test]
fn a_saved_collection_can_be_deleted_from_its_page() {
    let words = Words::new(crate::lang::Lang::Ru);
    let thumbs = HashMap::new();
    let mut state = state_with_songs("drop-saved");
    let mut pool = Pool::new(Frame::Free, "Their collection", 1_790_000_000);
    pool.collection = true;
    pool.compiler = "Builder".into();
    pool.slots = vec![Slot::empty(Mod::Nm)];
    pool.slots[0].hash = Some("a".repeat(32));
    pool.saved = Some(pools::Saved { id: "theirs".into(), code: "XQIS43GB".into(), revision: 1, publisher: "Builder".into(), collected: true });
    let id = pool.id.clone();
    state.list.push(pool);
    state.save(state.list.len() - 1, 1_790_000_000);
    state.flush_saves(true);
    state.update(Message::Open(id.clone()), 1_790_000_001);
    let asked = {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(980.0, 720.0), view(&state, &words, &thumbs, 980.0, 1.0, &Clocks::settled()));
        screen.click(words.t("collection-delete")).expect("the delete button is on the page");
        screen.into_messages().collect::<Vec<_>>()
    };
    for message in asked {
        state.update(message, 1_790_000_002);
    }
    let confirmed = {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(980.0, 720.0), view(&state, &words, &thumbs, 980.0, 1.0, &Clocks::settled()));
        screen.click(words.t("pool-delete-yes")).expect("the confirmation is on the page");
        screen.into_messages().collect::<Vec<_>>()
    };
    let mut effects = Vec::new();
    for message in confirmed {
        effects.extend(state.update(message, 1_790_000_003));
    }
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Uncollect(pool, name) if *pool == id && name == "Their collection")), "the game's copy is taken out first: {effects:?}");
    state.update(Message::Uncollected(id.clone(), Err("running".into())), 1_790_000_004);
    assert!(state.list.iter().any(|pool| pool.id == id), "with osu! open the collection stays");
    {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(980.0, 720.0), view(&state, &words, &thumbs, 980.0, 1.0, &Clocks::settled()));
        assert!(screen.find(words.t("collection-delete-running")).is_ok(), "and the player is asked to close the game");
    }
    state.update(Message::Uncollected(id.clone(), Err("failed: Permission denied".into())), 1_790_000_005);
    assert!(state.list.iter().any(|pool| pool.id == id));
    assert_eq!(state.delete_error.as_deref(), Some("failed: Permission denied"));
    state.update(Message::DeletePool, 1_790_000_006);
    assert!(state.delete_error.is_none() && !state.uncollect_refused);
    state.update(Message::Uncollected(id.clone(), Ok(true)), 1_790_000_005);
    assert!(state.list.iter().all(|pool| pool.id != id), "the collection is gone from the list");
    assert!(pools::load_all(&state.dir).iter().all(|pool| pool.id != id), "and from the disk");
}

#[test]
fn a_saved_pool_downloads_what_is_missing_and_then_asks_for_its_collection() {
    let mut state = state_with_songs("pull");
    let mut pool = Pool::new(Frame::Free, "Winter Cup", 1_790_000_000);
    pool.slots = vec![Slot::empty(Mod::Nm), Slot::empty(Mod::Hd), Slot::empty(Mod::Hr)];
    for (slot, hash) in pool.slots.iter_mut().zip(["a1", "zz8", "zz9"]) {
        slot.hash = Some(hash.to_owned());
    }
    pool.saved = Some(pools::Saved { id: "theirs".into(), code: "XQIS43GB".into(), revision: 1, publisher: "Builder".into(), collected: false });
    let id = pool.id.clone();
    state.list.push(pool);
    assert!(state.update(Message::Pull("no such pool".into()), 100).is_empty() && state.pull.is_none());
    let effects = state.update(Message::Pull(id.clone()), 101);
    let Some(Effect::Fetch(first, hash, _)) = effects.iter().find(|effect| matches!(effect, Effect::Fetch(..))).cloned() else { panic!("{effects:?}") };
    assert_eq!(hash, "zz8", "the map that is already in the game is not asked for");
    let pull = state.pull.as_ref().unwrap();
    assert_eq!((pull.total, pull.done(), pull.running()), (2, 0, true));
    assert!(state.update(Message::Pull(id.clone()), 102).is_empty(), "a second press while it runs starts nothing");
    let effects = state.update(Message::Step(first, "zz8".into(), crate::maps::Step::Done(map("Found", "A", "B"))), 103);
    let Some(Effect::Fetch(second, hash, _)) = effects.iter().find(|effect| matches!(effect, Effect::Fetch(..))).cloned() else { panic!("{effects:?}") };
    assert_eq!((hash.as_str(), state.pull.as_ref().unwrap().done()), ("zz9", 1));
    let effects = state.update(Message::Step(second, "zz9".into(), crate::maps::Step::Nowhere), 104);
    let Some(Effect::Collect(pool_id, name, hashes)) = effects.iter().find(|effect| matches!(effect, Effect::Collect(..))).cloned() else { panic!("{effects:?}") };
    assert_eq!((pool_id.as_str(), name.as_str(), hashes), (id.as_str(), "Winter Cup", vec!["a1".to_owned(), "zz8".to_owned()]));
    let pull = state.pull.as_ref().unwrap();
    assert_eq!((pull.lost, pull.running(), pull.over.clone()), (1, false, None));
    state.update(Message::Collected(id.clone(), Err("running".into())), 105);
    assert!(!state.list.iter().find(|pool| pool.id == id).unwrap().saved.as_ref().unwrap().collected);
    assert_eq!(state.pull.as_ref().unwrap().over, Some(Err("running".into())));
    let effects = state.update(Message::Pull(id.clone()), 106);
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Fetch(_, hash, _) if hash == "zz9")), "the lost map is tried again: {effects:?}");
    state.update(Message::StopPull, 107);
    let request = state.fetching.as_ref().unwrap().0;
    state.update(Message::Step(request, "zz9".into(), crate::maps::Step::Stopped), 108);
    assert!(state.pull.is_none());
    state.fetching = None;
    if let Some(songs) = state.songs.as_mut() { Arc::make_mut(songs).insert("zz9".into(), map("Late", "A", "B")); }
    let effects = state.update(Message::Pull(id.clone()), 109);
    assert!(matches!(effects.iter().find(|effect| matches!(effect, Effect::Collect(..))), Some(Effect::Collect(_, _, hashes)) if hashes.len() == 3), "with nothing missing the collection is asked for at once: {effects:?}");
    state.update(Message::Collected(id.clone(), Ok(3)), 110);
    assert!(state.list.iter().find(|pool| pool.id == id).unwrap().saved.as_ref().unwrap().collected);
    assert!(pools::load_all(&state.dir).iter().any(|pool| pool.id == id && pool.saved.as_ref().is_some_and(|saved| saved.collected)), "and it is remembered on disk");
}

#[test]
fn a_saved_pool_tells_of_a_newer_issue_and_takes_it_on_request() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut state = state_with_songs("renew");
    let mut theirs = Pool::new(Frame::Free, "Winter Cup", 1_790_000_000);
    theirs.compiler = "Builder".into();
    theirs.slots = vec![Slot::empty(Mod::Nm), Slot::empty(Mod::Hd)];
    theirs.slots[0].hash = Some("a".repeat(32));
    theirs.slots[1].hash = Some("b".repeat(32));
    let issue = |pool: &Pool, revision: u64| crate::bot::Publication { code: "XQIS43GB".into(), id: "theirs".into(), kind: "pool".into(), local_id: "x".into(), revision, name: pool.name.clone(), content: serde_json::from_slice(&pool_share::to_file(pool)).unwrap(), mine: false };
    state.publications = vec![issue(&theirs, 3)];
    state.update(Message::SavePublication("theirs".into()), 100);
    let id = state.list.iter().find(|pool| pool.saved.is_some()).unwrap().id.clone();
    state.update(Message::Collected(id.clone(), Ok(2)), 101);
    state.shelf = Shelf::Saved;
    assert!(state.newer_of(&state.list[0]).is_none());
    {
        let mut screen = look(&state, &words, &thumbs);
        assert!(screen.find(words.t("shelf-renew")).is_err() && screen.find(words.with("shelf-out", &[("n", "4".into())])).is_err());
    }
    assert!(state.update(Message::Renew(id.clone()), 102).is_empty(), "with nothing newer nothing changes");
    theirs.name = "Winter Cup, finals".into();
    theirs.slots.push(Slot::empty(Mod::Hr));
    theirs.slots[2].hash = Some("c".repeat(32));
    state.publications = vec![issue(&theirs, 4)];
    {
        let mut screen = look(&state, &words, &thumbs);
        assert!(screen.find(words.with("shelf-out", &[("n", "4".into())])).is_ok());
        assert!(screen.find(words.n("shelf-renew-added", 1)).is_ok());
        screen.click(words.t("shelf-renew")).unwrap();
        assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Renew(renew)] if *renew == id));
    }
    state.update(Message::Renew(id.clone()), 103);
    let pool = state.list.iter().find(|pool| pool.id == id).unwrap();
    assert_eq!((pool.name.as_str(), pool.filled()), ("Winter Cup, finals", 3));
    assert_eq!(pool.saved, Some(pools::Saved { id: "theirs".into(), code: "XQIS43GB".into(), revision: 4, publisher: "Builder".into(), collected: false }), "the collection has to be written again");
    assert_eq!(state.list.iter().filter(|pool| pool.saved.is_some()).count(), 1, "the pool is renewed in place");
    assert!(state.newer_of(pool).is_none());
    assert!(pools::load_all(&state.dir).iter().any(|pool| pool.id == id && pool.filled() == 3));
}

#[test]
fn a_published_collection_is_saved_downloaded_and_renewed_like_a_pool() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut state = state_with_songs("saved-collection");
    let issue = |hashes: &[&str], revision: u64| crate::bot::Publication { code: "C0LLECT1".into(), id: "their-collection".into(), kind: "collection".into(), local_id: "x".into(), revision, name: "Weekend practice".into(), content: serde_json::json!({"name": "Weekend practice", "hashes": hashes}), mine: false };
    state.collection_shelf = true;
    state.publications = vec![issue(&["a1", "zz8"], 1)];
    {
        let mut screen = look(&state, &words, &thumbs);
        assert!(screen.find("C0LLECT1").is_ok());
        screen.click(words.t("shelf-save")).unwrap();
        assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::SavePublication(id)] if id == "their-collection"));
    }
    state.update(Message::SavePublication("their-collection".into()), 100);
    state.update(Message::SavePublication("their-collection".into()), 101);
    let saved: Vec<Pool> = state.list.iter().filter(|pool| pool.saved.is_some()).cloned().collect();
    assert_eq!(saved.len(), 1);
    assert!(saved[0].collection && saved[0].name == "Weekend practice" && saved[0].filled() == 2);
    let id = saved[0].id.clone();
    {
        let mut screen = look(&state, &words, &thumbs);
        assert!(screen.find(words.t("shelf-saved-mark")).is_ok() && screen.find(words.t("shelf-own-collections")).is_ok());
        assert!(screen.find(words.t("shelf-empty-collections")).is_ok(), "a saved collection is not one of my own");
    }
    state.collection_shelf = false;
    state.shelf = Shelf::Saved;
    assert!(state.covers().iter().any(|(hash, _)| hash == "zz8"));
    {
        let mut screen = look(&state, &words, &thumbs);
        assert!(screen.find("Weekend practice").is_ok() && screen.find(words.n("shelf-missing", 1)).is_ok() && screen.find(words.t("shelf-pull")).is_ok());
    }
    let effects = state.update(Message::Pull(id.clone()), 102);
    let Some(Effect::Fetch(request, hash, _)) = effects.iter().find(|effect| matches!(effect, Effect::Fetch(..))).cloned() else { panic!("{effects:?}") };
    assert_eq!(hash, "zz8");
    let effects = state.update(Message::Step(request, "zz8".into(), crate::maps::Step::Done(map("Found", "A", "B"))), 103);
    assert!(matches!(effects.iter().find(|effect| matches!(effect, Effect::Collect(..))), Some(Effect::Collect(_, name, hashes)) if name == "Weekend practice" && hashes.len() == 2), "{effects:?}");
    state.update(Message::Collected(id.clone(), Ok(2)), 104);
    state.publications = vec![issue(&["a1", "zz8", "b2"], 2)];
    assert!(state.newer_of(state.list.iter().find(|pool| pool.id == id).unwrap()).is_some());
    state.update(Message::Renew(id.clone()), 105);
    let pool = state.list.iter().find(|pool| pool.id == id).unwrap();
    assert!(pool.collection && pool.filled() == 3);
    assert_eq!(pool.saved.as_ref().map(|saved| (saved.revision, saved.collected)), Some((2, false)));
}

#[test]
fn a_step_back_can_be_taken_again_until_something_new_is_done() {
    let mut state = pool_with_maps("redo", &["a1", "b2", "c3"]);
    let names = |state: &State| state.editing().unwrap().slots.iter().map(|slot| slot.title.clone()).collect::<Vec<_>>();
    let full = names(&state);
    state.update(Message::Clear(1), 200);
    let cut = names(&state);
    assert_ne!(full, cut);
    assert!(state.redo.is_empty());
    state.update(Message::Undo, 201);
    assert_eq!(names(&state), full);
    assert_eq!(state.redo.len(), 1);
    state.update(Message::Redo, 202);
    assert_eq!(names(&state), cut, "the step forward brings the change back");
    assert!(state.redo.is_empty());
    state.update(Message::Redo, 203);
    assert_eq!(names(&state), cut, "with nothing ahead a step forward does nothing");
    state.update(Message::Undo, 204);
    state.update(Message::Clear(0), 205);
    assert!(state.redo.is_empty(), "a new change closes the way forward");
    let changed = names(&state);
    state.update(Message::Redo, 206);
    assert_eq!(names(&state), changed);
}

#[test]
fn a_slot_is_doubled_right_after_itself_and_the_pool_is_searched_by_what_a_slot_says() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut state = pool_with_maps("find-and-double", &["a1", "b2", "c3"]);
    let before = state.editing().unwrap().slots.len();
    let second = state.editing().unwrap().slots[1].clone();
    state.update(Message::Duplicate(1), 300);
    let pool = state.editing().unwrap();
    assert_eq!(pool.slots.len(), before + 1);
    assert_eq!((pool.slots[1].hash.clone(), pool.slots[2].hash.clone()), (second.hash.clone(), second.hash.clone()));
    state.update(Message::Undo, 301);
    assert_eq!(state.editing().unwrap().slots.len(), before);
    let empty = state.editing().unwrap().slots.iter().position(Slot::is_empty);
    if let Some(empty) = empty {
        state.update(Message::Duplicate(empty), 302);
        assert_eq!(state.editing().unwrap().slots.len(), before, "an empty slot is not doubled");
    }
    assert!(slot_found(&second, "") && slot_found(&second, &second.title.to_uppercase()) && slot_found(&second, &second.artist[..3]));
    assert!(slot_found(&second, second.mods.code()) && !slot_found(&second, "no such words") && !slot_found(&Slot::empty(Mod::Nm), "nm"));
    state.update(Message::Find(second.title.clone()), 303);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.find == second.title));
    let filled = state.editing().unwrap().filled() as u64;
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1400.0, 900.0), view(&state, &words, &thumbs, 1400.0, 1.0, &Clocks::settled()));
    assert!(screen.find(words.of(1, filled)).is_ok(), "the count of found slots stands by the field");
    assert!(screen.find(words.t("shelf-cards")).is_ok(), "the foot of the pool tells its facts");
    assert!(pool_foot_says(&state, &words, 30).contains(&words.t("pool-foot-saved-now")) && pool_foot_says(&state, &words, 180) == words.n("pool-foot-saved-minutes", 3) && pool_foot_says(&state, &words, 7300) == words.n("pool-foot-saved-hours", 2));
}

#[test]
fn the_slot_menu_and_the_commands_act_and_close_themselves() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut state = pool_with_maps("menus", &["a1", "b2", "c3"]);
    state.update(Message::SlotMenu(Some(1)), 400);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.menu == Some(1)));
    {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1400.0, 900.0), view(&state, &words, &thumbs, 1400.0, 1.0, &Clocks::settled()));
        for key in ["pool-menu-note", "pool-menu-replace", "pool-menu-double", "pool-menu-category", "pool-menu-copy", "pool-menu-copy-id", "pool-menu-remove"] {
            assert!(screen.find(words.t(key)).is_ok(), "{key}");
        }
        screen.click(words.t("pool-menu-double")).unwrap();
        assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Chosen(inner)] if matches!(**inner, Message::Duplicate(1))));
    }
    let before = state.editing().unwrap().slots.len();
    state.update(Message::Chosen(Box::new(Message::Duplicate(1))), 401);
    assert_eq!(state.editing().unwrap().slots.len(), before + 1);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.menu.is_none()), "an action closes the menu");
    let hash = state.editing().unwrap().slots[0].hash.clone().unwrap();
    assert!(matches!(state.update(Message::Chosen(Box::new(Message::CopySlot(0))), 402).as_slice(), [Effect::Copy(copied, _)] if *copied == hash));
    assert!(matches!(state.update(Message::Chosen(Box::new(Message::CopySlotId(0))), 403).as_slice(), [Effect::Copy(copied, "pool-copied-hash")] if *copied == hash), "without a map id the hash is copied");
    {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1400.0, 900.0), view(&state, &words, &thumbs, 1400.0, 1.0, &Clocks::settled()));
        for key in ["pool-tool-categories", "pool-tool-backdrop", "pool-command-share"] {
            assert!(screen.find(words.t(key)).is_ok(), "{key} is a button of its own");
        }
        assert!(screen.find(words.t("pool-actions")).is_err(), "there is no menu of actions to open first");
        screen.click(words.t("pool-tool-categories")).unwrap();
        assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Categories(true)]));
    }
    state.update(Message::Palette(true), 403);
    state.update(Message::Chosen(Box::new(Message::OnlyEmpty)), 405);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.only_empty && !editor.palette && editor.command.is_empty()));
}

#[test]
fn sorting_by_stars_keeps_each_category_in_its_places() {
    let mut state = pool_with_maps("sort", &["a1", "b2", "c3"]);
    let at = state.editing_at().unwrap();
    let mods = [Mod::Nm, Mod::Hd, Mod::Nm, Mod::Hd, Mod::Nm];
    let stars = [Some(6.1), Some(5.0), Some(4.2), Some(4.9), None];
    state.list[at].slots = mods.iter().zip(stars).enumerate().map(|(spot, (mods, stars))| {
        let mut slot = Slot::empty(*mods);
        slot.hash = Some(format!("h{spot}"));
        slot.title = format!("t{spot}");
        slot.measure = stars.map(measure);
        slot
    }).collect();
    state.update(Message::SortStars, 500);
    let pool = state.editing().unwrap();
    assert_eq!(pool.slots.iter().map(|slot| slot.mods).collect::<Vec<_>>(), mods, "no slot leaves the places of its category");
    assert_eq!(pool.slots.iter().map(|slot| slot.title.as_str()).collect::<Vec<_>>(), ["t2", "t3", "t0", "t1", "t4"], "each category goes from easy to hard, unmeasured last");
    let undo = state.undo.len();
    state.update(Message::SortStars, 501);
    assert_eq!(state.undo.len(), undo, "an order that is already right is left alone");
    state.update(Message::Undo, 502);
    assert_eq!(state.editing().unwrap().slots[0].title, "t0");
}

fn pool_foot_says(state: &State, words: &Words, later: i64) -> String {
    let mut state = state.clone();
    state.unsaved.clear();
    state.clock = state.editing().unwrap().changed_at + later;
    saved_words(&state, state.editing().unwrap(), words)
}

#[test]
fn a_pool_takes_its_backdrop_from_a_map_or_a_file_and_keeps_its_dimming_in_bounds() {
    let mut state = pool_with_maps("backdrop", &["a1", "b2", "c3"]);
    let id = state.editing().unwrap().id.clone();
    state.update(Message::BackdropPanel(true), 100);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.backdrop));
    assert!(state.update(Message::BackdropDim(80), 101).is_empty() && state.editing().unwrap().backdrop.is_none(), "with no backdrop there is nothing to dim");
    state.update(Message::BackdropFrom(Some("b2".into())), 102);
    let held = state.editing().unwrap().backdrop.clone().unwrap();
    assert_eq!((held.from.clone(), held.dim, held.blur, held.on_card), (pools::BackdropFrom::Map("b2".into()), 60, true, true));
    for (asked, kept) in [(10, 30), (75, 75), (100, 90)] {
        state.update(Message::BackdropDim(asked), 103);
        assert_eq!(state.editing().unwrap().backdrop.as_ref().unwrap().dim, kept);
    }
    state.update(Message::BackdropBlur(false), 104);
    state.update(Message::BackdropCard(false), 105);
    let held = state.editing().unwrap().backdrop.clone().unwrap();
    assert!(!held.blur && !held.on_card && held.dim == 90);
    state.flush_saves(true);
    assert_eq!(pools::load_all(&state.dir).into_iter().find(|pool| pool.id == id).unwrap().backdrop, Some(held));
    let folder = scratch("backdrop-files");
    std::fs::create_dir_all(&folder).unwrap();
    let good = folder.join("picture.PNG");
    std::fs::write(&good, b"not really a picture, only a small file").unwrap();
    let wrong = folder.join("notes.txt");
    std::fs::write(&wrong, b"text").unwrap();
    state.update(Message::BackdropPicked(Some(wrong)), 106);
    assert!(state.backdrop_refused && matches!(state.editing().unwrap().backdrop.as_ref().unwrap().from, pools::BackdropFrom::Map(_)), "a file of another kind is refused and changes nothing");
    let effects = state.update(Message::BackdropPicked(Some(good)), 107);
    let pools::BackdropFrom::File(kept) = state.editing().unwrap().backdrop.clone().unwrap().from else { panic!("the file became the backdrop") };
    assert!(!state.backdrop_refused && kept.starts_with(state.dir.join("backdrops")) && kept.is_file(), "the picture is copied beside the pools: {kept:?}");
    assert!(matches!(effects.iter().find(|effect| matches!(effect, Effect::LoadBackdrop(..))), Some(Effect::LoadBackdrop(_, path, false)) if *path == kept));
    let (key, _, _) = state.backdrop_key().unwrap();
    state.update(Message::BackdropLoaded("stale".into(), Some(image::Handle::from_rgba(1, 1, vec![0, 0, 0, 255]))), 108);
    assert!(state.backdrop_shown().is_none(), "a picture for another choice is not shown");
    state.update(Message::BackdropLoaded(key, Some(image::Handle::from_rgba(1, 1, vec![0, 0, 0, 255]))), 109);
    assert_eq!(state.backdrop_shown().map(|(_, dim)| dim), Some(90));
    state.update(Message::BackdropFrom(None), 110);
    assert!(state.editing().unwrap().backdrop.is_none() && state.backdrop_shown().is_none());
}

#[test]
fn a_profile_link_is_told_apart_from_a_map_link_and_keys_reach_the_slot_in_hand() {
    assert_eq!(profile_link(" https://osu.ppy.sh/users/12345678 "), Some("osu.ppy.sh/users/12345678".into()));
    assert_eq!(profile_link("osu.ppy.sh/u/peppy/"), Some("osu.ppy.sh/u/peppy".into()));
    assert!(profile_link("https://osu.ppy.sh/beatmapsets/77#osu/100").is_none() && profile_link("two words osu.ppy.sh/users/1").is_none() && profile_link("").is_none());
    let mut state = pool_with_maps("keys", &["a1", "b2", "c3"]);
    state.update(Message::Pasted("https://osu.ppy.sh/users/2".into()), 100);
    assert_eq!(state.stray.as_deref(), Some("osu.ppy.sh/users/2"));
    assert!(state.finding.is_none(), "nothing is looked up for a profile");
    state.update(Message::DropStray, 101);
    assert!(state.stray.is_none());
    state.update(Message::Select(None), 102);
    let before = state.editing().unwrap().slots.len();
    state.update(Message::Hot(Hot::Double), 102);
    assert_eq!(state.editing().unwrap().slots.len(), before, "with no slot in hand a key does nothing");
    state.update(Message::SlotMenu(Some(1)), 103);
    state.update(Message::Hot(Hot::Double), 104);
    assert_eq!(state.editing().unwrap().slots.len(), before + 1);
    state.update(Message::Select(Some(0)), 105);
    state.update(Message::Hot(Hot::Remove), 106);
    assert!(state.editing().unwrap().slots[0].is_empty());
    assert!(matches!(state.update(Message::Hot(Hot::Text), 107).as_slice(), [Effect::Copy(_, "pool-copied")]));
    let folder = scratch("map-link");
    std::fs::create_dir_all(&folder).unwrap();
    let file = folder.join("map.osu");
    std::fs::write(&file, "osu file format v14\n\n[Metadata]\nTitle:Song\nBeatmapID:4242\nBeatmapSetID:7\n").unwrap();
    assert_eq!(map_link(&file), Some("https://osu.ppy.sh/b/4242".into()));
    assert_eq!(map_id(&file), Some(4242));
    std::fs::write(&file, "[Metadata]\nBeatmapID:0\n").unwrap();
    assert_eq!(map_link(&file), None, "a map that was never uploaded has no link");
}

#[test]
fn published_versions_are_kept_and_an_old_one_opens_as_its_own_draft() {
    let mut state = pool_with_maps("versions", &["a1", "b2", "c3"]);
    state.update(Message::Rename("Spring duel".into()), 100);
    let id = state.editing().unwrap().id.clone();
    state.update(Message::Published(id.clone(), Ok(publication(&id, 1))), 101);
    state.update(Message::Clear(0), 102);
    state.update(Message::Published(id.clone(), Ok(publication(&id, 2))), 103);
    assert_eq!(state.versions.iter().map(|held| (held.published_revision, held.filled())).collect::<Vec<_>>(), [(2, 2), (1, 3)]);
    let Screen::Editor(editor) = &state.screen else { panic!() };
    let listed = commands(&state, editor, state.editing().unwrap());
    assert!(listed.iter().any(|(key, hint, press)| *key == "pool-command-history" && hint == "2" && press.is_some()));
    let before = state.list.len();
    state.update(Message::Chosen(Box::new(Message::History(true))), 104);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.history && editor.palette));
    state.update(Message::OpenVersion(1), 105);
    assert_eq!(state.list.len(), before + 1);
    let opened = state.editing().unwrap();
    assert!(opened.id != id && opened.name == "Spring duel v1" && opened.filled() == 3 && opened.published_revision == 0, "the old version is a draft of its own");
}

fn saved_count(state: &State) -> u64 {
    state.list.iter().find(|pool| pool.saved.is_some()).map_or(0, |pool| pool.slots.iter().filter(|slot| slot.hash.is_some()).count() as u64)
}

#[test]
fn a_narrow_window_keeps_the_tiles_inside_and_an_empty_tab_says_what_to_do() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let (mut state, _) = shelf_with("showcase-narrow", &["Alice"]);
    let mut other = Pool::new(Frame::Free, "Second pool", 1_780_000_000);
    other.slots = vec![Slot::empty(Mod::Nm)];
    state.list.push(other);
    for width in [560.0_f32, 760.0, 1400.0] {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(width, 1400.0), view(&state, &words, &thumbs, width, 1.0, &Clocks::settled()));
        for name in ["Spring duel", "Second pool"] {
            let found = screen.find(name).unwrap().visible_bounds().unwrap();
            assert!(found.x >= 0.0 && found.x + found.width <= width, "{name} at {width}: {found:?}");
        }
    }
    state.shelf = Shelf::Saved;
    {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(980.0, 900.0), view(&state, &words, &thumbs, 980.0, 1.0, &Clocks::settled()));
        assert!(screen.find(words.t("shelf-empty-saved")).is_ok());
    }
    state.list.clear();
    state.shelf = Shelf::Mine;
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(980.0, 900.0), view(&state, &words, &thumbs, 980.0, 1.0, &Clocks::settled()));
    assert!(screen.find(words.t("shelf-empty-mine")).is_ok());
}

#[test]
fn closing_starts_its_own_clock_and_opening_again_forgets_it() {
    let mut state = pool_with_maps("leaving-clock", &["a1", "b2"]);
    let mut marks = Marks::default();
    let start = Instant::now();
    marks.observe(&state, start);
    state.update(Message::Select(Some(0)), 1_790_000_100);
    marks.observe(&state, start + Duration::from_secs(10));
    assert_eq!(marks.clocks(start + Duration::from_secs(11)).gone, SETTLED);
    state.update(Message::Select(None), 1_790_000_101);
    marks.observe(&state, start + Duration::from_secs(20));
    let soon = start + Duration::from_secs(20) + Duration::from_millis(100);
    assert!((marks.clocks(soon).gone - 0.1).abs() < 0.001);
    assert!(marks.animating(soon));
    assert!(!marks.animating(start + Duration::from_secs(30)));
    state.update(Message::Select(Some(1)), 1_790_000_102);
    marks.observe(&state, start + Duration::from_secs(20) + Duration::from_millis(150));
    assert_eq!(marks.clocks(start + Duration::from_secs(20) + Duration::from_millis(200)).gone, SETTLED);
}

#[test]
fn category_numbers_follow_each_category_after_changes_and_reordering() {
    let mut state = state_with_songs("category-numbers");
    state.update(Message::New, 100);
    state.update(Message::SetMod(1, Mod::Hd), 101);
    state.update(Message::SetMod(3, Mod::Hd), 102);
    let numbers = |pool: &Pool| (0..pool.slots.len()).map(|at| category_number(pool, at)).collect::<Vec<_>>();
    assert_eq!(numbers(state.editing().unwrap()), vec![1, 1, 2, 2]);
    state.list[0].slots.swap(1, 2);
    assert_eq!(numbers(state.editing().unwrap()), vec![1, 2, 1, 2]);
    state.update(Message::SetMod(2, Mod::Hr), 103);
    assert_eq!(numbers(state.editing().unwrap()), vec![1, 2, 1, 1]);
    state.update(Message::Category(0, "Tech".into()), 104);
    state.update(Message::Category(2, "Tech".into()), 105);
    assert_eq!(numbers(state.editing().unwrap()), vec![1, 1, 2, 1]);
}

#[test]
fn long_candidate_names_scroll_and_leave_the_star_rating_visible() {
    let mut pool = Pool::new(Frame::Free, "", 100);
    pool.slots.push(Slot::empty(Mod::Hd));
    let editor = Editor::at(pool.id.clone());
    let mut found = found(&["a1", "b2"], Some(0));
    found.title = "THE BADDEST feat. (G)I-DLE, Bea Miller, Wolftyla".into();
    found.difficulties[0].version = "All that you've left behind, has become my everything".into();
    let candidate = Candidate { found, choice: Some(0), place: Place::Slot(0), cover: None };
    let thumbs = HashMap::new();
    let words = Words::new(crate::lang::Lang::Ru);
    for width in [340.0, 400.0] {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(width, 800.0), candidate_card(&candidate, &editor, &pool, &words, &thumbs));
        let title = screen.find(candidate.found.title.clone()).unwrap().visible_bounds().unwrap();
        let difficulty = screen.find(candidate.found.difficulties[0].version.clone()).unwrap().visible_bounds().unwrap();
        let stars = screen.find(stars_of(&words, 3.0)).unwrap().visible_bounds().unwrap();
        assert!(title.height <= 25.0 && title.x + title.width <= width);
        assert!(difficulty.height <= 25.0 && difficulty.x + difficulty.width <= stars.x);
        assert!(stars.x + stars.width <= width);
        if let Some(dir) = std::env::var_os("DOSSIER_POOL_MENU_REVIEW").map(PathBuf::from) {
            crate::gallery::write_snapshot(&screen.snapshot(&theme::theme()).unwrap(), &dir.join(format!("long-map-{width}"))).unwrap();
        }
    }
}

#[test]
fn clicking_a_free_slot_retargets_the_visible_candidate_and_keeps_its_category() {
    let mut state = state_with_songs("free-slot-candidate");
    state.update(Message::New, 100);
    state.update(Message::Begin(Start::Empty), 100);
    state.finding = Some(Finding::Found(Candidate { found: found(&["a1", "b2"], Some(0)), choice: Some(0), place: Place::Slot(0), cover: None }));
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1000.0, 1000.0), view(&state, &words, &thumbs, 1000.0, 1.0, &Clocks::settled()));
    let number = screen.find("3").unwrap().visible_bounds().unwrap();
    screen.point_at(number.center() + iced::Vector::new(320.0, 0.0));
    let _ = screen.simulate(iced_test::simulator::click());
    let messages: Vec<_> = screen.into_messages().collect();
    assert!(matches!(messages.as_slice(), [Message::Select(Some(2))]), "{messages:?}");
    for message in messages { state.update(message, 101); }
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.panel == Panel::Add && editor.selected == Some(2)));
    assert!(matches!(&state.finding, Some(Finding::Found(candidate)) if candidate.place == Place::Slot(2)));
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1500.0, 1000.0), view(&state, &words, &thumbs, 1500.0, 1.0, &Clocks::settled()));
    assert!(screen.find(words.t("pool-link-put-all")).is_err());
    assert!(screen.find(words.t("pool-place-end")).is_err());
    drop(screen);
    state.update(Message::SetMod(2, Mod::Hr), 102);
    state.update(Message::Confirm, 103);
    assert!(state.editing().unwrap().slots[..2].iter().all(Slot::is_empty));
    assert_eq!(state.editing().unwrap().slots[2].hash.as_deref(), Some("a1"));
    assert_eq!(state.editing().unwrap().slots[2].mods, Mod::Hr);
    state.update(Message::Undo, 104);
    assert!(state.editing().unwrap().slots.iter().all(Slot::is_empty));
    assert_eq!(pools::load_all(&state.dir)[0].slots[2].mods, Mod::Hr);
}

#[test]
fn covers_include_imported_candidates_and_maps_without_local_art() {
    let mut state = state_with_songs("candidate-covers");
    state.update(Message::New, 100);
    state.finding = Some(Finding::Found(Candidate { found: found(&["a1", "b2"], None), choice: None, place: Place::End, cover: None }));
    Arc::make_mut(state.songs.as_mut().unwrap()).get_mut("b2").unwrap().background = None;
    let covers = state.covers();
    assert!(covers.contains(&("a1".into(), Some(PathBuf::from("/songs/Glass Orchard.jpg")))));
    assert!(covers.contains(&("b2".into(), None)));
}

#[test]
fn cached_measures_return_after_bulk_category_changes_without_a_new_answer() {
    let mut state = pool_with_maps("bulk-cache", &["a1", "b2"]);
    for hash in ["a1", "b2"] {
        state.update(Message::Measured(hash.into(), Mod::Nm, Ok(measure(4.5))), 101);
        state.update(Message::Measured(hash.into(), Mod::Dt, Ok(measure(5.6))), 102);
    }
    state.update(Message::Choosing(true), 103);
    state.update(Message::Mark(0), 104);
    state.update(Message::Mark(1), 105);
    for (mods, stars) in [(Mod::Dt, 5.6), (Mod::Nm, 4.5), (Mod::Dt, 5.6)] {
        assert!(state.update(Message::BulkMod(mods), 106).is_empty());
        assert!(state.editing().unwrap().slots[..2].iter().all(|slot| slot.measure.unwrap().stars == stars));
    }
    state.update(Message::Undo, 107);
    assert!(state.editing().unwrap().slots[..2].iter().all(|slot| slot.mods == Mod::Nm && slot.measure.unwrap().stars == 4.5));
}

#[test]
fn out_of_order_search_and_cancelled_downloads_cannot_replace_current_work() {
    let mut state = state_with_songs("network-stale");
    state.update(Message::New, 100);
    state.update(Message::Query("https://osu.ppy.sh/beatmaps/1".into()), 101);
    let old = state.resolve_request;
    state.update(Message::Query("https://osu.ppy.sh/beatmaps/2".into()), 102);
    state.update(Message::Resolved(old, Ok(found(&["wrong"], Some(0)))), 103);
    assert!(matches!(state.finding, Some(Finding::Asking)));
    state.update(Message::Resolved(state.resolve_request, Ok(found(&["zz9"], Some(0)))), 104);
    state.update(Message::Confirm, 105);
    let first = state.fetch_request;
    let stop = match &state.finding { Some(Finding::Fetching(fetching)) => fetching.stop.clone(), _ => panic!("download") };
    state.update(Message::Back, 106);
    assert!(stop.load(Ordering::SeqCst));
    state.update(Message::New, 107);
    state.update(Message::Query(LINK.into()), 108);
    state.update(Message::Resolved(state.resolve_request, Ok(found(&["zz9"], Some(0)))), 109);
    state.update(Message::Confirm, 110);
    let current = state.fetch_request;
    assert_ne!(first, current);
    state.update(Message::Step(first, "zz9".into(), crate::maps::Step::Done(map("Old", "A", "B"))), 111);
    assert_eq!(state.editing().unwrap().filled(), 0);
    assert!(matches!(state.finding, Some(Finding::Fetching(_))));
    state.update(Message::Step(current, "wrong".into(), crate::maps::Step::Nowhere), 112);
    assert!(matches!(state.finding, Some(Finding::Fetching(_))));
    state.update(Message::Step(current, "zz9".into(), crate::maps::Step::Done(map("Current", "A", "B"))), 113);
    assert_eq!(state.editing().unwrap().slots[0].title, "Current");
}

#[test]
fn pasting_links_over_focused_fields_keeps_their_text_and_uses_the_raw_link() {
    let mut state = state_with_songs("paste-fields");
    state.update(Message::New, 100);
    state.update(Message::Rename("My pool".into()), 101);
    state.update(Message::Put("a1".into()), 102);
    let id = state.editing().unwrap().id.clone();
    for input in [Input::Name, Input::Query, Input::Note(0)] {
        state.update(Message::PastedInto(id.clone(), input, format!("Existing {LINK}"), LINK.into()), 103);
        assert_eq!(state.editing().unwrap().name, "My pool");
        assert!(state.editing().unwrap().slots[0].note.is_empty());
        assert!(matches!(state.finding, Some(Finding::Asking)));
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.query == LINK));
    }
    state.update(Message::PastedInto(id.clone(), Input::Name, "My pool updated".into(), " updated".into()), 104);
    assert_eq!(state.editing().unwrap().name, "My pool updated");
    state.update(Message::New, 105);
    state.update(Message::PastedInto(id, Input::Name, "Wrong pool".into(), LINK.into()), 106);
    assert!(state.editing().unwrap().name.is_empty());
    assert!(state.finding.is_none());
}

#[test]
fn a_dropped_archive_offers_a_choice_and_waits_before_importing_the_next_file() {
    let mut state = state_with_songs("archive-choice");
    state.update(Message::New, 100);
    let effects = state.update(Message::Dropped("set.OSZ".into()), 101);
    let request = match effects.as_slice() { [Effect::ImportArchive(request, _)] => *request, _ => panic!("archive import") };
    state.update(Message::Dropped("next.osu".into()), 102);
    state.update(Message::ArchiveFile(request, Ok(vec![("e5".into(), map("Set", "Artist", "Easy")), ("f6".into(), map("Set", "Artist", "Hard"))])), 103);
    assert!(matches!(&state.finding, Some(Finding::Found(candidate)) if candidate.choice.is_none() && candidate.found.difficulties.len() == 2));
    assert!(state.importing.is_none());
    assert_eq!(state.dropped.len(), 1);
    state.update(Message::Measured("f6".into(), Mod::Nm, Ok(measure(5.6))), 104);
    assert!(matches!(&state.finding, Some(Finding::Found(candidate)) if candidate.found.difficulties[1].stars == 5.6));
    state.update(Message::Choose(1), 105);
    let next = state.update(Message::Confirm, 106);
    assert_eq!(state.editing().unwrap().filled(), 1);
    assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("f6"));
    assert_eq!(state.editing().unwrap().slots[0].measure.unwrap().stars, 5.6);
    assert!(next.iter().any(|effect| matches!(effect, Effect::ImportMap(_, _))));
    state.update(Message::Back, 107);
    state.update(Message::New, 108);
    state.update(Message::ArchiveFile(request, Ok(vec![("old".into(), map("Old", "Artist", "Easy"))])), 109);
    assert!(state.finding.is_none());
    assert_eq!(state.editing().unwrap().filled(), 0);
}

#[test]
fn dropped_maps_are_queued_keep_the_slot_mods_and_survive_a_later_songs_read() {
    let mut state = state_with_songs("dropped");
    let id = open_new(&mut state);
    state.update(Message::Select(Some(3)), 1_790_000_001);
    let first = state.update(Message::Dropped(PathBuf::from("first.osu")), 1_790_000_002);
    let request = match first.as_slice() {
        [Effect::ImportMap(request, path)] if path == &PathBuf::from("first.osu") => *request,
        _ => panic!("a chart is imported in the background"),
    };
    assert!(state.update(Message::Dropped(PathBuf::from("second.OSU")), 1_790_000_003).is_empty());
    let next = state.update(Message::MapFile(request, Ok(("e5".into(), map("First", "A", "Hard")))), 1_790_000_004);
    assert_eq!(state.editing().unwrap().slots[3].hash.as_deref(), Some("e5"));
    assert_eq!(state.editing().unwrap().slots[3].mods, Mod::Hr);
    assert!(next.iter().any(|effect| matches!(effect, Effect::Measure(hash, _, Mod::Hr) if hash == "e5")));
    let request = next.iter().find_map(|effect| match effect { Effect::ImportMap(request, _) => Some(*request), _ => None }).unwrap();
    state.update(Message::MapFile(request, Ok(("f6".into(), map("Second", "A", "Hard")))), 1_790_000_005);
    assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("f6"));
    state.update(Message::Songs(Arc::new(HashMap::new())), 1_790_000_006);
    assert!(state.songs.as_ref().unwrap().contains_key("e5"));
    assert!(state.songs.as_ref().unwrap().contains_key("f6"));
    let kept = pools::load_all(&state.dir);
    assert_eq!(kept.iter().find(|pool| pool.id == id).unwrap().slots[3].hash.as_deref(), Some("e5"));
    assert!(state.importing.is_none() && state.dropped.is_empty());
}

#[test]
fn an_import_answer_cannot_write_to_a_different_or_changed_pool() {
    let mut state = state_with_songs("dropped-stale");
    open_new(&mut state);
    state.update(Message::Dropped(PathBuf::from("first.osu")), 1_790_000_001);
    let request = state.importing.as_ref().unwrap().request;
    state.update(Message::New, 1_790_000_002);
    state.update(Message::UseFrame(Frame::Duel), 1_790_000_002);
    state.update(Message::MapFile(request, Ok(("e5".into(), map("First", "A", "Hard")))), 1_790_000_003);
    assert_eq!(state.editing().unwrap().filled(), 0);
    state.update(Message::Dropped(PathBuf::from("first.osu")), 1_790_000_004);
    let request = state.importing.as_ref().unwrap().request;
    state.update(Message::SetMod(0, Mod::Dt), 1_790_000_005);
    state.update(Message::MapFile(request, Ok(("e5".into(), map("First", "A", "Hard")))), 1_790_000_006);
    assert_eq!(state.editing().unwrap().filled(), 0);
    assert!(state.importing.is_none());
    assert!(matches!(state.update(Message::Dropped(PathBuf::from("pool.POOL")), 1_790_000_007).as_slice(), [Effect::ReadPool(_, _)]));
    let request = state.importing_pool.unwrap();
    state.update(Message::PoolFile(request, Err(pool_share::Refused::NotAPool)), 1_790_000_008);
    assert!(matches!(state.screen, Screen::Shelf));
    assert_eq!(state.refused, Some(pool_share::Refused::NotAPool));
    state.update(Message::Dropped(PathBuf::from("pool.POOL")), 1_790_000_009);
    let request = state.importing_pool.unwrap();
    state.update(Message::New, 1_790_000_010);
    state.update(Message::PoolFile(request, Ok(Pool::new(Frame::Duel, "Late", 1_790_000_011))), 1_790_000_011);
    assert!(matches!(state.screen, Screen::Editor(_)));
    assert_ne!(state.editing().unwrap().name, "Late");
}

#[test]
fn a_bad_dropped_map_reports_the_reason_and_continues_with_the_next_file() {
    let mut state = state_with_songs("dropped-error");
    state.screen = Screen::Shelf;
    state.update(Message::Dropped(PathBuf::from("bad.osu")), 1_790_000_001);
    let request = state.importing.as_ref().unwrap().request;
    state.update(Message::Dropped(PathBuf::from("good.osu")), 1_790_000_002);
    let next = state.update(Message::MapFile(request, Err(crate::pool_files::Refused::Mode)), 1_790_000_003);
    assert!(matches!(next.as_slice(), [Effect::Say("pool-file-mode"), Effect::ImportMap(_, _)]));
    assert_eq!(state.editing().unwrap().filled(), 0);
}

#[test]
fn best_plays_keep_one_play_per_map_and_choose_local_or_remote_maps() {
    let mut state = state_with_songs("best");
    let hash = "0123456789abcdef0123456789abcdef";
    Arc::make_mut(state.songs.as_mut().unwrap()).insert(hash.into(), map("A", "B", "C"));
    let score = |id, pp, hash: &str| crate::community::wire::Score { beatmap_id: id, pp, hash: hash.into(), ..Default::default() };
    state.set_best(Some(&[
        score(1.0, 200.0, &hash.to_ascii_uppercase()), score(1.0, 100.0, hash), score(0.0, 80.0, hash), score(2.0, 150.0, ""), score(0.0, 500.0, ""), score(3.5, 500.0, ""), score(4.0, f64::NAN, ""),
    ]));
    assert_eq!(state.best.as_ref().unwrap().len(), 2);
    open_new(&mut state);
    assert!(state.update(Message::Source(SourceTab::Best), 1_790_000_001).is_empty());
    state.update(Message::Best(0), 1_790_000_002);
    assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some(hash));
    state.update(Message::Best(0), 1_790_000_003);
    assert_eq!(state.notice, Some(Notice::Already(0)));
    let effects = state.update(Message::Best(1), 1_790_000_004);
    assert!(matches!(effects.as_slice(), [Effect::Resolve(_, Target::Beatmap { id: 2, .. })]));
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.query == "https://osu.ppy.sh/beatmaps/2"));
    state.update(Message::Resolved(state.resolve_request, Err(Why::Silent("offline".into()))), 1_790_000_005);
    assert!(matches!(state.update(Message::Retry, 1_790_000_006).as_slice(), [Effect::Resolve(_, Target::Beatmap { id: 2, .. })]));
    let downloaded = "fedcba9876543210fedcba9876543210";
    state.update(Message::Resolved(state.resolve_request, Ok(pool_links::Found {
        set: 10, artist: "B".into(), title: "A".into(), picked: Some(0),
        difficulties: vec![pool_links::Difficulty { id: 2, hash: downloaded.into(), version: "C".into(), stars: 5.0 }],
    })), 1_790_000_007);
    assert_eq!(state.best.as_ref().unwrap()[1].hash, downloaded);
    assert!(matches!(state.update(Message::Confirm, 1_790_000_008).as_slice(), [Effect::Fetch(_, hash, _)] if hash == downloaded));
    state.update(Message::Step(state.fetch_request, downloaded.into(), crate::maps::Step::Done(map("A", "B", "C"))), 1_790_000_009);
    assert_eq!(state.editing().unwrap().slots[1].hash.as_deref(), Some(downloaded));
    assert!(state.update(Message::Best(1), 1_790_000_010).is_empty());
    assert_eq!(state.notice, Some(Notice::Already(1)));
    state.set_best(None);
    assert!(matches!(state.update(Message::Source(SourceTab::Best), 1_790_000_011).as_slice(), [Effect::ReadBest(false)]));
    assert!(matches!(state.update(Message::RefreshBest, 1_790_000_012).as_slice(), [Effect::ReadBest(true)]));
}

#[test]
fn collections_add_local_maps_and_resolve_missing_hashes() {
    let mut state = state_with_songs("collections");
    open_new(&mut state);
    assert!(matches!(state.update(Message::AddPanel(true), 1_790_000_001).as_slice(), []));
    assert!(matches!(state.update(Message::Source(SourceTab::Collections), 1_790_000_002).as_slice(), [Effect::ReadCollections]));
    let missing = "0123456789abcdef0123456789abcdef".to_owned();
    state.update(Message::Collections(vec![Collection { name: "Favorites".into(), hashes: vec!["a1".into(), missing.clone()] }]), 1_790_000_003);
    state.update(Message::Collection(Some(0)), 1_790_000_004);
    state.update(Message::CollectionHash("a1".into()), 1_790_000_005);
    assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("a1"));
    let effects = state.update(Message::CollectionHash(missing.clone()), 1_790_000_006);
    assert!(matches!(effects.as_slice(), [Effect::Resolve(_, Target::Hash(hash))] if hash == &missing));
    assert!(matches!(state.finding, Some(Finding::Asking)));
}

#[test]
fn suggestions_match_neighbours_ignore_stale_answers_and_fill_the_empty_slot() {
    let mut state = state_with_songs("suggestions");
    open_new(&mut state);
    state.update(Message::Put("a1".into()), 1_790_000_001);
    state.update(Message::Measured("a1".into(), Mod::Nm, Ok(measure(5.0))), 1_790_000_002);
    let effects = state.update(Message::Suggest(1), 1_790_000_003);
    let request = match effects.as_slice() {
        [Effect::SuggestRemote(..), Effect::Suggest(request, _, Mod::Nm, target, excluded, _)] if *target == 5.0 && excluded.contains("a1") => *request,
        _ => panic!("the empty slot needs a background suggestion scan"),
    };
    state.update(Message::Suggested(request + 1, vec![("b2".into(), measure(5.1))]), 1_790_000_004);
    assert!(state.suggestions.as_ref().unwrap().maps.is_none());
    state.update(Message::Suggested(request, vec![("b2".into(), measure(5.1))]), 1_790_000_005);
    assert_eq!(state.suggestions.as_ref().unwrap().maps.as_ref().unwrap().len(), 1);
    state.update(Message::Put("b2".into()), 1_790_000_006);
    assert_eq!(state.editing().unwrap().slots[1].hash.as_deref(), Some("b2"));
    assert_eq!(state.editing().unwrap().slots[1].measure.unwrap().stars, 5.1);
    assert!(state.suggestions.is_none());
    assert!(matches!(state.screen, Screen::Editor(Editor { panel: Panel::Slot, .. })));
}

#[test]
fn suggestion_target_uses_the_nearest_maps_on_both_sides() {
    let mut pool = Pool::new(Frame::Duel, "Test", 1);
    pool.slots[0].measure = Some(measure(4.0));
    pool.slots[2].measure = Some(measure(6.0));
    pool.slots[5].measure = Some(measure(8.0));
    assert_eq!(suggestion_target(&pool, 1), Some(5.0));
    assert_eq!(suggestion_target(&pool, 3), Some(7.0));
    assert_eq!(suggestion_target(&pool, 6), Some(8.0));
    assert_eq!(suggestion_target(&Pool::new(Frame::Duel, "Empty", 1), 1), None);
}

#[test]
fn a_new_pool_opens_in_the_editor_and_an_untouched_one_is_removed_on_leaving() {
    let mut state = state_with_songs("untouched");
    let id = open_new(&mut state);
    assert_eq!(state.list.len(), 1);
    assert_eq!(state.editing().unwrap().slots.len(), 7);
    assert_eq!(pools::load_all(&state.dir).len(), 1, "it is on the disk at once");
    state.update(Message::Back, 1_790_000_001);
    assert!(state.list.is_empty());
    assert!(pools::load_all(&state.dir).is_empty());
    assert_eq!(state.screen, Screen::Shelf);
    assert!(!state.dir.join(format!("{id}.pool")).exists());
}

#[test]
fn a_named_pool_stays_when_it_has_no_maps_yet() {
    let mut state = state_with_songs("named");
    open_new(&mut state);
    state.update(Message::Rename("Spring".into()), 1_790_000_002);
    state.update(Message::Back, 1_790_000_003);
    assert_eq!(state.list.len(), 1);
    assert_eq!(pools::load_all(&state.dir)[0].name, "Spring");
}

#[test]
fn the_frame_can_be_changed_only_while_the_pool_is_empty() {
    let mut state = state_with_songs("frames");
    open_new(&mut state);
    state.update(Message::UseFrame(Frame::Stage), 1_790_000_002);
    assert_eq!(state.editing().unwrap().slots.len(), 12);
    state.update(Message::UseFrame(Frame::Free), 1_790_000_003);
    assert!(state.editing().unwrap().slots.is_empty());
    state.update(Message::Put("a1".into()), 1_790_000_004);
    state.update(Message::UseFrame(Frame::Duel), 1_790_000_005);
    assert_eq!(state.editing().unwrap().frame, Frame::Free, "a pool with a map keeps its frame");
}

#[test]
fn a_map_goes_to_the_first_empty_slot_then_to_the_selected_one() {
    let mut state = state_with_songs("put");
    open_new(&mut state);
    state.update(Message::Put("a1".into()), 1_790_000_002);
    state.update(Message::Put("b2".into()), 1_790_000_003);
    let pool = state.editing().unwrap();
    assert_eq!((pool.slots[0].hash.as_deref(), pool.slots[1].hash.as_deref()), (Some("a1"), Some("b2")));
    assert_eq!(pool.slots[0].title, "Glass Orchard");
    assert_eq!(state.editing().unwrap().filled(), 2, "the second map does not replace the first");
    state.update(Message::Select(Some(0)), 1_790_000_004);
    state.update(Message::Put("c3".into()), 1_790_000_004);
    assert_eq!(state.editing().unwrap().slots[2].hash.as_deref(), Some("c3"), "a filled slot that is only selected is not replaced");
    state.update(Message::Select(Some(0)), 1_790_000_005);
    state.update(Message::Replace, 1_790_000_005);
    state.update(Message::Put("d4".into()), 1_790_000_005);
    let pool = state.editing().unwrap();
    assert_eq!(pool.slots[0].hash.as_deref(), Some("d4"), "replacing is done on request");
    assert_eq!(pool.filled(), 3);
    assert_eq!(pools::load_all(&state.dir)[0].slots[0].hash.as_deref(), Some("d4"));
}

#[test]
fn a_map_already_in_the_pool_is_refused_with_its_slot() {
    let mut state = state_with_songs("twice");
    open_new(&mut state);
    state.update(Message::Put("a1".into()), 1_790_000_002);
    state.update(Message::Select(None), 1_790_000_003);
    state.update(Message::Put("a1".into()), 1_790_000_004);
    assert_eq!(state.notice, Some(Notice::Already(0)));
    assert_eq!(state.editing().unwrap().filled(), 1);
    state.update(Message::Query("g".into()), 1_790_000_005);
    assert_eq!(state.notice, None, "typing clears the notice");
}

#[test]
fn a_full_frame_grows_by_a_slot_and_a_free_pool_grows_from_nothing() {
    let mut state = state_with_songs("grow");
    open_new(&mut state);
    for (at, hash) in ["a1", "b2", "c3", "d4"].into_iter().enumerate() {
        state.update(Message::Select(None), 1_790_000_002 + at as i64);
        state.update(Message::Put(hash.into()), 1_790_000_010 + at as i64);
    }
    assert_eq!(state.editing().unwrap().filled(), 4);
    let mut free = state_with_songs("free");
    open_new(&mut free);
    free.update(Message::UseFrame(Frame::Free), 1_790_000_002);
    free.update(Message::Put("a1".into()), 1_790_000_003);
    assert_eq!(free.editing().unwrap().slots.len(), 1);
    assert_eq!(free.editing().unwrap().slots[0].mods, Mod::Nm);
    free.update(Message::Clear(0), 1_790_000_004);
    assert!(free.editing().unwrap().slots.is_empty(), "removing a map from a free pool removes its slot");
}

#[test]
fn clearing_a_slot_in_a_frame_keeps_the_slot_and_its_mod() {
    let mut state = state_with_songs("clear");
    open_new(&mut state);
    state.update(Message::Put("a1".into()), 1_790_000_002);
    state.update(Message::SetMod(0, Mod::Hd), 1_790_000_003);
    state.update(Message::Clear(0), 1_790_000_004);
    let pool = state.editing().unwrap();
    assert_eq!(pool.slots.len(), 7);
    assert!(pool.slots[0].is_empty());
    assert_eq!(pool.slots[0].mods, Mod::Hd);
}

#[test]
fn a_measure_arriving_fills_the_slots_and_reaches_the_disk() {
    let mut state = state_with_songs("measure");
    open_new(&mut state);
    let effects = state.update(Message::Put("a1".into()), 1_790_000_002);
    assert!(matches!(effects.as_slice(), [Effect::Measure(hash, _, Mod::Nm)] if hash == "a1"));
    assert!(state.editing().unwrap().slots[0].measure.is_none());
    state.update(Message::Measured("a1".into(), Mod::Nm, Ok(measure(4.5))), 1_790_000_003);
    assert_eq!(state.editing().unwrap().slots[0].measure, Some(measure(4.5)));
    assert_eq!(pools::load_all(&state.dir)[0].slots[0].measure, Some(measure(4.5)));
    let again = state.update(Message::Put("a1".into()), 1_790_000_004);
    assert!(again.is_empty(), "a map measured once is not measured again");
}

#[test]
fn changing_the_mod_asks_for_a_new_measure_and_keeps_the_old_ones_cached() {
    let mut state = state_with_songs("mods");
    open_new(&mut state);
    state.update(Message::Put("a1".into()), 1_790_000_002);
    state.update(Message::Measured("a1".into(), Mod::Nm, Ok(measure(4.5))), 1_790_000_003);
    let effects = state.update(Message::SetMod(0, Mod::Dt), 1_790_000_004);
    assert!(matches!(effects.as_slice(), [Effect::Measure(_, _, Mod::Dt)]));
    assert!(state.editing().unwrap().slots[0].measure.is_none());
    state.update(Message::Measured("a1".into(), Mod::Dt, Ok(measure(5.6))), 1_790_000_005);
    assert_eq!(state.editing().unwrap().slots[0].measure.unwrap().stars, 5.6);
    let back = state.update(Message::SetMod(0, Mod::Nm), 1_790_000_006);
    assert!(back.is_empty(), "the old measure comes from the cache");
    assert_eq!(state.editing().unwrap().slots[0].measure.unwrap().stars, 4.5);
    assert_eq!(pools::load_all(&state.dir)[0].slots[0].measure.unwrap().stars, 4.5);
    state.update(Message::Undo, 1_790_000_007);
    assert_eq!(state.editing().unwrap().slots[0].mods, Mod::Dt);
    assert_eq!(state.editing().unwrap().slots[0].measure.unwrap().stars, 5.6);
}

#[test]
fn opening_the_shelf_reads_songs_once_and_a_pool_from_an_older_calculation_is_measured_again() {
    let dir = scratch("old");
    let mut pool = Pool::new(Frame::Duel, "Old", 1_790_000_000);
    pool.slots[0].hash = Some("a1".into());
    pool.slots[0].measure = Some(measure(3.0));
    pool.calc = 0;
    pools::save(&dir, &pool).unwrap();
    let mut state = State::new(dir);
    let first = state.open();
    assert!(matches!(first.as_slice(), [Effect::ReadSongs, Effect::Catalogue(false, 0)]));
    assert!(state.list[0].slots[0].measure.is_none());
    assert_eq!(state.list[0].calc, pools::CALC_VERSION);
    assert!(state.open().is_empty(), "reading is already under way");
    let mut songs = HashMap::new();
    songs.insert("a1".to_owned(), map("Glass Orchard", "Nova Tide", "Garden"));
    let after = state.update(Message::Songs(Arc::new(songs)), 1_790_000_100);
    assert!(matches!(after.as_slice(), [Effect::Measure(hash, _, Mod::Nm)] if hash == "a1"));
}

#[test]
fn search_needs_every_word_and_lists_at_most_forty_by_title() {
    let state = state_with_songs("search");
    let found: Vec<String> = state.search("nova  glass").into_iter().map(|(_, map)| map.title).collect();
    assert_eq!(found, ["Glass Harbour", "Glass Orchard"]);
    assert!(state.search("   ").is_empty());
    assert!(state.search("zzz").is_empty());
    let mut many = HashMap::new();
    for at in 0..100 {
        many.insert(format!("h{at:03}"), map(&format!("Song {at:03}"), "Artist", "Normal"));
    }
    let mut crowded = State::new(scratch("crowd"));
    crowded.songs = Some(Arc::new(many));
    let listed = crowded.search("song");
    assert_eq!(listed.len(), RESULTS_MOST);
    assert!(listed.windows(2).all(|pair| pair[0].1.title <= pair[1].1.title));
}

#[test]
fn covers_ask_for_the_first_four_maps_of_each_pool_on_the_shelf() {
    let mut state = state_with_songs("covers");
    open_new(&mut state);
    for hash in ["a1", "b2", "c3", "d4"] {
        state.update(Message::Select(None), 1_790_000_002);
        state.update(Message::Put(hash.into()), 1_790_000_003);
    }
    state.update(Message::Rename("Four".into()), 1_790_000_004);
    state.update(Message::Back, 1_790_000_005);
    let wanted: Vec<String> = state.covers().into_iter().map(|(hash, _)| hash).collect();
    assert_eq!(wanted.len(), 4);
    assert!(wanted.contains(&"a1".to_owned()) && wanted.contains(&"d4".to_owned()));
}

#[test]
fn a_map_that_is_not_on_the_disk_keeps_its_label_and_has_no_measure() {
    let mut state = state_with_songs("away");
    open_new(&mut state);
    state.update(Message::Put("a1".into()), 1_790_000_002);
    state.update(Message::Measured("a1".into(), Mod::Nm, Err("gone".into())), 1_790_000_003);
    let slot = &state.editing().unwrap().slots[0];
    assert_eq!(slot.title, "Glass Orchard");
    assert!(slot.measure.is_none());
    assert!(matches!(state.measures.get("a1", Mod::Nm), Some(Err(_))));
}

fn found(hashes: &[&str], picked: Option<usize>) -> pool_links::Found {
    pool_links::Found {
        set: 77,
        artist: "Nova Tide".into(),
        title: "Glass Orchard".into(),
        difficulties: hashes.iter().enumerate().map(|(at, hash)| pool_links::Difficulty { id: 100 + at as u64, hash: (*hash).into(), version: format!("Diff {at}"), stars: 3.0 + at as f64 }).collect(),
        picked,
    }
}

const LINK: &str = "https://osu.ppy.sh/beatmapsets/77#osu/100";

fn in_conflict(name: &str) -> (State, String, crate::bot::Publication) {
    let (mut state, id) = ready_to_publish(name);
    state.update(Message::Published(id.clone(), Ok(publication(&id, 3))), 1_790_000_110);
    state.update(Message::PublishSheet(true), 1_790_000_111);
    state.update(Message::Publish, 1_790_000_112);
    let effects = state.update(Message::Published(id.clone(), Err("409".into())), 1_790_000_113);
    assert!(matches!(effects.as_slice(), [Effect::Catalogue(false, 0)]), "the catalogue is asked what the server holds: {effects:?}");
    assert_eq!(state.conflict.as_deref(), Some(id.as_str()));
    assert!(state.catalogue_error.is_none());
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.publish), "the sheet steps aside for the conflict");
    let mut theirs = state.editing().unwrap().clone();
    theirs.name = "Spring duel remote".into();
    let row = crate::bot::Publication { content: serde_json::from_slice(&pool_share::to_file(&theirs)).unwrap(), name: theirs.name.clone(), ..publication(&id, 4) };
    state.update(Message::CatalogueLoaded(false, false, Ok(vec![row.clone()])), 1_790_000_114);
    assert_eq!(state.server_revision(state.editing().unwrap()), Some(4));
    assert_eq!(state.conflict.as_deref(), Some(id.as_str()), "knowing the server version does not settle anything");
    (state, id, row)
}

#[test]
fn a_conflict_is_told_with_both_versions_and_two_ways_out() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let (state, _, _) = in_conflict("conflict-told");
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(&state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
    assert!(screen.find(words.t("catalogue-conflict-title")).is_ok());
    assert!(screen.find("the server has version 4, you have version 3").is_ok());
    assert!(screen.find(words.t("catalogue-conflict")).is_err(), "the bare sentence gives way to the banner");
    screen.click(words.t("catalogue-conflict-mine").as_str()).unwrap();
    assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Conflict(Keep::Mine)]));
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(&state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
    screen.click(words.t("catalogue-conflict-theirs").as_str()).unwrap();
    assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Conflict(Keep::Theirs)]));
}

#[test]
fn either_way_out_of_a_conflict_keeps_both_versions_and_opens_the_chosen_one() {
    for keep in [Keep::Theirs, Keep::Mine] {
        let (mut state, id, row) = in_conflict(&format!("conflict-{keep:?}"));
        let effects = state.update(Message::Conflict(keep), 1_790_000_120);
        assert!(matches!(effects.as_slice(), [Effect::Catalogue(false, 0)]), "{keep:?}: {effects:?}");
        assert!(state.update(Message::Conflict(keep), 1_790_000_121).is_empty(), "{keep:?}: one answer at a time");
        let effects = state.update(Message::CatalogueLoaded(false, false, Ok(vec![row])), 1_790_000_122);
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Say("catalogue-conflict-kept"))), "{keep:?}: {effects:?}");
        assert!(state.conflict.is_none() && state.resolving.is_none());
        let theirs = state.list.iter().find(|pool| pool.id == id).unwrap();
        assert_eq!((theirs.name.as_str(), theirs.published_revision), ("Spring duel remote", 4), "{keep:?}");
        let mine = state.list.iter().find(|pool| pool.id != id && pool.name == "Spring duel v3").expect("my variant is kept under its own name");
        assert_eq!(mine.published_revision, 0, "{keep:?}: the copy is a fresh draft");
        assert_eq!(mine.filled(), 2);
        let opened = state.editing().unwrap().id.clone();
        assert_eq!(opened == id, keep == Keep::Theirs, "{keep:?}");
        let kept = pools::load_all(&state.dir);
        assert!(kept.iter().any(|pool| pool.name == "Spring duel v3") && kept.iter().any(|pool| pool.name == "Spring duel remote"), "{keep:?}: both are on disk");
    }
}

#[test]
fn a_conflict_over_a_removed_publication_lets_the_pool_be_published_anew() {
    let (mut state, id, _) = in_conflict("conflict-gone");
    state.update(Message::Conflict(Keep::Theirs), 1_790_000_120);
    let effects = state.update(Message::CatalogueLoaded(false, false, Ok(Vec::new())), 1_790_000_121);
    assert!(matches!(effects.as_slice(), [Effect::Say("catalogue-conflict-gone")]), "{effects:?}");
    assert_eq!(state.editing().unwrap().published_revision, 0);
    assert_eq!(state.list.iter().filter(|pool| pool.id == id).count(), 1);
    assert!(state.conflict.is_none());
    let (mut failing, _, _) = in_conflict("conflict-offline");
    failing.update(Message::Conflict(Keep::Mine), 1_790_000_120);
    failing.update(Message::CatalogueLoaded(false, false, Err("503".into())), 1_790_000_121);
    assert!(failing.resolving.is_none() && failing.conflict.is_some(), "a failed refresh leaves the question open");
}

#[test]
fn the_tour_follows_what_was_done_waits_at_the_told_steps_and_ends_with_sharing() {
    let mut state = state_with_songs("guide-steps");
    state.guide = pools::Guide::default();
    state.update(Message::New, 100);
    state.update(Message::Begin(Start::Empty), 100);
    let step = |state: &State| state.guide.current(state.editing().unwrap());
    assert_eq!(step(&state), Some(0));
    let effects = state.update(Message::Rename("Spring duel".into()), 101);
    assert!(effects.iter().all(|effect| !matches!(effect, Effect::Guide(_))), "a name is read from the pool, nothing is kept for it");
    assert_eq!(step(&state), Some(1));
    state.update(Message::Put("a1".into()), 102);
    state.update(Message::Put("b2".into()), 103);
    assert_eq!(step(&state), Some(2));
    let effects = state.update(Message::SetMod(0, Mod::Hd), 104);
    assert!(matches!(effects.iter().filter(|effect| matches!(effect, Effect::Guide(_))).collect::<Vec<_>>().as_slice(), [Effect::Guide(guide)] if guide.modded && guide.on), "{effects:?}");
    let keys = slot_keys(state.editing().unwrap());
    let id = state.editing().unwrap().id.clone();
    let effects = state.update(Message::Move(id, keys.clone(), keys[1], Some(keys[0])), 105);
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Guide(guide) if guide.moved)));
    assert_eq!(step(&state), Some(4), "the menu, the tools and the backdrop are told, not done");
    for told in [5, 6, 7] {
        let effects = state.update(Message::GuideNext, 106);
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Guide(_))), "the place in the tour is kept");
        assert_eq!(step(&state), Some(told));
    }
    let effects = state.update(Message::Share(true), 107);
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Guide(guide) if guide.shared && !guide.on)), "sharing ends the hints: {effects:?}");
    assert!(!state.guide.on);
    assert!(state.update(Message::Share(false), 108).iter().all(|effect| !matches!(effect, Effect::Guide(_))));
}

#[test]
fn the_tour_hint_stands_on_the_screen_beside_what_it_tells_about_and_can_be_hidden() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut state = pool_with_maps("guide-card", &["a1", "b2"]);
    state.update(Message::Select(None), 1_790_000_100);
    let seen = |state: &State, width: f32, label: &str| iced_test::Simulator::with_size(crate::settings(), iced::Size::new(width, 900.0), view(state, &words, &thumbs, width, 1.0, &Clocks::settled())).find(label).is_ok();
    assert!(!seen(&state, 1300.0, &words.t("tour-name")), "hints are off until the settings say otherwise");
    state.guide = pools::Guide::default();
    for width in [980.0, 1300.0, 2096.0] {
        assert!(seen(&state, width, &words.t("tour-name")) && seen(&state, width, &words.t("tour-name-how")), "{width}");
        assert!(seen(&state, width, &words.with("guide-step-of", &[("n", "1".into()), ("total", "8".into())])), "{width}");
        assert!(!seen(&state, width, &words.t("tour-mod")), "{width}: one hint at a time");
    }
    state.update(Message::Rename("Spring duel".into()), 1_790_000_101);
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(&state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
    let row = screen.find("Glass Orchard").unwrap().visible_bounds().unwrap();
    let hint = screen.find(words.t("tour-mod")).unwrap().visible_bounds().unwrap();
    assert!(hint.y > row.y && hint.y - row.y < 90.0, "the hint about the mod stands right under the first map: {row:?} {hint:?}");
    screen.click(words.t("tour-next").as_str()).unwrap();
    assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::GuideNext]));
    let at = state.editing_at().unwrap();
    state.list[at].collection = true;
    assert!(!seen(&state, 1300.0, &words.t("tour-mod")), "a collection has no tour");
    state.list[at].collection = false;
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(&state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
    screen.click(words.t("guide-hide").as_str()).unwrap();
    assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::HideGuide]));
    let effects = state.update(Message::HideGuide, 1_790_000_102);
    assert!(matches!(effects.as_slice(), [Effect::Guide(guide)] if !guide.on));
    assert!(!seen(&state, 1300.0, &words.t("tour-mod")));
}

fn skilled(aim: f64, speed: f64, reading: f64, stamina: f64) -> Measure {
    Measure { aim, speed, reading, stamina, ..measure(5.0) }
}


#[test]
fn asking_for_a_map_of_a_skill_opens_the_suggestions_ordered_by_that_skill() {
    let mut state = pool_with_maps("balance-lean", &["a1", "b2"]);
    state.update(Message::Select(None), 1_790_000_100);
    let at = state.editing_at().unwrap();
    state.list[at].slots[0].measure = Some(skilled(4.0, 1.0, 0.3, 0.7));
    state.list[at].slots[1].measure = Some(skilled(3.6, 1.2, 0.4, 0.8));
    let effects = state.update(Message::Lean(Skill::Reading), 1_790_000_101);
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Suggest(..))), "{effects:?}");
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.panel == Panel::Add && editor.source == SourceTab::Suggest && editor.lean == Some(Skill::Reading)));
    let request = state.suggestions.as_ref().unwrap().request;
    let found: Vec<(String, Measure)> = (0..12).map(|at| (format!("m{at}"), skilled(3.0, 1.0, at as f64 * 0.2, 1.0))).collect();
    state.update(Message::Suggested(request, found), 1_790_000_102);
    let maps = state.suggestions.as_ref().unwrap().maps.clone().unwrap();
    assert_eq!(maps.len(), pools::SUGGESTED);
    assert_eq!(maps[0].0, "m11", "the map with the most reading comes first");
    assert!(maps.windows(2).all(|pair| pair[0].1.shares()[2] >= pair[1].1.shares()[2]));
    state.update(Message::Select(None), 1_790_000_103);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.lean.is_none()), "the leaning ends with the panel");
}

#[test]
fn the_checks_before_publishing_tell_what_stops_it_and_what_is_only_worth_knowing() {
    let mut state = state_with_songs("publish-checks");
    state.update(Message::New, 100);
    state.update(Message::Begin(Start::Empty), 100);
    let id = state.editing().unwrap().id.clone();
    let told = |state: &State| publish_checks(state.editing().unwrap()).into_iter().map(|check| (check.verdict, check.key)).collect::<Vec<_>>();
    assert_eq!(told(&state), vec![(Verdict::Stop, "catalogue-sign-in"), (Verdict::Stop, "publish-name-missing"), (Verdict::Stop, "publish-no-maps"), (Verdict::Note, "publish-no-mappoolers")]);
    assert!(!publish_ready(state.editing().unwrap()));
    state.update(Message::OwnCompiler(id, "NaumRedlo".into()), 101);
    state.update(Message::Rename("Spring duel".into()), 102);
    state.update(Message::Put("a1".into()), 103);
    assert_eq!(told(&state), vec![(Verdict::Fine, "publish-name-set"), (Verdict::Note, "publish-filled"), (Verdict::Note, "publish-unmeasured"), (Verdict::Note, "publish-no-mappoolers")]);
    let checks = publish_checks(state.editing().unwrap());
    assert_eq!((checks[1].count, checks[1].slot), (Some((1, 4)), Some(1)), "the first empty slot can be shown");
    assert_eq!((checks[2].count, checks[2].slot), (Some((1, 1)), Some(0)), "so can the first map still being measured");
    assert!(publish_ready(state.editing().unwrap()), "notes do not stop a publication");
    let words = Words::new(crate::lang::Lang::En);
    assert_eq!(checks[1].words(&words), "1 slot of 4 is filled");
    let at = state.editing_at().unwrap();
    state.list[at].slots.truncate(1);
    state.list[at].slots[0].measure = Some(measure(4.5));
    state.list[at].authors = vec!["kotofey".into()];
    assert_eq!(told(&state), vec![(Verdict::Fine, "publish-name-set"), (Verdict::Fine, "publish-filled"), (Verdict::Fine, "publish-measured")]);
    state.list[at].collection = true;
    assert_eq!(told(&state), vec![(Verdict::Fine, "publish-name-set"), (Verdict::Fine, "publish-maps")], "a collection has no slots to fill, no difficulty and no mappoolers");
}

fn ready_to_publish(name: &str) -> (State, String) {
    let mut state = pool_with_maps(name, &["a1", "b2"]);
    let id = state.editing().unwrap().id.clone();
    state.update(Message::Rename("Spring duel".into()), 1_790_000_100);
    state.update(Message::OwnCompiler(id.clone(), "NaumRedlo".into()), 1_790_000_101);
    state.update(Message::Select(None), 1_790_000_102);
    (state, id)
}

fn publication(id: &str, revision: u64) -> crate::bot::Publication {
    crate::bot::Publication { code: String::new(), id: format!("pub-{id}"), kind: "pool".into(), local_id: id.to_owned(), revision, name: "Spring duel".into(), content: serde_json::json!({}), mine: true }
}

#[test]
fn the_publication_sheet_opens_from_the_toolbar_sends_only_a_ready_pool_and_closes_once_published() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let (mut state, id) = ready_to_publish("publish-sheet");
    let pressed = |state: &State, label: &str| -> Vec<Message> {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
        screen.click(label).unwrap_or_else(|_| panic!("{label} is on screen"));
        screen.into_messages().collect()
    };
    assert!(matches!(pressed(&state, &words.t("catalogue-publish")).as_slice(), [Message::PublishSheet(true)]), "the toolbar asks first");
    state.update(Message::Select(Some(0)), 1_790_000_110);
    state.update(Message::PublishSheet(true), 1_790_000_111);
    assert_eq!(state.panel_key(), "publish");
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.publish && editor.panel == Panel::Closed), "the sheet replaces an open panel");
    {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(&state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
        for key in ["publish-before"] {
            assert!(screen.find(words.t(key)).is_ok(), "{key}");
        }
        for key in ["publish-name-set", "publish-no-mappoolers", "publish-show-slot", "publish-cancel"] {
            assert!(screen.find(words.t(key)).is_ok(), "{key}");
        }
        assert!(screen.find(words.t("catalogue-withdraw")).is_err(), "nothing is published yet");
    }
    let at = state.editing_at().unwrap();
    let name = std::mem::take(&mut state.list[at].name);
    assert!(state.update(Message::Publish, 1_790_000_112).is_empty(), "a pool without a name stays home");
    state.list[at].name = name;
    let effects = state.update(Message::Publish, 1_790_000_113);
    assert!(matches!(effects.as_slice(), [Effect::Publish(pool)] if pool.id == id));
    state.update(Message::Published(id.clone(), Err("503".into())), 1_790_000_114);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.publish), "a refusal is read on the sheet");
    assert_eq!(state.catalogue_error.as_deref(), Some("503"));
    state.update(Message::Publish, 1_790_000_115);
    let effects = state.update(Message::Published(id.clone(), Ok(publication(&id, 1))), 1_790_000_116);
    assert!(matches!(effects.as_slice(), [Effect::Say("catalogue-published-now")]), "{effects:?}");
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.publish));
    assert_eq!(state.panel_key(), "none");
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.gone.as_ref().is_some_and(|was| was.publish)), "the sheet fades out like the panels do");
    assert_eq!(state.editing().unwrap().published_revision, 1);
}

#[test]
fn the_sheets_grow_only_in_a_large_window() {
    assert_eq!(sheet_scale(929.0), 1.0);
    assert_eq!(sheet_scale(1334.0), 1.0);
    assert_eq!(sheet_scale(1600.0), 1.0);
    assert!((sheet_scale(1867.0) - 1.167).abs() < 0.01);
    assert_eq!(sheet_scale(2096.0), SHEET_GROWTH);
    let words = Words::new(crate::lang::Lang::Ru);
    let (mut state, _) = ready_to_publish("sheet-grown");
    state.update(Message::PublishSheet(true), 1_790_000_110);
    let button = |width: f32| {
        let image = picture(&state, &words, iced::Size::new(width, 1100.0), iced::Point::new(4.0, 4.0), &Clocks::settled(), &format!("sheet-grown-{width}"));
        let scale = image.width() as f32 / width;
        let red = |pixel: &::image::Rgba<u8>| pixel.0[0] > 190 && pixel.0[1] < 110 && pixel.0[2] < 110;
        let rows: Vec<u32> = (image.height() / 3..image.height()).filter(|y| (0..image.width()).filter(|x| red(image.get_pixel(*x, *y))).count() as f32 > 60.0 * scale).collect();
        assert!(!rows.is_empty(), "{width}: the main button of the sheet is drawn");
        (rows[rows.len() - 1] - rows[0] + 1) as f32 / scale
    };
    let (small, large) = (button(1400.0), button(2096.0));
    assert!(large > small * 1.2, "the sheet is larger in a large window: {small} and {large}");
}

#[test]
fn a_check_leads_to_its_slot_and_a_publication_is_withdrawn_only_after_a_second_yes() {
    let (mut state, id) = ready_to_publish("publish-withdraw");
    state.update(Message::PublishSheet(true), 1_790_000_110);
    state.update(Message::ShowSlot(1), 1_790_000_111);
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.publish && editor.selected == Some(1)));
    state.update(Message::Select(None), 1_790_000_112);
    assert!(state.update(Message::Withdraw, 1_790_000_113).is_empty(), "there is nothing to withdraw before a publication");
    state.update(Message::Published(id.clone(), Ok(publication(&id, 2))), 1_790_000_114);
    assert_eq!(state.publications.len(), 1);
    state.update(Message::PublishSheet(true), 1_790_000_115);
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let labels = |state: &State| {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
        (screen.find(words.t("catalogue-withdraw")).is_ok(), screen.find(words.t("catalogue-withdraw-yes")).is_ok(), screen.find(words.t("catalogue-update")).is_ok(), screen.find(words.n("publish-issue", 3)).is_ok())
    };
    assert_eq!(labels(&state), (true, false, true, true), "a published pool offers an update, its next version and the way out");
    state.update(Message::AskWithdraw(true), 1_790_000_116);
    assert_eq!(labels(&state), (false, true, true, true));
    let effects = state.update(Message::Withdraw, 1_790_000_117);
    assert!(matches!(effects.as_slice(), [Effect::Withdraw(pool)] if pool.id == id));
    assert!(state.publishing);
    state.update(Message::Withdrawn(id.clone(), Err("503".into())), 1_790_000_118);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.publish && !editor.asking_withdraw));
    assert_eq!(state.editing().unwrap().published_revision, 2, "a failed withdrawal changes nothing");
    state.update(Message::AskWithdraw(true), 1_790_000_119);
    state.update(Message::Withdraw, 1_790_000_120);
    let effects = state.update(Message::Withdrawn(id.clone(), Ok(())), 1_790_000_121);
    assert!(matches!(effects.as_slice(), [Effect::Say("catalogue-withdrawn")]), "{effects:?}");
    assert_eq!(state.editing().unwrap().published_revision, 0);
    assert!(state.publications.is_empty());
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.publish));
    assert_eq!(pools::load_all(&state.dir).iter().find(|pool| pool.id == id).unwrap().published_revision, 0);
}

#[test]
fn a_new_pool_begins_on_the_start_screen_and_a_new_collection_goes_straight_to_the_editor() {
    let mut state = state_with_songs("start-new");
    state.update(Message::New, 100);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.starting && editor.untouched));
    state.update(Message::Rename("Spring duel".into()), 101);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.starting), "a name does not leave the start");
    state.update(Message::Begin(Start::Empty), 102);
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting && editor.panel == Panel::Closed));
    assert_eq!(state.editing().unwrap().slots.len(), 4);
    let id = state.editing().unwrap().id.clone();
    state.update(Message::Back, 103);
    state.collection_shelf = true;
    state.update(Message::New, 104);
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting));
    state.update(Message::Back, 105);
    state.collection_shelf = false;
    state.update(Message::Open(id), 106);
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting), "an opened pool never starts over");
}

#[test]
fn the_start_screen_offers_two_frames_and_keeps_the_duel_for_later() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut state = state_with_songs("start-frames");
    state.update(Message::New, 100);
    let pressed = |state: &State, label: &str| -> Vec<Message> {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
        screen.click(label).unwrap_or_else(|_| panic!("{label} is on the start screen"));
        screen.into_messages().collect()
    };
    assert!(pressed(&state, &words.t("pool-start-duel")).is_empty(), "the duel is shown and cannot be chosen yet");
    assert!(matches!(pressed(&state, &words.t("pool-start-stage")).as_slice(), [Message::UseFrame(Frame::Stage)]));
    assert!(matches!(pressed(&state, &words.t("pool-frame-free")).as_slice(), [Message::UseFrame(Frame::Free)]));
    {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(&state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
        for label in ["NM 4", "HD 2", "HR 2", "DT 2", "FM", "TB", "NM 2"] {
            assert!(screen.find(label).is_ok(), "{label}");
        }
        for key in ["pool-frame-soon", "pool-frame-own", "pool-start-go"] {
            assert!(screen.find(words.t(key)).is_ok(), "{key}");
        }
        assert!(screen.find(words.t("pool-add")).is_err(), "the start has one main button");
    }
    state.update(Message::UseFrame(Frame::Stage), 101);
    assert_eq!(state.editing().unwrap().slots.len(), 12);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.starting));
    state.update(Message::UseFrame(Frame::Free), 102);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.starting));
    assert!(matches!(pressed(&state, &words.t("pool-start-go")).as_slice(), [Message::Begin(Start::Empty)]));
    state.update(Message::Begin(Start::Empty), 103);
    assert_eq!(state.editing().unwrap().slots.len(), 4, "a free pool begins with the same four slots whichever frame was tried first");
    assert!(state.editing().unwrap().slots.iter().all(|slot| slot.is_empty() && slot.mods == Mod::Nm));
}

#[test]
fn the_start_screen_grows_with_a_wide_window_and_stays_as_drawn_on_a_small_one() {
    assert_eq!(start_scale(929.0), 1.0);
    assert_eq!(start_scale(1286.0), 1.0);
    assert!((start_scale(1562.0) - 1.21).abs() < 0.01);
    assert!((start_scale(1800.0) - 1.395).abs() < 0.01);
    assert_eq!(start_scale(4000.0), START_GROWTH);
    let words = Words::new(crate::lang::Lang::Ru);
    let mut state = state_with_songs("start-grown");
    state.update(Message::New, 100);
    let button = |width: f32| {
        let image = picture(&state, &words, iced::Size::new(width, 1100.0), iced::Point::new(4.0, 4.0), &Clocks::settled(), &format!("start-grown-{width}"));
        let scale = image.width() as f32 / width;
        let red = |pixel: &::image::Rgba<u8>| pixel.0[0] > 190 && pixel.0[1] < 110 && pixel.0[2] < 110;
        let rows: Vec<u32> = (image.height() / 5..image.height()).filter(|y| (0..image.width()).filter(|x| red(image.get_pixel(*x, *y))).count() as f32 > 40.0 * scale).collect();
        assert!(!rows.is_empty(), "{width}: the main button is drawn");
        ((rows[rows.len() - 1] - rows[0] + 1) as f32 / scale, (rows[rows.len() - 1] + 1) as f32 / scale)
    };
    let (small_high, small_end) = button(1334.0);
    let (large_high, large_end) = button(2096.0);
    assert!(large_high > small_high * 1.3, "the controls are larger in a wide window: {small_high} and {large_high}");
    assert!(large_end > small_end * 1.25 && large_end < 1000.0, "the page fills more of a large window and still fits it: {small_end} and {large_end}");
}

#[test]
fn each_way_to_begin_leaves_the_start_and_opens_its_source() {
    let mut state = state_with_songs("start-ways");
    state.update(Message::New, 100);
    let id = state.editing().unwrap().id.clone();
    let effects = state.update(Message::Begin(Start::Links), 101);
    assert!(effects.iter().any(|effect| matches!(effect, Effect::ReadPaste(pool, Input::Query, contents) if pool == &id && contents.is_empty())), "{effects:?}");
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting && editor.panel == Panel::Add && editor.source == SourceTab::Search));
    state.update(Message::Back, 102);
    state.update(Message::New, 103);
    let effects = state.update(Message::Begin(Start::Collections), 104);
    assert!(effects.iter().any(|effect| matches!(effect, Effect::ReadCollections)), "{effects:?}");
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting && editor.wholesale && editor.panel == Panel::Add && editor.source == SourceTab::Collections));
    state.update(Message::Back, 105);
    state.update(Message::New, 106);
    let effects = state.update(Message::Begin(Start::Best), 107);
    assert!(effects.iter().any(|effect| matches!(effect, Effect::ReadBest(false))), "{effects:?}");
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting && editor.spread && editor.panel == Panel::Closed), "the best plays are waited for, no panel opens");
    state.update(Message::Back, 108);
    state.update(Message::New, 109);
    state.update(Message::Put("a1".into()), 110);
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting), "a map dropped on the start goes straight to the board");
}

#[test]
fn a_slot_in_trouble_says_what_is_wrong_and_offers_the_way_out() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut state = pool_with_maps("slot-ail", &["a1", "b2"]);
    state.update(Message::Select(None), 1_790_000_100);
    let at = state.editing_at().unwrap();
    assert_eq!(state.ail(&state.list[at].slots[0]), Ail::Well);
    state.list[at].slots[1].hash = Some("f".repeat(32));
    state.list[at].slots[1].measure = None;
    assert_eq!(state.ail(&state.list[at].slots[1]), Ail::Missing);
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(&state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
    assert!(screen.find(words.t("pool-no-songs")).is_ok());
    screen.click(words.t("pool-fetch-short").as_str()).unwrap();
    assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::FetchSlot(1)]), "the button acts on its own, the row behind it stays shut");
    let effects = state.update(Message::FetchSlot(1), 1_790_000_101);
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Fetch(..))), "{effects:?}");
    assert_eq!(state.ail(&state.list[at].slots[1]), Ail::Fetching);
    assert!(state.update(Message::FetchSlot(0), 1_790_000_102).is_empty(), "one download at a time");
    state.update(Message::Dismiss, 1_790_000_103);
    assert_eq!(state.ail(&state.list[at].slots[1]), Ail::Missing);
    state.list[at].slots[0].measure = None;
    state.measures.put("a1", state.list[at].slots[0].mods, Err("broken".into()));
    assert_eq!(state.ail(&state.list[at].slots[0]), Ail::Unmeasured);
}

#[test]
fn a_whole_collection_becomes_the_draft_when_the_start_asked_for_it() {
    let mut state = state_with_songs("start-collection");
    state.update(Message::New, 100);
    state.update(Message::Begin(Start::Collections), 101);
    state.update(Message::Collections(vec![Collection { name: "Favorites".into(), hashes: vec!["a1".into(), "b2".into(), "a1".into(), "c3".into()] }]), 102);
    state.update(Message::Collection(Some(0)), 103);
    let hashes: Vec<Option<String>> = state.editing().unwrap().slots.iter().map(|slot| slot.hash.clone()).collect();
    assert_eq!(&hashes[..3], &[Some("a1".to_owned()), Some("b2".to_owned()), Some("c3".to_owned())], "every map of the collection, each once");
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.wholesale));
    state.update(Message::Collection(None), 104);
    state.update(Message::Collection(Some(0)), 105);
    assert_eq!(state.editing().unwrap().filled(), 3, "opening a collection again takes nothing twice");
}

#[test]
fn best_plays_are_laid_out_by_the_mods_of_the_frame() {
    assert!(played_with(Mod::Nm, "") && played_with(Mod::Nm, "NF,CL") && !played_with(Mod::Nm, "HD"));
    assert!(played_with(Mod::Hd, "HD") && !played_with(Mod::Hd, "HD,DT") && !played_with(Mod::Hd, "HD,HR"));
    assert!(played_with(Mod::Hr, "HD,HR") && !played_with(Mod::Hr, "HR,DT"));
    assert!(played_with(Mod::Dt, "HD,DT") && played_with(Mod::Dt, "NC") && !played_with(Mod::Dt, "HD"));
    assert!(played_with(Mod::Fm, "EZ") && played_with(Mod::Tb, ""));
    let mut state = state_with_songs("start-best");
    state.update(Message::New, 100);
    state.update(Message::UseFrame(Frame::Duel), 100);
    let effects = state.update(Message::Begin(Start::Best), 101);
    assert!(effects.iter().any(|effect| matches!(effect, Effect::ReadBest(false))));
    assert_eq!(state.editing().unwrap().filled(), 0);
    let played = |hash: &str, mods: &str, pp: f64| crate::community::wire::Score { hash: hash.into(), mods: mods.into(), pp, beatmap_id: pp, ..Default::default() };
    state.set_best(Some(&[played("a1", "HD", 400.0), played("b2", "", 300.0), played("c3", "DT", 250.0), played("d4", "", 200.0), played("zz", "", 900.0)]));
    let pool = state.editing().unwrap();
    let placed: Vec<(Mod, Option<&str>)> = pool.slots.iter().map(|slot| (slot.mods, slot.hash.as_deref())).collect();
    assert_eq!(placed[0], (Mod::Nm, Some("b2")), "{placed:?}");
    assert_eq!(placed[1], (Mod::Nm, Some("d4")));
    assert_eq!(placed[2], (Mod::Hd, Some("a1")));
    assert_eq!(placed[4], (Mod::Dt, Some("c3")));
    assert!(pool.slots.iter().all(|slot| slot.hash.as_deref() != Some("zz")), "a play whose map is not on this computer is left out");
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.spread && editor.panel == Panel::Closed));
}

#[test]
fn several_links_pasted_at_once_are_taken_one_after_another() {
    let mut state = state_with_songs("links-queue");
    state.update(Message::New, 100);
    let pasted = format!("NM1 <{LINK}>\nNM2: https://osu.ppy.sh/beatmapsets/78#osu/200,\nhttps://osu.ppy.sh/users/5\nhttps://osu.ppy.sh/beatmapsets/79#osu/300");
    assert_eq!(links_in(&pasted).len(), 3);
    assert!(!is_paste(&pasted), "words around the links do not pull a paste out of a text field");
    assert!(is_paste(&format!("{LINK}\nhttps://osu.ppy.sh/beatmapsets/78#osu/200")));
    let id = state.editing().unwrap().id.clone();
    let effects = state.update(Message::PastedInto(id.clone(), Input::Query, String::new(), pasted.clone()), 101);
    assert!(matches!(effects.iter().filter(|effect| matches!(effect, Effect::Resolve(..))).collect::<Vec<_>>().as_slice(), [Effect::Resolve(_, Target::Set { id: 77, beatmap: Some(100), .. })]), "{effects:?}");
    assert_eq!(state.links.len(), 2);
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting && editor.panel == Panel::Add));
    let effects = state.update(Message::Resolved(state.resolve_request, Ok(found(&["a1"], Some(0)))), 102);
    assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("a1"), "a link that names one difficulty is taken without a question");
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Resolve(_, Target::Set { id: 78, beatmap: Some(200), .. }))), "{effects:?}");
    assert_eq!(state.links.len(), 1);
    let effects = state.update(Message::Resolved(state.resolve_request, Ok(found(&["b2", "c3"], None))), 103);
    assert!(effects.iter().all(|effect| !matches!(effect, Effect::Resolve(..))));
    assert!(matches!(&state.finding, Some(Finding::Found(candidate)) if candidate.choice.is_none()), "a set waits for the difficulty to be picked");
    state.update(Message::Choose(1), 104);
    let effects = state.update(Message::Confirm, 105);
    assert_eq!(state.editing().unwrap().slots[1].hash.as_deref(), Some("c3"));
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Resolve(_, Target::Set { id: 79, beatmap: Some(300), .. }))), "{effects:?}");
    state.update(Message::Resolved(state.resolve_request, Err(Why::Nowhere)), 106);
    assert!(matches!(state.finding, Some(Finding::Missing)));
    state.update(Message::Dismiss, 107);
    assert!(state.links.is_empty() && !state.linking);
    assert_eq!(state.editing().unwrap().filled(), 2);
    let effects = state.update(Message::PastedInto(id, Input::Name, "Name".into(), pasted), 108);
    assert!(effects.iter().all(|effect| !matches!(effect, Effect::Resolve(..))), "a pool name takes its text as text");
}

#[test]
fn closing_the_add_panel_drops_the_links_still_waiting() {
    let mut state = state_with_songs("links-closed");
    state.update(Message::New, 100);
    state.update(Message::Pasted(format!("{LINK} https://osu.ppy.sh/beatmapsets/78#osu/200 https://osu.ppy.sh/beatmapsets/79#osu/300")), 101);
    assert_eq!(state.links.len(), 2);
    state.update(Message::AddPanel(false), 102);
    assert!(state.links.is_empty() && !state.linking);
    let effects = state.update(Message::Resolved(state.resolve_request, Ok(found(&["a1"], Some(0)))), 103);
    assert!(effects.iter().all(|effect| !matches!(effect, Effect::Resolve(..))), "nothing reopens the panel");
    assert!(matches!(state.finding, Some(Finding::Found(_))));
}

#[test]
fn links_pasted_on_the_shelf_keep_their_queue_through_the_new_pool() {
    let mut state = state_with_songs("links-shelf");
    let effects = state.update(Message::Pasted(format!("{LINK}\nhttps://osu.ppy.sh/beatmapsets/78#osu/200")), 100);
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Resolve(_, Target::Set { id: 77, .. }))));
    assert_eq!(state.links.len(), 1);
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting && editor.panel == Panel::Add));
}

#[test]
fn a_link_pasted_on_the_shelf_makes_a_pool_opens_the_add_panel_and_asks_the_mirror() {
    let mut state = state_with_songs("paste-shelf");
    let effects = state.update(Message::Pasted(format!("  {LINK}\n")), 1_790_000_001);
    assert!(matches!(effects.as_slice(), [Effect::Resolve(_, Target::Set { id: 77, beatmap: Some(100), .. })]), "{effects:?}");
    let editor = match &state.screen {
        Screen::Editor(editor) => editor.clone(),
        Screen::Shelf | Screen::Open(_) => panic!("a pool was opened"),
    };
    assert_eq!(editor.panel, Panel::Add);
    assert_eq!(editor.query, LINK);
    assert!(matches!(state.finding, Some(Finding::Asking)));
}

#[test]
fn pasting_a_set_in_an_existing_pool_reveals_the_difficulty_picker_from_every_source() {
    for source in [SourceTab::Search, SourceTab::Collections, SourceTab::Best, SourceTab::Suggest] {
        let mut state = state_with_songs(&format!("paste-editor-{source:?}"));
        state.update(Message::New, 100);
        state.update(Message::Rename("My pool".into()), 101);
        state.update(Message::Put("a1".into()), 102);
        let before = state.editing().unwrap().clone();
        state.update(Message::Source(source), 103);
        state.update(Message::AddPanel(false), 104);
        let effects = state.update(Message::Pasted("https://osu.ppy.sh/beatmapsets/77".into()), 105);
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Resolve(_, Target::Set { id: 77, beatmap: None, .. }))));
        let Screen::Editor(editor) = &state.screen else { panic!("the same editor stays open") };
        assert_eq!(editor.id, before.id);
        assert_eq!(editor.panel, Panel::Add);
        assert_eq!(editor.source, SourceTab::Search);
        assert_eq!(state.editing().unwrap(), &before);
        state.update(Message::Resolved(state.resolve_request, Ok(found(&["b2", "c3"], None))), 106);
        let Some(Finding::Found(candidate)) = &state.finding else { panic!("difficulty selection is ready") };
        assert_eq!(candidate.choice, None);
        state.update(Message::Choose(1), 107);
        state.update(Message::Confirm, 108);
        let pool = state.editing().unwrap();
        assert_eq!(pool.id, before.id);
        assert_eq!(pool.name, before.name);
        assert_eq!(pool.slots.len(), before.slots.len());
        assert_eq!(pool.slots[0], before.slots[0]);
        assert_eq!(pool.slots[1].hash.as_deref(), Some("c3"));
    }
}

#[test]
fn words_pasted_on_the_shelf_do_nothing_but_inside_a_pool_they_become_the_search() {
    let mut state = state_with_songs("paste-words");
    assert!(state.update(Message::Pasted("hello there".into()), 1_790_000_001).is_empty());
    assert_eq!(state.screen, Screen::Shelf);
    assert!(state.list.is_empty());
    open_new(&mut state);
    state.update(Message::Pasted("glass".into()), 1_790_000_002);
    let Screen::Editor(editor) = &state.screen else { panic!("still in the editor") };
    assert_eq!((editor.panel, editor.query.as_str()), (Panel::Add, "glass"));
    assert!(state.finding.is_none());
}

#[test]
fn a_resolved_map_offers_the_automatic_slot_and_asks_for_its_cover() {
    let mut state = state_with_songs("resolved");
    open_new(&mut state);
    state.update(Message::Pasted(LINK.into()), 1_790_000_002);
    let effects = state.update(Message::Resolved(state.resolve_request, Ok(found(&["a1"], Some(0)))), 1_790_000_003);
    assert!(matches!(effects.as_slice(), [Effect::Cover(url, 77)] if url.ends_with("/77/covers/cover.jpg")));
    let Some(Finding::Found(candidate)) = &state.finding else { panic!("found") };
    assert_eq!((candidate.choice, candidate.place), (Some(0), Place::Slot(0)));
    state.update(Message::Cover(77, None), 1_790_000_004);
    assert!(matches!(state.finding, Some(Finding::Found(_))));
}

#[test]
fn a_set_waits_for_a_choice_and_a_map_on_the_disk_is_placed_without_a_download() {
    let mut state = state_with_songs("choice");
    open_new(&mut state);
    state.update(Message::Pasted("https://osu.ppy.sh/beatmapsets/77".into()), 1_790_000_002);
    state.update(Message::Resolved(state.resolve_request, Ok(found(&["a1", "b2", "c3"], None))), 1_790_000_003);
    let Some(Finding::Found(candidate)) = &state.finding else { panic!("found") };
    assert_eq!(candidate.choice, None);
    assert!(state.update(Message::Confirm, 1_790_000_004).is_empty(), "nothing is placed before a choice");
    assert!(matches!(state.finding, Some(Finding::Found(_))), "the card stays");
    state.update(Message::Choose(1), 1_790_000_005);
    state.update(Message::Aim(Place::Slot(3)), 1_790_000_005);
    let effects = state.update(Message::Confirm, 1_790_000_006);
    assert!(effects.iter().all(|effect| !matches!(effect, Effect::Fetch(..))), "the map is already on the disk");
    assert!(state.finding.is_none());
    assert_eq!(state.editing().unwrap().slots[3].hash.as_deref(), Some("b2"));
}

#[test]
fn a_map_that_is_not_on_the_disk_is_fetched_step_by_step_and_then_placed() {
    use crate::maps::Step;
    let mut state = state_with_songs("fetch");
    open_new(&mut state);
    state.update(Message::Pasted(LINK.into()), 1_790_000_002);
    state.update(Message::Resolved(state.resolve_request, Ok(found(&["zz9"], Some(0)))), 1_790_000_003);
    state.update(Message::Aim(Place::Slot(2)), 1_790_000_004);
    let effects = state.update(Message::Confirm, 1_790_000_005);
    assert!(matches!(effects.as_slice(), [Effect::Fetch(_, hash, _)] if hash == "zz9"));
    let Some(Finding::Fetching(fetching)) = &state.finding else { panic!("fetching") };
    assert_eq!((fetching.total, fetching.queue.len()), (1, 0));
    state.update(Message::Step(state.fetch_request, "zz9".into(), Step::Downloading { from: "osu.direct", done: 3_000_000, total: Some(5_000_000) }), 1_790_000_006);
    assert_eq!(stage_of(match &state.finding { Some(Finding::Fetching(f)) => f.step.as_ref(), _ => None }), 1);
    let map = map("Fetched Song", "Artist", "Normal");
    state.update(Message::Step(state.fetch_request, "zz9".into(), Step::Done(map)), 1_790_000_007);
    assert!(state.finding.is_none());
    assert_eq!(state.editing().unwrap().slots[2].hash.as_deref(), Some("zz9"));
    assert!(state.fetched.contains_key("zz9"));
    assert!(state.songs.as_ref().unwrap().contains_key("zz9"));
}

#[test]
fn putting_all_places_what_is_on_the_disk_at_once_and_fetches_the_rest_into_the_next_slots() {
    use crate::maps::Step;
    let mut state = state_with_songs("all");
    open_new(&mut state);
    state.update(Message::Pasted("https://osu.ppy.sh/beatmapsets/77".into()), 1_790_000_002);
    state.update(Message::Resolved(state.resolve_request, Ok(found(&["a1", "zz9", "c3"], None))), 1_790_000_003);
    let effects = state.update(Message::ConfirmAll, 1_790_000_004);
    let fetches: Vec<&Effect> = effects.iter().filter(|effect| matches!(effect, Effect::Fetch(..))).collect();
    assert!(matches!(fetches.as_slice(), [Effect::Fetch(_, hash, _)] if hash == "zz9"), "the first is placed, the second is fetched: {effects:?}");
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Measure(hash, _, _) if hash == "a1")), "the map on the disk is measured");
    assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("a1"));
    let Some(Finding::Fetching(fetching)) = &state.finding else { panic!("fetching") };
    assert_eq!((fetching.total, fetching.queue.clone()), (3, vec!["c3".to_owned()]));
    state.update(Message::Step(state.fetch_request, "zz9".into(), Step::Done(map("Second", "Artist", "Hard"))), 1_790_000_005);
    let pool = state.editing().unwrap();
    assert_eq!((pool.slots[0].hash.as_deref(), pool.slots[1].hash.as_deref(), pool.slots[2].hash.as_deref()), (Some("a1"), Some("zz9"), Some("c3")));
    assert!(state.finding.is_none());
}

#[test]
fn a_taiko_link_is_refused_and_a_not_a_map_link_is_told_under_the_field() {
    let mut state = state_with_songs("refuse");
    open_new(&mut state);
    let effects = state.update(Message::Query("https://osu.ppy.sh/beatmapsets/5#taiko/6".into()), 1_790_000_002);
    assert!(effects.is_empty());
    assert!(matches!(&state.finding, Some(Finding::Refused(mode)) if mode == "taiko"));
    state.update(Message::Dismiss, 1_790_000_003);
    assert!(state.finding.is_none());
    state.update(Message::Query("https://example.com/not-a-map".into()), 1_790_000_004);
    assert_eq!(state.notice, Some(Notice::NotALink), "a foreign address is told to be no map link");
    assert!(state.finding.is_none());
    state.update(Message::Query("https://osu.ppy.sh/users/2".into()), 1_790_000_005);
    assert_eq!(state.notice, Some(Notice::NotALink));
    state.update(Message::Query("glass".into()), 1_790_000_006);
    assert_eq!(state.notice, None);
}

#[test]
fn typing_ordinary_words_clears_an_old_finding_and_a_silent_mirror_can_be_retried() {
    let mut state = state_with_songs("silent");
    open_new(&mut state);
    state.update(Message::Query(LINK.into()), 1_790_000_002);
    assert!(matches!(state.finding, Some(Finding::Asking)));
    state.update(Message::Resolved(state.resolve_request, Err(Why::Silent("down".into()))), 1_790_000_003);
    assert!(matches!(state.finding, Some(Finding::Silent(_))));
    let again = state.update(Message::Retry, 1_790_000_004);
    assert!(matches!(again.as_slice(), [Effect::Resolve(_, _)]));
    assert!(matches!(state.finding, Some(Finding::Asking)));
    state.update(Message::Resolved(state.resolve_request, Err(Why::Nowhere)), 1_790_000_005);
    assert!(matches!(state.finding, Some(Finding::Missing)));
    state.update(Message::Query("glass".into()), 1_790_000_006);
    assert!(state.finding.is_none());
}

#[test]
fn an_answer_that_arrives_after_the_card_was_closed_is_ignored() {
    let mut state = state_with_songs("stale");
    open_new(&mut state);
    state.update(Message::Query(LINK.into()), 1_790_000_002);
    state.update(Message::Dismiss, 1_790_000_003);
    state.update(Message::Resolved(state.resolve_request, Ok(found(&["a1"], Some(0)))), 1_790_000_004);
    assert!(state.finding.is_none());
}

#[test]
fn closing_a_download_stops_it_and_a_failed_or_empty_download_is_told() {
    use crate::maps::Step;
    let mut state = state_with_songs("stop");
    open_new(&mut state);
    state.update(Message::Query(LINK.into()), 1_790_000_002);
    state.update(Message::Resolved(state.resolve_request, Ok(found(&["zz9"], Some(0)))), 1_790_000_003);
    state.update(Message::Confirm, 1_790_000_004);
    let stop = match &state.finding {
        Some(Finding::Fetching(fetching)) => fetching.stop.clone(),
        _ => panic!("fetching"),
    };
    state.update(Message::Dismiss, 1_790_000_005);
    assert!(stop.load(std::sync::atomic::Ordering::SeqCst));
    assert!(state.finding.is_none());
    state.update(Message::Step(state.fetch_request, "zz9".into(), Step::Stopped), 1_790_000_006);
    assert!(state.finding.is_none(), "a late step changes nothing");

    let mut failing = state_with_songs("fail");
    open_new(&mut failing);
    failing.update(Message::Query(LINK.into()), 1_790_000_002);
    failing.update(Message::Resolved(failing.resolve_request, Ok(found(&["zz9"], Some(0)))), 1_790_000_003);
    failing.update(Message::Confirm, 1_790_000_004);
    failing.update(Message::Step(failing.fetch_request, "zz9".into(), Step::Nowhere), 1_790_000_005);
    assert!(matches!(failing.finding, Some(Finding::Missing)));
    failing.update(Message::Dismiss, 1_790_000_006);
    failing.update(Message::Query(LINK.into()), 1_790_000_007);
    failing.update(Message::Resolved(failing.resolve_request, Ok(found(&["zz9"], Some(0)))), 1_790_000_008);
    failing.update(Message::Confirm, 1_790_000_009);
    failing.update(Message::Step(failing.fetch_request, "zz9".into(), Step::Failed("no space".into())), 1_790_000_010);
    assert!(matches!(&failing.finding, Some(Finding::Silent(why)) if why == "no space"));
}

#[test]
fn a_map_fetched_while_the_songs_are_still_being_read_survives_their_arrival() {
    use crate::maps::Step;
    let mut state = State::new(scratch("late"));
    state.loaded = true;
    state.update(Message::New, 1_790_000_001);
    state.update(Message::Query(LINK.into()), 1_790_000_002);
    state.update(Message::Resolved(state.resolve_request, Ok(found(&["zz9"], Some(0)))), 1_790_000_003);
    state.update(Message::Confirm, 1_790_000_004);
    let effects = state.update(Message::Step(state.fetch_request, "zz9".into(), Step::Done(map("Fetched", "A", "N"))), 1_790_000_005);
    assert!(effects.is_empty(), "no index yet, so nothing is measured yet");
    assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("zz9"));
    let mut songs = HashMap::new();
    songs.insert("a1".to_owned(), map("Other", "A", "N"));
    let after = state.update(Message::Songs(Arc::new(songs)), 1_790_000_006);
    assert!(state.songs.as_ref().unwrap().contains_key("zz9"), "the fetched map is merged into the index");
    assert!(matches!(after.as_slice(), [Effect::Measure(hash, _, _)] if hash == "zz9"));
}

fn shared_pool(hashes: &[&str]) -> Pool {
    let mut pool = Pool::new(Frame::Duel, "Cup: round/1", 1_790_000_000);
    for (at, hash) in hashes.iter().enumerate() {
        pool.slots[at].hash = Some((*hash).to_owned());
        pool.slots[at].artist = "Artist".into();
        pool.slots[at].title = format!("Song {at}");
        pool.slots[at].version = "Normal".into();
    }
    pool
}

#[test]
fn image_export_uses_a_snapshot_blocks_duplicates_and_unlocks_after_success_cancel_or_failure() {
    let mut state = pool_with_maps("image-export", &["a1", "b2"]);
    state.update(Message::Rename("First name".into()), 1_790_000_100);
    let effects = state.update(Message::SaveImage, 1_790_000_101);
    let (request, exported) = match effects.as_slice() {
        [Effect::SaveImage(request, pool)] => (*request, pool.clone()),
        _ => panic!("export the current pool"),
    };
    assert_eq!(state.exporting, Some(request));
    assert!(state.update(Message::SaveImage, 1_790_000_102).is_empty());
    state.update(Message::Rename("Changed later".into()), 1_790_000_103);
    assert_eq!(exported.name, "First name");
    assert_eq!(exported.filled(), 2);
    assert!(state.update(Message::ImageSaved(request + 1, Ok(None)), 1_790_000_104).is_empty());
    assert_eq!(state.exporting, Some(request));
    let effects = state.update(Message::ImageSaved(request, Ok(Some(PathBuf::from("pool.png")))), 1_790_000_105);
    assert!(matches!(effects.as_slice(), [Effect::Say("pool-image-saved")]));
    assert!(state.exporting.is_none());
    state.update(Message::SaveImage, 1_790_000_106);
    let cancelled = state.exporting.unwrap();
    assert_ne!(cancelled, request);
    assert!(state.update(Message::ImageSaved(request, Ok(None)), 1_790_000_107).is_empty());
    assert_eq!(state.exporting, Some(cancelled));
    assert!(state.update(Message::ImageSaved(cancelled, Ok(None)), 1_790_000_108).is_empty());
    assert!(state.exporting.is_none());
    state.update(Message::SaveImage, 1_790_000_109);
    let failed = state.exporting.unwrap();
    let effects = state.update(Message::ImageSaved(failed, Err("disk full".into())), 1_790_000_110);
    assert!(matches!(effects.as_slice(), [Effect::Say("pool-save-failed")]));
    assert!(state.exporting.is_none());
    state.update(Message::Back, 1_790_000_111);
    assert!(state.update(Message::SaveImage, 1_790_000_112).is_empty());
}

#[test]
fn sharing_offers_the_file_the_line_and_the_hash_and_closes() {
    let mut state = state_with_songs("share");
    open_new(&mut state);
    state.update(Message::Put("a1".into()), 1_790_000_002);
    state.update(Message::Rename("Cup: round/1".into()), 1_790_000_003);
    state.update(Message::Share(true), 1_790_000_004);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.share));
    let pool = state.editing().unwrap().clone();
    let copied = state.update(Message::CopyText, 1_790_000_005);
    assert!(matches!(copied.as_slice(), [Effect::Copy(text, "pool-copied")] if text == &pool_share::to_text(&pool)));
    let hashed = state.update(Message::CopyHash, 1_790_000_006);
    assert!(matches!(hashed.as_slice(), [Effect::Copy(text, "pool-copied-hash")] if text == &pool.fingerprint() && text.len() == 7));
    let saved = state.update(Message::SaveFile, 1_790_000_007);
    assert!(matches!(saved.as_slice(), [Effect::SaveFile(name, bytes)] if name == "Cup round 1.pool" && bytes == &pool_share::to_file(&pool)), "{saved:?}");
    state.update(Message::Share(false), 1_790_000_008);
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.share));
    assert_eq!(file_name_of(&Pool::new(Frame::Free, "", 1)), "pool.pool");
}

#[test]
fn a_pool_string_pasted_on_the_shelf_opens_the_pool_and_reads_the_songs() {
    let mut state = State::new(scratch("open-string"));
    state.loaded = true;
    let text = pool_share::to_text(&shared_pool(&["a1", "zz9"]));
    let effects = state.update(Message::Pasted(text), 1_790_000_001);
    assert!(matches!(effects.as_slice(), [Effect::ReadSongs]), "{effects:?}");
    let Screen::Open(opening) = &state.screen else { panic!("the open screen") };
    assert_eq!(opening.pool.name, "Cup: round/1");
    assert!(state.list.is_empty(), "nothing is saved before the person says so");
}

#[test]
fn a_cut_string_or_a_foreign_file_is_told_on_the_shelf() {
    let mut state = state_with_songs("refused");
    let text = pool_share::to_text(&shared_pool(&["a1"]));
    state.update(Message::Pasted(text[..text.len() / 2].to_owned()), 1_790_000_001);
    assert_eq!(state.refused, Some(pool_share::Refused::Damaged));
    assert_eq!(state.screen, Screen::Shelf);
    state.update(Message::Imported(Err(pool_share::Refused::NotAPool)), 1_790_000_002);
    assert_eq!(state.refused, Some(pool_share::Refused::NotAPool));
    state.update(Message::Imported(Ok(shared_pool(&["a1"]))), 1_790_000_003);
    assert!(state.refused.is_none());
    assert!(matches!(state.screen, Screen::Open(_)));
}

#[test]
fn the_missing_maps_are_downloaded_in_turn_and_measured_and_the_pool_is_kept() {
    use crate::maps::Step;
    let mut state = state_with_songs("open-flow");
    state.update(Message::Imported(Ok(shared_pool(&["a1", "zz9", "yy8", "a1"]))), 1_790_000_001);
    let Screen::Open(opening) = state.screen.clone() else { panic!("open") };
    assert_eq!(state.missing(&opening.pool), vec!["zz9".to_owned(), "yy8".to_owned()], "each missing map is listed once");
    let effects = state.update(Message::GetMissing, 1_790_000_002);
    assert!(matches!(effects.as_slice(), [Effect::Fetch(_, hash, _)] if hash == "zz9"));
    assert!(state.update(Message::GetMissing, 1_790_000_003).is_empty(), "a second press does not start a second run");
    let next = state.update(Message::Step(state.fetch_request, "zz9".into(), Step::Done(map("Fetched One", "A", "N"))), 1_790_000_004);
    assert!(next.iter().any(|effect| matches!(effect, Effect::Measure(hash, _, _) if hash == "zz9")), "the arrived map is measured");
    assert!(next.iter().any(|effect| matches!(effect, Effect::Fetch(_, hash, _) if hash == "yy8")), "the next one is asked for");
    let after = state.update(Message::Step(state.fetch_request, "yy8".into(), Step::Nowhere), 1_790_000_005);
    assert!(after.iter().all(|effect| !matches!(effect, Effect::Fetch(..))));
    let Screen::Open(opening) = state.screen.clone() else { panic!("open") };
    assert_eq!((opening.running, opening.lost), (false, 1));
    state.update(Message::Measured("zz9".into(), Mod::Nm, Ok(measure(5.5))), 1_790_000_006);
    let Screen::Open(opening) = state.screen.clone() else { panic!("open") };
    assert_eq!(opening.pool.slots[1].measure, Some(measure(5.5)));

    let kept = state.update(Message::Keep, 1_790_000_007);
    assert!(matches!(&state.screen, Screen::Editor(_)));
    assert_eq!(state.list.len(), 1);
    assert_eq!(state.list[0].made_at, 1_790_000_007);
    assert_eq!(pools::load_all(&state.dir).len(), 1);
    assert!(kept.iter().all(|effect| !matches!(effect, Effect::Fetch(..))));
}

#[test]
fn leaving_the_open_screen_stops_the_downloads_and_forgets_the_draft() {
    let mut state = state_with_songs("open-leave");
    state.update(Message::Imported(Ok(shared_pool(&["zz9"]))), 1_790_000_001);
    state.update(Message::GetMissing, 1_790_000_002);
    let stop = match &state.screen {
        Screen::Open(opening) => opening.stop.clone(),
        _ => panic!("open"),
    };
    state.update(Message::Back, 1_790_000_003);
    assert!(stop.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(state.screen, Screen::Shelf);
    assert!(state.list.is_empty());
    assert!(pools::load_all(&state.dir).is_empty());
}

#[test]
fn nothing_is_asked_for_while_the_songs_are_still_being_read() {
    let mut state = State::new(scratch("open-wait"));
    state.loaded = true;
    state.update(Message::Imported(Ok(shared_pool(&["zz9"]))), 1_790_000_001);
    assert!(state.update(Message::GetMissing, 1_790_000_002).is_empty());
    let Screen::Open(opening) = &state.screen else { panic!("open") };
    assert!(state.missing(&opening.pool).is_empty(), "what is missing is not known yet");
}

fn pool_with_maps(name: &str, hashes: &[&str]) -> State {
    let mut state = state_with_songs(name);
    open_new(&mut state);
    for (at, hash) in hashes.iter().enumerate() {
        state.update(Message::Select(None), 1_790_000_002 + at as i64);
        state.update(Message::Put((*hash).into()), 1_790_000_020 + at as i64);
    }
    state
}

fn hashes_of(state: &State) -> Vec<Option<String>> {
    state.editing().unwrap().slots.iter().map(|slot| slot.hash.clone()).collect()
}

#[test]
fn grouping_only_changes_the_view_and_keeps_selection_and_saved_order() {
    let mut state = pool_with_maps("group-view", &["a1", "b2", "c3"]);
    state.update(Message::SetMod(1, Mod::Hr), 1_790_000_100);
    state.update(Message::Select(Some(1)), 1_790_000_101);
    let pool = state.editing().unwrap().clone();
    let undo = state.undo.len();
    state.update(Message::Grouped(true), 1_790_000_102);
    let groups = slot_groups(state.editing().unwrap());
    assert_eq!(groups[0], (Mod::Nm, vec![0]));
    assert!(groups.iter().find(|(mods, _)| *mods == Mod::Hr).unwrap().1.contains(&1));
    assert_eq!(state.editing().unwrap().slots, pool.slots);
    assert_eq!(pools::load_all(&state.dir)[0].slots, pool.slots);
    assert_eq!(state.undo.len(), undo);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.grouped && editor.selected == Some(1)));
    state.update(Message::Grouped(false), 1_790_000_103);
    assert_eq!(state.editing().unwrap().slots, pool.slots);
}

#[test]
fn replacing_from_a_tile_targets_that_tile_and_undo_restores_its_map() {
    let mut state = pool_with_maps("inline-replace", &["a1", "b2"]);
    state.update(Message::Note(1, "Keep this role".into()), 1_790_000_100);
    let before = state.editing().unwrap().slots.clone();
    state.update(Message::Select(Some(0)), 1_790_000_101);
    state.update(Message::ReplaceAt(1), 1_790_000_102);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.selected == Some(1) && editor.replace && editor.panel == Panel::Add));
    state.update(Message::Put("c3".into()), 1_790_000_103);
    assert_eq!(state.editing().unwrap().slots[0], before[0]);
    assert_eq!(state.editing().unwrap().slots[1].hash.as_deref(), Some("c3"));
    assert_eq!(state.editing().unwrap().slots[1].mods, before[1].mods);
    state.update(Message::Undo, 1_790_000_104);
    assert_eq!(state.editing().unwrap().slots, before);
    state.update(Message::ReplaceAt(usize::MAX), 1_790_000_105);
    assert_eq!(state.editing().unwrap().slots, before);
}

#[test]
fn holding_the_category_header_does_not_open_details_or_start_a_drag() {
    let mut state = pool_with_maps("inline-controls", &["a1", "b2", "c3"]);
    state.editor_mut().unwrap().panel = Panel::Closed;
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1000.0, 900.0), view(&state, &words, &thumbs, 1000.0, 1.0, &Clocks::settled()));
    assert!(screen.find(words.t("pool-replace-short")).is_err());
    assert!(screen.find(words.t("pool-remove-short")).is_err());
    let number = screen.find("1").unwrap().visible_bounds().unwrap();
    let category = number.center() + iced::Vector::new(56.0, 0.0);
    screen.point_at(category);
    let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left))]);
    let position = category + iced::Vector::new(0.0, 100.0);
    screen.point_at(position);
    let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved { position })]);
    let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left))]);
    assert!(screen.into_messages().all(|message| !matches!(message, Message::Move(..) | Message::Select(..))));
}

#[test]
fn the_category_menu_changes_only_its_tile_and_replaces_the_old_measure() {
    let mut state = pool_with_maps("inline-category", &["a1", "b2"]);
    state.editor_mut().unwrap().panel = Panel::Closed;
    state.update(Message::Measured("a1".into(), Mod::Nm, Ok(measure(4.0))), 1_790_000_100);
    let before = state.editing().unwrap().slots.clone();
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1000.0, 900.0), view(&state, &words, &thumbs, 1000.0, 1.0, &Clocks::settled()));
    let number = screen.find("1").unwrap().visible_bounds().unwrap();
    let category = number.center() + iced::Vector::new(56.0, 0.0);
    screen.point_at(category);
    let _ = screen.simulate(iced_test::simulator::click());
    if let Some(dir) = std::env::var_os("DOSSIER_POOL_MENU_REVIEW").map(PathBuf::from) {
        let shot = screen.snapshot(&theme::theme()).unwrap();
        crate::gallery::write_snapshot(&shot, &dir.join("pool-category-menu")).unwrap();
    }
    let position = category + iced::Vector::new(0.0, 100.0);
    screen.point_at(position);
    let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved { position })]);
    let _ = screen.simulate(iced_test::simulator::click());
    let messages: Vec<_> = screen.into_messages().collect();
    assert!(matches!(messages.as_slice(), [Message::SetMod(0, Mod::Hr)]), "{messages:?}");
    for message in messages { state.update(message, 1_790_000_101); }
    assert_eq!(state.editing().unwrap().slots[0].mods, Mod::Hr);
    assert!(state.editing().unwrap().slots[0].measure.is_none());
    assert_eq!(&state.editing().unwrap().slots[1..], &before[1..]);
    state.update(Message::Measured("a1".into(), Mod::Hr, Ok(measure(6.0))), 1_790_000_102);
    assert_eq!(state.editing().unwrap().slots[0].measure.unwrap().stars, 6.0);
    state.update(Message::Undo, 1_790_000_103);
    assert_eq!(state.editing().unwrap().slots, before);
}

#[test]
fn dragging_to_the_end_of_a_category_keeps_the_next_category_in_place() {
    let mut state = pool_with_maps("group-drag", &["a1", "b2", "c3"]);
    state.editor_mut().unwrap().panel = Panel::Closed;
    state.update(Message::Grouped(true), 1_790_000_100);
    let before = state.editing().unwrap().slots.clone();
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1000.0, 900.0), view(&state, &words, &thumbs, 1000.0, 1.0, &Clocks::settled()));
    let first = screen.find("Glass Orchard").unwrap().visible_bounds().unwrap();
    let second = screen.find("Salt and Static").unwrap().visible_bounds().unwrap();
    screen.point_at(first.center());
    let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left))]);
    let position = iced::Point::new(first.center_x(), second.y + ROW_HIGH + 50.0);
    screen.point_at(position);
    let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved { position })]);
    let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left))]);
    let messages: Vec<_> = screen.into_messages().collect();
    let keys = slot_keys(state.editing().unwrap());
    assert!(matches!(messages.as_slice(), [Message::Move(_, _, key, Some(next))] if *key == keys[0] && *next == keys[2]), "{messages:?}");
    for message in messages { state.update(message, 1_790_000_101); }
    assert_eq!(&state.editing().unwrap().slots[..2], &[before[1].clone(), before[0].clone()]);
    assert_eq!(&state.editing().unwrap().slots[2..], &before[2..]);
    state.update(Message::Undo, 1_790_000_102);
    assert_eq!(state.editing().unwrap().slots, before);
}

fn move_slot(state: &mut State, from: usize, before: Option<usize>, now: i64) {
    let pool = state.editing().unwrap();
    let keys = slot_keys(pool);
    let message = Message::Move(pool.id.clone(), keys.clone(), keys[from], before.map(|at| keys[at]));
    state.update(message, now);
}

#[test]
fn dragging_keeps_the_entire_slot_selection_and_marks_and_undo_restores_the_order() {
    let mut state = pool_with_maps("drag", &["a1", "b2", "c3"]);
    state.update(Message::Note(2, "Keep this chart".into()), 1_790_000_090);
    state.update(Message::Measured("c3".into(), Mod::Hd, Ok(measure(5.5))), 1_790_000_091);
    state.update(Message::Select(Some(1)), 1_790_000_092);
    let before = state.editing().unwrap().slots.clone();
    let count = state.undo.len();
    move_slot(&mut state, 2, Some(0), 1_790_000_100);
    assert_eq!(&hashes_of(&state)[..3], [Some("c3".into()), Some("a1".into()), Some("b2".into())]);
    assert_eq!(state.editing().unwrap().slots[0], before[2], "mods, note and measures travel together");
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.selected == Some(2)));
    assert_eq!(state.undo.len(), count + 1);
    assert_eq!(pools::load_all(&state.dir)[0].slots, state.editing().unwrap().slots);
    state.update(Message::Undo, 1_790_000_101);
    assert_eq!(state.editing().unwrap().slots, before);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.selected == Some(1)));
    state.update(Message::Choosing(true), 1_790_000_102);
    state.update(Message::Mark(0), 1_790_000_103);
    state.update(Message::Mark(2), 1_790_000_104);
    move_slot(&mut state, 0, None, 1_790_000_105);
    assert_eq!(state.marked_slots(), vec![1, 6]);
    state.update(Message::Undo, 1_790_000_106);
    assert_eq!(state.marked_slots(), vec![0, 2]);
}

#[test]
fn a_drag_with_an_old_order_or_a_different_pool_is_ignored_and_noop_adds_no_undo() {
    let mut state = pool_with_maps("drag-stale", &["a1", "b2", "c3"]);
    let pool = state.editing().unwrap();
    let keys = slot_keys(pool);
    let message = Message::Move(pool.id.clone(), keys.clone(), keys[2], Some(keys[0]));
    let count = state.undo.len();
    move_slot(&mut state, 0, Some(1), 1_790_000_100);
    move_slot(&mut state, 0, Some(0), 1_790_000_101);
    assert_eq!(state.undo.len(), count);
    state.update(Message::SetMod(0, Mod::Dt), 1_790_000_102);
    let before = state.editing().unwrap().clone();
    state.update(message.clone(), 1_790_000_103);
    assert_eq!(state.editing().unwrap(), &before);
    state.update(Message::New, 1_790_000_104);
    let next = state.editing().unwrap().clone();
    state.update(message, 1_790_000_105);
    assert_eq!(state.editing().unwrap(), &next);
}

#[test]
fn an_empty_slot_moves_with_its_mod_and_an_inflight_file_import_follows_it() {
    let mut state = pool_with_maps("drag-import", &["a1"]);
    state.update(Message::Select(Some(3)), 1_790_000_100);
    state.update(Message::Dropped(PathBuf::from("first.osu")), 1_790_000_101);
    let request = state.importing.as_ref().unwrap().request;
    move_slot(&mut state, 3, Some(0), 1_790_000_102);
    assert_eq!(state.editing().unwrap().slots[0].mods, Mod::Hr);
    assert_eq!(state.importing.as_ref().unwrap().place, Place::Slot(0));
    state.update(Message::MapFile(request, Ok(("e5".into(), map("First", "A", "Hard")))), 1_790_000_103);
    assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("e5"));
    assert_eq!(state.editing().unwrap().slots[0].mods, Mod::Hr);
}

#[test]
fn moving_slots_does_not_revive_an_import_invalidated_by_an_earlier_edit() {
    let mut state = pool_with_maps("drag-stale-import", &["a1"]);
    state.update(Message::Select(Some(3)), 1_790_000_100);
    state.update(Message::Dropped(PathBuf::from("first.osu")), 1_790_000_101);
    let request = state.importing.as_ref().unwrap().request;
    state.update(Message::SetMod(3, Mod::Dt), 1_790_000_102);
    move_slot(&mut state, 3, Some(0), 1_790_000_103);
    state.update(Message::MapFile(request, Ok(("e5".into(), map("First", "A", "Hard")))), 1_790_000_104);
    assert_eq!(state.editing().unwrap().filled(), 1);
}

#[test]
fn downloaded_maps_and_replacements_follow_their_target_when_it_moves() {
    let mut state = pool_with_maps("drag-fetch", &["a1", "b2"]);
    state.update(Message::Select(Some(3)), 1_790_000_100);
    state.update(Message::Pasted(LINK.into()), 1_790_000_101);
    state.update(Message::Resolved(state.resolve_request, Ok(found(&["zz9"], Some(0)))), 1_790_000_102);
    state.update(Message::Aim(Place::Slot(3)), 1_790_000_103);
    state.update(Message::Confirm, 1_790_000_104);
    move_slot(&mut state, 3, Some(0), 1_790_000_105);
    assert!(matches!(&state.finding, Some(Finding::Fetching(fetching)) if fetching.place == Some(Place::Slot(0))));
    state.update(Message::Step(state.fetch_request, "zz9".into(), crate::maps::Step::Done(map("Fetched", "A", "Hard"))), 1_790_000_106);
    assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("zz9"));
    state.update(Message::Pasted(LINK.into()), 1_790_000_107);
    state.update(Message::Resolved(state.resolve_request, Ok(found(&["c3"], Some(0)))), 1_790_000_108);
    state.update(Message::Aim(Place::Replace(2)), 1_790_000_109);
    move_slot(&mut state, 2, Some(0), 1_790_000_110);
    assert!(matches!(&state.finding, Some(Finding::Found(candidate)) if candidate.place == Place::Replace(0)));
    state.update(Message::Confirm, 1_790_000_111);
    assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("c3"));
}

#[test]
fn slot_keys_stay_unique_for_repeated_empty_slots_and_ignore_measurement_updates() {
    let mut pool = Pool::new(Frame::Duel, "", 1_790_000_000);
    let keys = slot_keys(&pool);
    assert_eq!(keys.iter().collect::<HashSet<_>>().len(), pool.slots.len());
    pool.slots[0].measure = Some(measure(5.0));
    assert_eq!(slot_keys(&pool), keys);
    pool.slots.swap(3, 4);
    let after = slot_keys(&pool);
    assert_eq!(after[4], keys[3]);
    assert_eq!(after[3], keys[4]);
}

#[test]
fn adding_to_a_full_pool_that_is_not_the_newest_does_not_lose_its_place_in_the_list() {
    let mut state = pool_with_maps("full-and-older", &["a1", "b2", "c3", "d4"]);
    let id = state.editing().unwrap().id.clone();
    let at = state.editing_at().unwrap();
    state.list[at].slots.retain(|slot| !slot.is_empty());
    let mut other = Pool::new(Frame::Free, "Newer", 1_800_000_000);
    other.changed_at = 1_800_000_000;
    state.list.insert(0, other);
    state.update(Message::AddPanel(true), 1_790_000_500);
    let pool = state.editing().unwrap();
    assert_eq!(pool.id, id);
    assert_eq!(pool.slots.len(), 5, "a free slot was added for the new map");
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.selected == Some(4) && editor.panel == Panel::Add));
}

#[test]
fn a_free_slot_can_be_taken_away_and_a_saved_pool_cannot_be_changed() {
    let mut state = pool_with_maps("free-slot-and-saved", &["a1", "b2"]);
    let before = state.editing().unwrap().slots.len();
    let free = state.editing().unwrap().first_empty().expect("a new pool keeps free slots");
    state.update(Message::Select(None), 900);
    state.update(Message::Clear(free), 901);
    assert_eq!(state.editing().unwrap().slots.len(), before - 1);
    let at = state.editing_at().unwrap();
    state.list[at].saved = serde_json::from_str(r#"{"id":"publication"}"#).ok();
    assert!(state.list[at].saved.is_some());
    let held = state.editing().unwrap().clone();
    for message in [Message::Rename("Mine now".into()), Message::Clear(0), Message::AddPanel(true), Message::Put("c3".into()), Message::AuthorDraft("me,".into()), Message::NewCategory, Message::Hot(Hot::Remove)] {
        assert!(message.edits());
        state.update(message, 902);
    }
    assert_eq!(state.editing().unwrap().slots, held.slots);
    assert_eq!((state.editing().unwrap().name.as_str(), &state.editing().unwrap().authors), (held.name.as_str(), &held.authors));
    assert!(!Message::Back.edits() && !Message::Share(true).edits() && !Message::Select(Some(0)).edits());
}

#[test]
fn the_shelf_asks_for_the_backdrop_a_pool_really_has_whatever_its_source() {
    let mut state = pool_with_maps("backdrop-truth", &["a1", "b2"]);
    let at = state.editing_at().unwrap();
    let id = state.list[at].id.clone();
    state.list[at].backdrop = Some(pools::Backdrop::of(pools::BackdropFrom::File(PathBuf::from("/pictures/own.png"))));
    state.update(Message::Back, 950);
    let asked = state.covers();
    assert!(asked.iter().any(|(key, path)| *key == format!("backdrop:{id}:/pictures/own.png") && path.as_deref() == Some(std::path::Path::new("/pictures/own.png"))), "{asked:?}");
    let pool = state.list.iter().find(|pool| pool.id == id).unwrap().clone();
    assert_eq!(backing_key(&pool), Some(format!("backdrop:{id}:/pictures/own.png")));
    let at = state.list.iter().position(|pool| pool.id == id).unwrap();
    state.list[at].backdrop = Some(pools::Backdrop::of(pools::BackdropFrom::File(PathBuf::from("/pictures/other.png"))));
    assert_ne!(backing_key(&state.list[at]), backing_key(&pool), "another picture is another key, so an old one is never shown for it");
    state.list[at].backdrop = None;
    assert!(backing_key(&state.list[at]).is_none() && !state.covers().iter().any(|(key, _)| key.starts_with("backdrop:")));
}

#[test]
fn a_suggestion_also_asks_the_osu_database_and_a_pick_from_it_is_looked_up_by_its_id() {
    let mut state = pool_with_maps("suggest-online", &["a1", "b2"]);
    let at = state.editing_at().unwrap();
    state.list[at].slots[0].measure = Some(measure(5.0));
    state.list[at].slots[1].measure = Some(measure(5.4));
    let free = state.editing().unwrap().first_empty().unwrap();
    let effects = state.update(Message::Suggest(free), 1000);
    let (request, low, high) = effects.iter().find_map(|effect| match effect { Effect::SuggestRemote(request, _, low, high) => Some((*request, *low, *high)), _ => None }).expect("the server is asked too");
    assert!(high - low > 0.39 && high - low < 0.41);
    let offered = crate::bot::Offered { id: 5114204, artist: "Marrow".into(), title: "Far Shore".into(), version: "Extra".into(), stars: 5.3, md5: "z9".into() };
    state.update(Message::Offered(request + 1, Ok(vec![offered.clone()])), 1001);
    assert!(state.suggestions.as_ref().unwrap().offered.is_none(), "an answer to an older question is dropped");
    state.update(Message::Offered(request, Ok(vec![offered])), 1002);
    assert_eq!(state.suggestions.as_ref().unwrap().offered.as_ref().unwrap().as_ref().unwrap().len(), 1);
    let effects = state.update(Message::TakeOffered(5114204), 1003);
    assert!(matches!(effects.as_slice(), [Effect::Resolve(..)]), "{effects:?}");
    state.list[at].slots[free].category = "FS".into();
    state.update(Message::Select(None), 1004);
    let effects = state.update(Message::Suggest(free), 1005);
    assert!(!effects.iter().any(|effect| matches!(effect, Effect::SuggestRemote(..))), "a custom category has no star range to ask for");
}

#[test]
fn a_shut_sheet_is_still_drawn_for_a_moment_on_its_way_back_into_its_button() {
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut state = pool_with_maps("sheet-leaves", &["a1"]);
    state.update(Message::Select(None), 1100);
    let mut marks = Marks::default();
    let began = Instant::now();
    marks.observe(&state, began);
    state.update(Message::Categories(true), 1101);
    marks.observe(&state, began + Duration::from_millis(500));
    assert!(marks.clocks(began + Duration::from_millis(600)).shut.is_empty());
    state.update(Message::Categories(false), 1102);
    let closed = began + Duration::from_secs(1);
    marks.observe(&state, closed);
    assert!(marks.animating(closed + Duration::from_millis(40)), "frames keep coming while it leaves");
    let seen = |clocks: &Clocks| {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1400.0, 900.0), view(&state, &words, &thumbs, 1400.0, 1.0, clocks));
        screen.find(words.t("pool-category-add")).is_ok()
    };
    let leaving = marks.clocks(closed + Duration::from_millis(60));
    assert!(matches!(leaving.shut.as_slice(), [("categories", age)] if *age > 0.05 && *age < 0.07));
    assert!(seen(&leaving), "the sheet is on its way out");
    assert!(!seen(&marks.clocks(closed + Duration::from_millis(400))), "and then it is gone");
}

#[test]
fn shift_and_a_click_mark_the_whole_run_between_two_slots() {
    let mut state = pool_with_maps("shift-run", &["a1", "b2", "c3", "d4"]);
    state.update(Message::Select(Some(0)), 500);
    state.shift = true;
    state.update(Message::Select(Some(2)), 501);
    let Screen::Editor(editor) = &state.screen else { panic!() };
    assert!(editor.choosing && editor.selected.is_none());
    assert_eq!(editor.marked, [0, 1, 2]);
    state.update(Message::Mark(3), 502);
    assert_eq!(shown(&state).marked, [0, 1, 2, 3]);
    state.shift = false;
    state.update(Message::Mark(1), 503);
    assert_eq!(shown(&state).marked, [0, 2, 3], "without Shift a click still takes one slot in or out");
}

#[test]
fn escape_backs_out_one_step_at_a_time_and_enter_confirms_what_is_asked() {
    let mut state = pool_with_maps("escape-enter", &["a1", "b2"]);
    state.update(Message::Select(None), 600);
    assert!(state.cancel_next().is_none() && state.confirm_next().is_none(), "nothing is open");
    state.update(Message::Find("glass".into()), 601);
    state.update(Message::Choosing(true), 602);
    state.update(Message::Categories(true), 603);
    state.update(Message::AskDelete(true), 604);
    assert!(matches!(state.confirm_next(), Some(Message::DeletePool)));
    for expected in ["delete", "categories", "choosing", "find"] {
        let step = match state.cancel_next() {
            Some(Message::AskDelete(false)) => "delete",
            Some(Message::Categories(false)) => "categories",
            Some(Message::Choosing(false)) => "choosing",
            Some(Message::Find(_)) => "find",
            other => panic!("{other:?}"),
        };
        assert_eq!(step, expected);
        state.update(Message::Hot(Hot::Cancel), 605);
    }
    assert!(state.cancel_next().is_none());
    state.update(Message::AskDelete(true), 606);
    state.update(Message::Hot(Hot::Confirm), 607);
    assert!(matches!(state.screen, Screen::Shelf), "Enter answered the question");
}

#[test]
fn mappoolers_are_added_by_name_and_taken_away_one_by_one() {
    let mut state = pool_with_maps("makers-inline", &["a1"]);
    state.update(Message::AuthorDraft("Kite".into()), 700);
    state.update(Message::AddAuthor, 701);
    state.update(Message::AuthorDraft("Nova Tide,".into()), 702);
    state.update(Message::AuthorDraft("kite".into()), 703);
    state.update(Message::AddAuthor, 704);
    assert_eq!(state.editing().unwrap().authors, ["Kite", "Nova Tide"], "a name with a space stays one name");
    assert!(state.editing().unwrap().authors.contains(&"Kite".to_owned()) && state.editing().unwrap().authors.iter().filter(|name| name.eq_ignore_ascii_case("kite")).count() == 1, "a name is not added twice");
    assert!(shown(&state).author_draft.is_empty());
    state.update(Message::DropAuthor("Kite".into()), 705);
    assert!(!state.editing().unwrap().authors.contains(&"Kite".to_owned()));
    assert_eq!(pools::load_all(&state.dir)[0].authors, state.editing().unwrap().authors);
}

#[test]
fn a_category_is_made_with_a_colour_from_the_palette_recoloured_and_removed() {
    let mut state = pool_with_maps("categories-panel", &["a1"]);
    state.update(Message::Categories(true), 800);
    state.update(Message::CategoryDraft("fs1".into()), 801);
    assert_eq!(shown(&state).category_draft, "FS");
    state.update(Message::Tint(None, SWATCHES[3]), 802);
    let before = state.editing().unwrap().slots.len();
    state.update(Message::NewCategory, 803);
    let pool = state.editing().unwrap();
    assert_eq!(pool.categories.get("FS"), Some(&SWATCHES[3]));
    assert_eq!(pool.slots.len(), before + 1, "the new category comes with a free slot so that it is seen");
    assert_eq!((pool.slots[before].category.as_str(), pool.slots[before].colour), ("FS", Some(SWATCHES[3])));
    state.update(Message::CategoryDraft("fs".into()), 804);
    state.update(Message::NewCategory, 805);
    assert_eq!(state.editing().unwrap().slots.len(), before + 1, "a name already taken makes nothing");
    state.update(Message::CategoryDraft("hd".into()), 806);
    state.update(Message::NewCategory, 807);
    assert!(!state.editing().unwrap().categories.contains_key("HD"), "the standard names are not for taking");
    state.update(Message::Tint(Some("FS".into()), SWATCHES[6]), 808);
    assert_eq!(state.editing().unwrap().slots[before].colour, Some(SWATCHES[6]));
    state.update(Message::DropCategory("FS".into()), 809);
    let pool = state.editing().unwrap();
    assert!(pool.categories.is_empty() && pool.slots[before].category.is_empty() && pool.slots[before].colour.is_none());
    state.update(Message::Undo, 810);
    assert!(state.editing().unwrap().categories.contains_key("FS"));
}

#[test]
fn categories_stand_one_under_another_and_their_tiles_in_centred_rows() {
    let mut state = pool_with_maps("responsive-view", &["a1", "b2", "c3"]);
    state.editor_mut().unwrap().panel = Panel::Closed;
    let at = state.editing_at().unwrap();
    state.list[at].slots[2].mods = Mod::Hd;
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    for width in [480.0, 980.0, 1920.0] {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(width, 900.0), view(&state, &words, &thumbs, width, 1.0, &Clocks::settled()));
        let first = screen.find("Glass Orchard").unwrap().visible_bounds().unwrap();
        let second = screen.find("Salt and Static").unwrap().visible_bounds().unwrap();
        let third = screen.find("Ninth Window").unwrap().visible_bounds().unwrap();
        assert!(third.y > first.y && third.y > second.y, "{width}: another category stands below");
        if width < 500.0 {
            assert!(second.y > first.y, "{width}: a narrow window holds one tile in a row");
        } else {
            assert!((first.y - second.y).abs() < 1.0 && second.x > first.x, "{width}: one category shares a row");
            assert!(third.x > first.x + 40.0, "{width}: a row that is not full stands in the middle, not at the left edge");
        }
    }
}

#[test]
fn adding_a_map_in_a_small_window_keeps_the_source_buttons_visible() {
    let mut state = pool_with_maps("add-overlay-view", &["a1", "b2", "c3"]);
    state.update(Message::AddPanel(true), 1_790_000_100);
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(980.0, 600.0), view(&state, &words, &thumbs, 980.0, 1.0, &Clocks::settled()));
    for key in ["pool-tab-search", "pool-tab-collections", "pool-tab-suggest"] {
        let bounds = screen.find(words.t(key)).unwrap().visible_bounds().unwrap();
        assert!(bounds.x > 0.0 && bounds.x + bounds.width < 980.0 && bounds.y > 0.0 && bounds.y + bounds.height < 600.0, "{key}: {bounds:?}");
    }
}

#[test]
fn empty_search_stays_compact_and_its_answer_hangs_under_the_slot_at_every_width() {
    let mut state = pool_with_maps("compact-add-panel", &["a1", "b2"]);
    state.update(Message::AddPanel(true), 101);
    state.update(Message::Query("nothing matches this".into()), 102);
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let Screen::Editor(editor) = &state.screen else { panic!("editor") };
    let mut panel = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(PANEL_WIDE, 800.0), iced::widget::column![add_drop(&state, editor, state.editing().unwrap(), &words, &thumbs), text("end-of-panel")]);
    let bottom = panel.find("end-of-panel").unwrap().visible_bounds().unwrap();
    assert!(bottom.y < 300.0, "{bottom:?}");
    for width in [980.0, 1280.0, 1920.0] {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(width, 800.0), view(&state, &words, &thumbs, width, 1.0, &Clocks::settled()));
        assert!(screen.find(words.t("pool-grouped")).is_err());
        assert!(screen.find(words.t("pool-nothing")).is_ok(), "{width}: the answer hangs under the slot");
    }
}

#[test]
fn dragging_in_the_editor_publishes_a_move_without_opening_the_slot() {
    let mut state = pool_with_maps("drag-view", &["a1", "b2", "c3"]);
    state.editor_mut().unwrap().panel = Panel::Closed;
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1000.0, 900.0), view(&state, &words, &thumbs, 1000.0, 1.0, &Clocks::settled()));
    let first = screen.find("Glass Orchard").unwrap().visible_bounds().unwrap();
    let third = screen.find("Salt and Static").unwrap().visible_bounds().unwrap();
    screen.point_at(first.center());
    let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left))]);
    let position = iced::Point::new(first.center_x(), third.y + ROW_HIGH + 20.0);
    screen.point_at(position);
    let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved { position })]);
    if let Some(dir) = std::env::var_os("DOSSIER_POOL_DRAG_REVIEW").map(PathBuf::from) {
        let mut now = std::time::Instant::now();
        for _ in 0..12 {
            now += std::time::Duration::from_millis(16);
            let _ = screen.simulate([iced::Event::Window(iced::window::Event::RedrawRequested(now))]);
        }
        let shot = screen.snapshot(&theme::theme()).unwrap();
        crate::gallery::write_snapshot(&shot, &dir.join("pool-dragging")).unwrap();
    }
    let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left))]);
    let keys = slot_keys(state.editing().unwrap());
    let messages = screen.into_messages().collect::<Vec<_>>();
    assert!(matches!(messages.as_slice(), [Message::Move(_, _, key, Some(before))] if *key == keys[0] && *before == keys[2]), "{messages:?}");
}

#[test]
fn carrying_a_slot_to_the_page_edge_scrolls_and_escape_stops_it() {
    let mut state = pool_with_maps("drag-edge", &["a1", "b2", "c3", "d4"]);
    state.editor_mut().unwrap().panel = Panel::Closed;
    let words = Words::new(crate::lang::Lang::En);
    let thumbs = HashMap::new();
    let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1000.0, 480.0), view(&state, &words, &thumbs, 1000.0, 1.0, &Clocks::settled()));
    let first = screen.find("Glass Orchard").unwrap().visible_bounds().unwrap();
    screen.point_at(first.center());
    let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left))]);
    let position = iced::Point::new(first.center_x(), 470.0);
    screen.point_at(position);
    let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved { position })]);
    let mut now = std::time::Instant::now();
    for _ in 0..40 {
        now += std::time::Duration::from_millis(16);
        let _ = screen.simulate([iced::Event::Window(iced::window::Event::RedrawRequested(now))]);
    }
    assert!(screen.find(words.t("pool-tool-categories")).unwrap().visible_bounds().is_none(), "a carried slot must scroll the header off screen");
    screen.tap_key(iced::keyboard::key::Named::Escape);
    for _ in 0..40 {
        now += std::time::Duration::from_millis(16);
        let _ = screen.simulate([iced::Event::Window(iced::window::Event::RedrawRequested(now))]);
    }
    assert!(screen.into_messages().all(|message| !matches!(message, Message::Move(..) | Message::Select(..))));
}

#[test]
fn marks_toggle_and_leaving_the_choosing_mode_clears_them() {
    let mut state = pool_with_maps("marks", &["a1", "b2", "c3"]);
    state.update(Message::Choosing(true), 1_790_000_100);
    state.update(Message::Mark(0), 1_790_000_101);
    state.update(Message::Mark(2), 1_790_000_102);
    state.update(Message::Mark(0), 1_790_000_103);
    assert_eq!(state.marked_slots(), vec![2]);
    state.update(Message::Choosing(false), 1_790_000_104);
    assert!(state.marked_slots().is_empty());
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.choosing));
}

#[test]
fn a_bulk_mod_change_reaches_every_marked_slot_and_can_be_undone() {
    let mut state = pool_with_maps("bulk-mod", &["a1", "b2", "c3"]);
    for (at, hash) in ["a1", "b2", "c3"].into_iter().enumerate() {
        state.update(Message::Measured(hash.into(), Mod::Nm, Ok(measure(4.0 + at as f64))), 1_790_000_050);
    }
    state.update(Message::Choosing(true), 1_790_000_100);
    state.update(Message::Mark(0), 1_790_000_101);
    state.update(Message::Mark(1), 1_790_000_102);
    state.update(Message::Bulk(Some(Bulk::Mod)), 1_790_000_103);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.bulk == Some(Bulk::Mod)));
    let effects = state.update(Message::BulkMod(Mod::Dt), 1_790_000_104);
    let pool = state.editing().unwrap();
    assert_eq!((pool.slots[0].mods, pool.slots[1].mods), (Mod::Dt, Mod::Dt));
    assert_ne!(pool.slots[2].mods, Mod::Dt, "an unmarked slot keeps its mod");
    assert!(pool.slots[0].measure.is_none() && pool.slots[1].measure.is_none());
    assert_eq!(effects.iter().filter(|effect| matches!(effect, Effect::Measure(_, _, Mod::Dt))).count(), 2);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.bulk.is_none()));
    state.update(Message::Undo, 1_790_000_105);
    let pool = state.editing().unwrap();
    assert_eq!((pool.slots[0].mods, pool.slots[1].mods), (Mod::Nm, Mod::Nm));
    assert!(pool.slots[0].measure.is_some(), "the old numbers come back with the slot");
}

#[test]
fn marked_slots_move_as_a_block_and_stop_at_the_ends() {
    assert_eq!(shifted(&[1, 2], 5, false), vec![0, 1]);
    assert_eq!(shifted(&[0, 1], 5, false), vec![0, 1], "the top is a wall");
    assert_eq!(shifted(&[2, 4], 5, true), vec![3, 4], "a block at the bottom stays and the other moves up to it");
    assert_eq!(shifted(&[3, 4], 5, true), vec![3, 4]);
    let mut state = pool_with_maps("shift", &["a1", "b2", "c3", "d4"]);
    state.update(Message::Choosing(true), 1_790_000_100);
    state.update(Message::Mark(1), 1_790_000_101);
    state.update(Message::Mark(2), 1_790_000_102);
    state.update(Message::Shift(true), 1_790_000_103);
    assert_eq!(&hashes_of(&state)[..4], [Some("a1".to_owned()), Some("d4".to_owned()), Some("b2".to_owned()), Some("c3".to_owned())]);
    assert_eq!(state.marked_slots(), vec![2, 3], "the marks follow the maps");
    state.update(Message::Shift(false), 1_790_000_104);
    assert_eq!(&hashes_of(&state)[..4], [Some("a1".to_owned()), Some("b2".to_owned()), Some("c3".to_owned()), Some("d4".to_owned())]);
    state.update(Message::Undo, 1_790_000_105);
    assert_eq!(&hashes_of(&state)[..4], [Some("a1".to_owned()), Some("d4".to_owned()), Some("b2".to_owned()), Some("c3".to_owned())], "undo steps back one move at a time");
    assert_eq!(pools::load_all(&state.dir)[0].slots[1].hash.as_deref(), Some("d4"));
}

#[test]
fn removing_the_marked_keeps_frame_slots_and_drops_free_ones_and_undo_brings_them_back() {
    let mut state = pool_with_maps("remove", &["a1", "b2", "c3"]);
    state.update(Message::Choosing(true), 1_790_000_100);
    state.update(Message::Mark(0), 1_790_000_101);
    state.update(Message::Mark(2), 1_790_000_102);
    state.update(Message::RemoveMarked, 1_790_000_103);
    let pool = state.editing().unwrap();
    assert_eq!(pool.slots.len(), 7);
    assert_eq!((pool.slots[0].hash.clone(), pool.slots[1].hash.clone(), pool.slots[2].hash.clone()), (None, Some("b2".to_owned()), None));
    assert!(state.marked_slots().is_empty());
    state.update(Message::Undo, 1_790_000_104);
    assert_eq!(state.editing().unwrap().filled(), 3);

    let mut free = state_with_songs("remove-free");
    open_new(&mut free);
    free.update(Message::UseFrame(Frame::Free), 1_790_000_002);
    for hash in ["a1", "b2", "c3"] {
        free.update(Message::Put(hash.into()), 1_790_000_003);
        free.update(Message::Select(None), 1_790_000_004);
    }
    free.update(Message::Choosing(true), 1_790_000_100);
    free.update(Message::Mark(1), 1_790_000_101);
    free.update(Message::RemoveMarked, 1_790_000_102);
    assert_eq!(hashes_of(&free), vec![Some("a1".to_owned()), Some("c3".to_owned())]);
}

#[test]
fn undo_steps_back_through_a_put_and_does_nothing_when_there_is_nothing_to_undo() {
    let mut state = pool_with_maps("undo", &["a1", "b2"]);
    assert_eq!(state.editing().unwrap().filled(), 2);
    state.update(Message::Undo, 1_790_000_100);
    assert_eq!(state.editing().unwrap().filled(), 1);
    state.update(Message::Undo, 1_790_000_101);
    assert_eq!(state.editing().unwrap().filled(), 0);
    let before = state.editing().unwrap().clone();
    state.update(Message::Undo, 1_790_000_102);
    assert_eq!(state.editing().unwrap(), &before);
}

#[test]
fn failed_delete_keeps_the_collection_and_allows_retry() {
    let mut state = pool_with_maps("delete-failed", &["a1"]);
    let id = state.editing().unwrap().id.clone();
    let file = state.dir.join(format!("{id}.pool"));
    std::fs::remove_file(&file).unwrap();
    std::fs::create_dir(&file).unwrap();
    state.update(Message::AskDelete(true), 1_790_000_100);
    state.update(Message::DeletePool, 1_790_000_101);
    assert!(state.editing().is_some());
    assert!(state.delete_error.is_some());
    std::fs::remove_dir(&file).unwrap();
    state.update(Message::DeletePool, 1_790_000_102);
    assert!(state.list.is_empty());
    assert!(state.delete_error.is_none());
}

#[test]
fn deleting_a_pool_asks_first_removes_the_file_and_returns_to_the_shelf() {
    let mut state = pool_with_maps("delete", &["a1"]);
    let id = state.editing().unwrap().id.clone();
    state.update(Message::AskDelete(true), 1_790_000_100);
    assert!(matches!(&state.screen, Screen::Editor(editor) if editor.asking_delete));
    state.update(Message::AskDelete(false), 1_790_000_101);
    assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.asking_delete));
    assert!(state.dir.join(format!("{id}.pool")).exists());
    state.update(Message::AskDelete(true), 1_790_000_102);
    state.update(Message::DeletePool, 1_790_000_103);
    assert_eq!(state.screen, Screen::Shelf);
    assert!(state.list.is_empty());
    assert!(!state.dir.join(format!("{id}.pool")).exists());
    assert!(state.undo.iter().all(|(pool, _)| *pool != id), "nothing of a deleted pool stays in the undo steps");
}

fn published(id: &str) -> crate::bot::Publication {
    crate::bot::Publication { code: String::new(), id: id.to_owned(), kind: "pool".to_owned(), local_id: String::new(), revision: 1, name: id.to_owned(), content: serde_json::json!({}), mine: false }
}

#[test]
fn the_first_look_is_settled_and_a_new_screen_a_panel_and_a_new_publication_each_start_their_own_clock() {
    let mut state = State::new(scratch("clocks"));
    state.publications.push(published("old"));
    let mut marks = Marks::default();
    let start = Instant::now();
    assert!(marks.clocks(start).screen >= SETTLED - 0.01, "before anything is seen nothing is young");
    marks.observe(&state, start);
    let first = marks.clocks(start);
    assert!(first.screen >= SETTLED - 0.01 && first.panel >= SETTLED - 0.01, "what is already there does not animate");
    assert!(first.of("old") >= SETTLED - 0.01);
    assert!(!marks.animating(start), "and does not ask for frames");
    marks.observe(&state, start + Duration::from_secs(5));
    assert!(marks.clocks(start + Duration::from_secs(5)).screen >= SETTLED - 0.01, "nothing restarts while nothing changes");

    let _ = state.update(Message::New, 1);
    assert!(matches!(state.screen, Screen::Editor(_)));
    let at = start + Duration::from_secs(10);
    marks.observe(&state, at);
    assert!(marks.clocks(at).screen < 0.01, "a new screen starts its clock");
    assert!(marks.animating(at), "and asks for frames");
    let later = marks.clocks(at + Duration::from_millis(500));
    assert!((later.screen - 0.5).abs() < 0.02, "it keeps counting: {}", later.screen);
    assert!(later.panel >= SETTLED - 0.01, "a panel that was never opened has no clock");
    assert!(!marks.animating(at + Duration::from_secs(4)), "and stops asking once it is over");

    let _ = state.update(Message::AddPanel(true), 1);
    let opened = start + Duration::from_secs(12);
    marks.observe(&state, opened);
    assert!(marks.clocks(opened).panel < 0.01, "opening a panel starts its clock");
    let again = marks.clocks(opened + Duration::from_millis(300));
    assert!((again.panel - 0.3).abs() < 0.02);
    assert!(again.screen > 1.0, "and leaves the screen's clock alone");

    let _ = state.update(Message::Back, 1);
    marks.observe(&state, start + Duration::from_secs(14));
    assert!(marks.clocks(start + Duration::from_secs(14)).screen < 0.01, "going back is a new screen too");

    state.publications.push(published("new"));
    let seen = start + Duration::from_secs(20);
    marks.observe(&state, seen);
    let clocks = marks.clocks(seen);
    assert!(clocks.of("new") < 0.01, "an arrival is stamped when it is first seen");
    assert!(clocks.of("old") >= SETTLED - 0.01, "and the ones that were there are not");
    assert!(marks.animating(seen));
    let next = marks.clocks(seen + Duration::from_millis(200));
    assert!((next.of("new") - 0.2).abs() < 0.02);
    assert_eq!(clocks.of("nobody"), SETTLED);
}

#[test]
fn a_stamp_taken_after_the_last_tick_counts_from_nothing_instead_of_going_backwards() {
    let mut state = State::new(scratch("late-stamp"));
    let mut marks = Marks::default();
    let tick = Instant::now();
    marks.observe(&state, tick);
    let _ = state.update(Message::New, 1);
    marks.observe(&state, tick + Duration::from_secs(1));
    let stale = marks.clocks(tick);
    assert_eq!(stale.screen, 0.0, "a clock that is behind the stamp reads zero");
    assert!(marks.animating(tick), "and still asks for the frames that will move it");
}

#[test]
fn a_screen_is_told_from_another_by_what_it_shows_and_a_panel_by_whether_one_is_open() {
    let mut state = State::new(scratch("keys"));
    assert_eq!(state.screen_key(), "shelf:false:Mine");
    state.shelf = Shelf::Saved;
    assert_eq!(state.screen_key(), "shelf:false:Saved");
    state.shelf = Shelf::Mine;
    state.collection_shelf = true;
    assert_eq!(state.screen_key(), "shelf:true:Mine");
    assert_eq!(state.panel_key(), "none");
    state.collection_shelf = false;
    let _ = state.update(Message::New, 1);
    let id = match &state.screen {
        Screen::Editor(editor) => editor.id.clone(),
        _ => panic!("an editor"),
    };
    assert_eq!(state.screen_key(), format!("editor:{id}"));
    let _ = state.update(Message::AddPanel(true), 1);
    assert_eq!(state.panel_key(), "panel");
    let _ = state.update(Message::Select(None), 1);
    assert_eq!(state.panel_key(), "none");
    if let Screen::Editor(editor) = &mut state.screen {
        editor.share = true;
    }
    assert_eq!(state.panel_key(), "share");
}
