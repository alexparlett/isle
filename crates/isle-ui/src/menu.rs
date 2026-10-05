//! Menus: rows, marks, shortcut hints and submenus, as `p1/surfaces/menus.md` draws them,
//! for the menu bar, a window's menu and context menus.

use std::{rc::Rc, time::Duration};

use freya::prelude::*;

use crate::{Glyph, colour, shadows, text, use_theme};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mark {
    #[default]
    None,
    /// Checked, or the chosen one of a radio group.
    Check,
    /// Some of what it stands for is on.
    Mixed,
    /// A minimised window in the Window menu's list.
    Diamond,
}

/// A shortcut as the menu shows it: Mac symbols in Apple's order and the key, or for a PC
/// keyboard the whole chord in words with `modifiers` empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hint {
    pub modifiers: String,
    pub key: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MenuItem {
    /// The owner's id, handed back when the item is chosen or its submenu opens.
    pub id: u64,
    pub label: String,
    pub enabled: bool,
    pub mark: Mark,
    pub hint: Option<Hint>,
    pub submenu: Option<Rc<Vec<MenuEntry>>>,
    /// A name for an item drawn as something other than its label.
    pub alt: Option<String>,
}

impl MenuItem {
    pub fn new(id: u64, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            enabled: true,
            ..Default::default()
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn mark(mut self, mark: Mark) -> Self {
        self.mark = mark;
        self
    }

    pub fn hint(mut self, hint: Option<Hint>) -> Self {
        self.hint = hint;
        self
    }

    pub fn submenu(mut self, entries: Vec<MenuEntry>) -> Self {
        self.submenu = Some(Rc::new(entries));
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MenuEntry {
    Item(MenuItem),
    Separator,
    /// A heading row, not selectable.
    Section(String),
}

impl MenuEntry {
    pub fn item(&self) -> Option<&MenuItem> {
        match self {
            MenuEntry::Item(item) => Some(item),
            _ => None,
        }
    }
}

/// `entries` with leading, trailing and repeated separators dropped.
pub fn tidy(entries: &[MenuEntry]) -> Vec<MenuEntry> {
    let mut out: Vec<MenuEntry> = Vec::with_capacity(entries.len());
    for entry in entries {
        if matches!(entry, MenuEntry::Separator)
            && matches!(out.last(), None | Some(MenuEntry::Separator))
        {
            continue;
        }
        out.push(entry.clone());
    }
    while matches!(out.last(), Some(MenuEntry::Separator)) {
        out.pop();
    }
    out
}

/// The next enabled item from `from` in direction `dir`, wrapping; from nothing, the first
/// or the last.
pub fn step(entries: &[MenuEntry], from: Option<usize>, dir: i32) -> Option<usize> {
    let n = entries.len() as i32;
    if n == 0 {
        return None;
    }
    let start = match from {
        Some(i) => i as i32,
        None if dir > 0 => -1,
        None => n,
    };
    (1..=n)
        .map(|k| (start + dir * k).rem_euclid(n) as usize)
        .find(|&i| entries[i].item().is_some_and(|item| item.enabled))
}

/// The next enabled item after `from` whose label starts with `typed`, ignoring case.
pub fn type_select(entries: &[MenuEntry], from: Option<usize>, typed: &str) -> Option<usize> {
    let typed = typed.to_lowercase();
    let n = entries.len();
    let start = from.map_or(0, |i| i + 1);
    (0..n).map(|k| (start + k) % n).find(|&i| {
        entries[i]
            .item()
            .is_some_and(|item| item.enabled && item.label.to_lowercase().starts_with(&typed))
    })
}

/// What a menu tells its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuEvent {
    /// An item was chosen; the menu is done.
    Chosen(u64),
    /// A submenu is about to show, so its owner can fill it.
    Opening(u64),
    /// Escape at the top level, or Left there: the owner moves on or closes.
    Back,
    /// Right on an item with no submenu: the owner may move to the next menu.
    Forward,
}

/// Where a menu is drawn: glass from the shell, opaque in a window, solid over glass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    Glass,
    Window,
    Solid,
}

/// A menu and its open submenus, laid out from its top-left corner. The surface that hosts
/// it gives it room; submenus open to the right.
#[derive(Clone, PartialEq)]
pub struct Menu {
    entries: Rc<Vec<MenuEntry>>,
    material: Material,
    on_event: EventHandler<MenuEvent>,
    /// The row to select when it opens, as from the keyboard.
    first_selected: bool,
}

impl Menu {
    pub fn new(entries: Rc<Vec<MenuEntry>>, on_event: impl Into<EventHandler<MenuEvent>>) -> Self {
        Self {
            entries,
            material: Material::Glass,
            on_event: on_event.into(),
            first_selected: false,
        }
    }

    pub fn material(mut self, material: Material) -> Self {
        self.material = material;
        self
    }

    pub fn first_selected(mut self, first: bool) -> Self {
        self.first_selected = first;
        self
    }
}

/// One open level: which row is selected and whether its submenu shows.
#[derive(Clone, Debug, Default, PartialEq)]
struct Level {
    selected: Option<usize>,
    open: bool,
}

/// A menu's width from its longest label and hint, within the design's least and most. Text
/// is estimated from the type size, as the layout cannot size a row with a spring in it to
/// its content.
fn panel_width(theme: &isle_tokens::Theme, entries: &[MenuEntry], pad: f32) -> f32 {
    let t = &theme.tokens;
    let size = text::metrics(theme, text::Style::Body).0;
    let estimate = |s: &str| s.chars().count() as f32 * size * 0.58;
    let (mut label, mut hint, mut chevron) = (0.0f32, 0.0f32, false);
    for item in entries.iter().filter_map(MenuEntry::item) {
        label = label.max(estimate(&item.label));
        if let Some(h) = &item.hint {
            hint = hint.max(estimate(&h.modifiers) * 1.3 + estimate(&h.key));
        }
        chevron |= item.submenu.is_some();
    }
    let trailing = if hint > 0.0 { t.space_xxl + hint } else { 0.0 }
        + if chevron {
            t.space_s + t.size_glyph_small
        } else {
            0.0
        };
    let width = 2.0 * pad + t.space_xs + 16.0 + t.space_s + label + trailing + t.space_s;
    width.clamp(t.size_menu_min, t.size_menu_max).ceil()
}

/// The width of the widest shortcut key in `entries`.
fn key_slot(theme: &isle_tokens::Theme, entries: &[MenuEntry]) -> f32 {
    let size = text::metrics(theme, text::Style::Body).0;
    entries
        .iter()
        .filter_map(MenuEntry::item)
        .filter_map(|i| i.hint.as_ref())
        .map(|h| h.key.chars().count() as f32 * size * 0.62)
        .fold(0.0, f32::max)
        .ceil()
}

/// Opens `item`'s submenu below level `depth`, telling the owner first.
fn open_sub(
    on_event: &EventHandler<MenuEvent>,
    levels: &mut Vec<Level>,
    depth: usize,
    item: &MenuItem,
) {
    if item.submenu.is_some() && item.enabled {
        on_event.call(MenuEvent::Opening(item.id));
        levels.truncate(depth + 1);
        levels[depth].open = true;
        levels.push(Level::default());
    }
}

/// The entries of each open level, the root first.
fn levels_of(root: &Rc<Vec<MenuEntry>>, levels: &[Level]) -> Vec<Rc<Vec<MenuEntry>>> {
    let mut out = vec![root.clone()];
    for level in levels {
        let Some(sub) = level
            .open
            .then_some(())
            .and(level.selected)
            .and_then(|i| out.last().unwrap().get(i).and_then(|e| e.item()))
            .and_then(|item| item.submenu.clone())
        else {
            break;
        };
        out.push(Rc::new(tidy(&sub)));
    }
    out
}

impl Component for Menu {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        // Entries can change while the menu is open, a submenu the app filled for one.
        let root = Rc::new(tidy(&self.entries));
        let first = self.first_selected;
        let mut levels = use_state({
            let root = root.clone();
            move || {
                vec![Level {
                    selected: if first { step(&root, None, 1) } else { None },
                    open: false,
                }]
            }
        });
        let mut typed = use_state(String::new);
        let mut typed_at = use_state(std::time::Instant::now);
        let on_event = self.on_event.clone();
        let menus = levels_of(&root, &levels.read());
        let depth = menus.len() - 1;

        let on_key = {
            let menus = menus.clone();
            let on_event = on_event.clone();
            move |e: Event<KeyboardEventData>| {
                let entries = menus[depth].clone();
                let mut ls = levels.write();
                let selected = ls[depth].selected;
                match &e.key {
                    Key::Named(NamedKey::ArrowDown) => {
                        ls[depth].selected = step(&entries, selected, 1)
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        ls[depth].selected = step(&entries, selected, -1)
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        match selected.and_then(|i| entries[i].item()) {
                            Some(item) if item.submenu.is_some() => {
                                let item = item.clone();
                                open_sub(&on_event, &mut ls, depth, &item);
                                let sub = levels_of(&menus[0], &ls);
                                if let Some(sub) = sub.get(depth + 1) {
                                    ls[depth + 1].selected = step(sub, None, 1);
                                }
                            }
                            _ => {
                                drop(ls);
                                on_event.call(MenuEvent::Forward);
                            }
                        }
                    }
                    Key::Named(NamedKey::ArrowLeft) if depth > 0 => {
                        ls.truncate(depth);
                        ls[depth - 1].open = false;
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        drop(ls);
                        on_event.call(MenuEvent::Back);
                    }
                    Key::Named(NamedKey::Escape) if depth > 0 => {
                        ls.truncate(depth);
                        ls[depth - 1].open = false;
                    }
                    Key::Named(NamedKey::Escape) => {
                        drop(ls);
                        on_event.call(MenuEvent::Back);
                    }
                    Key::Named(NamedKey::Enter) | Key::Character(_)
                        if matches!(&e.key, Key::Named(NamedKey::Enter))
                            || (matches!(&e.key, Key::Character(c) if c == " ")
                                && typed.peek().is_empty()) =>
                    {
                        if let Some(item) = selected.and_then(|i| entries[i].item()) {
                            let item = item.clone();
                            if item.submenu.is_some() {
                                open_sub(&on_event, &mut ls, depth, &item);
                            } else if item.enabled {
                                drop(ls);
                                on_event.call(MenuEvent::Chosen(item.id));
                            }
                        }
                    }
                    Key::Character(c) => {
                        let now = std::time::Instant::now();
                        let fresh =
                            now.duration_since(*typed_at.peek()) > Duration::from_millis(1000);
                        typed_at.set(now);
                        let mut buffer = if fresh {
                            String::new()
                        } else {
                            typed.peek().clone()
                        };
                        buffer.push_str(c);
                        // A longer prefix may still match the item already selected.
                        let n = entries.len();
                        let from = if buffer.chars().count() > 1 {
                            selected.map(|i| (i + n - 1) % n)
                        } else {
                            selected
                        };
                        if let Some(i) = type_select(&entries, from, &buffer) {
                            ls[depth].selected = Some(i);
                        }
                        typed.set(buffer);
                    }
                    _ => {}
                }
            }
        };

        let row_height = text::metrics(&theme, text::Style::Body).1 + 6.0;
        let row_height = row_height.max(t.size_menu_row);
        let pad = (t.radius_card - t.radius_row).max(0.0);

        let (fill, solid) = match self.material {
            Material::Glass => (Color::TRANSPARENT, false),
            Material::Window => (colour(t.colour_window_raised), true),
            Material::Solid => (colour(t.material_solid), true),
        };

        let panels = menus.iter().enumerate().map(|(level, entries)| {
            let selected = levels.read().get(level).and_then(|l| l.selected);
            let open = levels.read().get(level).is_some_and(|l| l.open);
            let deepest = level == depth;
            let key_slot = key_slot(&theme, entries);
            let rows = entries.iter().enumerate().map(|(i, entry)| {
                match entry {
                    MenuEntry::Separator => rect()
                        .width(Size::fill())
                        .height(Size::px(9.))
                        .padding((4., t.space_s))
                        .child(
                            rect()
                                .width(Size::fill())
                                .height(Size::px(t.stroke_hairline.max(1.0)))
                                .background(colour(t.colour_hairline)),
                        )
                        .into_element(),
                    MenuEntry::Section(heading) => rect()
                        .width(Size::fill())
                        .height(Size::px(row_height))
                        .padding((0., t.space_xs + 16.0 + t.space_s))
                        .main_align(Alignment::Center)
                        .child(
                            text::line(
                                &theme,
                                text::Style::Subheadline,
                                colour(t.colour_text_2),
                                heading.clone(),
                            )
                            .font_weight(t.type_weight_semibold as i32),
                        )
                        .into_element(),
                    MenuEntry::Item(item) => {
                        let is_selected = selected == Some(i) && item.enabled;
                        let accent = is_selected && deepest;
                        let parent = is_selected && open && !deepest;
                        let (ink, ink2) = if !item.enabled {
                            (
                                colour(t.colour_text_disabled),
                                colour(t.colour_text_disabled),
                            )
                        } else if accent {
                            (colour(t.colour_on_accent), colour(t.colour_on_accent))
                        } else {
                            (colour(t.colour_text), colour(t.colour_text_2))
                        };
                        let background = if accent {
                            colour(t.colour_accent)
                        } else if parent {
                            colour(t.colour_fill_strong)
                        } else {
                            Color::TRANSPARENT
                        };
                        let mark = match item.mark {
                            Mark::None => None,
                            Mark::Check => Some(("check", false)),
                            Mark::Mixed => Some(("minus", false)),
                            Mark::Diamond => Some(("diamond", true)),
                        };
                        let item_c = item.clone();
                        let item_p = item.clone();
                        let (on_move, on_pick) = (on_event.clone(), on_event.clone());
                        rect()
                            .width(Size::fill())
                            .height(Size::px(row_height))
                            .corner_radius(t.radius_row)
                            .background(background)
                            .horizontal()
                            .content(Content::flex())
                            .cross_align(Alignment::Center)
                            .padding((0., t.space_s, 0., t.space_xs))
                            .a11y_role(AccessibilityRole::MenuItem)
                            .a11y_alt(item.alt.clone().unwrap_or_else(|| item.label.clone()))
                            .maybe(!item.enabled, |el| el.a11y_builder(|n| n.set_disabled()))
                            .on_pointer_move(move |_| {
                                if !item_c.enabled {
                                    return;
                                }
                                let mut ls = levels.write();
                                if ls.get(level).and_then(|l| l.selected) == Some(i) {
                                    return;
                                }
                                ls.truncate(level + 1);
                                ls[level].selected = Some(i);
                                ls[level].open = false;
                                if item_c.submenu.is_some() {
                                    let item = item_c.clone();
                                    let on_event = on_move.clone();
                                    drop(ls);
                                    // A short dwell, so a pointer crossing rows on its way to an
                                    // open submenu does not swap it.
                                    spawn(async move {
                                        freya::prelude::timer(Duration::from_millis(120)).await;
                                        let mut ls = levels.write();
                                        if ls.get(level).and_then(|l| l.selected) == Some(i)
                                            && !ls[level].open
                                        {
                                            open_sub(&on_event, &mut ls, level, &item);
                                        }
                                    });
                                }
                            })
                            .on_press(move |_| {
                                if !item_p.enabled {
                                    return;
                                }
                                if item_p.submenu.is_some() {
                                    open_sub(&on_pick, &mut levels.write(), level, &item_p);
                                } else {
                                    on_pick.call(MenuEvent::Chosen(item_p.id));
                                }
                            })
                            .child(rect().width(Size::px(16.)).center().map(
                                mark,
                                |el, (name, fill)| {
                                    el.child(Glyph::new(name, t.size_glyph_small, ink).fill(fill))
                                },
                            ))
                            .child(rect().width(Size::px(t.space_s)))
                            .child(text::line(
                                &theme,
                                text::Style::Body,
                                ink,
                                item.label.clone(),
                            ))
                            .child(rect().width(Size::flex(1.)))
                            .map(item.hint.clone(), |el, hint| {
                                // Modifiers right-aligned against a key slot as wide as the
                                // menu's widest key, so the keys line up.
                                el.child(rect().width(Size::px(t.space_xxl)))
                                    .child(text::line(
                                        &theme,
                                        text::Style::Body,
                                        ink2,
                                        hint.modifiers,
                                    ))
                                    .child(rect().width(Size::px(key_slot)).child(text::line(
                                        &theme,
                                        text::Style::Body,
                                        ink2,
                                        hint.key,
                                    )))
                            })
                            .maybe(item.submenu.is_some(), |el| {
                                el.child(rect().width(Size::px(t.space_s)))
                                    .child(Glyph::new("caret-right", t.size_glyph_small, ink2))
                            })
                            .into_element()
                    }
                }
            });
            // An auto height would let the glass behind, filling it, take the room below.
            let height: f32 = entries
                .iter()
                .map(|e| match e {
                    MenuEntry::Separator => 9.0,
                    _ => row_height,
                })
                .sum::<f32>()
                + 2.0 * pad;
            rect()
                .width(Size::px(panel_width(&theme, entries, pad)))
                .height(Size::px(height))
                .padding(pad)
                .corner_radius(t.radius_card)
                .background(fill)
                .map(Some(shadows(&t.shadow_menu)), |el, list| {
                    list.into_iter().fold(el, |el, s| el.shadow(s))
                })
                .maybe(!solid, |el| el.child(crate::Glass::new(t.radius_card)))
                .a11y_role(AccessibilityRole::Menu)
                .children(rows)
                .into_element()
        });

        rect()
            .horizontal()
            .a11y_focusable(true)
            .a11y_auto_focus(true)
            .on_global_key_down(on_key)
            .children(panels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(labels: &[(&str, bool)]) -> Vec<MenuEntry> {
        labels
            .iter()
            .enumerate()
            .map(|(i, (label, enabled))| match *label {
                "-" => MenuEntry::Separator,
                _ => MenuEntry::Item(MenuItem::new(i as u64, *label).enabled(*enabled)),
            })
            .collect()
    }

    #[test]
    fn arrows_skip_separators_and_disabled_items_and_wrap() {
        let menu = items(&[
            ("Undo", true),
            ("-", true),
            ("Redo", false),
            ("Cut", true),
            ("Copy", true),
        ]);
        assert_eq!(step(&menu, None, 1), Some(0));
        assert_eq!(
            step(&menu, Some(0), 1),
            Some(3),
            "past the separator and Redo"
        );
        assert_eq!(step(&menu, Some(4), 1), Some(0), "wraps to the top");
        assert_eq!(
            step(&menu, None, -1),
            Some(4),
            "up from nothing is the last"
        );
        assert_eq!(step(&menu, Some(3), -1), Some(0));
        assert_eq!(step(&items(&[("-", true), ("Off", false)]), None, 1), None);
    }

    #[test]
    fn separators_collapse() {
        let menu = items(&[
            ("-", true),
            ("A", true),
            ("-", true),
            ("-", true),
            ("B", true),
            ("-", true),
        ]);
        let tidied = tidy(&menu);
        assert_eq!(tidied.len(), 3);
        assert!(matches!(tidied[1], MenuEntry::Separator));
    }

    #[test]
    fn typing_selects_the_next_item_that_starts_with_it() {
        let menu = items(&[
            ("Copy", true),
            ("Cut", true),
            ("Close", false),
            ("Paste", true),
        ]);
        assert_eq!(type_select(&menu, None, "c"), Some(0));
        assert_eq!(type_select(&menu, Some(0), "c"), Some(1), "the next one");
        assert_eq!(
            type_select(&menu, Some(1), "c"),
            Some(0),
            "wraps, skipping disabled Close"
        );
        assert_eq!(type_select(&menu, None, "PA"), Some(3));
        assert_eq!(type_select(&menu, None, "x"), None);
    }
}
