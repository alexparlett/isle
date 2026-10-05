# P5 · Keyboard

Planned 2026-10-05. Reviewed by the user the same day: every
recommendation taken, with v1's chords revisited rather than carried
(choice 11) and games, inhibitors and Isle's own keys reworked (choice 15).

The roadmap's brief: per-device profiles and the Mac preset in the
compositor, with the Alt and Super swap on PC keyboards on by default
(B30); Cmd translation with per-app exceptions, off in games and under
shortcuts-inhibit (B21); the default shortcut set. Retires xremap.

P4 left one XKB keymap for the seat from `keyboard` in settings, and a
filter that matches a field of chords in `binds` against the seat's
modifiers and forwards everything else untouched. Super reaches apps as
Super, so Cmd+C copies nowhere, and the host still needs v1's xremap to
type like a Mac. P5 makes the compositor the keyboard's only reader: each
keyboard types through its own profile, Cmd chords become each app's own
shortcuts, the desktop's chords are one named set with no clashes, and a
VM, a remote desktop or a game gets its keys raw. What it must get right is
the stream a client sees: every key and modifier it is sent must agree, and
nothing may stay held after a chord, a focus change or a mode change.

## What it answers to

- Decisions: B17, B20 (Cmd+Tab and Cmd+` stay the desktop's), B21 (Cmd
  everywhere, per-device profiles, inhibit and games pass through, injected
  keys untranslated, releases before focus moves), B30 (the swap, Option on
  a Mac keyboard, the escape set), B35 (the loop, the worker, Test1), B36
  (settings written into the file's text), B38 (how a shipped data file is
  read with a user's file over it, the model for the translation tables).
- Intent: `compositor.md` Keyboards and per-device profiles 1–7, Cmd
  translation 1–10, Shortcuts and keybinds 1–9, Keyboard-shortcuts-inhibit
  and injected input 1–3, Pointer 3 (the drag chord in the set); its
  acceptance bars for translation and the escape set. `apps.md` Settings 9
  and 10 (the recording grab, keys named by XKB) as mechanism; Settings ›
  Apps's per-app translation switch as its setting. `platform.md` Settings
  for the new sections.
- Design: `p1/structure/competition.md` Keys (what wins each chord),
  `settings.md` (the recorder), `lock.md` (the layout chord, keys while
  locked), `capture.md` Keys, `spotlight.md` and `menu-bar.md` (their
  chords are desktop chords taken before translation), `window-chrome.md`
  Keyboard and the inhibit notice, `island.md` (the inhibit glyph and
  notice row), `review/accessibility.md` 7 (Sticky Keys and Slow Keys with
  P5's chord table), `copy.md` rule 11 (Mac symbols on a Mac keyboard,
  words on a PC one).
- Evidence: `macos-model.md` §2 (the precedents, the tables to start from,
  the edge cases, how a compositor should do it), `isle-v1-inventory.md`
  §11 (v1's keymap, profiles, default chords and lessons D7, D32, D58–D61)
  and §16 (the switcher's release), the host's keyboards (a Keychron in Mac
  mode and a PC keyboard behind a KVM, UK layout, v1 giving the Keychron
  `gb(mac)` on `pc105`).

## Choices this plan makes

1. **An engine of its own.** A crate `isle-keys` holds the keyboard's
   logic with no Smithay state in it: the chord grammar and its names, the
   shortcut set's table, the translation tables and their parser, and an
   engine that takes physical key and button events with their device and
   context and returns what to emit and which actions to run. It uses
   xkbcommon directly and is unit-tested against compiled keymaps (`gb`,
   `gb(mac)`, `us`, `de`, `fr`, a Dvorak). `isle-compositor`'s `keyboard`
   module owns the devices, the seat's keymap, the protocols and the bus,
   and feeds the engine.
2. **The seat sees only what Isle emits.** The engine's output, key
   presses and releases by keycode, is fed into Smithay's keyboard with
   `input_from_source` and a filter that always forwards; desktop actions
   are the engine's decision, not the filter's. The seat's XKB state is
   then built from the emitted keys alone, so the modifiers a client is
   sent always agree with the modifier keys it was sent, Xwayland included
   (answers Cmd translation, Open: real modifier key events are always
   sent, with the `modifiers` event Smithay derives from them). The engine
   keeps its own XKB state per keymap for the physical keys, which is what
   chords match against.
3. **Keyboards and their profiles.** Each keyboard is known by its name,
   as pointers are, and has a `keyboard-device "name" { }` block:
   `profile` (`auto`, `mac`, `pc`), `swap-alt-super` and `model` (`auto`,
   `iso`, `ansi`). `auto` guesses Mac from the name (Apple, Magic Keyboard,
   Keychron; v1's list) and ISO from the first layout's region. A Mac
   keyboard gets each layout's `mac` variant where XKB has one, asked of
   XKB rather than assumed (D61), on that device only (D32); a PC keyboard
   gets the plain layouts. Both translate: the profile decides the swap,
   the variant and what Option does (choice 7), not whether Cmd works.
4. **One seat keymap, following the keyboard that types.** Keymaps are
   compiled when a keyboard appears or the settings change and kept by
   their configuration, so keyboards with the same configuration share one.
   A key from a keyboard whose keymap is not the seat's switches the seat
   to it first, and clients are sent the new keymap, as wlroots does; the
   switch waits while any key is held, and the key then types through the
   current keymap. On the host this is a switch between the Mac and the PC
   keyboard behind the KVM, not per key (answers Keyboards, Open). Num
   Lock is set at each keymap's start from `keyboard.numlock`.
5. **The swap is on keycodes, left side only.** On a PC keyboard with
   `swap-alt-super` (on by default) left Alt and left Super trade keycodes
   before anything else reads them, so bindings and translation see Cmd
   beside the space bar. Right Alt stays AltGr, which a UK PC keyboard
   needs for its third level and which a right Super key is often missing
   to swap with. The swap is decided per key at its press: a key held
   while a game takes focus releases what it pressed.
6. **Cmd and Option are withheld.** While translation is on, a Cmd or
   Option press is not sent; the next key or button decides what it
   becomes. Ctrl and Shift are sent at once (B21: Ctrl stays Ctrl). A lone
   tap of Cmd or Option sends nothing, as on macOS, so no app opens a menu
   on a released Alt. A button pressed with Cmd held is sent with Ctrl (a
   Cmd-click opens a link in a new tab), with Option held, Alt; scrolling is
   sent with neither.
7. **What a key becomes.** At each press, in order: the recording grab
   takes it (choice 16); under an inhibitor or in an untranslated app only
   the escape chord acts and the rest goes raw; over a game only the
   escape set acts and the rest goes raw; a desktop chord from the shortcut
   set, matched on the physical keys, runs its action; the focused app's
   table; the shared tables (text motion, Ctrl's emacs motion, terminals);
   Cmd+key as Ctrl+key; on a Mac keyboard, Option with a key that has a
   third-level symbol types that symbol from either Option (Option+3 types
   # on a UK Mac keyboard), otherwise Alt+key (Option+↑ moves a line in VS
   Code). A rule's target is sent with exactly its modifiers, which are put
   back as the physical keys imply when its key is released. A single
   target key held down repeats through the client's own repeat; a
   sequence is sent once.
8. **Chords are symbols, resolved through XKB.** Rules and shortcuts are
   written as symbols (`Cmd+grave`), matched against a key's first-level
   symbol in the device keymap's first Latin layout, so Cmd+C is the C key
   on a Russian layout and Cmd+` is the key left of Z on a UK Mac keyboard
   (D61). A target is emitted by the keycode that gives its symbol at the
   first level in the seat's keymap; a rule whose target has no such key is
   dropped with a warning naming it. Rules do not tell left from right
   modifiers.
9. **Focus never moves with keys held.** Every change of keyboard focus
   goes through one function. Before it, the engine sends the old focus a
   release for every key and modifier it emitted; after it, the new focus
   is sent the modifiers its own context implies for the keys still
   physically held (Shift and Ctrl as they are, Cmd and Option withheld
   again). A key whose press was translated is spent: its release sends
   nothing. The same unwinding runs when the context changes under held
   keys (a game in front, an inhibitor granted, a table reloaded).
10. **The shortcut set replaces `binds`.** A `shortcuts` section with a
    field per action, as P4 found the derive types, validates and
    describes them; the field's name is the action's id, its doc comment
    its label, its default the Mac chord, and the empty string turns it
    off. Each action also carries flags in `isle-keys`: in the escape set,
    live while locked, repeating, and the packet that builds it. Chords are
    written in Mac names (`Ctrl+Option+Cmd+W`), with ⌘ ⌥ ⌃ ⇧ and P4's
    `Super` and `Alt` also read. A chord bound twice is refused through a
    new cross-field check on `Section`, so a hand edit and a `Set` are
    both refused with a message naming both actions and the last good set
    stays (Shortcuts 4). An old `binds` block is ignored with a warning.
11. **Every designed chord is in the set now.** The defaults are the
    design's, collected from `compositor.md` and P1's sheets (the
    windows, desktops, apps, shell surfaces, capture, system and
    accessibility chords). An action a later packet builds is reserved: its
    chord is the desktop's from today, so translation never hands Cmd+Space
    to an app as Ctrl+Space, and it logs that it is not built. Where the
    design left a chord open:
    - **The inhibit escape chord: Cmd+Escape**, GNOME's Super+Escape; it is
      neither Cmd+Ctrl+Escape (stop recording) nor Cmd+Option+Escape (Force
      Quit), and no app chord on macOS uses it.
    - **The layout chord: Ctrl+Space**, macOS's, live only while two or
      more layouts are set; with one it stays the app's, as
      `competition.md` has it.
    - **v1's chords are not carried over** (the user, at review): an
      action keeps its capability, but has a chord only where macOS has one
      or where nothing else reaches it, so `compositor.md` Shortcuts'
      "every v1 action keeps a chord" is revised. Settings, Monitor, Files,
      a terminal, pin and focus by direction have none by default; they are
      reached from Spotlight, the system menu and Mission Control, and
      the user can bind them. Redetect displays keeps one,
      Ctrl+Option+Cmd+R on the layer the dashboard, Notification Centre,
      Do Not Disturb and Game Mode already use, because after a KVM switch
      it is the one-keypress recovery `compositor.md`'s acceptance bar
      asks for, with nothing on screen to click. v1's power menu, clipboard,
      emoji, capture, record and colour-pick chords are already replaced
      by P1's (the session chooser, Spotlight, Ctrl+Cmd+Space, the capture
      chords).
    - **Zoom**: macOS's Option+Cmd+8, Option+Cmd+= and Option+Cmd+−, off
      by default as on macOS until turned on in Settings › Accessibility.
    - P4's Cmd+Return terminal and Super+Shift+Escape quit go: Cmd+Return
      is the app's (and Spotlight's), and quitting the session now is
      Option+Shift+Cmd+Q, P9's log out without confirmation, built here as
      the plain quit.
12. **Translation tables are a shipped data file.** `data/keys/
    translation.kdl` holds groups of apps (terminals, Chromium browsers,
    Firefox browsers, VS Code, JetBrains, file managers, dialogs, remote
    desktops and VMs) and rules per group and per app, each a Mac chord to
    a chord, a sequence of chords, or one of the desktop's close and quit.
    A user's `~/.config/isle/translation.kdl` lies over it by group, app
    and chord, read like `tokens.kdl` (B38): reloaded live by the worker, a
    file that does not parse refused with its place while the last good
    tables stay. Apps match by resolved desktop id, raw app id or
    `WM_CLASS`, case-insensitively, with a trailing `*`; dialog rules also
    match the title. The tables are written for Isle from v1's
    `profiles.mac` and the record Toshy and Kinto keep of what apps need,
    not copied from them (GPL). `settings.kdl` keeps only `keyboard.
    translate` (on) and `keyboard.untranslated-apps`, the per-app switch
    Settings › Apps writes, its default the remote desktops and VMs.
13. **Close and quit belong to the desktop, delegated per app.** Cmd+W and
    Cmd+Q stay desktop chords (P4's close path with keep-running, and quit
    with the polite signal), and an app's table may name its own chord for
    them, which the action sends instead: Ctrl+W closes a browser's tab,
    Ctrl+Shift+W kitty's window, Ctrl+Q quits Firefox. A dialog with no
    close chord of its own is closed by the compositor.
14. **The inverse for menus.** `isle-keys` answers which Mac chord gives an
    app a chord, from that app's tables, so P6 can show ⌘C beside a menu
    item an app declares as Ctrl+C (Cmd translation 10).
15. **Games, inhibitors and Isle's own keys.**
    - **A game is a flag on the window's record**, which P5 sets and P12
      later sets better, so every reader (keys here, VRR, Game Mode, idle)
      asks one question. Neither KWin nor CachyOS detects games (KDE turns
      Do Not Disturb on for any full-screen window; CachyOS's
      `game-performance` is an opt-in launch wrapper and ananicy only sets
      priorities), but Steam labels its own, so P5 reads, strongest first:
      an X11 window's `STEAM_GAME` property, the app id Steam's overlay
      sets and gamescope relies on (read by Isle's window manager on the
      fork, like SHAPE); a `steam_app_*` or gamescope class; `SteamGameId`
      in the process's environment (from `/proc` on the worker when the
      window maps), for Wayland-native games; a process registered with
      Feral GameMode (`ListGames` and its signals, which Lutris and Heroic
      use); an X11 class ending in `.exe`. `windows.games` and
      `windows.not-games` list apps by id to correct a guess either way.
      Full screen alone is no sign: B30 follows games windowed or not, and
      a full-screen video is not one. ananicy's list of game process names
      is left to P12 as a fallback for non-Steam native games. With a game
      in focus nothing is translated, withheld or swapped, and only the
      escape set acts (B30).
    - **keyboard-shortcuts-inhibit** is served and every inhibitor is
      granted (P1). An X11 client's `xwayland-keyboard-grab` counts as one.
      While the focused surface holds one, keys go raw and only Cmd+Escape
      acts. Cmd+Escape toggles: once to take the keys back, again to give
      them back to the same window without refocusing it, as a VM viewer's
      own release key does; leaving the window takes them back too, and
      returning gives them back. Whether one is active, and for which app,
      is on the bus and in the shell's model for P6's glyph and notice;
      "Take keys back" calls the same action.
    - **A game's inhibitor does not hide the escape set.** SDL and Wine
      ask for an inhibitor or a keyboard grab when a game goes full
      screen; it is granted, but over a game Cmd+Tab, Mission Control,
      lock and the screenshot chords still act, as B30 promises (answers
      `gaming.md`, Open). Only a window that is not a game loses them to
      its inhibitor.
    - **Isle's own keys** (Spotlight's paste, emoji, the on-screen
      keyboard) are aimed at a window by id, never at whatever has focus:
      Isle gives that window focus if it lacks it, and types nothing if it
      is gone. They are never translated and never mixed with held keys:
      the modifiers the window was sent are released around them and put
      back after, so a Shift still held from the chord that opened
      Spotlight does not capitalise its paste. Three forms: an action sent
      as that app's own chord through its table (paste is Ctrl+Shift+V in
      a terminal, Ctrl+V elsewhere); text committed through text-input-v3
      when the window has a text field active, which carries any
      character, emoji included, in one commit; otherwise text typed
      through a keymap made for those characters and the seat's keymap put
      back after (the fork's `inject_text_keysyms`, as KWin does).
    - **virtual-keyboard** is served; its keys are a client's own and are
      not translated (Smithay sends them with the client's keymap).
16. **The recording grab.** `Keyboard1.Record` suspends desktop chords and
    translation and returns the next chord as text, with the action that
    already holds it if any; Escape cancels, Delete returns an empty chord,
    focus leaving ends it. `Keyboard1.Detect` returns the keyboard the next
    key came from, and with the key left of Z reports ISO, for Settings ›
    Keyboard's Detect.
17. **Media keys, now, through commands.** The volume, mute, microphone,
    brightness and media keys are actions in the set, live while locked
    and over games, volume and brightness repeating in the compositor at
    the keyboard's rate. Until P8's services they run v1's commands
    (`wpctl`, `brightnessctl`, `playerctl`); P8 swaps the bodies for its
    services and OSDs.
18. **Held chords.** An action may hold its chord until the chord's
    modifiers are released, and is told when they are, whatever was pressed
    between; the switcher (P7) commits this way, as macOS does and as v1
    could not (inventory §16). Built and tested here with a test action.
19. **Sticky Keys and Slow Keys** latch in the engine:
    `keyboard.sticky-keys` (a modifier tapped once applies to the next
    key, twice locks; Shift five times toggles it, as macOS) and
    `keyboard.slow-keys` (a key counts only when held for the delay). Their
    drawing with the on-screen keyboard's held and locked forms is P20's,
    their switches in Settings P13's.
20. **xremap retires.** `isle-session.target` conflicts with
    `xremap.service`, so starting Isle stops v1's remapper, which would
    otherwise grab the keyboards and send Cmd chords as Ctrl. A keyboard
    named `xremap` is reported on the bus and logged as v1's remapper.
    v1's own xremap files go with v1's tree in P21.

## The workspace

```
crates/
  isle-keys/             chords and names, the shortcut table, translation
                         tables, the engine (new)
  isle-compositor/src/
    keyboard/            devices and profiles, keymaps, the seat's feed,
                         focus unwinding, inhibit, injection, recording
    binds.rs             gone; actions move to keyboard/actions.rs
  isle-settings/         `keyboard`, `keyboard-device`, `shortcuts`; the
                         cross-field check
  isle-ipc/              Keyboard1 (new)
  isle-cli/              isle keyboard …, isle shortcuts …
data/keys/translation.kdl
data/systemd/isle-session.target
```

`Test1` gains keyboards: `AddKeyboard(name)` returns a device, and `Key`
takes one. The test client prints each key as its keysym and every
`modifiers` event, from the keymap it is sent, and the X11 test client
prints each key's state mask. A third test client holds a
keyboard-shortcuts inhibitor on command.

## Phases

Each phase ends in a check run by script or in the VM and lands as its
own commits.

### 5.0 · Keyboards and keymaps

- Keyboards on every backend (libinput devices with the keyboard
  capability; winit and headless as one each; Test1's added ones), their
  names, `keyboard-device` settings, the guessed profile.
- Keymaps per configuration, the seat following the typing keyboard, Num
  Lock, the swap on keycodes.
- `Keyboard1`: keyboards with profile and guess, layouts, the active
  layout, Caps Lock; `isle keyboard list|profile`.

**Check:** headless: a keyboard named "Keychron K2" gets `gb(mac)` and no
swap, one named "AT Translated Set 2 keyboard" gets `gb` and its left Alt
reaches the client as Super; typing on one then the other sends the
client the second keymap once, not per key; a profile set by `isle
keyboard profile` applies to the next key and is kept in the file.

### 5.1 · The engine

- `isle-keys`' engine (choices 1, 2, 6–9) with its unit tests on the
  fixture keymaps; the compositor feeding it and feeding the seat what it
  emits; focus changes through one function with the unwinding; buttons
  with Cmd and Option.
- Only the general rules yet: Cmd+key as Ctrl+key, Option on a Mac
  keyboard, the shared text-motion and emacs tables from v1.

**Check:** headless: Cmd+C reaches the client as Control_L, C and
modifiers Ctrl, with no Super ever sent; a Cmd tap sends nothing; Option+3
on the Mac keyboard types `numbersign` and Option+↑ arrives as Alt+↑;
Cmd+← arrives as Home with no modifier; Cmd held across a focus change
leaves no key or modifier down in either client; a Cmd-click arrives with
Ctrl; a held translated key's release is the only release.

### 5.2 · The shortcut set

- `shortcuts` replacing `binds` (choices 10, 11), the cross-field check,
  the chord grammar's names in Mac words and symbols, every designed chord
  with reserved actions logging, the layout chord, media keys with repeat
  (choice 17), held chords (choice 18), the drag chord from the set.
- `isle shortcuts list` with id, label, chord and the packet that builds
  it.

**Check:** headless: every built action's default chord runs it and never
reaches the client; a reserved chord reaches neither; two actions on one
chord in a hand edit are refused naming both and the old set still acts;
a held test action is told of the release after Tab and arrows in
between, in a hundred scripted attempts; Ctrl+Space reaches the client
with one layout and switches with two; volume up held repeats.

### 5.3 · Translation tables

- `translation.kdl` (choice 12): its format, parser and the user's file
  over it, live; groups and apps from the window registry's app; the
  tables written; close and quit delegated (choice 13); the per-app
  switch; the inverse (choice 14).

**Check:** headless, table-driven: every rule in the shipped tables, fed
through a client with a matching app id, arrives as its target with the
right modifiers and nothing held after (the acceptance bar's "covers every
table entry"); Cmd+W in a client of the browser group sends Ctrl+W and in
a plain one closes the window; an untranslated app gets Super; a broken
user file is refused with its line and the shipped tables still act.

### 5.4 · Games, inhibitors, Isle's keys, recording

- The game flag on the registry with its five sources and the two
  override lists, and its effects (choice 15); keyboard-shortcuts-inhibit and
  xwayland-keyboard-grab with Cmd+Escape's toggle and the escape set kept
  over a game's inhibitor; the state on the bus and in the shell's model;
  injection aimed at a window in its three forms, with text-input-v3
  served; virtual-keyboard; `Keyboard1.Record` and `Detect` (choice 16).

**Check:** headless: an X11 client with `STEAM_GAME` set, a client
started with `SteamGameId` set, a `steam_app_1` one and one registered with
a stand-in GameMode service on the test bus get Ctrl as Ctrl and Super
as Super, unswapped, and only
the escape set acts over them, inhibitor or not; one listed in
`windows.not-games` is translated; an inhibiting client gets Cmd+Tab raw,
Cmd+Escape takes the keys back and again gives them back, and refocusing
gives them back; text injected at a window that lost focus reaches it
untranslated and unshifted while Cmd and Shift are physically held, and an
emoji arrives through the injection keymap and through text-input-v3; a
recording returns "Shift+Cmd+K" and names the holder of a used chord.

### 5.5 · Sticky Keys and Slow Keys

- Latching and locking in the engine, Shift five times, the slow-keys
  delay; the settings.

**Check:** unit tests in `isle-keys` and one headless test: with Sticky
Keys on, Cmd tapped then Tab opens the held test action and the next Cmd
tap releases it; with Slow Keys a short press sends nothing.

### 5.6 · The VM and the host

- `isle-session.target` against `xremap.service` (choice 20).
- The VM: kitty, a GTK 4 app, a Qt 6 app and xterm with `xev`: copy,
  paste, close, quit, word and line motion, the terminal rules, modifiers
  in Xwayland as `xev` reports them, a focus change mid-chord.
- The host, in the user's run on tty8: the Keychron and the PC keyboard
  behind the KVM typing the same password and £ € # correctly; Cmd+C,
  Cmd+V, Cmd+W and Cmd+Q in Chromium, kitty, VS Code, a GTK 4 and a Qt 6
  app; a game with Ctrl as crouch; xremap stopped by the session.
- `docs/research/rust-desktop/p5-keyboard.md`: what Xwayland did, keymap
  switch cost, findings per app.

**Check:** the VM's `xev` logs and screenshots; the host run's notes from
the user.

### 5.7 · Close

- ARCHITECTURE: how keys are read, from device to client.
- DECISIONS for what later packets must keep (the seat sees only what Isle
  emits; withheld Cmd and Option; the one focus path; the tables as a
  shipped data file).
- `compositor.md`'s open points answered and Shortcuts' v1 lesson revised
  (choice 11); `state.md`, `progress.md`, the
  roadmap's P5 entry.

## Not in P5

Telling a text field from the rest of an app with text-input-v3, so Cmd+←
is Back outside a field in a browser (Cmd+[ and Cmd+] are Back and Forward
there now); libei for the portal's RemoteDesktop (P11); the island's
inhibit glyph and the window's notice (P6, from the state P5 exposes);
Settings' keyboard, shortcut and accessibility pages (P13); the switcher
(P7), Spotlight (P10), lock (P9) and the other actions reserved here; the
OSDs for volume and brightness (P8); game detection proper (P12); the
on-screen keyboard and emoji typing (P20, through P5's injection);
focus by direction (P20).

## Depends on, and feeds

- Needs P2 (seat, settings, Test1) and P4 (the window registry and apps,
  which the tables key on, and the actions the set binds).
- P6 shows chords beside menu items through the inverse and draws the
  inhibit state. P7 commits the switcher by a held chord. P8 replaces the
  media keys' commands. P9 reads which keys act while locked. P10 and P20
  type through the injection source. P12 replaces the game rule. P13
  draws the recorder and the keyboard pages over `Keyboard1`.

## Done when

- `cargo test` passes headless: keyboards and profiles, the engine's
  rules, every shortcut default, the clash refusal, every shipped table
  rule, focus changes mid-chord, games, inhibitors, injection, recording,
  with no stuck modifier in any of them.
- In the VM, kitty, GTK 4, Qt 6 and X11 apps take Cmd chords as a Mac user
  expects, and `xev` shows consistent modifiers.
- On the host, both keyboards type correctly with xremap gone, and the
  acceptance bar's apps behave.

## Risks

- **Xwayland's modifiers.** It derives its state from keys and the
  `modifiers` event; if it disagrees with what was sent, the fix is in
  how the engine orders them, found in 5.6 with `xev`.
- **Keymap switches.** Some clients may handle a new keymap mid-session
  badly; the switch happens only between keyboards with different
  configurations, and 5.6 tries it on the host with the KVM.
- **The tables.** They are where translation will be wrong in practice;
  they are data, reloaded live, and the user's file fixes an app without a
  build.
- **Injection keymaps and Xwayland.** Xwayland has been slow to take a
  keymap sent for a moment; if typed characters are lost there, X11
  windows get only characters the seat's keymap has, found in 5.6.
- **Games in a container.** Steam's runtime may hide a game's process
  from `/proc` lookups by the PID Isle sees; the class rules still hold,
  and 5.6 checks a native and a Proton game on the host.
- **Alt in Electron and Chromium.** Withholding Option is meant to stop
  menus opening; VS Code's Option chords are checked on the host.

## Outcome

Phases 5.0 to 5.6 passed their checks headless and in the VM, with these
differences from the plan:

- **Text injection** goes through a keymap made for the characters
  (Smithay's `inject_text_keysyms`), not text-input-v3: serving it with the
  compositor as the input method makes GTK 4 hand composition to it, which
  would take dead keys from every GTK app. text-input-v3 is not served.
- **Dialog rules by title** are not needed: the close and quit chords of
  an app's table apply to main windows only, and dialogs are closed by the
  compositor.
- **An old `binds` block** is ignored silently; the settings derive gained
  retired names and a cross-field check for the clash refusal.
- **GameMode** is session-activated, so asking it for games started it (the
  test bus found this); Isle asks only while the name has an owner.
- **`STEAM_GAME`** needed the Smithay fork to read it:
  alexparlett/smithay#2, merged; Isle pins `764ad1b`.
- **Devices with no letter keys** (the power button) are kept but left out
  of the keyboard list. The list is empty after start until the first
  input event, since libinput's added devices are dispatched with it.
- **The layout**, unset, is the system's as `localectl` sets it
  (`XKB_DEFAULT_*`, then `/etc/vconsole.conf`, then Xorg's keyboard class),
  not US English: the first host try typed a UK keyboard as American.
- **Input after an inactive start.** A session started on a console that
  was not in front opened the GPU when it came to the front but never
  opened input, so keyboard and mouse were dead (found on the host, a P2
  bug); libinput is now suspended and resumed then. Checked in the VM.
- **The X11 map test** read a window's focus from before Xwayland's focus
  arrived; it now waits, as the extra worker thread at map moved the
  timing.

Tests: `cargo test --workspace --features isle-compositor/test-hooks`, with
`isle-keys`' 25 unit tests (engine, chords, keymaps, tables) and 24 headless
tests for P5: keyboards and keymaps, the engine end to end, every chord of the
set, clashes, held chords, every shipped table rule typed into an app it
applies to, games, inhibitors, injection, recording, Sticky and Slow Keys.
Findings in `docs/research/rust-desktop/p5-keyboard.md`.

Left for the user's host run on tty8: both keyboards (the Keychron and the PC
keyboard behind the KVM) typing the same password and £ € #; Cmd+C, Cmd+V,
Cmd+W and Cmd+Q in Chromium, kitty, VS Code, a GTK 4 and a Qt 6 app; a game
with Ctrl as crouch; xremap stopped by the session.

