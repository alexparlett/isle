//! Banners on the loop: cards from notifications, the stack's place at the top right of the
//! main screen, its one timer, hover, actions, replies and sounds (`p8-services.md` choices
//! 7, 8, 11).

use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use isle_settings::schema::BannerStyle;
use isle_shell::banners::{
    BannerAction, BannersSurface,
    model::{Banners, Card, Timing},
    surface::Frame,
};
use smithay::{
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
    utils::Rectangle,
};
use tracing::debug;

use crate::{
    notifications::{Entry, Fate},
    state::{Isle, State},
};

/// Arrivals this close to the last sound make none (`notifications.md`, Sounds).
const SOUND_GAP: Duration = Duration::from_millis(500);

pub struct BannersState {
    pub model: Banners,
    pub surface: Rc<BannersSurface>,
    pub replying: Option<u32>,
    timer: Option<RegistrationToken>,
    last_sound: Option<Instant>,
}

impl BannersState {
    pub fn new() -> Self {
        Self {
            model: Banners::default(),
            surface: BannersSurface::new(),
            replying: None,
            timer: None,
            last_sound: None,
        }
    }
}

impl Isle {
    /// A notification's card.
    fn banner_card(&self, entry: &Entry) -> Card {
        let n = &entry.n;
        let app = self.notification_app(n);
        let app_name = {
            let name = self.app_name(&app);
            if name.is_empty() {
                n.app_name.clone()
            } else {
                name
            }
        };
        let icon = self.app_icon(&app, 32).or_else(|| {
            let name = n.app_icon.clone()?;
            let dark = self.shell.theme().appearance.mode == isle_tokens::Mode::Dark;
            isle_apps::icons::Icons::papirus(dark).lookup(&name, 32)
        });
        let style = self
            .settings
            .notification_app
            .get(&app)
            .map_or(BannerStyle::Temporary, |a| a.banner_style);
        Card {
            id: n.id,
            app,
            app_name,
            icon,
            glyph: None,
            summary: n.summary.clone(),
            body: strip_markup(&n.body),
            image: n.image.clone(),
            actions: n
                .actions
                .iter()
                .filter(|(k, _)| k != "default" && k != "inline-reply")
                .cloned()
                .collect(),
            reply: n.inline_reply(),
            progress: n.value.map(|v| v as f32 / 100.0),
            count: 1,
            persistent: n.urgency == 2 || style == BannerStyle::Persistent,
            time: "Now".into(),
        }
    }

    /// Whether a panel, menu or overlay on the main screen makes arrivals wait.
    fn banners_blocked(&self) -> bool {
        let main = self.main_output().map(|o| o.name());
        self.island
            .panel
            .as_ref()
            .is_some_and(|p| Some(&p.output) == main.as_ref())
            || self.bar.open.is_some()
    }

    /// Lays the stack out on the main screen and arms its one timer.
    pub fn refresh_banners(&mut self) {
        let now = Instant::now();
        let blocked = self.banners_blocked();
        self.banners.model.timing = Timing::of(self.shell.theme());
        self.banners.model.set_blocked(blocked, now);
        self.banners.model.tick(now);
        let cards: Vec<Card> = self.banners.model.cards().into_iter().cloned().collect();
        // A chat app both notifies and asks for attention for one message: the banner wins.
        let shown: Vec<String> = cards.iter().map(|c| c.app.clone()).collect();
        for app in self.banners_apps.drain(..).collect::<Vec<_>>() {
            if !shown.contains(&app) {
                self.island.model.set_banner(&app, false);
            }
        }
        for app in &shown {
            self.island.model.set_banner(app, true);
        }
        self.banners_apps = shown;
        let theme = self.shell.theme().clone();
        let game = self.game_mode_on();
        let frame = Frame {
            cards,
            overflow: self.banners.model.overflow(),
            replying: self.banners.replying,
            solid: game || theme.appearance.reduce_transparency,
            still: game || theme.appearance.reduce_motion,
        };
        if frame.cards.is_empty() && frame.overflow == 0 {
            self.banners.replying = None;
            self.shell.close("banners");
        } else if let Some(output) = self.main_output() {
            let size = BannersSurface::size_for(&frame, &theme);
            let screen = self.space.output_geometry(&output).unwrap_or_default();
            let t = &theme.tokens;
            let margin = BannersSurface::margin(&theme) as i32;
            let geometry = Rectangle::new(
                (
                    screen.size.w - size.w - t.space_l as i32 + margin,
                    t.size_strip as i32 - margin,
                )
                    .into(),
                size,
            );
            let surface = self.banners.surface.clone();
            surface.set_size(size);
            surface.set_model(frame.clone());
            self.shell
                .with_root("banners", || surface.set(frame.clone()));
            let place = isle_shell::OutputPlace {
                output: output.clone(),
                geometry: screen,
                scale: output.current_scale().fractional_scale(),
            };
            self.shell.open_placed("banners", &place, geometry);
            self.queue_redraw(&output);
        }
        self.arm_banners_timer();
    }

    fn arm_banners_timer(&mut self) {
        if let Some(token) = self.banners.timer.take() {
            self.loop_handle.remove(token);
        }
        let Some(at) = self.banners.model.next_change() else {
            return;
        };
        let wait = at.saturating_duration_since(Instant::now());
        self.banners.timer = self
            .loop_handle
            .insert_source(Timer::from_duration(wait), |_, _, state| {
                state.isle.banners.timer = None;
                state.isle.refresh_banners();
                TimeoutAction::Drop
            })
            .ok();
    }

    /// A notification's sound: the app's own file or name, else the alert, unless something
    /// says not to; one for a burst.
    fn notification_sound(&mut self, entry: &Entry) {
        let n = &entry.n;
        let app = self.notification_app(n);
        let allowed = self.settings.notifications.sounds
            && !n.suppress_sound
            && self
                .settings
                .notification_app
                .get(&app)
                .is_none_or(|a| a.sound);
        let now = Instant::now();
        if !allowed || self.banners.last_sound.is_some_and(|t| now - t < SOUND_GAP) {
            return;
        }
        self.banners.last_sound = Some(now);
        let file = n
            .sound_file
            .clone()
            .map(std::path::PathBuf::from)
            .filter(|p| p.exists())
            .or_else(|| {
                n.sound_name
                    .as_deref()
                    .and_then(|name| isle_audio::sounds::lookup(name, "freedesktop"))
            })
            .or_else(|| {
                isle_audio::sounds::lookup(&self.settings.notifications.alert_sound, "freedesktop")
            });
        if let Some(file) = file {
            self.run_service(
                async move { isle_audio::sounds::play(&file).await },
                |_, result| {
                    if let Err(err) = result {
                        debug!("an alert sound: {err}");
                    }
                },
            );
        }
    }
}

/// The sanitised body's text: bold, italic and underline are drawn as text, links as their
/// words.
pub fn strip_markup(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut in_tag = false;
    for c in body.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

impl State {
    /// An arrival decided: its banner and sound, when it is one.
    pub fn banner_for(&mut self, id: u32, fate: Fate) {
        if !matches!(fate, Fate::Banner | Fate::Transient) {
            return;
        }
        let Some(entry) = self
            .isle
            .notifications
            .entries
            .iter()
            .find(|e| e.n.id == id)
            .cloned()
        else {
            return;
        };
        let card = self.isle.banner_card(&entry);
        let mut life =
            (!card.persistent).then(|| Timing::of(self.isle.shell.theme()).life(entry.n.expire));
        // Over a game holding the pointer, a critical banner cannot be closed: it leaves.
        if card.persistent && self.isle.game_mode_on() {
            life = Some(Timing::of(self.isle.shell.theme()).max);
        }
        self.isle.banners.model.arrive(card, life, Instant::now());
        self.isle.notification_sound(&entry);
        self.isle.refresh_banners();
    }

    pub fn banner_gone(&mut self, id: u32) {
        if self.isle.banners.model.remove(id) {
            if self.isle.banners.replying == Some(id) {
                self.end_reply();
            }
            self.isle.refresh_banners();
        }
    }

    pub fn banner_action(&mut self, action: BannerAction) {
        let now = Instant::now();
        match action {
            BannerAction::Hover(on) => {
                self.isle.banners.model.set_hovered(on, now);
            }
            BannerAction::Open(id) => {
                self.notification_invoke(id, "default");
            }
            BannerAction::Action(id, key) => {
                self.notification_invoke(id, &key);
            }
            BannerAction::Close(id) => {
                self.notification_close(id);
            }
            BannerAction::StartReply(id) => {
                self.isle.banners.replying = Some(id);
                self.isle.refresh_banners();
                if let Some(output) = self.isle.main_output()
                    && let Some(focus) = self.isle.shell.instance_focus("banners", &output)
                {
                    let keyboard = self.isle.seat.get_keyboard().unwrap();
                    let previous = keyboard.current_focus();
                    self.isle
                        .shell_keyboard_return
                        .push((focus.clone(), previous));
                    let serial = smithay::utils::SERIAL_COUNTER.next_serial();
                    self.focus_keyboard(Some(crate::focus::Focus::Shell(focus)), serial);
                }
            }
            BannerAction::Send(id, text) => {
                self.end_reply();
                self.notification_reply(id, &text);
            }
            BannerAction::CancelReply => self.end_reply(),
            BannerAction::Overflow => self.open_notification_centre(None),
        }
        self.isle.refresh_banners();
    }

    /// The reply field closes and the keyboard goes back to what had it.
    fn end_reply(&mut self) {
        self.isle.banners.replying = None;
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        let Some(crate::focus::Focus::Shell(focus)) = keyboard.current_focus() else {
            return;
        };
        if self.isle.shell.name_of(&focus) != Some("banners") {
            return;
        }
        let previous = self
            .isle
            .shell_keyboard_return
            .iter()
            .position(|(f, _)| *f == focus)
            .map(|i| self.isle.shell_keyboard_return.remove(i).1);
        match previous {
            Some(Some(previous)) if smithay::utils::IsAlive::alive(&previous) => {
                let serial = smithay::utils::SERIAL_COUNTER.next_serial();
                self.focus_keyboard(Some(previous), serial);
            }
            _ => self.focus_next(),
        }
    }

    /// Notification Centre, which P8 9 builds; until then the stack's overflow opens nothing.
    pub fn open_notification_centre(&mut self, _focused: Option<u32>) {}

    #[cfg(feature = "test-hooks")]
    pub fn banner_test_action(&mut self, json: &str) -> Result<(), String> {
        let value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let (key, args) = value
            .as_object()
            .and_then(|o| o.iter().next())
            .ok_or("one key")?;
        let id = |v: &serde_json::Value| v.as_u64().unwrap_or(0) as u32;
        let action = match key.as_str() {
            "hover" => BannerAction::Hover(args.as_bool().unwrap_or(false)),
            "open" => BannerAction::Open(id(args)),
            "close" => BannerAction::Close(id(args)),
            "reply" => BannerAction::StartReply(id(args)),
            "action" => BannerAction::Action(id(&args[0]), args[1].as_str().unwrap_or("").into()),
            "send" => BannerAction::Send(id(&args[0]), args[1].as_str().unwrap_or("").into()),
            "cancel" => BannerAction::CancelReply,
            other => return Err(format!("no action {other}")),
        };
        self.banner_action(action);
        Ok(())
    }

    pub fn banners_status(&self) -> String {
        let cards: Vec<serde_json::Value> = self
            .isle
            .banners
            .model
            .cards()
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id, "app": c.app, "summary": c.summary, "count": c.count,
                    "persistent": c.persistent,
                })
            })
            .collect();
        serde_json::json!({
            "cards": cards,
            "overflow": self.isle.banners.model.overflow(),
            "hovered": self.isle.banners.model.hovered(),
            "replying": self.isle.banners.replying,
            "open": self.isle.shell.is_open("banners"),
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::strip_markup;

    #[test]
    fn markup_is_read_as_its_words() {
        assert_eq!(
            strip_markup("<b>Sam</b> sent <a href=\"x\">a link</a> &amp; more"),
            "Sam sent a link & more"
        );
    }
}
