//! Physical keys in, the keys a client is sent out. Cmd and Option are withheld until the next
//! key or button decides them; a translated chord is sent with exactly its modifiers, which
//! are put back as the held keys imply when it ends.

use xkbcommon::xkb::Keysym;

use crate::{
    chord::{Chord, Mods},
    keyinfo::{KeyInfo, Role},
};

/// The modifiers a stroke is sent with, by what the client sees.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Sent {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub super_: bool,
    pub level3: bool,
}

impl Sent {
    fn has(self, role: Role) -> bool {
        match role {
            Role::Ctrl => self.ctrl,
            Role::Shift => self.shift,
            Role::Option => self.alt,
            Role::Cmd => self.super_,
            Role::Level3 => self.level3,
            Role::Key => false,
        }
    }

    fn set(&mut self, role: Role) {
        match role {
            Role::Ctrl => self.ctrl = true,
            Role::Shift => self.shift = true,
            Role::Option => self.alt = true,
            Role::Cmd => self.super_ = true,
            Role::Level3 => self.level3 = true,
            Role::Key => {}
        }
    }
}

const MODIFIER_ROLES: [Role; 5] = [
    Role::Ctrl,
    Role::Shift,
    Role::Option,
    Role::Cmd,
    Role::Level3,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrokeKey {
    Sym(Keysym),
    /// The physical key itself, for the general rules.
    Code(u32),
}

/// One key sent with its modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stroke {
    pub mods: Sent,
    pub key: StrokeKey,
}

impl Stroke {
    /// A chord read as what to send: Cmd is Super, Option is Alt.
    pub fn from_chord(chord: Chord) -> Self {
        Stroke {
            mods: Sent {
                ctrl: chord.mods.ctrl,
                shift: chord.mods.shift,
                alt: chord.mods.option,
                super_: chord.mods.cmd,
                level3: false,
            },
            key: StrokeKey::Sym(chord.sym),
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Chord::parse(text).map(Self::from_chord)
    }
}

/// Rules from a Mac chord to what is sent instead.
#[derive(Clone, Debug, Default)]
pub struct Rules {
    rules: Vec<(Chord, Vec<Stroke>)>,
}

impl Rules {
    /// Rules written as text; a pair that does not parse is skipped with a warning.
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        let mut rules = Rules::default();
        for (from, to) in pairs {
            let chord = Chord::parse(from);
            let strokes: Option<Vec<Stroke>> = to.split(',').map(Stroke::parse).collect();
            match (chord, strokes) {
                (Some(chord), Some(strokes)) => rules.rules.push((chord, strokes)),
                _ => tracing::warn!(from, to, "a translation rule does not parse"),
            }
        }
        rules
    }

    pub fn push(&mut self, chord: Chord, strokes: Vec<Stroke>) {
        self.rules.retain(|(c, _)| *c != chord);
        self.rules.push((chord, strokes));
    }

    pub fn lookup(&self, chord: &Chord) -> Option<&[Stroke]> {
        self.rules
            .iter()
            .find(|(c, _)| c == chord)
            .map(|(_, s)| s.as_slice())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Chord, &[Stroke])> {
        self.rules.iter().map(|(c, s)| (c, s.as_slice()))
    }
}

pub struct Ctx<'a> {
    pub keys: &'a KeyInfo,
    /// Off in games, under an inhibitor and for untranslated apps: every key goes raw.
    pub translate: bool,
    /// Option types a key's third-level symbol where it has one: a Mac keyboard.
    pub option_level3: bool,
    /// The rules in force, most specific first.
    pub rules: &'a [&'a Rules],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Out<A> {
    Press(u32),
    Release(u32),
    Action(A),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Held {
    /// A modifier key, by role.
    Mod(Role),
    /// Sent as itself, as this code.
    Passed(u32),
    /// Sent as this code with these modifiers until it is released.
    Translated(u32, Sent),
    /// Ran an action, or was sent once; its release sends nothing.
    Spent,
}

#[derive(Default)]
pub struct Engine {
    /// Physical keys held, in the order pressed.
    held: Vec<(u32, Held)>,
    /// Codes the client holds down, in the order sent.
    sent: Vec<u32>,
    /// A pointer button is down, so withheld Cmd and Option are sent as Ctrl and Alt.
    button: bool,
}

impl Engine {
    /// The modifiers physically held, as chords name them.
    pub fn mods(&self) -> Mods {
        let mut mods = Mods::default();
        for (_, held) in &self.held {
            match held {
                Held::Mod(Role::Cmd) => mods.cmd = true,
                Held::Mod(Role::Option) => mods.option = true,
                Held::Mod(Role::Ctrl) => mods.ctrl = true,
                Held::Mod(Role::Shift) => mods.shift = true,
                _ => {}
            }
        }
        mods
    }

    /// Whether a physical key is held.
    pub fn is_held(&self, code: u32) -> bool {
        self.held.iter().any(|(c, _)| *c == code)
    }

    /// The modifiers the client should hold with no translated key down.
    fn base(&self, ctx: &Ctx) -> Sent {
        let mut sent = Sent::default();
        for (_, held) in &self.held {
            if let Held::Mod(role) = held {
                match role {
                    Role::Cmd if !ctx.translate => sent.super_ = true,
                    Role::Cmd if self.button => sent.ctrl = true,
                    Role::Option if !ctx.translate || self.button => sent.alt = true,
                    Role::Cmd | Role::Option => {}
                    role => sent.set(*role),
                }
            }
        }
        sent
    }

    fn desired(&self, ctx: &Ctx) -> Sent {
        self.held
            .iter()
            .rev()
            .find_map(|(_, h)| match h {
                Held::Translated(_, mods) => Some(*mods),
                _ => None,
            })
            .unwrap_or_else(|| self.base(ctx))
    }

    fn send_press<A>(&mut self, code: u32, out: &mut Vec<Out<A>>) {
        if !self.sent.contains(&code) {
            self.sent.push(code);
            out.push(Out::Press(code));
        }
    }

    fn send_release<A>(&mut self, code: u32, out: &mut Vec<Out<A>>) {
        if let Some(i) = self.sent.iter().position(|c| *c == code) {
            self.sent.remove(i);
            out.push(Out::Release(code));
        }
    }

    /// Presses and releases modifiers until the client holds exactly `want`.
    fn sync<A>(&mut self, want: Sent, ctx: &Ctx, out: &mut Vec<Out<A>>) {
        let extra: Vec<u32> = self
            .sent
            .iter()
            .copied()
            .filter(|c| {
                let role = ctx.keys.role(*c);
                role != Role::Key && !want.has(role)
            })
            .collect();
        for code in extra.into_iter().rev() {
            self.send_release(code, out);
        }
        for role in MODIFIER_ROLES {
            if want.has(role)
                && !self.sent.iter().any(|c| ctx.keys.role(*c) == role)
                && let Some(code) = ctx.keys.code_for_role(role)
            {
                self.send_press(code, out);
            }
        }
    }

    fn resolve(&self, key: StrokeKey, ctx: &Ctx) -> Option<u32> {
        match key {
            StrokeKey::Code(code) => Some(code),
            StrokeKey::Sym(sym) => ctx.keys.code_for(sym),
        }
    }

    /// The general rules: Cmd as Ctrl, and Option as the key's third level or as Alt.
    fn general(&self, code: u32, mods: Mods, ctx: &Ctx) -> Option<Stroke> {
        let stroke = |sent: Sent| Stroke {
            mods: sent,
            key: StrokeKey::Code(code),
        };
        if mods.cmd {
            return Some(stroke(Sent {
                ctrl: true,
                shift: mods.shift,
                alt: mods.option,
                ..Sent::default()
            }));
        }
        if mods.option {
            let level3 = ctx.option_level3
                && !mods.ctrl
                && ctx.keys.has_level3(code)
                && ctx.keys.code_for_role(Role::Level3).is_some();
            return Some(stroke(Sent {
                ctrl: mods.ctrl,
                shift: mods.shift,
                alt: !level3,
                level3,
                super_: false,
            }));
        }
        None
    }

    /// A physical key pressed. `bind` is asked first with the physical chord and may claim
    /// it for a desktop action.
    pub fn press<A>(
        &mut self,
        code: u32,
        ctx: &Ctx,
        bind: impl FnOnce(Chord) -> Option<A>,
    ) -> Vec<Out<A>> {
        let mut out = Vec::new();
        if self.is_held(code) {
            return out;
        }
        let role = ctx.keys.role(code);
        if role != Role::Key {
            self.held.push((code, Held::Mod(role)));
            let withheld = ctx.translate && matches!(role, Role::Cmd | Role::Option);
            if !withheld {
                self.send_press(code, &mut out);
            } else if self.button {
                let want = self.desired(ctx);
                self.sync(want, ctx, &mut out);
            }
            return out;
        }
        let mods = self.mods();
        if let Some(sym) = ctx.keys.sym(code)
            && let Some(action) = bind(Chord::new(mods, sym))
        {
            self.held.push((code, Held::Spent));
            out.push(Out::Action(action));
            return out;
        }
        if !ctx.translate {
            self.held.push((code, Held::Passed(code)));
            self.send_press(code, &mut out);
            return out;
        }
        let chord = ctx.keys.sym(code).map(|sym| Chord::new(mods, sym));
        let ruled: Option<Vec<Stroke>> = chord.and_then(|chord| {
            ctx.rules
                .iter()
                .find_map(|rules| rules.lookup(&chord))
                .map(<[Stroke]>::to_vec)
        });
        let strokes = ruled.or_else(|| self.general(code, mods, ctx).map(|s| vec![s]));
        match strokes.as_deref() {
            None => {
                self.held.push((code, Held::Passed(code)));
                self.send_press(code, &mut out);
            }
            Some([stroke]) => match self.resolve(stroke.key, ctx) {
                Some(sent) => {
                    self.held.push((code, Held::Translated(sent, stroke.mods)));
                    self.sync(stroke.mods, ctx, &mut out);
                    self.send_press(sent, &mut out);
                }
                None => self.held.push((code, Held::Spent)),
            },
            Some(strokes) => {
                for stroke in strokes {
                    let Some(sent) = self.resolve(stroke.key, ctx) else {
                        continue;
                    };
                    self.sync(stroke.mods, ctx, &mut out);
                    self.send_press(sent, &mut out);
                    self.send_release(sent, &mut out);
                }
                self.held.push((code, Held::Spent));
                let want = self.desired(ctx);
                self.sync(want, ctx, &mut out);
            }
        }
        out
    }

    pub fn release<A>(&mut self, code: u32, ctx: &Ctx) -> Vec<Out<A>> {
        let mut out = Vec::new();
        let Some(i) = self.held.iter().position(|(c, _)| *c == code) else {
            self.send_release(code, &mut out);
            return out;
        };
        let (_, held) = self.held.remove(i);
        match held {
            Held::Mod(_) => {
                self.send_release(code, &mut out);
                let want = self.desired(ctx);
                self.sync(want, ctx, &mut out);
            }
            Held::Passed(sent) => self.send_release(sent, &mut out),
            Held::Translated(sent, _) => {
                self.send_release(sent, &mut out);
                let want = self.desired(ctx);
                self.sync(want, ctx, &mut out);
            }
            Held::Spent => {}
        }
        out
    }

    /// A pointer button pressed or the last one released: withheld Cmd and Option go with a
    /// click as Ctrl and Alt.
    pub fn buttons<A>(&mut self, down: bool, ctx: &Ctx) -> Vec<Out<A>> {
        let mut out = Vec::new();
        if self.button != down {
            self.button = down;
            let want = self.desired(ctx);
            self.sync(want, ctx, &mut out);
        }
        out
    }

    /// Before focus moves or the rules change: releases everything the client holds. Keys
    /// held become spent; modifiers stay held and are sent again by `resync`.
    pub fn unwind<A>(&mut self) -> Vec<Out<A>> {
        let out = self.sent.drain(..).rev().map(Out::Release).collect();
        for (_, held) in &mut self.held {
            if !matches!(held, Held::Mod(_)) {
                *held = Held::Spent;
            }
        }
        out
    }

    /// After focus has moved: the modifiers the new context implies for the keys held.
    pub fn resync<A>(&mut self, ctx: &Ctx) -> Vec<Out<A>> {
        let mut out = Vec::new();
        let want = self.desired(ctx);
        self.sync(want, ctx, &mut out);
        out
    }

    /// Sends strokes once each, then the modifiers the held keys imply: an app's own close
    /// or quit chord, or Isle's own keys.
    pub fn send<A>(&mut self, strokes: &[Stroke], ctx: &Ctx) -> Vec<Out<A>> {
        let mut out = Vec::new();
        for stroke in strokes {
            let Some(sent) = self.resolve(stroke.key, ctx) else {
                continue;
            };
            self.sync(stroke.mods, ctx, &mut out);
            self.send_press(sent, &mut out);
            self.send_release(sent, &mut out);
        }
        let want = self.desired(ctx);
        self.sync(want, ctx, &mut out);
        out
    }

    /// Codes the client holds down now.
    pub fn sent(&self) -> &[u32] {
        &self.sent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyinfo::info_for;

    const CMD: u32 = 125;
    const OPTION: u32 = 56;
    const CTRL: u32 = 29;
    const SHIFT: u32 = 42;
    const LEVEL3: u32 = 100;
    const C: u32 = 46;
    const K3: u32 = 4;
    const UP: u32 = 103;
    const LEFT: u32 = 105;
    const HOME: u32 = 102;
    const BACKSPACE: u32 = 14;
    const TAB: u32 = 15;

    type O = Out<&'static str>;

    struct Fixture {
        keys: KeyInfo,
        text: Rules,
        engine: Engine,
        translate: bool,
    }

    fn bind(chord: Chord) -> Option<&'static str> {
        (chord.mods.cmd && chord.sym == Keysym::from(xkbcommon::xkb::keysyms::KEY_Tab))
            .then_some("switcher")
    }

    impl Fixture {
        fn new() -> Self {
            Fixture {
                keys: info_for("gb", "mac"),
                text: Rules::from_pairs(&[
                    ("Cmd+Left", "Home"),
                    ("Cmd+Backspace", "Shift+Home, Backspace"),
                ]),
                engine: Engine::default(),
                translate: true,
            }
        }

        /// Runs `f` with the engine and a context over the fixture's keymap and rules.
        fn with<R>(&mut self, f: impl FnOnce(&mut Engine, &Ctx) -> R) -> R {
            let rules = [&self.text];
            let ctx = Ctx {
                keys: &self.keys,
                translate: self.translate,
                option_level3: true,
                rules: &rules,
            };
            f(&mut self.engine, &ctx)
        }

        fn down(&mut self, code: u32) -> Vec<O> {
            self.with(|e, ctx| e.press(code, ctx, bind))
        }

        fn up(&mut self, code: u32) -> Vec<O> {
            self.with(|e, ctx| e.release(code, ctx))
        }
    }

    use Out::{Action, Press, Release};

    #[test]
    fn cmd_c_is_sent_as_ctrl_c_and_cmd_never_reaches_the_client() {
        let mut f = Fixture::new();
        assert_eq!(f.down(CMD), vec![]);
        assert_eq!(f.down(C), vec![Press(CTRL), Press(C)]);
        assert_eq!(f.up(C), vec![Release(C), Release(CTRL)]);
        assert_eq!(f.up(CMD), vec![]);
        assert!(f.engine.sent().is_empty());
    }

    #[test]
    fn a_lone_tap_of_cmd_or_option_sends_nothing() {
        let mut f = Fixture::new();
        for key in [CMD, OPTION] {
            assert_eq!(f.down(key), vec![]);
            assert_eq!(f.up(key), vec![]);
        }
    }

    #[test]
    fn ctrl_and_shift_are_sent_at_once() {
        let mut f = Fixture::new();
        assert_eq!(f.down(CTRL), vec![Press(CTRL)]);
        assert_eq!(f.down(C), vec![Press(C)]);
        assert_eq!(f.up(C), vec![Release(C)]);
        assert_eq!(f.up(CTRL), vec![Release(CTRL)]);
    }

    #[test]
    fn option_types_the_third_level_from_the_left_side_and_is_alt_otherwise() {
        let mut f = Fixture::new();
        f.down(OPTION);
        assert_eq!(f.down(K3), vec![Press(LEVEL3), Press(K3)]);
        assert_eq!(f.up(K3), vec![Release(K3), Release(LEVEL3)]);
        assert_eq!(f.down(UP), vec![Press(OPTION), Press(UP)]);
        assert_eq!(f.up(UP), vec![Release(UP), Release(OPTION)]);
        f.up(OPTION);
        assert!(f.engine.sent().is_empty());
    }

    #[test]
    fn a_text_rule_is_sent_with_exactly_its_modifiers() {
        let mut f = Fixture::new();
        f.down(CTRL);
        f.down(CMD);
        // Cmd+Ctrl+Left has no rule, so it is Ctrl+Left by the general rule.
        assert_eq!(f.down(LEFT), vec![Press(LEFT)]);
        f.up(LEFT);
        f.up(CTRL);
        assert_eq!(f.engine.sent(), &[] as &[u32]);
        assert_eq!(f.down(LEFT), vec![Press(HOME)]);
        assert_eq!(f.up(LEFT), vec![Release(HOME)]);
        f.up(CMD);
    }

    #[test]
    fn a_sequence_is_sent_once_and_its_release_sends_nothing() {
        let mut f = Fixture::new();
        f.down(CMD);
        assert_eq!(
            f.down(BACKSPACE),
            vec![
                Press(SHIFT),
                Press(HOME),
                Release(HOME),
                Release(SHIFT),
                Press(BACKSPACE),
                Release(BACKSPACE)
            ]
        );
        assert_eq!(f.up(BACKSPACE), vec![]);
    }

    #[test]
    fn a_desktop_chord_reaches_no_client() {
        let mut f = Fixture::new();
        f.down(CMD);
        assert_eq!(f.down(TAB), vec![Action("switcher")]);
        assert_eq!(f.up(TAB), vec![]);
        assert_eq!(f.up(CMD), vec![]);
    }

    #[test]
    fn untranslated_every_key_goes_raw() {
        let mut f = Fixture::new();
        f.translate = false;
        assert_eq!(f.down(CMD), vec![Press(CMD)]);
        assert_eq!(f.down(C), vec![Press(C)]);
        f.up(C);
        assert_eq!(f.up(CMD), vec![Release(CMD)]);
    }

    #[test]
    fn a_focus_change_mid_chord_leaves_nothing_held() {
        let mut f = Fixture::new();
        f.down(SHIFT);
        f.down(CMD);
        f.down(C);
        let released: Vec<O> = f.engine.unwind();
        assert_eq!(released, vec![Release(C), Release(CTRL), Release(SHIFT)]);
        let pressed: Vec<O> = f.with(|e, ctx| e.resync(ctx));
        assert_eq!(pressed, vec![Press(SHIFT)]);
        assert_eq!(f.up(C), vec![], "a spent key's release sends nothing");
        assert_eq!(f.up(CMD), vec![]);
        assert_eq!(f.up(SHIFT), vec![Release(SHIFT)]);
        assert!(f.engine.sent().is_empty());
    }

    #[test]
    fn a_click_with_cmd_held_goes_with_ctrl() {
        let mut f = Fixture::new();
        f.down(CMD);
        let down: Vec<O> = f.with(|e, ctx| e.buttons(true, ctx));
        assert_eq!(down, vec![Press(CTRL)]);
        let up: Vec<O> = f.with(|e, ctx| e.buttons(false, ctx));
        assert_eq!(up, vec![Release(CTRL)]);
        assert_eq!(f.up(CMD), vec![]);
    }
}
