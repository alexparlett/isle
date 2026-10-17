//! System Settings (`docs/design/apps.md` Settings; `p1/structure/settings.md`).

use freya::prelude::*;
use isle_app::{Spec, ToolbarExt};
use isle_ui::{Switch, colour, form, use_theme};

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
                                    .text("Appearance"),
                            ),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::fill())
                            .main_align(Alignment::Start)
                            .cross_align(Alignment::Center)
                            .child(sample()),
                    ),
            )
    }
}

fn sample() -> impl IntoElement {
    let theme = use_theme();
    let t = &theme.tokens;
    let mut mode = use_state(|| 1usize);
    let mut accent = use_state(|| 0usize);
    let mut margin = use_state(|| 8i64);
    let mut action = use_state(|| 0usize);
    let mut motion = use_state(|| false);
    let accents = vec![
        ("Blue".to_owned(), colour(t.colour_accent), colour(t.colour_on_accent)),
        ("Purple".to_owned(), colour(t.pane_purple), Color::WHITE),
        ("Pink".to_owned(), colour(t.pane_pink), Color::WHITE),
        ("Red".to_owned(), colour(t.pane_red), Color::WHITE),
        ("Orange".to_owned(), colour(t.pane_orange), Color::WHITE),
        ("Green".to_owned(), colour(t.pane_green), Color::WHITE),
    ];
    rect()
        .width(Size::px(t.size_form))
        .padding((t.space_xl, 0., t.space_xxxl, 0.))
        .spacing(t.space_xl)
        .child(form::PaneHeader {
            glyph: "circle-half".into(),
            hue: t.pane_graphite,
            name: "Appearance".into(),
            subtitle: "Choose how Isle looks.".into(),
        })
        .child(form::group(
            None,
            vec![
                form::Row::new("Appearance")
                    .control(form::Segmented::new(
                        vec!["Auto".into(), "Light".into(), "Dark".into()],
                        *mode.read(),
                        "Appearance",
                        move |i| mode.set(i),
                    ))
                    .into_element(),
                form::Row::new("Colour")
                    .control(form::Swatches::new(accents, Some(*accent.read()), move |i| accent.set(i)))
                    .into_element(),
                form::Row::new("Reduce motion")
                    .control(Switch::new(*motion.read(), "Reduce motion", move |on| motion.set(on)))
                    .into_element(),
            ],
        ))
        .child(form::group(
            Some("Windows".into()),
            vec![
                form::Row::new("Double-click a title bar to")
                    .control(form::PopUp::new(
                        vec!["Fill".into(), "Minimise".into(), "Do nothing".into()],
                        *action.read(),
                        "Double-click a title bar to",
                        move |i| action.set(i),
                    ))
                    .into_element(),
                form::Row::new("Tile margins")
                    .description("Space left between tiled windows and the screen's edges.")
                    .control(form::Stepper::new(*margin.read(), (0, 32), "Tile margins", move |v| margin.set(v)).unit("pt"))
                    .into_element(),
                form::Row::new("Hot Corners").opens(Some("2 set".into()), |_| {}).into_element(),
            ],
        ))
}

fn main() {
    isle_app::init_logging(module_path!());
    isle_app::run(SPEC, Settings);
}
