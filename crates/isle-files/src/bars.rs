//! The path bar and the status bar at the content's foot, each turned on in View
//! (`p1/surfaces/files.md` § Path bar and status bar).

use std::path::{Path, PathBuf};

use freya::prelude::*;
use isle_ui::{Glyph, colour, text, use_theme};

use crate::{format, places, window::off_thread};

fn bar(theme: &isle_tokens::Theme) -> Rect {
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .height(Size::px(t.size_control))
        .padding((0., t.space_s))
        .horizontal()
        .cross_align(Alignment::Center)
        .background(colour(t.colour_window))
        .border(
            Border::new()
                .fill(colour(t.colour_hairline))
                .width(BorderWidth { top: 1., right: 0., bottom: 0., left: 0. })
                .alignment(BorderAlignment::Inner),
        )
}

#[derive(PartialEq)]
pub struct PathBar {
    pub folder: Option<PathBuf>,
    pub on_go: EventHandler<PathBuf>,
}

impl Component for PathBar {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let Some(folder) = &self.folder else {
            return bar(&theme).into_element();
        };
        let chain: Vec<&Path> = folder.ancestors().collect::<Vec<_>>().into_iter().rev().collect();
        let last = chain.len() - 1;
        bar(&theme)
            .spacing(t.space_xxs)
            .a11y_role(AccessibilityRole::Navigation)
            .a11y_alt("Path")
            .children(chain.into_iter().enumerate().flat_map(|(i, path)| {
                let on_go = self.on_go.clone();
                let target = path.to_owned();
                let segment = rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(t.space_xxs)
                    .a11y_role(AccessibilityRole::Link)
                    .on_press(move |_| on_go.call(target.clone()))
                    .child(Glyph::new("folder-simple", t.size_glyph, colour(t.colour_text_2)))
                    .child(text::line(
                        &theme,
                        text::Style::Callout,
                        colour(if i == last { t.colour_text } else { t.colour_text_2 }),
                        places::folder_name(path),
                    ))
                    .into_element();
                let caret = (i < last).then(|| Glyph::new("caret-right", t.size_glyph_small, colour(t.colour_text_3)).into_element());
                std::iter::once(segment).chain(caret)
            }))
            .into_element()
    }
}

#[derive(PartialEq)]
pub struct StatusBar {
    pub folder: Option<PathBuf>,
    pub count: usize,
    pub selected: usize,
    pub selected_size: u64,
}

impl Component for StatusBar {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut free = use_state(|| None::<(PathBuf, u64)>);
        use_side_effect_with_deps(&self.folder, move |folder| {
            let Some(folder) = folder.clone() else { return };
            spawn(async move {
                let path = folder.clone();
                let found = off_thread(move || rustix::fs::statvfs(&path).ok().map(|s| s.f_bavail * s.f_frsize)).await;
                if let Some(Some(bytes)) = found {
                    free.set(Some((folder, bytes)));
                }
            });
        });
        let free = free
            .read()
            .as_ref()
            .filter(|(p, _)| Some(p) == self.folder.as_ref())
            .map(|(_, b)| format::size(*b));
        let words = if self.selected > 0 && self.selected_size > 0 {
            format!("{} of {} selected, {}", self.selected, self.count, format::size(self.selected_size))
        } else if self.selected > 0 {
            format!("{} of {} selected", self.selected, self.count)
        } else {
            match free {
                Some(free) => format!("{}, {free} available", format::items(self.count)),
                None => format::items(self.count),
            }
        };
        bar(&theme)
            .main_align(Alignment::Center)
            .a11y_role(AccessibilityRole::Status)
            .child(text::line(&theme, text::Style::Callout, colour(t.colour_text_2), words))
    }
}
