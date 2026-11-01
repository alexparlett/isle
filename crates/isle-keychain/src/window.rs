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
    edit::{self, Editing, Fields},
    model::{self, Detail, Item, ItemId, Keyring, Row, Snapshot},
    settings, sheets, sidebar, state,
    store::{self, Ask, LoginFields},
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

/// What a fetched secret is for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Then {
    Show,
    Copy,
    Edit,
}

/// Work the window does off its handlers, in order.
enum Job {
    /// Fetches the item's secret, behind the gate.
    Reveal { id: ItemId, then: Then },
    Save { id: ItemId, login: bool, fields: LoginFields, secret: Option<Zeroizing<String>> },
    Create { collection: String, fields: LoginFields, password: Zeroizing<String> },
    Delete(ItemId),
    AddKey { name: String, passphrase: Zeroizing<String> },
    GenerateKey { name: String, kind: isle_secrets::ssh::NewKind, comment: String, passphrase: Zeroizing<String> },
    TrashKey(String),
}

/// Where the generator's popover is open.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GeneratorAt {
    Edit,
    NewLogin,
}

/// What leaving Edit with changes was on its way to.
#[derive(Clone, PartialEq)]
enum Leave {
    Select(ItemId),
    Row(Row),
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
    can_undo: bool,
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
        edit_field(
            Command::new("undo", "Undo Move to Trash", Menu::Edit, 0)
                .chord(Chord::cmd('z'))
                .enabled(can_undo),
        ),
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
        // Narrow, the list and the detail stack: the detail over the list, with Back.
        let mut detail_open = use_state(|| false);
        let mut width = use_state(|| 1000f32);
        let mut item_menu = use_state(|| None::<(ItemId, (f32, f32))>);
        let mut source_menu = use_state(|| None::<(String, (f32, f32))>);
        let mut more_menu = use_state(|| None::<(f32, f32)>);
        let mut dragging = use_state(|| None::<(f32, f32)>);
        let gate = use_hook(|| Rc::new(RefCell::new(Gate::default())));
        let mut editing = use_state(|| None::<Editing>);
        let mut fields = Fields {
            title: use_state(String::new),
            user: use_state(String::new),
            website: use_state(String::new),
            notes: use_state(String::new),
            password: use_state(String::new),
        };
        let mut new_fields = Fields {
            title: use_state(String::new),
            user: use_state(String::new),
            website: use_state(String::new),
            notes: use_state(String::new),
            password: use_state(String::new),
        };
        let mut new_open = use_state(|| false);
        let mut save_in = use_state(|| 0usize);
        let mut generator_at = use_state(|| None::<GeneratorAt>);
        let mut generator_sheet = use_state(|| false);
        let mut generated = use_state(|| Rc::new(Zeroizing::new(String::new())));
        let mut delete_ask = use_state(|| None::<ItemId>);
        let mut leave_ask = use_state(|| None::<Leave>);
        let mut passphrase_for = use_state(|| None::<String>);
        let mut passphrase_text = use_state(isle_ui::FieldText::default);
        let mut passphrase_wrong = use_state(|| false);
        let mut new_key_open = use_state(|| false);
        let mut key_name = use_state(String::new);
        let mut key_kind = use_state(|| 0usize);
        let mut key_comment = use_state(String::new);
        let mut key_pass = use_state(isle_ui::FieldText::default);
        let mut key_confirm = use_state(isle_ui::FieldText::default);
        let mut generating = use_state(|| false);
        let mut key_error = use_state(|| None::<String>);
        let mut undo_trash = use_state(|| None::<Vec<isle_fs::trash::Trashed>>);
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
                        let (id, then) = match job {
                            Job::Reveal { id, then } => (id, then),
                            Job::Save { id, login, fields: f, secret } => {
                                let (tx, rx) = oneshot::channel();
                                store.ask(Ask::Update { id, fields: f, login, secret, reply: tx });
                                match rx.await {
                                    Ok(Ok(())) => editing.set(None),
                                    Ok(Err(failure)) => tracing::warn!(?failure, "the item was not saved"),
                                    Err(_) => {}
                                }
                                continue;
                            }
                            Job::Create { collection, fields: f, password } => {
                                let (tx, rx) = oneshot::channel();
                                store.ask(Ask::CreateLogin { collection, fields: f, password, reply: tx });
                                match rx.await {
                                    Ok(Ok(id)) => {
                                        new_open.set(false);
                                        selected.set(Some(id));
                                    }
                                    Ok(Err(failure)) => tracing::warn!(?failure, "the login was not saved"),
                                    Err(_) => {}
                                }
                                continue;
                            }
                            Job::AddKey { name, passphrase } => {
                                let (tx, rx) = oneshot::channel();
                                store.ask(Ask::AddToAgent { key: name, passphrase, reply: tx });
                                match rx.await {
                                    Ok(Ok(())) => {
                                        passphrase_for.set(None);
                                        passphrase_text.set(isle_ui::FieldText::default());
                                    }
                                    Ok(Err(store::Failure::Passphrase)) => {
                                        passphrase_wrong.set(true);
                                        passphrase_text.set(isle_ui::FieldText::default());
                                    }
                                    Ok(Err(failure)) => {
                                        passphrase_for.set(None);
                                        tracing::warn!(?failure, "the key was not added");
                                    }
                                    Err(_) => {}
                                }
                                continue;
                            }
                            Job::GenerateKey { name, kind, comment, passphrase } => {
                                // Generating is work in flight: the app stays until it ends.
                                let _hold = app.hold();
                                generating.set(true);
                                let (tx, rx) = oneshot::channel();
                                store.ask(Ask::GenerateKey { name, kind, comment, passphrase, reply: tx });
                                let result = rx.await;
                                generating.set(false);
                                match result {
                                    Ok(Ok(name)) => {
                                        new_key_open.set(false);
                                        key_pass.set(isle_ui::FieldText::default());
                                        key_confirm.set(isle_ui::FieldText::default());
                                        selected.set(Some(ItemId::Ssh(name)));
                                    }
                                    Ok(Err(store::Failure::Other(why))) => key_error.set(Some(why)),
                                    Ok(Err(failure)) => key_error.set(Some(format!("{failure:?}"))),
                                    Err(_) => {}
                                }
                                continue;
                            }
                            Job::TrashKey(name) => {
                                let (tx, rx) = oneshot::channel();
                                store.ask(Ask::TrashKey(name, tx));
                                match rx.await {
                                    Ok(Ok(items)) => undo_trash.set(Some(items)),
                                    Ok(Err(failure)) => tracing::warn!(?failure, "the key was not moved to the Trash"),
                                    Err(_) => {}
                                }
                                continue;
                            }
                            Job::Delete(id) => {
                                let (tx, rx) = oneshot::channel();
                                store.ask(Ask::Delete(id, tx));
                                match rx.await {
                                    Ok(Ok(())) => editing.set(None),
                                    Ok(Err(failure)) => tracing::warn!(?failure, "the item was not deleted"),
                                    Err(_) => {}
                                }
                                continue;
                            }
                        };
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
                            Ok(Ok(Some(value))) if then == Then::Copy => {
                                copy::secret(&app, &copies, value, settings::clear_after(&desktop));
                            }
                            Ok(Ok(value)) if then == Then::Edit => {
                                let snap = snapshot.peek().clone();
                                let Some(item) = snap.items.iter().find(|i| i.id == id) else { continue };
                                let (user, website, notes) = match &item.detail {
                                    Detail::Secret { user, website, notes, .. } => (user.clone(), website.clone(), notes.clone()),
                                    _ => continue,
                                };
                                let original = LoginFields {
                                    title: item.title.clone(),
                                    user: user.unwrap_or_default(),
                                    website: website.unwrap_or_default(),
                                    notes: notes.unwrap_or_default(),
                                };
                                let password = Rc::new(value.unwrap_or_default());
                                fields.fill(&original, &password);
                                generator_at.set(None);
                                editing.set(Some(Editing {
                                    id: id.clone(),
                                    login: item.kind == Kind::Login,
                                    original,
                                    password,
                                }));
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
            editing.read().is_some(),
            undo_trash.read().is_some(),
        ));

        let reveal = {
            let jobs = jobs.clone();
            move |id: ItemId, then: Then| {
                let _ = jobs.unbounded_send(Job::Reveal { id, then });
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

        let mut regenerate = move || {
            let recipe = remembered.peek().recipe;
            generated.set(Rc::new(isle_secrets::generator::generate(&recipe)));
        };
        let mut open_new_key = move || {
            let dir = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default().join(".ssh");
            let name = ["id_ed25519", "id_ed25519_2", "id_ed25519_3", "id_isle"]
                .into_iter()
                .find(|n| !dir.join(n).exists())
                .unwrap_or("id_isle");
            let user = std::env::var("USER").unwrap_or_default();
            let host = std::fs::read_to_string("/etc/hostname").unwrap_or_default();
            key_name.set(name.to_owned());
            key_kind.set(0);
            key_comment.set(format!("{user}@{}", host.trim()));
            key_pass.set(isle_ui::FieldText::default());
            key_confirm.set(isle_ui::FieldText::default());
            key_error.set(None);
            new_key_open.set(true);
        };
        let mut open_new_login = move || {
            new_fields.fill(&LoginFields::default(), "");
            save_in.set(0);
            generator_at.set(None);
            new_open.set(true);
        };
        // Leaving Edit with changes asks first; without, it just ends.
        let mut leave = move |to: Leave| {
            let changed = editing.peek().as_ref().is_some_and(|e| fields.changed(e));
            if changed {
                leave_ask.set(Some(to));
                return;
            }
            editing.set(None);
            match to {
                Leave::Select(id) => selected.set(Some(id)),
                Leave::Row(row) => {
                    remembered.write().row = row;
                    selected.set(None);
                }
            }
        };
        let save_edit = {
            let jobs = jobs.clone();
            move || {
                let Some(e) = editing.peek().clone() else { return };
                let password = fields.password();
                let secret = (password.as_str() != e.password.as_str()).then_some(password);
                let _ = jobs.unbounded_send(Job::Save { id: e.id, login: e.login, fields: fields.read(), secret });
            }
        };
        use_side_effect({
            let jobs = jobs.clone();
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
                                reveal(id, Then::Copy);
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
                                (false, Some(id)) => reveal(id, Then::Show),
                                _ => {}
                            }
                        }
                        "sidebar" => {
                            let hidden = *sidebar_hidden.peek();
                            sidebar_hidden.set(!hidden);
                        }
                        "edit" => {
                            if item.as_ref().is_some_and(Item::editable) && let Some(id) = id.clone() {
                                reveal(id, Then::Edit);
                            }
                        }
                        "delete" => match &item {
                            Some(Item { id: ItemId::Ssh(name), .. }) => {
                                let _ = jobs.unbounded_send(Job::TrashKey(name.clone()));
                            }
                            Some(i) if i.editable() => delete_ask.set(id.clone()),
                            _ => {}
                        },
                        "new-key" => open_new_key(),
                        "undo" => {
                            let kept = undo_trash.peek().clone();
                            if let Some(items) = kept {
                                undo_trash.set(None);
                                store.ask(Ask::Untrash(items));
                            }
                        }
                        "new-login" => open_new_login(),
                        "generate" => {
                            regenerate();
                            generator_sheet.set(true);
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
                list::Act::Select(id) => {
                    detail_open.set(true);
                    leave(Leave::Select(id));
                }
                list::Act::Menu(id, at) => {
                    selected.set(Some(id.clone()));
                    item_menu.set(Some((id, at)));
                }
                list::Act::Unlock(path) => store.ask(Ask::Unlock(path)),
                list::Act::NewKey => open_new_key(),
            })
        };
        let on_detail = {
            let jobs_detail = jobs.clone();
            let (app, store, reveal) = (app.clone(), store.clone(), reveal.clone());
            let item = item.clone();
            EventHandler::new(move |act: detail::Act| {
                let Some(item) = item.clone() else { return };
                match act {
                    detail::Act::Show => reveal(item.id.clone(), Then::Show),
                    detail::Act::Hide => shown.set(None),
                    detail::Act::CopySecret => reveal(item.id.clone(), Then::Copy),
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
                    detail::Act::Edit => reveal(item.id.clone(), Then::Edit),
                    detail::Act::AddToAgent => {
                        if let (ItemId::Ssh(name), Detail::Ssh { key, .. }) = (&item.id, &item.detail) {
                            if key.encrypted == Some(true) {
                                passphrase_wrong.set(false);
                                passphrase_text.set(isle_ui::FieldText::default());
                                passphrase_for.set(Some(name.clone()));
                            } else {
                                let _ = jobs_detail.unbounded_send(Job::AddKey { name: name.clone(), passphrase: Zeroizing::new(String::new()) });
                            }
                        }
                    }
                }
            })
        };

        // The toolbar.
        let narrow = *width.read();
        let sidebar_shown = (narrow >= 760.) != *sidebar_hidden.read();
        let stacked = narrow < 680.;
        let over = stacked && *detail_open.read();
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
            .width(if stacked { Size::flex(1.) } else { Size::px(list_width) })
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
            .child(Button::new(new_label).enabled(creates.is_some()).on_press(move |_| match creates {
                Some(Kind::Login) => open_new_login(),
                Some(Kind::SshKey) => desktop.commands.clone().write().push("new-key".into()),
                _ => {}
            }))
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
                    .child(if over {
                        Button::icon("caret-left", "Back").on_press(move |_| detail_open.set(false))
                    } else {
                        Button::icon("sidebar-simple", "Sidebar").on_press(move |_| {
                            let hidden = *sidebar_hidden.peek();
                            sidebar_hidden.set(!hidden);
                        })
                    }),
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
                    on_select: EventHandler::new(move |next: Row| leave(Leave::Row(next))),
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
            .width(if stacked { Size::flex(1.) } else { Size::px(list_width) })
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
        let editing_now = editing.read().clone().filter(|e| Some(&e.id) == current.as_ref());
        let detail_view = match (&editing_now, item.as_ref()) {
            (Some(e), Some(it)) => {
                let save = save_edit.clone();
                edit::view(
                    &theme,
                    edit::View {
                        item: it,
                        fields,
                        login: e.login,
                        generator: (*generator_at.read() == Some(GeneratorAt::Edit))
                            .then(|| (r.recipe, generated.read().as_str().to_owned())),
                        on_act: EventHandler::new(move |act: edit::Act| match act {
                            edit::Act::Cancel => {
                                editing.set(None);
                                fields.password.set(String::new());
                            }
                            edit::Act::Done => save(),
                            edit::Act::Delete => {
                                let id = editing.peek().as_ref().map(|e| e.id.clone());
                                delete_ask.set(id);
                            }
                            edit::Act::Generator(open) => {
                                if open {
                                    regenerate();
                                }
                                generator_at.set(open.then_some(GeneratorAt::Edit));
                            }
                            edit::Act::Recipe(next) => {
                                remembered.write().recipe = next;
                                regenerate();
                            }
                            edit::Act::Again => regenerate(),
                            edit::Act::Use => {
                                let value = generated.peek().as_str().to_owned();
                                fields.password.set(value);
                                generator_at.set(None);
                            }
                        }),
                    },
                )
            }
            _ => detail::view(
                &theme,
                detail::View {
                    item: item.as_ref(),
                    shown: shown_now.as_ref().map(|s| s.value.as_str()),
                    can_edit,
                    on_act: on_detail,
                },
            ),
        };

        // Sheets and alerts.
        let unlocked: Vec<model::Source> = {
            let mut list: Vec<model::Source> = snap.sources.iter().filter(|s| !s.locked).cloned().collect();
            list.sort_by_key(|s| !s.default);
            list
        };
        let overlay_new = new_open.read().then(|| {
            let jobs = jobs.clone();
            let targets = unlocked.clone();
            sheets::new_login(
                &theme,
                sheets::NewLogin {
                    fields: new_fields,
                    sources: &unlocked,
                    save_in: *save_in.read(),
                    generator: (*generator_at.read() == Some(GeneratorAt::NewLogin))
                        .then(|| (r.recipe, generated.read().as_str().to_owned())),
                    on_save_in: EventHandler::new(move |i| save_in.set(i)),
                    on_generator: EventHandler::new(move |act: edit::Act| match act {
                        edit::Act::Generator(open) => {
                            if open {
                                regenerate();
                            }
                            generator_at.set(open.then_some(GeneratorAt::NewLogin));
                        }
                        edit::Act::Recipe(next) => {
                            remembered.write().recipe = next;
                            regenerate();
                        }
                        edit::Act::Again => regenerate(),
                        edit::Act::Use => {
                            let value = generated.peek().as_str().to_owned();
                            new_fields.password.set(value);
                            generator_at.set(None);
                        }
                        _ => {}
                    }),
                    on_cancel: EventHandler::new(move |_| {
                        new_open.set(false);
                        new_fields.fill(&LoginFields::default(), "");
                    }),
                    on_save: EventHandler::new(move |_| {
                        let Some(target) = targets.get(*save_in.peek()) else { return };
                        let _ = jobs.unbounded_send(Job::Create {
                            collection: target.path.clone(),
                            fields: new_fields.read(),
                            password: new_fields.password(),
                        });
                    }),
                },
            )
        });
        let overlay_delete = delete_ask.read().clone().and_then(|id| {
            let it = snap.items.iter().find(|i| i.id == id)?.clone();
            let jobs = jobs.clone();
            Some(isle_ui::alert(
                &format!("Delete “{}”?", it.title),
                &sheets::delete_words(&it),
                Vec::new(),
                vec![
                    isle_ui::AlertButton::from(("Cancel".to_owned(), EventHandler::new(move |_| delete_ask.set(None)))),
                    isle_ui::AlertButton::destructive(
                        "Delete",
                        EventHandler::new(move |_| {
                            delete_ask.set(None);
                            let _ = jobs.unbounded_send(Job::Delete(id.clone()));
                        }),
                    ),
                ],
            ))
        });
        let overlay_leave = leave_ask.read().clone().map(|to| {
            let (to_discard, to_save) = (to.clone(), to);
            let save = save_edit.clone();
            let mut go = move |to: Leave| match to {
                Leave::Select(id) => selected.set(Some(id)),
                Leave::Row(row) => {
                    remembered.write().row = row;
                    selected.set(None);
                }
            };
            isle_ui::alert(
                "Save changes to this item?",
                "Your changes will be lost if you don't save them.",
                Vec::new(),
                vec![
                    isle_ui::AlertButton::destructive(
                        "Discard",
                        EventHandler::new(move |_| {
                            leave_ask.set(None);
                            editing.set(None);
                            go(to_discard.clone());
                        }),
                    ),
                    isle_ui::AlertButton::from(("Keep Editing".to_owned(), EventHandler::new(move |_| leave_ask.set(None)))),
                    isle_ui::AlertButton::from((
                        "Save".to_owned(),
                        EventHandler::new(move |_| {
                            leave_ask.set(None);
                            save();
                            go(to_save.clone());
                        }),
                    )),
                ],
            )
        });
        let overlay_passphrase = passphrase_for.read().clone().map(|name| {
            let (add_jobs, key) = (jobs.clone(), name.clone());
            crate::ssh_sheets::passphrase(
                &theme,
                &name,
                passphrase_text.read().clone(),
                *passphrase_wrong.read(),
                EventHandler::new(move |_| {
                    passphrase_for.set(None);
                    passphrase_text.set(isle_ui::FieldText::default());
                }),
                EventHandler::new(move |_| {
                    let passphrase = Zeroizing::new(passphrase_text.peek().borrow().as_str().to_owned());
                    let _ = add_jobs.unbounded_send(Job::AddKey { name: key.clone(), passphrase });
                }),
            )
        });
        let overlay_new_key = new_key_open.read().then(|| {
            let save_jobs = jobs.clone();
            crate::ssh_sheets::new_key(
                &theme,
                crate::ssh_sheets::NewKey {
                    name: key_name,
                    kind: *key_kind.read(),
                    comment: key_comment,
                    passphrase: key_pass.read().clone(),
                    confirm: key_confirm.read().clone(),
                    generating: *generating.read(),
                    error: key_error.read().clone(),
                    on_kind: EventHandler::new(move |i| key_kind.set(i)),
                    on_cancel: EventHandler::new(move |_| new_key_open.set(false)),
                    on_save: EventHandler::new(move |_| {
                        let pass = Zeroizing::new(key_pass.peek().borrow().as_str().to_owned());
                        let confirm = Zeroizing::new(key_confirm.peek().borrow().as_str().to_owned());
                        if pass != confirm {
                            key_error.set(Some("The passphrases don't match.".into()));
                            return;
                        }
                        key_error.set(None);
                        let kind = if *key_kind.peek() == 1 {
                            isle_secrets::ssh::NewKind::Rsa3072
                        } else {
                            isle_secrets::ssh::NewKind::Ed25519
                        };
                        let _ = save_jobs.unbounded_send(Job::GenerateKey {
                            name: key_name.peek().trim().to_owned(),
                            kind,
                            comment: key_comment.peek().clone(),
                            passphrase: pass,
                        });
                    }),
                },
            )
        });
        let overlay_generator = generator_sheet.read().then(|| {
            let (app, copies) = (app.clone(), shared.copies.clone());
            let password = generated.read().as_str().to_owned();
            let mut body = vec![text::line(&theme, text::Style::Headline, colour(t.colour_text), "Generate Password").into_element()];
            body.extend(crate::generator_view::controls(
                &theme,
                r.recipe,
                &password,
                true,
                EventHandler::new(move |next| {
                    remembered.write().recipe = next;
                    regenerate();
                }),
            ));
            body.push(
                rect()
                    .child(Button::new("Generate again").on_press(move |_| regenerate()))
                    .into_element(),
            );
            isle_ui::sheet(
                400.,
                body,
                vec![
                    (
                        "Copy".into(),
                        EventHandler::new(move |_| {
                            let value = Zeroizing::new(generated.peek().as_str().to_owned());
                            copy::secret(&app, &copies, value, settings::clear_after(&desktop));
                        }),
                    ),
                    ("Done".into(), EventHandler::new(move |_| generator_sheet.set(false))),
                ],
            )
        });

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
                                1 => reveal(it.id.clone(), Then::Copy),
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
                    .maybe_child((!over).then_some(list_pane))
                    .maybe_child((!stacked || over).then(|| {
                        rect()
                            .width(Size::flex(1.))
                            .height(Size::fill())
                            .child(detail_view)
                            .into_element()
                    })),
            )
            .maybe_child(overlay_item)
            .maybe_child(overlay_source)
            .maybe_child(overlay_more)
            .maybe_child(overlay_new)
            .maybe_child(overlay_generator)
            .maybe_child(overlay_delete)
            .maybe_child(overlay_leave)
            .maybe_child(overlay_passphrase)
            .maybe_child(overlay_new_key)
    }
}
