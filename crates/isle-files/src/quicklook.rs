//! Quick Look's window: the items it was handed, one at a time, with ‹ › and an index sheet;
//! Space, Escape or ⌘Y close it (`p1/structure/files.md` § Quick Look).

use std::{path::PathBuf, rc::Rc};

use freya::prelude::*;
use isle_app::{ToolbarExt, WindowKey};
use isle_ui::{colour, text, use_theme};

use crate::{Shared, look::Look, preview::Preview, toolbar::icon_button, window::off_thread};

/// Who opened Quick Look, and so whose selection it follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Caller {
    Window(WindowKey),
    /// Another app over `QuickLook1`.
    Bus,
}

/// The open Quick Look: its window, its items and the one shown.
#[derive(Clone, Copy, PartialEq)]
pub struct Open {
    pub key: WindowKey,
    pub items: State<Vec<PathBuf>>,
    pub index: State<usize>,
    pub marking: State<bool>,
    pub trimming: State<bool>,
    pub caller: Caller,
}

#[derive(Clone)]
pub struct QuickLookWindow {
    pub shared: Rc<Shared>,
    pub items: State<Vec<PathBuf>>,
    pub index: State<usize>,
    /// Markup is on.
    pub marking: State<bool>,
    /// Trim is on.
    pub trimming: State<bool>,
}

impl App for QuickLookWindow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let shared = self.shared.clone();
        let (items, mut index, mut marking) = (self.items, self.index, self.marking);
        // Counts the files Markup wrote, so what shows is read again.
        let mut written = use_state(|| 0u64);
        let mut indexing = use_state(|| false);
        let count = items.read().len();
        let at = (*index.read()).min(count.saturating_sub(1));
        let path = items.read().get(at).cloned();
        let mut entry = use_state(|| None::<isle_fs::Entry>);
        let mut opener = use_state(|| None::<(String, String)>);
        let mut editable = use_state(|| false);
        // Video or audio, which Trim cuts: whether it has pictures.
        let mut media = use_state(|| None::<bool>);
        let mut trimming = self.trimming;
        use_side_effect_with_deps(&path, move |path| {
            let Some(path) = path.clone() else { return };
            spawn(async move {
                let read = path.clone();
                let found = off_thread(move || {
                    let entry = isle_fs::Entry::at(&read).ok();
                    if let Some(mime) = entry.as_ref().and_then(|e| e.mime.as_deref()) {
                        crate::icons::warm(mime, crate::icons::LARGE);
                    }
                    let (launches, _) = crate::open::decide(std::slice::from_ref(&read));
                    let index = isle_apps::Index::scan(&isle_apps::Index::data_dirs());
                    let app = launches.into_iter().next().map(|(id, _)| (index.name(&id), id));
                    let kind = isle_apps::mimeapps::kind_of(&read);
                    let editable = crate::markup::editable(&kind);
                    let media = (kind.starts_with("video/") || kind.starts_with("audio/")).then(|| kind.starts_with("video/"));
                    (entry, app, editable, media)
                })
                .await;
                if let Some((found, app, can_mark, kind)) = found {
                    entry.set(found);
                    opener.set(app);
                    editable.set(can_mark);
                    media.set(kind);
                }
            });
        });
        use_drop({
            let shared = shared.clone();
            move || shared.quick_look_closed()
        });
        use_side_effect({
            let shared = shared.clone();
            move || shared.quick_look_at(*index.read())
        });

        let step = move |by: isize| {
            let mut index = index;
            let n = items.peek().len();
            if n > 1 {
                let next = (*index.peek() as isize + by).rem_euclid(n as isize) as usize;
                index.set(next);
            }
        };
        let close = {
            let shared = shared.clone();
            move || shared.close_quick_look()
        };
        let on_key = {
            let close = close.clone();
            move |e: Event<KeyboardEventData>| match &e.key {
                Key::Named(NamedKey::Escape) => close(),
                Key::Character(c) if c == " " => close(),
                Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowUp) => step(-1),
                Key::Named(NamedKey::ArrowRight) | Key::Named(NamedKey::ArrowDown) => step(1),
                Key::Named(NamedKey::Enter) if e.modifiers.contains(Modifiers::META) => indexing.toggle(),
                _ => {}
            }
        };

        let name = path.as_deref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let open_with = opener.read().clone().map(|(app_name, app)| {
            let (shared, path) = (shared.clone(), path.clone());
            isle_ui::Button::new(format!("Open with {app_name}"))
                .small()
                .on_press(move |_| {
                    let (Some(bus), Some(path)) = (shared.app().bus(), path.clone()) else { return };
                    spawn(crate::open::launch(bus, app.clone(), vec![isle_thumbnail::uri(&path)]));
                })
                .into_element()
        });
        let toolbar = rect()
            .width(Size::fill())
            .height(Size::px(t.size_toolbar))
            .padding((0., t.space_l, 0., t.space_xxxl + t.space_xl + t.space_l))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_xs)
            .background(colour(t.colour_window))
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline))
                    .width(BorderWidth { top: 0., right: 0., bottom: 1., left: 0. })
                    .alignment(BorderAlignment::Inner),
            )
            .toolbar()
            .maybe_child((count > 1).then(|| icon_button(&theme, "caret-left", "Previous", true, false, move || step(-1))))
            .maybe_child((count > 1).then(|| icon_button(&theme, "caret-right", "Next", true, false, move || step(1))))
            .maybe_child((count > 1).then(|| {
                text::line(&theme, text::Style::Callout, colour(t.colour_text_2), format!("{} of {count}", at + 1)).into_element()
            }))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .main_align(Alignment::Center)
                    .horizontal()
                    .child(text::line(&theme, text::Style::Headline, colour(t.colour_text), name)),
            )
            .maybe_child((count > 1).then(|| {
                icon_button(&theme, "squares-four", "Index Sheet", true, *indexing.read(), move || {
                    let mut indexing = indexing;
                    indexing.toggle()
                })
            }))
            .child(icon_button(&theme, "pencil-simple", "Markup", *editable.read(), *marking.read(), move || {
                let mut marking = marking;
                marking.toggle();
            }))
            .maybe_child(media.read().is_some().then(|| {
                icon_button(&theme, "scissors", "Trim", true, *trimming.read(), move || {
                    let mut trimming = trimming;
                    trimming.toggle();
                })
            }))
            .child(rect().width(Size::px(t.space_l)))
            .maybe_child(open_with);

        let facts = rect()
            .expanded()
            .center()
            .child(Preview { items: entry.read().clone().into_iter().collect(), folder: None, width: 360., pane: false, action: None })
            .into_element();
        let body = match path {
            Some(_) if *indexing.read() => IndexSheet {
                items: items.read().clone(),
                on_pick: EventHandler::new(move |i: usize| {
                    index.set(i);
                    indexing.set(false);
                }),
            }
            .into_element(),
            Some(path) if *marking.read() && *editable.read() => crate::markup_view::MarkupView {
                path,
                on_end: EventHandler::new(move |wrote: bool| {
                    marking.set(false);
                    if wrote {
                        let n = *written.peek();
                        written.set(n + 1);
                    }
                }),
            }
            .into_element(),
            Some(path) if *trimming.read() && media.read().is_some() => crate::trim::TrimView {
                video: media.read().unwrap_or(false),
                path,
                on_end: EventHandler::new(move |wrote: bool| {
                    trimming.set(false);
                    if wrote {
                        let n = *written.peek();
                        written.set(n + 1);
                    }
                }),
            }
            .into_element(),
            Some(path) => rect().key(*written.read()).expanded().child(Look { path, facts }).into_element(),
            None => facts,
        };
        rect()
            .expanded()
            .content(Content::flex())
            .background(colour(t.colour_view))
            .on_global_key_down(on_key)
            .child(toolbar)
            .child(rect().width(Size::fill()).height(Size::flex(1.)).child(body))
    }
}

/// Every item as a thumbnail; a click shows it.
#[derive(PartialEq)]
struct IndexSheet {
    items: Vec<PathBuf>,
    on_pick: EventHandler<usize>,
}

impl Component for IndexSheet {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let dark = theme.appearance.mode == isle_tokens::Mode::Dark;
        let tiles = self.items.iter().enumerate().filter_map(|(i, path)| {
            let entry = isle_fs::Entry::at(path).ok()?;
            let pick = self.on_pick.clone();
            Some(
                rect()
                    .key(i)
                    .width(Size::px(180.))
                    .padding(t.space_s)
                    .spacing(t.space_xs)
                    .cross_align(Alignment::Center)
                    .corner_radius(t.radius_control)
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_alt(entry.name.clone())
                    .on_press(move |_| pick.call(i))
                    .child(crate::thumbnail::Thumbnail { entry: entry.clone(), size: 160., dark, remote: false })
                    .child(text::line(&theme, text::Style::Callout, colour(t.colour_text), entry.name.clone()))
                    .into_element(),
            )
        });
        isle_ui::scroll_area(&theme)
            .expanded()
            .child(rect().width(Size::fill()).padding(t.space_l).horizontal().content(Content::Wrap { wrap_spacing: Some(t.space_s) }).children(tiles))
    }
}
