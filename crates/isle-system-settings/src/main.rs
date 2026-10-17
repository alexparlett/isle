//! System Settings (`docs/design/apps.md` Settings; `p1/structure/settings.md`).

use freya::prelude::*;
use isle_app::{Spec, ToolbarExt};
use isle_ui::{colour, use_theme};

const SPEC: Spec = Spec {
    id: "io.github.alexparlett.Isle.Settings",
    title: "System Settings",
    size: (900.0, 760.0),
    min_size: (784.0, 564.0),
    // The lights' 32 point group centred in the 52 point toolbar.
    traffic_light_inset: (0.0, 10.0),
    preferences: false,
};

struct Settings;

impl App for Settings {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        rect()
            .expanded()
            .horizontal()
            .background(colour(t.colour_window))
            .child(
                rect()
                    .width(Size::px(t.size_sidebar))
                    .height(Size::fill())
                    .background(colour(t.colour_sidebar))
                    .child(rect().width(Size::fill()).height(Size::px(t.size_toolbar)).toolbar()),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::px(t.size_toolbar))
                            .padding((0., t.space_l, 0., t.space_l))
                            .main_align(Alignment::Center)
                            .toolbar()
                            .child(
                                label()
                                    .font_size(t.type_style_headline_size)
                                    .font_weight(t.type_style_headline_weight as i32)
                                    .color(colour(t.colour_text))
                                    .text("General"),
                            ),
                    ),
            )
    }
}

fn main() {
    isle_app::init_logging(module_path!());
    isle_app::run(SPEC, Settings);
}
