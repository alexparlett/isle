//! The Open and Save panels: Files' browser for another app's FileChooser request, a modal
//! child of the asking window (`p1/structure/files.md` § The Open and Save panels).

use std::{cell::Cell, path::PathBuf, rc::Rc};

use freya::prelude::*;
use isle_ui::{Glyph, colour, field::CheckRow, form::PopUp, text, use_theme};

use crate::{
    Shared,
    chooser::{Answer, Ask, Kind, SUCCESS},
    interact::{Choosable, Interact},
    list_view::{Held, Pick},
    memory::PanelMemory,
    places::{self, Place, Places},
    tab::{self, Live},
    toolbar::icon_button,
    view::View,
    window::{Sidebar, TabHandle, content, off_thread},
};

/// The panel's sizes from the shipped tokens, which do not change with the appearance.
pub struct Sizes {
    pub open: (f64, f64),
    pub save: (f64, f64),
    pub min: (f64, f64),
}

/// Save collapsed is only as tall as its rows.
const SAVE_HEIGHT: f64 = 236.;

pub fn sizes() -> &'static Sizes {
    static SIZES: std::sync::OnceLock<Sizes> = std::sync::OnceLock::new();
    SIZES.get_or_init(|| {
        let theme = isle_tokens::Source::shipped().theme(&isle_tokens::Appearance::default()).expect("the shipped tokens resolve");
        let t = &theme.tokens;
        Sizes {
            open: (t.size_chooser_width as f64, t.size_chooser_height as f64),
            save: (t.size_chooser_save as f64, SAVE_HEIGHT),
            min: (t.size_chooser_save as f64, 200.),
        }
    })
}

#[derive(Clone)]
pub struct PanelWindow {
    pub shared: Rc<Shared>,
    pub ask: Ask,
}

/// Where the panel opens: the folder asked for, else the one this app last used, else Home.
fn start_folder(ask: &Ask, kept: &PanelMemory) -> PathBuf {
    [ask.current_folder.clone(), kept.folder.clone()]
        .into_iter()
        .flatten()
        .find(|p| p.is_dir())
        .or_else(places::home)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// The folders from `folder` up to the root, as the folder pop-up lists them.
fn ancestors(folder: &std::path::Path) -> Vec<PathBuf> {
    folder.ancestors().map(PathBuf::from).collect()
}

/// The names SaveFiles or Save would replace in `folder`.
fn taken(folder: &std::path::Path, names: &[String]) -> Vec<String> {
    names.iter().filter(|n| folder.join(n).exists()).cloned().collect()
}

impl App for PanelWindow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let shared = self.shared.clone();
        let ask = Rc::new(self.ask.clone());
        let kept = use_hook({
            let (shared, app) = (shared.clone(), ask.app_id.clone());
            move || shared.panel_memory(&app)
        });
        let handle: TabHandle = use_hook({
            let (shared, ask, kept) = (shared.clone(), ask.clone(), kept.clone());
            move || {
                let mut first = crate::tab::Tab::new(shared.next_tab(), Place::Folder(start_folder(&ask, &kept)));
                first.folder.set_show_hidden(*shared.hidden.peek());
                let view = kept.view;
                let memory = shared.clone();
                let handle = TabHandle {
                    tab: State::create(first),
                    live: Live::new(Rc::new(move |place: &Place| {
                        let mut shown = memory.shown_for(place);
                        if let Some(view) = view {
                            shown.view = view;
                        }
                        shown
                    })),
                    scroll: ScrollController::new(0, 0),
                };
                tab::load(handle.tab, &handle.live, false);
                handle
            }
        });
        let mut tab = handle.tab;
        let mut places = use_state(Places::default);
        use_hook(move || {
            spawn(async move {
                if let Some(read) = off_thread(places::read).await {
                    places.set(read);
                }
            });
        });
        let held = use_state(Held::default);
        let query = use_state(String::new);
        let mut filter_at = use_state(|| ask.current_filter.unwrap_or(0));
        let mut values = use_state({
            let ask = ask.clone();
            move || ask.choices.iter().map(|c| (c.id.clone(), c.initial.clone())).collect::<Vec<_>>()
        });
        let mut name = use_state({
            let ask = ask.clone();
            move || ask.current_name.clone().unwrap_or_else(|| "Untitled".into())
        });
        let expanded = use_state(|| kept.expanded);
        // The names a save would replace, and the paths it answers with once replaced.
        let mut replacing = use_state(|| None::<(Vec<String>, Vec<PathBuf>)>);
        let answered = use_hook(|| Rc::new(Cell::new(false)));
        let drop_target = use_state(|| None::<PathBuf>);
        let press = use_state(|| None::<(PathBuf, f64, f64)>);

        // Closed any other way, it answers cancel.
        use_drop({
            let (shared, answered, id) = (shared.clone(), answered.clone(), ask.id);
            move || {
                if !answered.replace(true) {
                    shared.answer_choose(id, Answer::cancelled());
                }
            }
        });
        // What is typed in search filters the folder shown.
        use_side_effect(move || {
            let typed = query.read().trim().to_lowercase();
            let want = (!typed.is_empty()).then_some(typed);
            if tab.peek().folder.filter() != want.as_deref() {
                let mut t = tab.write();
                t.folder.set_filter(want.as_deref());
                t.prune();
                t.rev += 1;
            }
        });

        let filter = ask.filters.get(*filter_at.read()).cloned();
        let folders_only = ask.directory || ask.kind == Kind::SaveFiles;
        let chooses = Choosable { filter: filter.clone().filter(|_| !folders_only), folders_only };
        let folder = tab.read().place.folder().map(PathBuf::from);

        let finish = {
            let (shared, ask, answered) = (shared.clone(), ask.clone(), answered.clone());
            let filter = filter.clone();
            move |paths: Vec<PathBuf>| {
                if answered.replace(true) {
                    return;
                }
                let view = tab.peek().shown.view;
                let here = tab.peek().place.folder().map(PathBuf::from);
                shared.keep_panel(&ask.app_id, PanelMemory { folder: here, view: Some(view), expanded: *expanded.peek() });
                if ask.kind != Kind::SaveFiles && !ask.directory {
                    let (files, app) = (paths.clone(), ask.app_id.clone());
                    std::thread::spawn(move || {
                        for path in files {
                            let mime = isle_apps::mimeapps::kind_of(&path);
                            if let Err(err) = isle_fs::recents::add(&path, &mime, &app, &app) {
                                tracing::warn!("not added to Recents: {err}");
                            }
                        }
                    });
                }
                let answer = Answer {
                    response: SUCCESS,
                    uris: paths.iter().map(|p| isle_fs::recents::uri(p)).collect(),
                    choices: values.peek().clone(),
                    filter: filter.clone(),
                };
                shared.answer_choose(ask.id, answer);
            }
        };
        let cancel = {
            let (shared, ask, answered) = (shared.clone(), ask.clone(), answered.clone());
            move || {
                if !answered.replace(true) {
                    shared.answer_choose(ask.id, Answer::cancelled());
                }
            }
        };
        let accept = {
            let (ask, finish, chooses, live) = (ask.clone(), finish.clone(), chooses.clone(), handle.live.clone());
            move || {
                let mut replacing = replacing;
                let t = tab.peek();
                let Some(here) = t.place.folder().map(PathBuf::from) else { return };
                let picked: Vec<_> = t.selection.iter().filter_map(|p| t.folder.get(p).cloned()).collect();
                drop(t);
                match ask.kind {
                    Kind::Open => {
                        // A folder alone selected in a file request is gone into, as Finder does.
                        if let [one] = picked.as_slice()
                            && one.is_dir()
                            && !ask.directory
                        {
                            tab::go(tab, &live, Place::Folder(one.path.clone()), None);
                            return;
                        }
                        let mut paths: Vec<PathBuf> = picked.iter().filter(|e| chooses.allows(e)).map(|e| e.path.clone()).collect();
                        if paths.is_empty() && ask.directory {
                            paths.push(here);
                        }
                        if !ask.multiple {
                            paths.truncate(1);
                        }
                        if !paths.is_empty() {
                            finish(paths);
                        }
                    }
                    Kind::Save => {
                        let typed = name.peek().trim().to_owned();
                        if typed.is_empty() || typed.contains('/') {
                            return;
                        }
                        let path = here.join(&typed);
                        if path.exists() {
                            replacing.set(Some((vec![typed], vec![path])));
                        } else {
                            finish(vec![path]);
                        }
                    }
                    Kind::SaveFiles => {
                        let into = match picked.as_slice() {
                            [one] if one.is_dir() => one.path.clone(),
                            _ => here,
                        };
                        let paths: Vec<PathBuf> = ask.files.iter().map(|n| into.join(n)).collect();
                        let names = taken(&into, &ask.files);
                        if names.is_empty() {
                            finish(paths);
                        } else {
                            replacing.set(Some((names, paths)));
                        }
                    }
                }
            }
        };

        let on_pick = EventHandler::new({
            let (ask, finish, chooses, live) = (ask.clone(), finish.clone(), chooses.clone(), handle.live.clone());
            move |pick: Pick| match pick {
                Pick::Click(path, held) => {
                    let mut t = tab.write();
                    if ask.multiple && held.cmd {
                        t.toggle(path.clone());
                    } else if ask.multiple && held.shift {
                        t.extend_to(&path);
                    } else {
                        t.select_only(path.clone());
                    }
                    let entry = t.folder.get(&path).cloned();
                    drop(t);
                    // Choosing a file in Save takes its name.
                    if ask.kind == Kind::Save
                        && let Some(entry) = entry.filter(|e| !e.is_dir())
                    {
                        name.set(entry.name);
                    }
                }
                Pick::Open(path, _) => {
                    let entry = tab.peek().folder.get(&path).cloned();
                    let Some(entry) = entry else { return };
                    if entry.is_dir() {
                        tab::go(tab, &live, Place::Folder(path), None);
                    } else if ask.kind == Kind::Open && chooses.allows(&entry) {
                        finish(vec![path]);
                    }
                }
                Pick::Nothing => tab.write().deselect(),
            }
        });

        let on_key = {
            let (accept, cancel) = (accept.clone(), cancel.clone());
            move |e: Event<KeyboardEventData>| match e.key {
                Key::Named(NamedKey::Escape) => cancel(),
                Key::Named(NamedKey::Enter) if replacing.peek().is_none() => accept(),
                _ => {}
            }
        };

        let interact = Interact {
            rename: None,
            cut: Vec::new(),
            dragging: false,
            drop_target,
            press,
            chooses: Some(chooses.clone()),
        };
        let focused = *Platform::get().is_app_focused.read();
        let rev = tab.read().rev;
        let saving_collapsed = ask.kind == Kind::Save && !*expanded.read();
        let accept_label = ask.accept_label.clone().unwrap_or_else(|| match ask.kind {
            Kind::Open if ask.directory => "Choose".into(),
            Kind::Open => "Open".into(),
            _ => "Save".into(),
        });
        let can_accept = match ask.kind {
            Kind::Open => {
                let t = tab.read();
                ask.directory || t.selection.iter().filter_map(|p| t.folder.get(p)).any(|e| e.is_dir() || chooses.allows(e))
            }
            Kind::Save => !name.read().trim().is_empty(),
            Kind::SaveFiles => true,
        };

        let resize = move |open: bool| {
            let (w, h) = if open { sizes().open } else { sizes().save };
            Platform::get().with_window(Platform::window_id(), move |window| {
                let _ = window.request_inner_size(freya::winit::dpi::LogicalSize::new(w, h));
            });
        };
        let toggle_expanded = move || {
            let mut expanded = expanded;
            let open = !*expanded.peek();
            expanded.set(open);
            resize(open);
        };

        // The foot: Show and the choices, then New Folder, Cancel and the default.
        let filters_row = (!ask.filters.is_empty() && !folders_only).then(|| {
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .child(text::line(&theme, text::Style::Body, colour(t.colour_text), if ask.kind == Kind::Save { "Format:" } else { "Show:" }))
                .child(PopUp::new(
                    ask.filters.iter().map(|f| f.name.clone()).collect(),
                    *filter_at.read(),
                    "Show",
                    {
                        let ask = ask.clone();
                        move |i: usize| {
                            filter_at.set(i);
                            if ask.kind == Kind::Save
                                && let Some(f) = ask.filters.get(i)
                            {
                                let next = crate::chooser::with_extension(&name.peek(), f);
                                name.set(next);
                            }
                        }
                    },
                ))
                .into_element()
        });
        let choice_rows = ask.choices.iter().map(|c| {
            let id = c.id.clone();
            let now = values.read().iter().find(|(i, _)| *i == c.id).map(|(_, v)| v.clone()).unwrap_or_default();
            if c.options.is_empty() {
                CheckRow::new(c.label.clone(), now == "true")
                    .on_toggle(move |on: bool| {
                        let mut list = values.write();
                        if let Some(v) = list.iter_mut().find(|(i, _)| *i == id) {
                            v.1 = on.to_string();
                        }
                    })
                    .into_element()
            } else {
                let options = c.options.clone();
                let at = options.iter().position(|(o, _)| *o == now).unwrap_or(0);
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(t.space_s)
                    .child(text::line(&theme, text::Style::Body, colour(t.colour_text), format!("{}:", c.label)))
                    .child(PopUp::new(options.iter().map(|(_, l)| l.clone()).collect(), at, c.label.clone(), move |i: usize| {
                        let mut list = values.write();
                        if let Some(v) = list.iter_mut().find(|(k, _)| *k == id) {
                            v.1 = options[i].0.clone();
                        }
                    }))
                    .into_element()
            }
        });
        let options_row = rect()
            .width(Size::fill())
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_l)
            .maybe_child(filters_row)
            .children(choice_rows);
        let new_folder = move || {
            let mut tab = tab;
            let Some(into) = tab.peek().place.folder().map(PathBuf::from) else { return };
            let made = into.join(isle_fs::jobs::free_name(&into, "untitled folder", true));
            if std::fs::create_dir(&made).is_ok() {
                tab.write().reveal = Some(made);
            }
        };
        let buttons = rect()
            .width(Size::fill())
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .child(isle_ui::Button::new("New Folder").enabled(folder.is_some()).on_press(move |_| new_folder()).into_element())
            .child(rect().width(Size::flex(1.)))
            .child(isle_ui::Button::new("Cancel").on_press({
                let cancel = cancel.clone();
                move |_| cancel()
            }))
            .child(
                isle_ui::Button::new(accept_label)
                    .kind(isle_ui::button::Kind::Default)
                    .enabled(can_accept)
                    .on_press({
                        let accept = accept.clone();
                        move |_| accept()
                    }),
            );
        let save_field = |width: f32| {
            let accept = accept.clone();
            isle_ui::TextField::new(name, "Save As")
                .editing()
                .width(Size::px(width))
                .on_submit(move |_: String| accept())
                .into_element()
        };
        let label = |words: &str| {
            rect()
                .width(Size::px(t.size_chooser_label))
                .main_align(Alignment::End)
                .horizontal()
                .child(text::line(&theme, text::Style::Body, colour(t.colour_text), words.to_owned()))
                .into_element()
        };
        let expand_button = |up: bool| {
            rect()
                .width(Size::px(t.size_control))
                .height(Size::px(t.size_control))
                .center()
                .corner_radius(t.radius_control)
                .background(colour(t.colour_control))
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt(if up { "Show less" } else { "Show the browser" })
                .on_press(move |_| toggle_expanded())
                .child(Glyph::new(if up { "caret-up" } else { "caret-down" }, 12., colour(t.colour_text)))
                .into_element()
        };

        let body: Element = if saving_collapsed {
            // Where: the folder shown, recent folders, then Favourites.
            let mut wheres: Vec<PathBuf> = folder.iter().cloned().collect();
            for p in places::recents(20).iter().filter_map(|p| p.parent().map(PathBuf::from)) {
                if !wheres.contains(&p) && wheres.len() < 6 {
                    wheres.push(p);
                }
            }
            for row in &places.read().favourites {
                if let Place::Folder(p) = &row.place
                    && !wheres.contains(p)
                {
                    wheres.push(p.clone());
                }
            }
            let names = wheres.iter().map(|p| places::folder_name(p)).collect();
            let live = handle.live.clone();
            rect()
                .expanded()
                .padding((t.space_l, t.space_xl))
                .spacing(t.space_s)
                .background(colour(t.colour_window))
                .child(rect().horizontal().cross_align(Alignment::Center).spacing(t.space_s).child(label("Save As:")).child(save_field(320.)))
                .child(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(t.space_s)
                        .child(label("Where:"))
                        .child(rect().width(Size::px(320.)).child(PopUp::new(names, 0, "Where", move |i: usize| {
                            if let Some(p) = wheres.get(i) {
                                tab::go(tab, &live, Place::Folder(p.clone()), None);
                            }
                        })))
                        .child(expand_button(false)),
                )
                .child(rect().height(Size::px(t.space_s)))
                .child(options_row)
                .child(rect().height(Size::px(t.space_l)))
                .child(buttons)
                .into_element()
        } else {
            let live = handle.live.clone();
            let shown = tab.read().shown;
            let set_view = move |view: View| {
                let mut tab = tab;
                let mut t = tab.write();
                t.shown.view = view;
                t.rev += 1;
            };
            let here = folder.clone().unwrap_or_default();
            let ups = ancestors(&here);
            let nav = rect()
                .width(Size::fill())
                .height(Size::px(t.size_row_large))
                .padding((0., t.space_m))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .background(colour(t.colour_window))
                .border(
                    Border::new()
                        .fill(colour(t.colour_hairline))
                        .width(BorderWidth { top: 0., right: 0., bottom: 1., left: 0. })
                        .alignment(BorderAlignment::Inner),
                )
                .child(icon_button(&theme, "caret-left", "Back", !tab.read().back.is_empty(), false, {
                    let live = live.clone();
                    move || tab::back(tab, &live)
                }))
                .child(icon_button(&theme, "caret-right", "Forward", !tab.read().forward.is_empty(), false, {
                    let live = live.clone();
                    move || tab::forward(tab, &live)
                }))
                .child(icon_button(&theme, "squares-four", "as Icons", true, shown.view == View::Icons, move || set_view(View::Icons)))
                .child(icon_button(&theme, "list-bullets", "as List", true, shown.view == View::List, move || set_view(View::List)))
                .child(icon_button(&theme, "columns", "as Columns", true, shown.view == View::Columns, move || set_view(View::Columns)))
                .child(rect().width(Size::px(200.)).child(PopUp::new(
                    ups.iter().map(|p| places::folder_name(p)).collect(),
                    0,
                    "Folder",
                    {
                        let live = live.clone();
                        move |i: usize| {
                            if let Some(p) = ups.get(i) {
                                tab::go(tab, &live, Place::Folder(p.clone()), None);
                            }
                        }
                    },
                )))
                .child(rect().width(Size::flex(1.)))
                .child(
                    isle_ui::TextField::new(query, "Search")
                        .placeholder("Search")
                        .width(Size::px(t.size_toolbar_search))
                        .leading(Glyph::new("magnifying-glass", t.size_glyph_small, colour(t.colour_text_2))),
                );
            let sidebar = Sidebar {
                places: places.read().clone(),
                prefs: shared.prefs(),
                saved: Vec::new(),
                shown: tab.read().place.clone(),
                volumes: shared.locations.volumes.read().clone(),
                shares: shared.locations.shares.read().clone(),
                space: shared.locations.space.read().clone(),
                dragging: false,
                drop_target,
                on_go: EventHandler::new({
                    let live = live.clone();
                    move |place: Place| tab::go(tab, &live, place, None)
                }),
                on_volume: EventHandler::new({
                    let live = live.clone();
                    move |volume: isle_disks::Volume| {
                        if let Some(point) = volume.mount_point {
                            tab::go(tab, &live, Place::Folder(PathBuf::from(point)), None);
                        }
                    }
                }),
                on_eject: EventHandler::new(|_| {}),
                width: t.size_sidebar_min,
                under_toolbar: false,
            };
            let body = content(&theme, &handle, rev, focused, held, on_pick, interact);
            let foot = rect()
                .width(Size::fill())
                .padding((t.space_m, t.space_l))
                .spacing(t.space_s)
                .background(colour(t.colour_window))
                .border(
                        Border::new()
                            .fill(colour(t.colour_hairline))
                            .width(BorderWidth { top: 1., right: 0., bottom: 0., left: 0. })
                            .alignment(BorderAlignment::Inner),
                )
                .child(options_row)
                .child(buttons);

            rect()
                .expanded()
                .content(Content::flex())
                .background(colour(t.colour_window))
                .maybe_child((ask.kind == Kind::Save).then(|| {
                    rect()
                        .width(Size::fill())
                        .height(Size::px(t.size_row_large))
                        .padding((0., t.space_m))
                        .horizontal()
                        .center()
                        .spacing(t.space_s)
                        .child(label("Save As:"))
                        .child(save_field(280.))
                        .child(expand_button(true))
                        .into_element()
                }))
                .child(nav)
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::flex(1.))
                        .horizontal()
                        .content(Content::flex())
                        .child(sidebar)
                        .child(rect().width(Size::flex(1.)).height(Size::fill()).child(body)),
                )
                .child(foot)
                .into_element()
        };

        let replace = replacing.read().clone().map(|(names, paths)| {
            let finish = finish.clone();
            let place = folder.as_deref().map(places::folder_name).unwrap_or_default();
            let title = match names.as_slice() {
                [one] => format!("An item named “{one}” already exists in “{place}”."),
                many => format!("{} items with these names already exist in “{place}”.", many.len()),
            };
            isle_ui::sheet(
                t.size_alert,
                vec![isle_ui::alert_text(&title, "Replacing it overwrites it.")],
                vec![
                    ("Replace".into(), EventHandler::new(move |_| finish(paths.clone()))),
                    ("Cancel".into(), EventHandler::new(move |_| replacing.set(None))),
                ],
            )
        });

        rect()
            .expanded()
            .background(colour(t.colour_window))
            .on_global_key_down(on_key)
            .child(body)
            .maybe_child(replace)
    }
}
