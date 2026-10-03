# Design brief

P1 stage 1, gate 1. What the new Isle is designed to: the principles, the
user's answers, and what it takes from macOS as is, changes, or leaves out.
Accepted at gate 1 on 2026-10-03, open to revision at later gates.

Evidence: `docs/research/rust-desktop/design-macos.md` (macOS 27),
`design-references.md` (others), `design-v1-lessons.md` (v1), and the
interview below.

## Who it is for

One person, on one machine: CachyOS on an RTX 5070 Ti, a single 3440×1440
ultrawide shared through a KVM, used mostly for gaming and coding, with a
Mac-layout keyboard as well as PC ones. A second screen is supported, not
designed for. The user decides by looking, quickly, and expects
platform-standard behaviour exactly.

## The interview

Asked 2026-10-03 in four rounds of choices.

| Question | Answer |
|---|---|
| Light and dark | Both, dark first: light fully designed, Auto as on macOS; dark tuned and shown first |
| Material | Frosted: heavily blurred, mostly opaque; colour from behind, never shapes |
| A glass setting | Yes: one slider from more see-through to fully tinted, frosted by default; Reduce Transparency still makes it solid |
| Density | Mac default sizes |
| Colour | One accent, the user picks it |
| Typeface | Left to the directions (free to ship; no SF Pro) |
| Motion | Restrained: short, critically damped moves and fades, no bounce |
| Dock | Bottom, always shown |
| Island at rest | A small dark pill, always present, like a notch |
| Wallpapers | Dark and quiet |
| Windowed game in front | Shell stays as normal; Gaming Focus only holds notifications |
| Sounds | A few, Mac-like (alert, screenshot, volume tick, trash), each switchable |
| Leave out of macOS | Forcing icons into a squircle; icons on menu items; large window corners |
| Carry from v1 | The island opens on hover; a full-screen widget dashboard on a chord |
| Not carried from v1 | Desktop dots at the top; status glyphs only when meaningful (as a fixed rule) |
| Dislikes | Visual noise; apps that look like a different desktop |

## Principles

To design to. Each is checked at every gate.

1. **Calm at rest.** The screen at rest is the wallpaper, the menu bar, the
   pill and the Dock. Nothing else shows unless it has something to say,
   and nothing reports the normal state loudly. (Interview: visual noise;
   v1 lessons §1.1.)
2. **One desktop.** Shell, Isle's apps and installed apps read as one
   thing: the same type, accent, icons and cursor, from one token file.
   (Interview: inconsistent apps; v1 D82.)
3. **A Mac, built for this machine.** Where a choice is open, the answer a
   Mac user expects (B17), sized for the ultrawide and tuned for gaming
   and coding. Departures from macOS are deliberate and listed below.
4. **Legible before translucent.** Frosted glass carries colour, never
   shapes. Text contrast is guaranteed against the surface it sits on;
   every material has a solid form, used by Reduce Transparency, game mode
   and the slider's tinted end. Windows are opaque, with their own raised
   material for menus and popovers. (v1 lessons §8.)
5. **Quick and still.** Motion is short, critically damped and anchored:
   nothing bounces, nothing slides into a place it should have opened in,
   a resize never re-centres. Exits are faster than entrances. Reduced
   motion removes movement. (Interview: restrained; v1 D91.)
6. **The accent is a signal.** One user-chosen accent marks the one thing
   that is active, focused or live. Status colours are for status;
   everything else is neutral. (Interview; v1 D16.)
7. **Out of the way of a game.** A game in front owns the screen and the
   keys; the shell keeps only its escape set and a one-line summary
   afterwards. (B30.)

## From macOS

**As is**

- The structure: menu bar with the app's menus, status items and Control
  Centre; the Dock at the bottom; Cmd+Tab app switching; Notification
  Centre from the clock; Spotlight; Mission Control and Spaces; System
  Settings' sidebar.
- Light, dark and Auto; one accent; Reduce Transparency, Increase Contrast,
  Reduce Motion and text size as first-class settings.
- 27's glass slider (clear to tinted, a middle default), here starting
  frosted.
- 27's corrections to Tahoe: uniform, smaller window corners; stronger
  shadows on the active window; sidebars to the window edge; one toolbar
  style across apps.
- A few system sounds.

**Changed**

- The glass is frosted, not Liquid Glass: no refraction or moving
  highlights; blur strong enough that shapes behind never read.
- Motion is quieter: no bounce, no genie; minimise uses Scale.
- The island: a small dark pill in the menu bar's centre, always present,
  that grows to show live activity and opens on hover (with v1's hover
  guards), where macOS has a notch and no island.
- Status items: macOS's per-item "show always / when active / never"
  choice, with Isle's own defaulting to "when active" so the bar stays
  calm. (Reconciles the interview's dislike of noise with not making v1's
  rule absolute.)
- Widgets: a full-screen widget dashboard on a chord, alongside or
  instead of macOS's desktop widgets; which one the structure stage
  decides (B24).
- Closing windows from Mission Control, as v1 did.

**Left out**

- Squircle enforcement: app icons keep their own shape; no grey tile.
- Icons beside menu items.
- Large or varying window corners.
- Anything that needs SF Pro, SF Symbols or Apple artwork.

## Constraints

- Everything drawn by Freya on Skia in the compositor's GL context (B7);
  blur is the compositor's, and its cost at 3440×1440 beside a game is
  measured before a material is final.
- Fonts, icons, cursors and wallpapers must be free to ship.
- Tokens are data in `tokens.kdl`, reloaded live (B10).
- Installed GTK and Qt apps are themed from the same tokens (P14).

## Takeaways from the research

**macOS 27** (`design-macos.md`):

- Apple publishes few numbers: the menu bar is 24 pt, hit targets 28 pt
  (20 minimum), body text 13 pt (10 minimum), contrast WCAG AA. Window
  radii, bar heights, the Dock, banners, Control Centre, the OSD and every
  motion value are unpublished, so Isle sets its own and is not copying a
  hidden spec.
- 27 is Tahoe made legible, and its fixes are the brief's: one tighter
  window radius everywhere, deeper active-window shadows, darker glass
  edges, toolbars that become a solid bar when content scrolls under them.
  The faults it fixed are the ones to avoid: unreadable menus and titles,
  a menu bar with no edge, glass that reads white on non-Apple monitors,
  several corner radii on one screen.
- Glass sits only on controls floating over content, never on glass; text
  only on the regular, more opaque kind. Every glass surface has a solid
  form (Reduce Transparency), borders under Increase Contrast and 27's
  Show Borders, and no morph under Reduce Motion. Isle takes all four.
- Corners are concentric: a nested shape's radius is its parent's minus
  the padding between them; a capsule's is half its height. Adopted as a
  system rule (v1's first build had it and lost it).
- Apple stacks the top right: status items, Control Centre, the clock to
  Notification Centre, banners, and since 26 the volume and brightness
  OSD as a small capsule under the menu bar. The competition sheet weighs
  that against B24's proposal to put OSDs in the island.
- Control Centre is a grid where a control takes one, two or four cells,
  edited from a gallery, any control pinnable to the menu bar. A good fit
  for "one switch per thing in one place".
- Menus in 27 keep icons for a few common actions; Isle has none
  (interview).
- Spotlight's Apps view replaced Launchpad without folders, and people miss
  arranging apps their own way: the structure stage decides whether Isle's
  Apps view can be arranged.

**From others** (`design-references.md`), taken to the structure stage as
candidates, each still to earn its place against principle 1:

- Window zones on the ultrawide: a layout grid on the zoom button with
  thirds and a 25/50/25 centre (Windows 11), a target panel near the
  pointer while dragging so nothing needs a trip to a far edge (Rectangle
  Pro), almost-maximise and a centred fixed width, cycling half, third and
  two-thirds on repeated chords (Rectangle).
- Status items with a visibility tier (always, when active, hidden) and an
  overflow strip under the bar (KDE, Ice): matches the brief's change
  above, and Isle draws its own bar, so nothing breaks it.
- Island intent: hover peeks, click commits, and its hover zone never takes
  clicks from neighbouring items (NotchNook); progress pushed to the island
  by any process over D-Bus (Otto).
- Do Not Disturb tied to a full-screen window, told to apps through the
  freedesktop `Inhibited` property (KDE 6.4).
- Switcher: apps first, with an arrow into that app's windows in place
  (GNOME).
- Theming: one semantic palette rendered to templates, hand-written files
  overriding them (Omarchy); a minimum contrast per role pair; the accent's
  text colour clamped for contrast (libadwaita); terminal colours tinted
  toward the accent but checked; the portal's appearance keys for apps
  templates cannot reach.
- Accessibility finer than macOS's: motion speed separate from Reduce
  Motion, editable high-contrast palettes (COSMIC, Windows).
- Gaming: a performance overlay on one slider from FPS only to full
  telemetry, drawn by the compositor (Steam Deck, gamescope).
- Smaller: a clock that opens a month and agenda (Itsycal); a stack of
  capture thumbnails, OCR and redaction (CleanShot X); "highlight changed
  settings" (KDE); the greeter in the session's theme; password prompts
  that dim the screen and hold focus (elementary).

**From v1** (`design-v1-lessons.md` §8): "Carry as requirement" and "Do not
repeat" are adopted as they stand, except the conditional-glyph rule, which
becomes the "when active" default above.
