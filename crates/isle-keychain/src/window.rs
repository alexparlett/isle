//! Keychain's window (`p1/structure/keychain.md`, `p1/surfaces/keychain.md`): the toolbar,
//! the sidebar of kinds and keyrings, the list and the detail. Secrets are fetched only for
//! the click that asks, behind the reveal gate (choices 16 and 17).

use std::{cell::RefCell, rc::Rc, sync::Arc, time::Instant};

use freya::prelude::*;
use futures_channel::oneshot;
use isle_app::{AppHandle, Chord, ChordKey, Command, Desktop, Menu, ToolbarExt};
use isle_secrets::{gate::Gate, kinds::Kind};
use isle_ui::{Button, SearchField, colour, text, use_theme};
use zeroize::Zeroizing;

use crate::{
    Shared, copy, detail,
    list::{self, List},
    model::{self, Detail, Item, ItemId, Keyring, Row, Snapshot},
    settings, sidebar, state,
    store::{self, Ask},
};

/// The compositor's lights over the toolbar's leading end (`isle_shell::chrome::LIGHTS_END`).
const LIGHTS: f32 = 68.;

/// A secret shown in the detail, until hidden.
#[derive(Clone)]
struct Shown {
    id: ItemId,
    value: Rc<Zeroizing<String>>,
    /// Counts each showing, so a hide timer knows its own.
    n: u64,
}

impl PartialEq for Shown {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.n == other.n
    }
}

/// Work the window does off its handlers, in order.
enum Job {
    /// Fetches the item's secret, behind the gate, to show it or copy it.
    Reveal { id: ItemId, copy: bool },
}

#[derive(Clone)]
pub struct KeychainWindow {
    pub shared: Shared,
}

fn commands(
    item: Option<&Item>,
    row: &Row,
    snapshot: &Snapshot,
    shown: bool,
    editing: bool,
) -> Vec<Command> {
    let keyring = snapshot.keyring == Keyring::Ready;
    let has_secret = item.is_some_and(Item::has_secret);
    let edit_field = |c: Command| if editing { c.field() } else { c };
    let source = selected_source(row, item, snapshot);
    let locked = source.as_ref().is_some_and(|s| s.locked);
    vec![
        Command::new("new-login", "New Login…", Menu::File, 1)
            .chord(Chord::cmd('n'))
            .enabled(keyring && snapshot.sources.iter().any(|s| !s.locked)),
        Command::new("new-key", "New SSH Key…", Menu::File, 1).chord(Chord::cmd('n').shift()),
        edit_field(
            Command::new("copy", "Copy Password", Menu::Edit, 1)
                .chord(Chord::cmd('c'))
                .enabled(has_secret),
        ),
        Command::new("copy-user", "Copy User Name", Menu::Edit, 1)
            .chord(Chord::cmd('c').shift())
            .enabled(item.and_then(Item::user).is_some()),
        Command::new("find", "Search", Menu::Edit, 9).chord(Chord::cmd('f')),
        Command::new(
            "show",
            if shown {
                "Hide Password"
            } else {
                "Show Password"
            },
            Menu::Item,
            1,
        )
        .chord(Chord::cmd('r'))
        .enabled(has_secret),
        Command::new("edit", "Edit", Menu::Item, 2).enabled(item.is_some_and(Item::editable)),
        Command::new("delete", "Delete…", Menu::Item, 2)
            .chord(Chord::cmd_key(ChordKey::Backspace))
            .enabled(item.is_some_and(|i| i.editable() || i.kind == Kind::SshKey)),
        Command::new("generate", "Generate Password…", Menu::Item, 3)
            .chord(Chord::cmd('g').option()),
        Command::new("lock", "Lock", Menu::Item, 4).enabled(keyring && source.is_some() && !locked),
        Command::new("unlock", "Unlock…", Menu::Item, 4).enabled(keyring && locked),
        Command::new("change-password", "Change Password…", Menu::Item, 4)
            .enabled(keyring && source.is_some() && snapshot.can_change_password),
        Command::new("sidebar", "Show Sidebar", Menu::View, 1).chord(Chord::cmd('s').ctrl()),
    ]
}

/// The keyring Lock, Unlock and Change password… act on: the row's, the item's, else the
/// default.
fn selected_source(row: &Row, item: Option<&Item>, snapshot: &Snapshot) -> Option<model::Source> {
    let path = match row {
        Row::Source(p) => Some(p.clone()),
        _ => item.and_then(|i| i.collection.clone()),
    };
    match path {
        Some(p) => snapshot.sources.iter().find(|s| s.path == p).cloned(),
        None => snapshot
            .sources
            .iter()
            .find(|s| s.default)
            .or(snapshot.sources.first())
            .cloned(),
    }
}

impl App for KeychainWindow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let desktop = Desktop::get();
        let app = use_consume::<AppHandle>();
        let worker = isle_app::worker();
        let window_shown = isle_app::use_shown();
        // Unfocused, the window takes `conventions.md`'s inactive form.
        let active = *Platform::get().is_app_focused.read();
        let mut remembered = use_state(state::load);
        use_side_effect(move || state::save(&remembered.read()));
        let mut snapshot = use_state(|| Arc::new(Snapshot::default()));
        let mut selected = use_state(|| None::<ItemId>);
        let query = use_state(String::new);
        let mut shown = use_state(|| None::<Shown>);
        let mut sidebar_hidden = use_state(|| false);
        let mut width = use_state(|| 1000f32);
        let mut item_menu = use_state(|| None::<(ItemId, (f32, f32))>);
        let mut source_menu = use_state(|| None::<(String, (f32, f32))>);
        let mut more_menu = use_state(|| None::<(f32, f32)>);
        let mut dragging = use_state(|| None::<(f32, f32)>);
        let gate = use_hook(|| Rc::new(RefCell::new(Gate::default())));
        let shared = self.shared.clone();

        let (store, incoming) = use_hook(move || {
            let (store, rx) = store::start(&worker);
            (store, Rc::new(RefCell::new(Some(rx))))
        });
        use_hook({
            let gate = gate.clone();
            move || {
                let Some(mut rx) = incoming.borrow_mut().take() else {
                    return;
                };
                spawn(async move {
                    use futures_util::StreamExt;
                    while let Some(event) = rx.next().await {
                        match event {
                            store::Event::Snapshot(snap) => snapshot.set(snap),
                            store::Event::Locked(true) => {
                                shown.set(None);
                                gate.borrow_mut().end();
                            }
                            store::Event::Locked(false) => {}
                        }
                    }
                });
            }
        });
        let jobs = use_hook({
            let (store, gate, app, copies) = (
                store.clone(),
                gate.clone(),
                app.clone(),
                shared.copies.clone(),
            );
            move || {
                let (tx, mut rx) = futures_channel::mpsc::unbounded::<Job>();
                spawn(async move {
                    use futures_util::StreamExt;
                    let mut count = 0u64;
                    while let Some(job) = rx.next().await {
                        let Job::Reveal { id, copy } = job;
                        if !gate.borrow().held(Instant::now()) {
                            let (tx, rx) = oneshot::channel();
                            store.ask(Ask::Authorize(tx));
                            if rx.await != Ok(true) {
                                continue;
                            }
                            gate.borrow_mut().grant(Instant::now());
                        }
                        let (tx, rx) = oneshot::channel();
                        store.ask(Ask::Reveal(id.clone(), tx));
                        match rx.await {
                            Ok(Ok(Some(value))) if copy => {
                                copy::secret(&app, &copies, value, settings::clear_after(&desktop));
                            }
                            Ok(Ok(Some(value))) => {
                                count += 1;
                                let n = count;
                                shown.set(Some(Shown {
                                    id,
                                    value: Rc::new(value),
                                    n,
                                }));
                                let after = settings::hide_after(&desktop);
                                spawn(async move {
                                    copy::sleep(after).await;
                                    if shown.peek().as_ref().is_some_and(|s| s.n == n) {
                                        shown.set(None);
                                    }
                                });
                            }
                            Ok(Ok(None)) => {}
                            Ok(Err(failure)) => tracing::info!(?failure, "nothing revealed"),
                            Err(_) => {}
                        }
                    }
                });
                tx
            }
        });

        // A shown value hides when the window is not shown: minimised, covered or locked.
        use_side_effect(move || {
            if !*window_shown.read() && shown.peek().is_some() {
                shown.set(None);
            }
        });

        let snap = snapshot.read().clone();
        let r = remembered.read().clone();
        let row = match &r.row {
            Row::Source(p) if !snap.sources.iter().any(|s| &s.path == p) && snap.loaded => Row::All,
            other => other.clone(),
        };
        let items: Rc<Vec<Item>> = Rc::new(
            model::listed(&snap, &row, &query.read())
                .into_iter()
                .cloned()
                .collect(),
        );
        // The selection follows the list: kept while listed, else the first row.
        let current = selected
            .read()
            .clone()
            .filter(|id| items.iter().any(|i| &i.id == id));
        let current = current.or_else(|| items.first().map(|i| i.id.clone()));
        if *selected.peek() != current {
            selected.set(current.clone());
        }
        let item = current
            .as_ref()
            .and_then(|id| items.iter().find(|i| &i.id == id))
            .cloned();
        if shown
            .peek()
            .as_ref()
            .is_some_and(|s| Some(&s.id) != current.as_ref())
        {
            shown.set(None);
        }
        let shown_now = shown
            .read()
            .clone()
            .filter(|s| Some(&s.id) == current.as_ref());

        isle_app::use_commands(commands(
            item.as_ref(),
            &row,
            &snap,
            shown_now.is_some(),
            false,
        ));

        let reveal = {
            let jobs = jobs.clone();
            move |id: ItemId, copy: bool| {
                let _ = jobs.unbounded_send(Job::Reveal { id, copy });
            }
        };
        let act_on_source = {
            let store = store.clone();
            move |what: &str, path: String| match what {
                "lock" => store.ask(Ask::Lock(path)),
                "unlock" => store.ask(Ask::Unlock(path)),
                _ => store.ask(Ask::ChangePassword(path)),
            }
        };

        use_side_effect({
            let (app, shared, store, reveal, act_on_source) = (
                app.clone(),
                shared.clone(),
                store.clone(),
                reveal.clone(),
                act_on_source.clone(),
            );
            move || {
                if desktop.commands.read().is_empty() {
                    return;
                }
                let mut queue = desktop.commands;
                let snap = snapshot.peek().clone();
                let id = selected.peek().clone();
                let item = id
                    .as_ref()
                    .and_then(|id| snap.items.iter().find(|i| &i.id == id))
                    .cloned();
                let row = remembered.peek().row.clone();
                for command in std::mem::take(&mut *queue.write()) {
                    match command.as_str() {
                        "settings" => shared.open_settings(&app),
                        "copy" => {
                            if let Some(id) = id.clone() {
                                reveal(id, true);
                            }
                        }
                        "copy-user" => {
                            if let Some(u) = item.as_ref().and_then(Item::user) {
                                copy::plain(&app, u);
                            }
                        }
                        "show" => {
                            let showing = shown.peek().is_some();
                            match (showing, id.clone()) {
                                (true, _) => shown.set(None),
                                (false, Some(id)) => reveal(id, false),
                                _ => {}
                            }
                        }
                        "sidebar" => {
                            let hidden = *sidebar_hidden.peek();
                            sidebar_hidden.set(!hidden);
                        }
                        "lock" | "unlock" | "change-password" => {
                            if let Some(s) = selected_source(&row, item.as_ref(), &snap) {
                                act_on_source(&command, s.path);
                            }
                        }
                        _ => {}
                    }
                }
                let _ = &store;
            }
        });

        let on_list = {
            let store = store.clone();
            EventHandler::new(move |act: list::Act| match act {
                list::Act::Select(id) => selected.set(Some(id)),
                list::Act::Menu(id, at) => {
                    selected.set(Some(id.clone()));
                    item_menu.set(Some((id, at)));
                }
                list::Act::Unlock(path) => store.ask(Ask::Unlock(path)),
                list::Act::NewKey => {}
            })
        };
        let on_detail = {
            let (app, store, reveal) = (app.clone(), store.clone(), reveal.clone());
            let item = item.clone();
            EventHandler::new(move |act: detail::Act| {
                let Some(item) = item.clone() else { return };
                match act {
                    detail::Act::Show => reveal(item.id.clone(), false),
                    detail::Act::Hide => shown.set(None),
                    detail::Act::CopySecret => reveal(item.id.clone(), true),
                    detail::Act::CopyUser => {
                        if let Some(u) = item.user() {
                            copy::plain(&app, u);
                        }
                    }
                    detail::Act::CopyPublicKey => {
                        if let Detail::Ssh { key, .. } = &item.detail {
                            copy::plain(&app, &key.public_key);
                        }
                    }
                    detail::Act::OpenSite(url) => {
                        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
                    }
                    detail::Act::WifiSettings => {
                        let _ = std::process::Command::new("isle-system-settings")
                            .arg("isle-settings://wifi")
                            .spawn();
                    }
                    detail::Act::RemoveFromAgent => {
                        if let ItemId::Ssh(name) = &item.id {
                            store.ask(Ask::RemoveFromAgent(name.clone()));
                        }
                    }
                    detail::Act::AddToAgent | detail::Act::Edit => {}
                }
            })
        };

        // The toolbar.
        let narrow = *width.read();
        let sidebar_shown = (narrow >= 760.) != *sidebar_hidden.read();
        let list_width = r
            .list_width
            .clamp(t.size_list_pane_min, t.size_list_pane_max);
        let lead = if sidebar_shown {
            t.size_sidebar
        } else {
            LIGHTS + t.space_l + t.size_control + t.space_l
        };
        let count = items.len();
        let title_block = rect()
            .width(Size::px(list_width))
            .padding((0., 0., 0., t.space_l))
            .main_align(Alignment::Center)
            .spacing(t.space_xxs)
            .child(text::line(
                &theme,
                text::Style::Headline,
                colour(t.colour_text),
                row.title(&snap),
            ))
            .child(text::line(
                &theme,
                text::Style::Subheadline,
                colour(t.colour_text_2),
                if count == 1 {
                    "1 item".to_owned()
                } else {
                    format!("{count} items")
                },
            ));
        let creates = row.creates().filter(|k| match k {
            Kind::Login => snap.keyring == Keyring::Ready && snap.sources.iter().any(|s| !s.locked),
            _ => true,
        });
        let new_label = match row.creates() {
            Some(Kind::SshKey) => "New SSH key…",
            _ => "New login…",
        };
        let trailing = rect()
            .width(Size::flex(1.))
            .horizontal()
            .main_align(Alignment::End)
            .cross_align(Alignment::Center)
            .spacing(t.space_l)
            .child(Button::new(new_label).enabled(creates.is_some()))
            .child(
                rect()
                    .on_mouse_down(move |e: Event<MouseEventData>| {
                        e.stop_propagation();
                        more_menu.set(Some((
                            e.global_location.x as f32,
                            e.global_location.y as f32 + 12.,
                        )));
                    })
                    .child(Button::icon("dots-three-circle", "More")),
            )
            .child(SearchField {
                value: query.into_writable(),
                placeholder: "Search".into(),
                width: if narrow < 820. { 120. } else { 180. },
            });
        let toolbar = rect()
            .width(Size::fill())
            .height(Size::px(t.size_toolbar))
            .padding((0., t.space_l, 0., 0.))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .toolbar()
            .child(
                rect()
                    .width(Size::px(lead))
                    .height(Size::fill())
                    .padding((0., t.space_s, 0., LIGHTS + t.space_l))
                    .horizontal()
                    .main_align(Alignment::End)
                    .cross_align(Alignment::Center)
                    .maybe(sidebar_shown, |el| el.background(colour(t.colour_sidebar)))
                    .child(
                        Button::icon("sidebar-simple", "Sidebar").on_press(move |_| {
                            let hidden = *sidebar_hidden.peek();
                            sidebar_hidden.set(!hidden);
                        }),
                    ),
            )
            .child(title_block)
            .child(trailing);

        // The sidebar, the list and the detail.
        let sidebar_view = sidebar_shown.then(|| {
            sidebar::view(
                &theme,
                sidebar::Sidebar {
                    snapshot: &snap,
                    selected: &row,
                    active,
                    on_select: EventHandler::new(move |next: Row| {
                        remembered.write().row = next;
                        selected.set(None);
                    }),
                    on_menu: EventHandler::new(move |(row, at): (Row, (f32, f32))| {
                        if let Row::Source(path) = row {
                            source_menu.set(Some((path, at)));
                        }
                    }),
                },
            )
        });
        let keyring_note = row.spans_keyring().then(|| match &snap.keyring {
            Keyring::Missing => Some(
                "No keyring service is running. Choose one in Settings › General › Passwords."
                    .to_owned(),
            ),
            Keyring::Failed(why) => Some(format!("The keyring isn't answering: {why}")),
            _ => None,
        });
        let list_view = List {
            items: items.clone(),
            selected: current.clone(),
            locked: model::locked_in(&snap, &row)
                .into_iter()
                .map(|s| (s.path.clone(), s.label.clone()))
                .collect(),
            keyring_note: keyring_note.flatten(),
            row: row.clone(),
            spans_sources: row.spans_sources(),
            query: query.read().clone(),
            active,
            loaded: snap.loaded,
            on_act: on_list,
        };
        let edge = rect()
            .width(Size::px(6.))
            .height(Size::fill())
            .position(Position::new_absolute().right(-3.).top(0.))
            .on_mouse_down(move |e: Event<MouseEventData>| {
                e.stop_propagation();
                dragging.set(Some((
                    e.global_location.x as f32,
                    remembered.peek().list_width,
                )));
            });
        let list_pane = rect()
            .width(Size::px(list_width))
            .height(Size::fill())
            .background(colour(t.colour_view))
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline))
                    .width(BorderWidth {
                        top: 0.,
                        right: t.stroke_hairline,
                        bottom: 0.,
                        left: 0.,
                    })
                    .alignment(BorderAlignment::Inner),
            )
            .child(list_view)
            .child(edge);
        let can_edit = item.as_ref().is_some_and(Item::editable);
        let detail_view = detail::view(
            &theme,
            detail::View {
                item: item.as_ref(),
                shown: shown_now.as_ref().map(|s| s.value.as_str()),
                can_edit,
                on_act: on_detail,
            },
        );

        // Menus.
        let overlay_item = item_menu.read().clone().and_then(|(id, at)| {
            let it = snap.items.iter().find(|i| i.id == id)?.clone();
            let mut entries = Vec::new();
            if it.has_secret() {
                entries.push(isle_ui::MenuEntry::Item(isle_ui::MenuItem::new(
                    1,
                    "Copy Password",
                )));
            }
            if it.user().is_some() {
                entries.push(isle_ui::MenuEntry::Item(isle_ui::MenuItem::new(
                    2,
                    "Copy User Name",
                )));
            }
            if let Detail::Ssh { in_agent, .. } = &it.detail {
                entries.push(isle_ui::MenuEntry::Item(isle_ui::MenuItem::new(
                    5,
                    if *in_agent {
                        "Remove from agent"
                    } else {
                        "Add to agent"
                    },
                )));
                entries.push(isle_ui::MenuEntry::Item(isle_ui::MenuItem::new(
                    6,
                    "Copy public key",
                )));
            }
            if it.editable() {
                if !entries.is_empty() {
                    entries.push(isle_ui::MenuEntry::Separator);
                }
                entries.push(isle_ui::MenuEntry::Item(isle_ui::MenuItem::new(3, "Edit")));
                entries.push(isle_ui::MenuEntry::Item(isle_ui::MenuItem::new(
                    4,
                    "Delete…",
                )));
            }
            let (app, store, reveal) = (app.clone(), store.clone(), reveal.clone());
            Some(
                isle_ui::ContextMenu::new(
                    Rc::new(entries),
                    at,
                    move |event: isle_ui::MenuEvent| match event {
                        isle_ui::MenuEvent::Chosen(choice) => {
                            item_menu.set(None);
                            match choice {
                                1 => reveal(it.id.clone(), true),
                                2 => {
                                    if let Some(u) = it.user() {
                                        copy::plain(&app, u);
                                    }
                                }
                                5 => {
                                    if let (ItemId::Ssh(name), Detail::Ssh { in_agent: true, .. }) =
                                        (&it.id, &it.detail)
                                    {
                                        store.ask(Ask::RemoveFromAgent(name.clone()));
                                    }
                                }
                                6 => {
                                    if let Detail::Ssh { key, .. } = &it.detail {
                                        copy::plain(&app, &key.public_key);
                                    }
                                }
                                _ => {}
                            }
                        }
                        isle_ui::MenuEvent::Back => item_menu.set(None),
                        _ => {}
                    },
                )
                .into_element(),
            )
        });
        let source_entries = |locked: bool, can_change: bool| {
            Rc::new(vec![
                isle_ui::MenuEntry::Item(isle_ui::MenuItem::new(1, "Lock").enabled(!locked)),
                isle_ui::MenuEntry::Item(isle_ui::MenuItem::new(2, "Unlock…").enabled(locked)),
                isle_ui::MenuEntry::Item(
                    isle_ui::MenuItem::new(3, "Change password…").enabled(can_change),
                ),
            ])
        };
        let overlay_source = source_menu.read().clone().map(|(path, at)| {
            let locked = snap.sources.iter().any(|s| s.path == path && s.locked);
            let act = act_on_source.clone();
            isle_ui::ContextMenu::new(
                source_entries(locked, snap.can_change_password),
                at,
                move |event: isle_ui::MenuEvent| match event {
                    isle_ui::MenuEvent::Chosen(choice) => {
                        source_menu.set(None);
                        act(
                            ["", "lock", "unlock", "change"][choice.min(3) as usize],
                            path.clone(),
                        );
                    }
                    isle_ui::MenuEvent::Back => source_menu.set(None),
                    _ => {}
                },
            )
            .into_element()
        });
        let more_source = selected_source(&row, item.as_ref(), &snap);
        let overlay_more = (*more_menu.read()).map(|at| {
            let locked = more_source.as_ref().is_some_and(|s| s.locked);
            let ready = more_source.is_some();
            let mut entries = vec![
                isle_ui::MenuEntry::Item(isle_ui::MenuItem::new(4, "Generate password…")),
                isle_ui::MenuEntry::Separator,
            ];
            entries.extend(
                source_entries(locked, snap.can_change_password)
                    .iter()
                    .cloned()
                    .map(|e| match e {
                        isle_ui::MenuEntry::Item(i) if !ready => {
                            isle_ui::MenuEntry::Item(i.enabled(false))
                        }
                        other => other,
                    }),
            );
            let act = act_on_source.clone();
            let path = more_source.as_ref().map(|s| s.path.clone());
            isle_ui::ContextMenu::new(Rc::new(entries), at, move |event: isle_ui::MenuEvent| {
                match event {
                    isle_ui::MenuEvent::Chosen(choice) => {
                        more_menu.set(None);
                        if let (Some(path), 1..=3) = (path.clone(), choice) {
                            act(["", "lock", "unlock", "change"][choice as usize], path);
                        }
                    }
                    isle_ui::MenuEvent::Back => more_menu.set(None),
                    _ => {}
                }
            })
            .into_element()
        });

        let touch = gate.clone();
        let touch_keys = gate.clone();
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .background(colour(t.colour_window))
            .on_sized(move |e: Event<SizedEventData>| width.set_if_modified(e.area.width()))
            .on_global_pointer_move(move |e: Event<PointerEventData>| {
                touch.borrow_mut().touch(Instant::now());
                if let Some((from, start)) = *dragging.peek() {
                    let x = e.global_location().x as f32;
                    remembered.write().list_width = (start + x - from).clamp(240., 420.);
                }
            })
            .on_global_pointer_up(move |_| {
                if dragging.peek().is_some() {
                    dragging.set(None);
                }
            })
            .on_global_key_down({
                let order: Vec<ItemId> = items.iter().map(|i| i.id.clone()).collect();
                move |e: Event<KeyboardEventData>| {
                    touch_keys.borrow_mut().touch(Instant::now());
                    // ↑ and ↓ move in the list from wherever focus is, as Spotlight's do.
                    let step: isize = match e.key {
                        Key::Named(NamedKey::ArrowUp) => -1,
                        Key::Named(NamedKey::ArrowDown) => 1,
                        _ => return,
                    };
                    if order.is_empty() || !e.modifiers.is_empty() {
                        return;
                    }
                    let at = selected
                        .peek()
                        .as_ref()
                        .and_then(|id| order.iter().position(|o| o == id));
                    let next = match at {
                        Some(i) => (i as isize + step).clamp(0, order.len() as isize - 1) as usize,
                        None => 0,
                    };
                    selected.set(Some(order[next].clone()));
                }
            })
            .child(toolbar)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .horizontal()
                    .content(Content::flex())
                    .maybe_child(sidebar_view)
                    .child(list_pane)
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .height(Size::fill())
                            .child(detail_view),
                    ),
            )
            .maybe_child(overlay_item)
            .maybe_child(overlay_source)
            .maybe_child(overlay_more)
    }
}
