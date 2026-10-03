# How macOS 27 presents its surfaces

State as of 2026-10-03. Evidence for P1 stage 1a (`docs/work/rust-desktop/p1-design.md`):
how macOS 27 "Golden Gate" (released 2026-09-14) draws each surface Isle is
taking on, and how it got there from macOS 26 Tahoe, which introduced Liquid
Glass. Behaviour (what a surface does, its shortcuts, its settings as
behaviour) is in `macos-model.md` §3 and is not repeated; this note is about
presentation: content, states, sizes, material, type, colour, motion, and
reception.

**Method.** Every source was read on 2026-10-03; the links below carry each
source's own publication date where it has one, and the sources table at the
end lists them all with that read date. Apple's HIG pages render only with
JavaScript, so they were read through their JSON data endpoints
(`developer.apple.com/tutorials/data/design/human-interface-guidelines/<page>.json`)
and are cited at their normal URLs. Most support pages now serve the macOS 27
edition. Ars Technica blocked most of the fetches made; its two reviews were
read in one pass only, so claims resting on them alone are marked *(Ars only)*.
The Verge was not reached at all. Where a claim comes from a search-result
summary rather than an opened page it is marked *(summary)*. Points measured
by third parties rather than published by Apple are marked *(measured)*.
Nothing is filled in by guess; what could not be found is in **Open**.

Apple's spellings are kept for the names of settings and controls; the prose
uses Isle's ("Control Centre", "minimise").

---

## 0. What matters most for Isle's design

1. **Apple publishes almost no dimensions for macOS 26/27.** The HIG gives the
   menu bar (24 pt), text styles, hit targets (28 pt default, 20 pt minimum),
   contrast ratios and the clear-glass dimming (35 %). Window radii, title-bar
   and toolbar heights, Dock geometry, banner width, Control Centre's panel and
   the OSD capsule are unpublished; the few numbers around are community
   measurements. Isle's sizes are its own decision, not a transcription.
2. **Liquid Glass is a two-layer model**: content underneath, a "functional
   layer" of glass controls floating over it, never glass on glass, regular
   glass for anything with text, clear glass only over media with a dimming
   layer. Large glass reads thicker (deeper shadow, more refraction) than small
   glass, and small glass flips light/dark with what is behind it.
3. **27 is Tahoe made legible.** It replaced 26.1's Clear/Tinted choice with a
   continuous slider (default in the middle, offered during setup) governing
   every glass surface at once, darkened glass edges, brightened highlights,
   diffused busy backdrops more, deepened window shadows, unified window corners
   at a tighter radius, ran sidebars to the window edge, gave toolbars a real
   bar when content scrolls under them, and hid most menu-item icons. Reviewers
   broadly agree the legibility regressions are fixed; the direction itself is
   unchanged.
4. **What people disliked about 26** is a checklist for Isle: unreadable
   menus and titles over content, a menu bar "without being a place", glass
   that reads as white on non-Apple monitors, four or five corner radii on one
   desktop, a resize hit area pushed outside the window by large corners, an
   icon beside every menu item, and the grey squircle "jail" for icons of the
   wrong shape.
5. **Every glass surface has a solid form in Apple's own system**: Reduce
   Transparency gives solid sidebars, chrome and a grey menu bar; Increase
   Contrast adds borders; 27's Show Borders outlines controls; Reduce Motion
   removes the morphing. This matches P1's plan for a fallback per material.
6. **Concentricity is the corner rule**: a nested shape's radius is the
   parent's radius minus the padding, capsules are half their height, and
   near a window corner a control takes the concentric radius with a fixed
   fallback. 26 applied it so literally that window radius varied with the
   toolbar; 27 kept the rule and made the window radius uniform.
7. **The top-right corner is crowded in Apple's own design**: status items,
   Control Centre, Notification Centre from the clock, banners, and since 26
   the volume and brightness OSDs as a small capsule under the menu bar
   (segmented in 27.2 beta). B24's competition sheet has a precedent here.
8. **Notches change the menu bar**: on notched machines it is taller
   (*measured* 32–39 pt depending on model and scaled mode), and 27 added a
   native » overflow for status items hidden by the notch.
9. **Menus in 27 carry icons only where they help** (common actions, places,
   devices; all or none within a group). Isle's menus should start there, not
   at 26.
10. **Squircle-only app icons stay controversial.** 27 kept the mandate and the
    grey tile; Gruber and others argue shape aids recognition. Linux icon
    themes are mostly not squircles, so this is a live choice for Isle.

---

## 1. Liquid Glass

### What it is

Apple introduced it in June 2025 as a translucent material that reflects and
refracts what is around it, with highlights that move and a colour taken from
the content beneath, meant to put focus on content by moving controls into a
separate layer that floats over it ([newsroom, 2025-06][nr25];
[macOS Tahoe newsroom, 2025-06][nr26]).

From "Meet Liquid Glass" ([WWDC25 219][w25-219]) and the HIG
([Materials][hig-mat], [Color][hig-col]):

- **Lensing.** It bends and concentrates light at its edges to show a shape
  while staying see-through. Elements appear by building up that lensing,
  not by fading in.
- **Highlights and glow.** Specular highlights follow the shape's geometry;
  on interaction the glass glows from within and the glow spreads to nearby
  glass. It reacts strongly to touch and more subtly to a pointer
  ([HIG Motion][hig-mot]).
- **Adaptivity.** It is built from layers (highlight, shadow, tint) that each
  respond to the backdrop. Shadow strengthens over text and weakens over light
  plain backgrounds. Small elements flip between light and dark appearances;
  large ones (menus, sidebars) adapt without flipping and behave as thicker
  glass, with deeper shadow and stronger refraction.
- **Variants.** *Regular* blurs the backdrop and adjusts its luminosity; it is
  the default for system parts and for anything text-heavy (alerts, sidebars,
  popovers). *Clear* is highly translucent, meant only over rich media, and
  needs a dark dimming layer at 35 % over bright media; Apple's test is that
  it sits over media, the content can stand dimming, and what is on it is bold
  and bright.
- **Colour.** Glass has no colour of its own. Labels on it are monochrome and
  follow the backdrop. Tint only primary actions, and tint the glass rather
  than the glyph; a "prominent" button fills with the accent colour.
- **Layering rules.** Glass is for the functional layer of controls and
  navigation, never content. Sliders and toggles turn to glass only while
  being used. Never stack glass on glass; nearby glass elements go in one
  container so they sample the backdrop once and can merge and separate
  ([WWDC25 310][w25-310]).
- **The content layer** on macOS keeps the older materials and vibrancy,
  blended behind the window or within it ([Materials][hig-mat]).
- **Scroll edge effects** separate floating bars from content scrolling under
  them; they are functional, one per view, matched in height across split
  panes. *Soft* fades and blurs the content progressively; *hard* is a more
  opaque backing, used mostly on macOS behind text, background-less controls
  and pinned headers such as column headers. "Automatic" picks the harder
  style when the bar is busy ([Scroll views][hig-sv], [WWDC25 356][w25-356]).
  In 27, free-floating title text forces the hard style
  ([WWDC26 289][w26-289]).
- **Focus.** Glass changes and visually recedes when its window loses focus
  ([WWDC25 219][w25-219]).

### Settings across releases

| Release | User control over the glass |
|---|---|
| 26.0 | None beyond Reduce Transparency; "Show menu bar background" added in the betas ([MacRumors, 2025-09-24][mr26f]) |
| 26.1 (2025-11-03) | Appearance › Liquid Glass: Clear (default) or Tinted, the latter more opaque in apps and on Lock Screen notifications ([Apple, 26 updates][sup-26up]; [MacRumors, 2025-11-03][mr261]). Wikipedia calls the 26.1 control a slider; Apple and reviewers do not ([Wikipedia][wp-lg]) |
| 26.1–26.2 | Reduce Transparency stopped making sidebars, title bars and toolbars solid; fixed in 26.3 ([OSXDaily, 2026-02-13][osx-rt]) |
| 27.0 | Appearance › Liquid Glass becomes a continuous slider from "ultra-clear to fully tinted", with three marked stops, the middle being the default, offered during setup; one slider for every glass surface (Dock, widgets, sidebars, toolbars, buttons, Now Playing); needs Reduce Transparency off ([9to5Mac, 2026-09-22][9to5-slider]; [MacRumors, 2026-09-17][mr27-50]; [Tsai, 2026-09-18][tsai-gg-glass]) |

Whether the slider also reaches the menu bar and Control Centre is not stated.
Apple's Appearance help page says it adjusts "app icons", while a WWDC26 Icon
Composer lab summary says clear-mode icon glass ignores it
([Appearance][sup-app]; [WWDC26 8012][w26-8012]). One unattributed summary
says its clearest stop is less clear than the 2025 demo; not confirmed.

### What 27 changed in the material

- Darker glass edges and brighter specular highlights for depth and
  separation; busy content behind glass is diffused more; overall opacity
  changed; HDR is used for depth on capable displays
  ([MacRumors, 2026-06-09][mr27-glass]; [WWDC26 State of the Union][w26-sotu];
  [MacStories 27 p.2][ms27-2]). On HDR displays some find the edge highlights
  too bright ([Tsai, 2026-09-18][tsai-gg-glass]).
- Interactive glass bounces slightly when clicked
  ([WWDC26 289][w26-289]; [WWDC26 SwiftUI][w26-269]).
- A new accessibility option, Show Borders (§4.6).

---

## 2. Reception, 26.0 to 27

### Tahoe

- **Legibility.** Snell found the transparent menu bar oddly readable but
  without definition, "a place without being a place", and Mac glass
  half-committed, with lifeless toolbar buttons
  ([Six Colors 26 review, 2025-09][6c26]). Gruber called menus and window
  titles illegible in places ([Tsai, 2025-12-29][tsai-disbelief]); Oakley
  described light mode as a whiteout ([same][tsai-disbelief]). Troughton-Smith
  saw glass sidebars and toolbars render as plain white on non-Apple monitors;
  Grosjean said glass blurs the line between interface and content
  ([Tsai, 2025-09-15][tsai-26]). Heer found the Mac version the weaker one,
  missing the motion that makes it work on iOS ([Tsai, 2025-10-16][tsai-onlg]).
  Clover called it a work in progress, not built for the Mac
  ([MacRumors review, 2025-08-01][mr26r]).
- **Menus.** A parent menu and its open submenu swap opacity as the pointer
  moves, so the one being read loses contrast
  ([Mac Observer, 2025-10][macobs]). Prokopov argued that icons on every item
  mean none stands out, the same icon is reused for different actions, and the
  tiny icons sit off the pixel grid ([tonsky.me, 2026-01][tonsky]).
- **Corners.** Window radius depended on whether a toolbar was present and
  could change while a window was in use; Gruber called the result
  Fisher-Price-like; the large radius pushed most of the resize hit area
  outside the window (a 26.3 fix was claimed then withdrawn; *Ars only*: 26.4
  partly fixed it) ([Tsai, 2025-10-16][tsai-corners];
  [9to5Mac, 2026-02-11][9to5-263]).
- **Icons.** Kafasis, Hackett, Engst, Nielsen and Gruber criticised
  homogenised, less recognisable icons and the grey squircle for icons of
  another shape; Engst's point was that uniform shapes slow finding an app in
  the Dock ([512 Pixels, 2025-11][512-icons]; [Tsai, 2025-11-11][tsai-icons]).
- **Positive.** Voorhees liked the transparent menu bar, the translucent
  sidebars and the rounder corners, while criticising glass controls over busy
  content in Music and Photos ([MacStories 26 p.2][ms26-2]). Snell liked
  Control Centre's redesign ([Six Colors][6c26]).
- **Nielsen Norman Group** (iOS 26, not the Mac): text over images and over
  text, distracting motion, controls that appear and vanish
  ([NN/g][nng]).

### Golden Gate

- Snell: a Snow Leopard-style polish that largely fixes Tahoe's legibility;
  windows feel squarer; he set the slider just past the middle towards opaque.
  Remaining complaints: the grey squircle and an "Ask Siri" item at the top of
  every context menu ([Six Colors 27 review, 2026-09][6c27]).
- Voorhees: more cohesive and readable; the default slider position is well
  balanced; edge-to-edge sidebars are a usability win
  ([MacStories 27 p.2][ms27-2], [p.3][ms27-3]).
- Zac Hall prefers the opaque end; his only reservation is that one slider
  governs everything ([9to5Mac][9to5-slider]).
- *Ars only*: refinements that leave the Tahoe look intact, "different" more
  than "better" ([Ars 27][ars27]).
- Remaining complaints: low contrast in Mail's sidebar (TidBITS), bold
  selected sidebar text that changes width and truncates (Mayo), rough edges
  in Music, Notes and inspectors ([Tsai, 2026-09-18][tsai-gg-glass]); the glass
  window buttons too bright and pale in dark mode (Hackett,
  [512 Pixels, 2026-08][512-stop]); Gruber still wants the squircle mandate
  dropped and notes clear and tinted icon modes strip colour that helps tell
  icons apart ([Daring Fireball, 2026-07][df-squircle]).

---

## 3. Surfaces

### 3.1 Window chrome

**Anatomy and states.** A frame (title bar, window controls, optional toolbar,
rarely a bottom bar) around a body. Three states: key (active), main but not
key, and inactive. Only the key window shows coloured close, minimise and
zoom buttons; the others show them grey and drop the material, so they look
subdued ([HIG Windows][hig-win]). In 27, inactive windows dim their icons and
text, the same on Mac and iPad, and sidebar icons regain colour partly so the
key window is easier to spot ([WWDC26 SwiftUI][w26-269];
[WWDC26 State of the Union][w26-sotu]). Snell noticed inactive toolbars
reading with more contrast than active ones ([Six Colors][6c27]).

**Traffic lights.** In 27 (beta 6 onward) the three buttons became Liquid
Glass: shaded, paler than ever, and bouncing slightly when clicked
([512 Pixels, 2026-08][512-stop]; [9to5Mac, 2026-09-14][9to5-27new]). Snell
likes them; Hackett finds them too bright, especially in dark mode. No source
gives their diameter, spacing, inset, hover glyphs or exact grey in 26 or 27.

**The green button's menu.** Hovering over it opens a menu in three parts:
Move & Resize (halves of the screen; Option shows more), Fill & Arrange
(layouts of several windows: halves, quarters, mixed, plus Fill and, with
Option, Center), and Full Screen (whole screen, left or right half). The same
items are in the Window menu. Option-click restores a filled window
([tiling][sup-tile], [tiling shortcuts][sup-tileicons],
[windows][sup-win]). Dragging to an edge shows an outline of where the
window will land; tiled windows may have margins (a setting).

**Toolbars.**

- 26: items sit on glass and the toolbar appears to float over the content.
  Plain buttons placed together share one piece of glass; segmented controls,
  pop-up buttons and the search field each get their own. Non-interactive text
  gets no glass. A toolbar may flip light/dark over very bright or dark
  content ([WWDC25 310][w25-310]).
- Placement (HIG): leading edge for back/forward, the sidebar toggle and the
  title; centre for common controls, which collapse into an overflow menu as
  the window narrows; trailing edge for inspectors, search, a More menu and at
  most one tinted primary action. Aim for three groups or fewer; do not mix
  text-labelled and symbol buttons in one group; prefer borderless symbols;
  keep titles short and never the app's name ([HIG Toolbars][hig-tb]).
- Control sizes in 26: mini, small and medium are slightly taller than before
  and stay rounded rectangles; large and a new extra-large size are capsules
  ([WWDC25 310][w25-310]). Heights in points are not given.
- 27: "uniform" toolbars. When content scrolls beneath, a continuous bar
  appears behind the controls; bordered items over a sidebar use glass;
  *Ars only*: the title bar regains a hard bottom edge, and Messages fades it
  in on hover ([WWDC26 289][w26-289]; [MacStories 27 p.2][ms27-2];
  [Ars 27][ars27]). Snell: Tahoe's toolbars were flat and ghostly, these read
  better, and Photos has a real bar again ([Six Colors][6c27]). One developer
  called the new backing a macOS 15-style bar that defeats the glass
  ([Tsai, 2026-06-09][tsai-gg-sb]).

**Sidebars.**

- 26: a floating inset pane of glass with margins from the window edges;
  content runs under it, and light from colourful content spills onto it.
  Inspectors, by contrast, are edge-to-edge glass ([WWDC25 310][w25-310]).
  Readers mistook the inset sidebar for a separate window
  ([MacStories 27 p.3][ms27-3]); Snell said it looked awful.
- 27: sidebars reach the top, bottom and anchored edge, still glass with
  content passing under, on a darker background with less shadow. Icons are
  in the accent colour again (or the app's colour under Multicolor), and the
  selected row's text is semibold ([WWDC26 289][w26-289];
  [MacRumors, 2026-06-09][mr27-glass]; [Six Colors][6c27]). The user picks
  small, medium or large sidebar icons in Appearance ([Appearance][sup-app]).
  The HIG sidebars page still says sidebars can float above content
  ([HIG Sidebars][hig-sb]).

**Corners and shadows.** Apple publishes no radii.

| | Radius | Source |
|---|---|---|
| Before 26 | about 10 pt | *measured*, [Zed discussion][zed] |
| 26, toolbar windows | about 26 pt, grows with toolbar size, concentric around the glass toolbar | rule from [WWDC25 310][w25-310]; value *measured*, [Zed][zed], [MacRumors forum][mrf] |
| 26, title-bar-only windows | about 16 pt, wrapping the window controls | *measured* from an Apple video frame, [Zed][zed] |
| 27, every window | one tighter radius, between Tahoe's and Sequoia's; about 20 pt claimed from a hidden defaults key, unverified | [WWDC26 State of the Union][w26-sotu]; *Ars only*; [MacRumors forum][mrf] |

27 adds a concentric corner API that takes a minimum radius
([WWDC26 289][w26-289]). Shadows in 27 are deeper and separate overlapping
windows better, especially in dark mode ([MacStories 27 p.3][ms27-3]); no
blur, offset or opacity values are published.

**Sheets.** A card with rounded corners floating over its parent window, which
it dims; it should not cover most of the window, and other windows stay
usable. Cancel and Done are never shown together with Back (HIG change of
2026-03) ([HIG Sheets][hig-sh]). Interrupting tasks pair glass with a dimming
layer ([WWDC25 356][w25-356]). No size or radius is given.

**Alerts.** A title, optional informative text, up to three buttons, and on
macOS an optional suppression checkbox, Help button and accessory view; the
app icon shows by default, or a symbol, with the caution triangle used
sparingly; the default button is trailing in a row or on top in a stack
([HIG Alerts][hig-al]). 26 left-aligned alert text after centring it since
Big Sur, with bolder type; reactions split between relief and dislike of the
icon stacked above the text, and checkboxes looked disabled
([Tsai, 2025-06-10][tsai-26ann]); *Ars only*: the asymmetry is disorienting.
Wider buttons overlapped in three-button alerts in 26
([Late Night Software forum][lnsw]). No 27 change was found.

### 3.2 Menu bar and menus

**Size.** 24 pt in the HIG ([HIG The menu bar][hig-mb]). On notched laptops
Apple publishes nothing; third parties measure 39 pt on a 16-inch MacBook Pro
at its 2056×1329 mode and about 32 pt on a 13-inch MacBook Air M4, and the
legacy status bar thickness API still reports 22 pt
([crestnotch][crest]; [memo.d.foundation][memo-notch]). A "Large" menu bar
size exists under Accessibility › Display, after a re-login
([text size][sup-text]).

**Content.** The Apple menu leads; the app's name is bold; then the app's
menus in a fixed order ending with Window and Help; menu bar extras trail.
Extras are template images the system colours, open a menu rather than a
popover, and give way to app menus when space runs short
([HIG The menu bar][hig-mb]; [What's in the menu bar][sup-mb]). Extras are
reordered by Command-dragging.

**Material.** 26 made the bar fully transparent with a faint shadow; its text
and icons switch between white and black with the wallpaper, and the system
may dim or brighten the backdrop for contrast ([newsroom][nr26];
[Wikipedia, Tahoe][wp-tahoe]; [Six Colors][6c26]). "Show menu bar background"
(System Settings › Menu Bar, added in beta 2) restores roughly the Sequoia
look; Reduce Transparency restores the old bar and Dock
([MacRumors][mr26f]; [Eclectic Light][ecl]).

**Settings pane (26, new).** Auto-hide (always, on the desktop only, in full
screen only, never), the background switch, recent items in the Apple menu, a
per-control list where system items are on/off or "Show When Active" /
"Always Show", an "Allow in the Menu Bar" switch per third-party app, and a
reset ([Menu bar settings][sup-mbset]; [MacRumors][mr26f]). This replaced the
Control Center settings pane.

**The clock.** Always shown. Date when space allows, always or never, with an
optional weekday; digital or analogue; optional AM/PM, flashing separators and
seconds; spoken announcements on the hour, half or quarter; 24-hour time is
set in Date & Time. Clicking it opens Notification Centre
([clock][sup-clock]).

**Menus.**

- 26 put icons beside most items in menu-bar and context menus, aligned in one
  column per section ([WWDC25 310][w25-310]); the reception is under §2.
- 27 hides symbol images on menu items by default (standard items such as
  Settings, Share and Print keep theirs), apps opt back in per item, and the
  HIG (2026-06-08) says to use icons sparingly for common actions, places,
  devices and user content, and to give icons to all items in a group or none
  ([Tsai, 2026-06-09][tsai-gg-menu]; [HIG Menus][hig-menu];
  [MacRumors, 2026-06-11][mr27-menu]; [Daring Fireball, 2026-06][df-menu]).
- An "Ask Siri" item heads every context menu in 27 ([Six Colors][6c27]).
- No source gives menu corner radius, row height, insets or the shape and
  colour of the selection highlight for 26 or 27.

**27.** The whole menu bar became one window instead of one per status item,
breaking third-party bar managers ([devsreviews][devs-mb]). A native »
control appears at the edge of the status area when items would sit behind
the notch and toggles the hidden ones; it cannot be configured beyond the
Command-drag order, and the rightmost items fold last
([memo.d.foundation][memo-notch]; [MacStories 27 p.3][ms27-3]). Wired Macs show
an Ethernet icon instead of Wi-Fi ([MacStories 27 p.2][ms27-2]). Apple lists
"updated menu bar icons" without detail ([apple.com/macos][apple-macos]).
Snell calls the management light and overdue; Riley found the area
crash-prone ([Tsai, 2026-09-14][tsai-gg]).

### 3.3 Dock

**Size.** Icon size runs from 16 to 128 pt, default 64; the magnified size
from 16 to 128, default 128 (from the `tilesize` and `largesize` defaults keys,
[configuration profile reference][pc-dock]; [StillDock guide][stilldock]).

**Content.** Apps on the leading side, then a divider, the recent section (up
to three apps not already in the Dock), and files, folders, minimised windows
and the Trash; red badges; Stacks open as a fan, grid or list; dragging the
divider resizes ([Use the Dock][sup-dock]). The Dock's menu carries system
items that change with the app's state and short custom items from the app
([HIG Dock menus][hig-dockm]). Full-screen apps should let the Dock be
revealed; games may hide it ([HIG Going full screen][hig-fs]).

**Material.** 26 replaced the vibrancy backing with Liquid Glass and added
inner padding; translucent by default ([Wikipedia, Dock][wp-dock];
[AppleInsider, 2025-09-11][ai26]). With clear icons, apps in grey squircles
became hard to find. Show Borders reduces the Dock's reflections (§4.6).
Corner radius and padding are not published or measured.

**Motion.** Minimise is Genie or Scale (the `mineffect` key also takes a
hidden "suck"); "Animate opening applications" bounces the icon
([Desktop & Dock][sup-dd]; [StillDock][stilldock]). No source says either
changed in 26 or 27; auto-hide delay and duration are not stated.

**27.** A quit app with a process still running shows a light grey dot
instead of black, labelled "Running in Background" on hover, with "Stop
Running in Background" in its menu; repeated stops lead to an offer to block
it ([9to5Mac, 2026-06-09][9to5-bg]; [MacStories 27 p.13][ms27-13]). Snell
objects that the Dock should launch and monitor apps, not act as a privacy
board; Voorhees expects grey against black to confuse ([Six Colors][6c27]).
Launchpad's icon became "Apps", which opens Spotlight's Apps view (26,
[MacRumors][mr26f]).

### 3.4 App switcher (Cmd+Tab, Cmd+`)

26 gave the switcher panel a Liquid Glass background (a user request on
[alt-tab-macos #4658][alttab]; *summary* of a How-To Geek article). No source
gives icon size, panel radius, the selection highlight, Cmd+` presentation or
any 27 change.

### 3.5 Notification banners and Notification Centre

**Banners.** Always top right, not movable ([notchy][notchy]). Banners leave
after a few seconds; alerts stay until dismissed (*summary*). Hovering shows a
close button, an Options menu or the app's actions (the HIG allows up to four),
a details arrow, and expand/collapse for a stack
([notifications][sup-notif]; [HIG Notifications][hig-notif], last changed
2023, no macOS-specific guidance). In 26 they use the same reflective glass as
iOS 26 (*summary*, AppleInsider). Width, radius and timeout are unpublished.

**Notification Centre.** Opens from the date and time or a two-finger swipe
in from the trackpad's right edge; closes by clicking the desktop or the time.
Notifications from one app stack and expand on click, collapse with "Show
less", and clear singly or per stack. Widgets sit below the notifications with
"Edit Widgets" at the bottom ([get notifications][sup-nc];
[widgets][sup-widg]). Settings cover previews (always, when unlocked, never),
showing while asleep, locked or mirroring, and Apple Intelligence summaries of
long or stacked notifications ([notification settings][sup-nset]).

**27.** No visual change reported. Messages stops re-notifying what was read
on the iPhone and groups tapbacks (*summary*).

### 3.6 Widgets

**Sizes.** Small, medium, large and extra large on the Mac; 27 adds
Extra-Large Portrait, picked from a widget's Control-click menu, supported at
first by some Apple apps only ([MacRumors][mr27-50]). The HIG still lists the
portrait size for visionOS only, publishes point sizes for iPad only, gives
margins of 16 pt (11 pt tight, smaller on the Mac desktop) and asks for
container-relative corners ([HIG Widgets][hig-widg]).

**Rendering.** Full colour on the desktop and in Notification Centre, or
"vibrant" (desaturated) on the desktop. Desktop & Dock › Widgets has "Dim
widgets on desktop": automatically (monochrome while windows are in front,
full colour when the desktop is shown), always, or never; and "Show widgets" on
the desktop and in Stage Manager ([widgets][sup-widg]). Icon & widget style
(§4.4) applies to widgets too. In 27 widgets update live while their app is
open.

**Editing and gallery.** Control-click the wallpaper, Edit Widgets: a gallery
with categories on the left and widgets on the right; click to place
automatically or drag to a spot; Control-click a placed widget to edit,
resize or remove it. iPhone widgets appear with iOS 17 or later on the same
account ([widgets][sup-widg]).

### 3.7 Control Centre

**Opening.** The Control Centre item at the trailing end of the menu bar.
Sliders drag, tiles toggle, an arrow on a module drills into more options, and
some modules (Display) open a separate panel. Privacy dots beside its icon
show orange for microphone, green for camera, purple for system audio
recording, and an arrow for location; the top of the panel names the apps
using them ([Control Center][sup-cc]).

**Grid.** A rectangular grid of 32 spots; a control takes one spot, two side
by side, or four as a square or a horizontal line; Control-click changes its
size ([Macworld, 2025-07-31][mw-cc]; [MacForce][macforce]). The column and
row counts and the point sizes of spots and panel are not published.
MacStories reads a panel as about half the screen height on a Studio Display
([MacStories 26 p.3][ms26-3]).

**Customisation (26).** "Edit Controls" at the bottom shows minus badges on
controls and a gallery like the widget gallery, searchable and browsable by
app; a control can go into the panel or onto the menu bar as its own item.
Extra pages can be added, each with an icon from a set of eleven; third-party
controls were nearly missing at launch through a bug
([MacStories 26 p.3][ms26-3]; [Six Colors][6c26]). iPhone controls open
iPhone Mirroring ([MacRumors][mr26f]).

**Material.** Liquid Glass, matching iOS 26 (*summary*). Which variant, and
its open and close motion, are not documented; Wikipedia alone says 27 gave it
a consistent closing animation.

**27.** System items such as Wi-Fi and Bluetooth can leave the bar and stay
in Control Centre ([devsreviews][devs-mb]); no visual change found.

### 3.8 OSDs (volume and brightness)

26 dropped the large centred square HUD for a small oblong capsule under the
menu bar at top right, styled like a notification: a thin slider with an icon
at each end, animating in below the Control Centre item and highlighting the
matching menu bar item ([MacRumors][mr26f]; [MacStories 26 p.3][ms26-3];
[MacRumors, 2026-09-21][mr272]). It was criticised as smaller than a
notification, hard to see on light backgrounds and too brief; third-party apps
restore the old HUD ([volumeHUD][volhud]). Low battery and AirPods notices
moved into their own menu bar items ([Wikipedia, Tahoe][wp-tahoe]).

27.0 kept it. 27.2 beta 2 segments the bar (one segment per key press),
replaces the two end icons with one larger leading icon, and shows it under
the matching menu bar item; Control Centre's sliders are unchanged
([MacRumors, 2026-09-21][mr272]). Segment count, position when there is no
Sound item, size and duration are not stated.

### 3.9 Focus

An active Focus is highlighted in Control Centre's Focus module and shows its
icon in the menu bar ("Show When Active" by default); turning one off shows
a dimmed icon briefly. The module offers durations (an hour, until an event
ends). Focus filters change what Calendar, Mail, Messages and Safari show;
Apple Intelligence adds a "Reduce Interruptions" Focus; time-sensitive
notifications can break through ([Focus][sup-focus];
[set up Focus][sup-focus2]; [Menu bar settings][sup-mbset]). No 27 change
found.

### 3.10 Lock screen and login window

The large date and time came in Sonoma, with the power buttons moved into a
menu and a zoom on lock and unlock ([Wikipedia, Sonoma][wp-sonoma]). 26 adds
Clock Appearance under Wallpaper: six typefaces, a weight slider, and the
large clock on the lock screen and screen saver, the lock screen only, or
never; unlike iOS it cannot be enlarged, and its digits are translucent glass
whose transparency is not adjustable
([AppleWorld.Today, 2025-10][awt]; [iDownloadBlog, 2025-10-22][idb-lock];
[MacStories 26 beta][ms26-beta]). The lock screen uses the desktop wallpaper.
Settings cover password hints (after three wrong tries or the "?"), a message
when locked, a user list or name-and-password fields, and the Sleep, Restart
and Shut Down buttons ([Lock Screen][sup-lock]). At the login window the user
picks their name and then may use Touch ID ([Touch ID][sup-tid]).

27 adds Golden Gate wallpapers that move on unlock and double as screen savers
([9to5Mac, 2026-07-06][9to5-wall]); commenters found the default one soft
([Tsai, 2026-09-14][tsai-gg]). The iOS 26 adaptive clock that tucks behind a
photo's subject is not on the Mac ([newsroom][nr25]). Positions and sizes of
clock, avatar and password field are not published.

### 3.11 System prompts

Alerts as in §3.1. In 27 the Privacy & Security pane formerly called
Accessibility is named "Device Control and Data Access" (Jeff Johnson, via
[Tsai, 2026-09-14][tsai-gg]). The layout of the Touch ID prompt, the admin
password sheet and the privacy (TCC) prompts in 26/27 could not be
established; the one forum thread found on the Touch ID prompt's appearance
blocked fetching. For screen recording, macOS 15 settled on a monthly
re-permission prompt with an "Allow For One Month" button
([9to5Mac, 2024-08-14][9to5-scr]); no 26/27 change found.

### 3.12 Spotlight

**26.** A glass search field; four browse modes, Apps, Files, Actions and
Clipboard, on Cmd+1 to Cmd+4 and shown as round buttons beside the field.
Results list under the field; Files adds filter pills; Apps shows a row of
suggestions and then an alphabetical list switchable to a grid; Clipboard rows
have copy and delete buttons; Actions show fields to fill (a recipient, a
range) inline ([MacRumors][mr26f]; [iDownloadBlog, 2025-08-06][idb-spot]).
Quick keys are short user aliases for actions ("ft" for FaceTime) that take an
argument and are suggested after an action's first use
([actions][sup-act]). "/" filters, Tab narrows to one app
([Spotlight shortcuts][sup-spotk]). The panel can be moved and resized
([Spotlight][sup-spot]). Sources disagree on clipboard history: 8 hours
([MacRumors][mr26f]; [AppleInsider][ai26]) or 24 ([MacRumors review][mr26r]).

The Apps view replaced Launchpad: scrolling not paging, alphabetical, with
categories but no folders or custom order. People miss arranging by workflow,
and some with dyslexia or RSI found browsing icons easier than typing
([OSXDaily, 2025-09-23][osx-lp]). 27 does not bring Launchpad back
([Launchie][launchie]).

**27.** Cmd+Space opens one "Search or Ask" field on a darker background;
Return sends the text to Siri and the answer opens inside the panel, with a
button to continue in the Siri app; "Ask Siri" can be the top hit
([MacRumors][mr27-50]; [apple.com/macos][apple-macos]; [9to5Mac][9to5-siri]
*summary*). With Siri AI on, the Spotlight menu bar item and its setting go
([heise][heise]). MacStories found results nearly instant against 5–6 s on 26
([MacStories 27 beta][ms27-beta]). Panel sizes are not published.

### 3.13 Screenshot toolbar and thumbnail

Shift+Cmd+5 shows a floating bar of six capture buttons (screen, window,
selection; record screen, window, selection) and an Options menu (save
location, timer, microphone, floating thumbnail, remember last selection, HDR
or SDR, show pointer). The thumbnail appears briefly at bottom right: swipe to
save, drag into a document, click to mark up or trim ([screenshots][sup-shot]).
26 added HDR capture (HEIF stills, HEVC video) (*summary*). No restyling of
the bar in 26/27 was found; the thumbnail's duration is not published.

### 3.14 Screen sharing indicator and picker

Since macOS 14 a Video item appears in the menu bar whenever an app captures
the screen, listing each app with a live preview, letting the user change or
stop what is shared, and holding camera effects and Presenter Overlay
([WWDC23 10136][w23-scr]). The system picker lets the user choose a window, an
app or a display, and apps can limit which are offered
([SCContentSharingPicker][sccsp]). The menu bar privacy dots are in §3.7. No
26/27 change found.

### 3.15 Mission Control, Spaces, App Exposé, Split View, Stage Manager

Mission Control shows the Spaces bar along the top with desktops, full-screen
apps and Split View pairs as thumbnails, and can group windows by app
([Mission Control][sup-mc]). Stage Manager keeps the current window centred
with recent apps in a strip on the left that can hide until the pointer
reaches the edge ([Stage Manager][sup-sm]). A *summary* says 26 put glass on
the thumbnails and the Spaces bar; not traced to a primary source. 27 has
smoother Mission Control and Spaces animations by Apple's account
([MacRumors][mr27-50]; [9to5Mac][9to5-27new]), against forum reports of
dropped frames and minimised windows missing from App Exposé (*summary*). No
26/27 visual change is documented for Split View or Stage Manager.

### 3.16 System Settings

A sidebar of panes with the chosen pane on the right ([System Settings][sup-ss]).
Appearance holds, in order: Light, Dark or Auto; the Liquid Glass slider
(27); Color and Text highlight color; Icon & widget style; folder colour;
sidebar icon size; "Tint window background with wallpaper color"; scroll bar
options ([Appearance][sup-app]). 26 added the Menu Bar pane, more
personalisation, and glass sliders in some lists ([MacStories 26 p.2][ms26-2];
*Ars only*). 27 removed the progressive blur at the scroll edge and runs the
sidebar to the edge ([Tsai, 2026-06-09][tsai-gg-sb]). No source describes the
account row or search field for 26/27.

### 3.17 Finder, Open and Save panels, Quick Look

Finder: a sidebar of favourites and locations, toolbar buttons for icon, list,
column and gallery views and grouping, an optional preview pane, Quick Actions
under the gallery preview ([Finder][sup-finder]). In 26 folders follow the
accent colour and content shows faintly under the toolbar through a fade and
blur (*Ars only*). In 27 the sidebar runs to the edge with coloured icons, the
title bar has a hard edge, and column view gains autosize and reset controls
([MacRumors][mr27-50]; [Macworld][mw-gg]; [Six Colors][6c27]). Open and Save:
use the system panels, rename the Open button for the task, add an accessory
view to Save, hide extensions by default ([HIG File management][hig-fm]). No
26/27 visual change was found for the panels or Quick Look.

### 3.18 Activity Monitor, Passwords, storage

Activity Monitor's only 27 change found is its icon (*Ars only*). Passwords
gained passkey import and export in 26 (*Ars only*) and is promised automatic
weak-password fixes later in 27 ([MacRumors][mr27-50]); no interface
description found. Storage (General › Storage) offers recommendations (store
in iCloud, optimise, empty Trash automatically) and a bar coloured by
category with sizes on hover and free space in light grey
([storage][sup-stor]). *Ars only*: Apple Intelligence takes 14–22 GB there in
27 and cannot be removed.

### 3.19 Game Mode

Game Mode turns on when a game goes full screen. Since 26 a rocket item in the
menu bar, or Cmd+Esc, opens Game Overlay, where the Game Mode switch lives with
achievements, friends, brightness, volume and controllers; turning it off is
remembered per game ([Game Mode][sup-gm]; [MacRumors][mr26f]). The pre-26
menu bar icon and any notification on activation are not described by Apple.
The Games app (26) has Home, Arcade, Play Together and Library; MacRumors did
not find it very useful yet ([MacRumors review][mr26r]). No 27 change found.

---

## 4. Systems across surfaces

### 4.1 App icons

- **Grid and shape (26).** One square 1024 px canvas for iOS, iPadOS and
  macOS, masked by the system into a rounded rectangle with a rounder corner
  than before, matched to other UI and the hardware. Supply unmasked square
  layers: a background and one or more foreground layers, which the system
  renders as glass with highlights, refraction and translucency. No baked
  shadows or bevels; prefer Apple's light and dark system gradients to pure
  white or black ([HIG App icons][hig-icon]; [WWDC25 220][w25-220]).
- **Appearances.** Default, dark, clear light, clear dark, tinted light,
  tinted dark; the system derives any not supplied. Icon Composer builds them
  from layers ([HIG App icons][hig-icon]). The user picks in Appearance ›
  Icon & widget style: Default, Dark (always or auto), Clear (light, dark or
  auto), or Tinted with a chosen colour; folder colour is separate
  ([Appearance][sup-app]).
- **Legacy icons** of any other shape are drawn inside a grey squircle and lose
  their drop shadow ([Lapcat][lapcat]; [WWDC25 220][w25-220]).
- **27.** Apple's icons carry more glass layers in the artwork, with sharper
  edges, bolder colour, more defined refraction and fixed highlights at top
  and bottom; Finder was redrawn nearer its classic look
  ([MacStories, WWDC26 icons][ms-icons]; [MacRumors][mr27-50]). Icon Composer
  adds refraction controls, ring shadows, generated gradient backgrounds, HDR
  export and lower default translucency (from a summary of the WWDC26 lab
  transcript, [8012][w26-8012]). The squircle rule and the grey tile remain.
- **Reception.** §2: Gruber calls 27's icon changes subtle and better while
  wanting the mandate gone ([Daring Fireball, 2026-06-21][df-icons27]).

### 4.2 SF Symbols

Symbols are the system's icon language, drawn to match SF in nine weights
(ultralight to black) and three scales (small, medium default, large,
relative to cap height), in four rendering modes: monochrome, hierarchical
(one colour at several opacities), palette (a colour per layer) and
multicolour. Variable colour lights layers by value; SF Symbols 7 (2025) added
gradients and Draw On / Draw Off animations, alongside bounce, pulse, scale,
wiggle, breathe, rotate and Magic Replace, now the default replace
([HIG SF Symbols][hig-sfs]; [WWDC25 337][w25-337]). In 2026 the app is
distributed as `SF-Symbols-27.dmg` with over 7,000 symbols and semantic
search; no "SF Symbols 8" or WWDC26 symbols session was found
([SF Symbols][sfs]). In menus, 27 shows symbols only where they earn it
(§3.2).

### 4.3 Typography

SF Pro is one variable font with optical sizes, merging the old Text and
Display cuts, with tracking applied by size (13 pt carries slightly negative
tracking, 12 pt none). macOS text styles in points, size over leading: large
title 26/32, title 1 22/26, title 2 17/22, title 3 15/20, headline 13/16 bold,
body 13/16, callout 12/15, subheadline 11/14, footnote 10/13, captions 10/13
(caption 2 medium). Each has an emphasised weight one or two steps heavier.
Default text is 13 pt, minimum 10 pt; avoid ultralight, thin and light at
small sizes. macOS has no Dynamic Type; AppKit's named fonts (menu, menu bar,
title bar, tooltip, fixed pitch) stand in ([HIG Typography][hig-type]). SF
Mono is the monospaced member. 27 sets the selected sidebar row semibold
([WWDC26 289][w26-289]); 26 alerts use bolder type. No change to the style
sizes since 2025 was found.

### 4.4 Colour and appearance

- Light, Dark or Auto (by sunset or Night Shift schedule, switching once idle
  for a minute) ([Appearance][sup-app]).
- 26 renamed "Accent color" to "Color", defaulting to Multicolor, and added a
  separate "Text highlight color" ([MacRumors][mr26f];
  [MacStories 26 beta][ms26-beta]). An app's own accent applies only under
  Multicolor; fixed-colour sidebar icons are the exception
  ([HIG Color][hig-col]). The full list of colours on offer in 26/27 was not
  confirmed.
- "Tint window background with wallpaper color" tints toolbars and status bars
  from the wallpaper ([Appearance][sup-app]); the HIG's Dark Mode page still
  ties desktop tinting to the graphite accent, which looks stale
  ([HIG Dark Mode][hig-dark]).
- Glass colour rules are in §1.

### 4.5 Motion

The HIG asks for motion that has a purpose, is brief and precise, can be
cancelled and stays off frequent interactions, and publishes no spring values
or durations ([HIG Motion][hig-mot]). Liquid Glass motion is described in
qualities only: lensing builds up on appear, menus grow out of their button,
grouped glass merges and separates, controls flex like gel and glow on press,
and in 27 bounce on click ([WWDC25 219][w25-219]; [WWDC26 289][w26-289]).
Window minimise is Genie or Scale (§3.3). Nick Heer's complaint that the Mac
lacks the motion that makes the glass work on iOS (§2) is the main critique
of motion.

### 4.6 Accessibility

- **Contrast.** WCAG AA as checked by Accessibility Inspector: 4.5:1 for text
  up to 17 pt, 3:1 at 18 pt and above or bold; an app that falls short should
  at least meet it under Increase Contrast. The Dark Mode page asks for no
  less than 4.5:1 and 7:1 where possible with custom colours, especially small
  text; no 7:1 rule is tied to Increase Contrast
  ([HIG Accessibility][hig-a11y]; [HIG Dark Mode][hig-dark]).
- **Hit targets.** macOS controls default to 28×28 pt, minimum 20×20 pt, with
  about 12 pt around bezelled controls and 24 pt around unbezelled ones; text
  should scale to 200 % ([HIG Accessibility][hig-a11y]).
- **Reduce Transparency** replaces see-through effects with solid backgrounds:
  solid sidebars and chrome, a grey menu bar; glass becomes frostier
  ([display a11y][sup-a11y]; [WWDC25 219][w25-219]).
- **Increase Contrast** strengthens borders; glass goes to black or white with
  borders ([display a11y][sup-a11y]; [WWDC25 219][w25-219]).
- **Show Borders (27)** adds thin dark outlines or underlines to buttons,
  toolbars, sidebars and search fields and calms the Dock's reflections
  ([iDownloadBlog, 2026-09-24][idb-borders]; [MacRumors][mr27-50]). Sources
  disagree on whether it replaced "Show toolbar button shapes"; Apple's page
  as fetched still lists the older option.
- **Reduce Motion** removes Liquid Glass morphing; the HIG asks to cut
  zooming, scaling and peripheral motion, tighten springs, avoid animating
  depth or into blur, and use fades instead of movement
  ([reduce motion][sup-motion]; [HIG Accessibility][hig-a11y]).
- **Text size.** A system text size slider with per-app overrides, and a large
  menu bar option ([text size][sup-text]).

---

## 5. Rules the HIG states

Summarised from [Layout][hig-lay], [Toolbars][hig-tb], [Materials][hig-mat],
[Scroll views][hig-sv], [WWDC25 356][w25-356] and [WWDC26 289][w26-289].

- **Content first.** Content runs edge to edge and under bars and sidebars;
  controls float above it in glass. Artwork may be extended under a sidebar
  by mirroring and blurring it rather than covering it. Controls are
  separated from content by a scroll edge effect, not by a solid fill.
- **Hierarchy.** The most important things go at the top and leading edge.
  Alignment makes scanning easy; indentation shows subordination; progressive
  disclosure hides the rest. Emphasis comes from placement and grouping, not
  from extra backgrounds or borders.
- **Grouping.** Related items are grouped by spacing, a shared container shape
  or a separator. In bars, items group by function and frequency of use, with
  text buttons kept apart from symbol buttons, and few groups.
- **Concentric corners.** Three kinds of shape: fixed radius; capsule (radius
  half the height); concentric (the container's radius minus the padding
  between them). Nested shapes share a centre so corners neither pinch nor
  flare. Near a window corner a control takes the concentric radius, with a
  fixed radius as a fallback when it stands alone; optical centring may
  override the mathematical one.
- **Spacing and margins.** Use the system's safe areas, layout guides and
  margins; keep critical controls away from the bottom of windows and
  sidebars and away from window corners (26 adds corner-avoiding layout
  regions); keep content out from behind the camera housing.
- **Glass discipline.** Glass only in the functional layer, sparingly on
  custom controls, never stacked; regular for text, clear only over media with
  dimming; tint only the primary action.
- **Menus.** Icons sparingly and consistently, all or none within a group
  (2026 change).
- **Accessibility first.** Every material and motion has a form under Reduce
  Transparency, Increase Contrast and Reduce Motion, which the system applies
  to standard components automatically.

---

## Sources

All read 2026-10-03. Publication dates are given where the source has one.

| Key | Source | Published |
|---|---|---|
| nr25, nr26 | Apple newsroom, new design; macOS Tahoe 26 | 2025-06-09 |
| apple-macos | apple.com/macos | current |
| w25-219, 220, 310, 337, 356 | WWDC25: Meet Liquid Glass; new look of app icons; AppKit with the new design; SF Symbols 7; new design system | 2025-06 |
| w26-sotu, 269, 289, 8012 | WWDC26: Platforms State of the Union; SwiftUI; Modernize your AppKit app; Icon Composer lab | 2026-06 |
| w23-scr | WWDC23 10136, ScreenCaptureKit | 2023-06 |
| hig-* | Human Interface Guidelines pages, change logs as noted in text | 2023–2026 |
| sup-* | Apple support, macOS User Guide (27 edition) and release notes | current |
| 6c26, 6c27 | Six Colors reviews of 26 and 27 | 2025-09, 2026-09 |
| ms26-*, ms27-*, ms-icons | MacStories reviews and previews | 2025–2026 |
| mr26r, mr26f, mr261, mr27-* , mr272 | MacRumors | 2025-08 to 2026-09 |
| 9to5-* | 9to5Mac | 2024-08 to 2026-09 |
| ars27 | Ars Technica, macOS 27 review (one pass only) | 2026-09 |
| 512-* | 512 Pixels | 2025-11, 2026-08 |
| df-* | Daring Fireball | 2026-06, 2026-07 |
| tsai-* | Michael Tsai's blog, collected reactions | 2025-06 to 2026-09 |
| others | as linked: AppleInsider, Macworld, iDownloadBlog, OSXDaily, Eclectic Light, heise, Mac Observer, NN/g, tonsky.me, Lapcat, Wikipedia, crestnotch, memo.d.foundation, devsreviews, notchy, MacForce, StillDock, Launchie, Zed, GitHub | as linked |

[nr25]: https://www.apple.com/newsroom/2025/06/apple-introduces-a-delightful-and-elegant-new-software-design/
[nr26]: https://www.apple.com/newsroom/2025/06/macos-tahoe-26-makes-the-mac-more-capable-productive-and-intelligent-than-ever/
[apple-macos]: https://www.apple.com/os/macos/
[w25-219]: https://developer.apple.com/videos/play/wwdc2025/219/
[w25-220]: https://developer.apple.com/videos/play/wwdc2025/220/
[w25-310]: https://developer.apple.com/videos/play/wwdc2025/310/
[w25-337]: https://developer.apple.com/videos/play/wwdc2025/337/
[w25-356]: https://developer.apple.com/videos/play/wwdc2025/356/
[w26-sotu]: https://developer.apple.com/videos/play/wwdc2026/102/
[w26-269]: https://developer.apple.com/videos/play/wwdc2026/269/
[w26-289]: https://developer.apple.com/videos/play/wwdc2026/289/
[w26-8012]: https://developer.apple.com/videos/play/wwdc2026/8012/
[w23-scr]: https://developer.apple.com/videos/play/wwdc2023/10136/
[sccsp]: https://developer.apple.com/documentation/screencapturekit/sccontentsharingpicker
[sfs]: https://developer.apple.com/sf-symbols/
[hig-mat]: https://developer.apple.com/design/human-interface-guidelines/materials
[hig-col]: https://developer.apple.com/design/human-interface-guidelines/color
[hig-mot]: https://developer.apple.com/design/human-interface-guidelines/motion
[hig-sv]: https://developer.apple.com/design/human-interface-guidelines/scroll-views
[hig-win]: https://developer.apple.com/design/human-interface-guidelines/windows
[hig-tb]: https://developer.apple.com/design/human-interface-guidelines/toolbars
[hig-sb]: https://developer.apple.com/design/human-interface-guidelines/sidebars
[hig-sh]: https://developer.apple.com/design/human-interface-guidelines/sheets
[hig-al]: https://developer.apple.com/design/human-interface-guidelines/alerts
[hig-mb]: https://developer.apple.com/design/human-interface-guidelines/the-menu-bar
[hig-menu]: https://developer.apple.com/design/human-interface-guidelines/menus
[hig-dockm]: https://developer.apple.com/design/human-interface-guidelines/dock-menus
[hig-fs]: https://developer.apple.com/design/human-interface-guidelines/going-full-screen
[hig-notif]: https://developer.apple.com/design/human-interface-guidelines/notifications
[hig-widg]: https://developer.apple.com/design/human-interface-guidelines/widgets
[hig-fm]: https://developer.apple.com/design/human-interface-guidelines/file-management
[hig-icon]: https://developer.apple.com/design/human-interface-guidelines/app-icons
[hig-sfs]: https://developer.apple.com/design/human-interface-guidelines/sf-symbols
[hig-type]: https://developer.apple.com/design/human-interface-guidelines/typography
[hig-dark]: https://developer.apple.com/design/human-interface-guidelines/dark-mode
[hig-a11y]: https://developer.apple.com/design/human-interface-guidelines/accessibility
[hig-lay]: https://developer.apple.com/design/human-interface-guidelines/layout
[sup-26up]: https://support.apple.com/en-us/122868
[sup-app]: https://support.apple.com/guide/mac-help/change-appearance-settings-mchlp1225/mac
[sup-tile]: https://support.apple.com/guide/mac-help/tile-app-windows-mchlef287e5d/mac
[sup-tileicons]: https://support.apple.com/guide/mac-help/mac-window-tiling-icons-keyboard-shortcuts-mchl9674d0b0/mac
[sup-win]: https://support.apple.com/guide/mac-help/work-with-app-windows-mchlp2469/mac
[sup-mb]: https://support.apple.com/guide/mac-help/whats-in-the-menu-bar-mchlp1446/mac
[sup-mbset]: https://support.apple.com/guide/mac-help/mchlad96d366/mac
[sup-clock]: https://support.apple.com/guide/mac-help/change-the-menu-bar-clock-format-on-mac-mchlb3236e90/mac
[sup-text]: https://support.apple.com/guide/mac-help/make-text-and-icons-bigger-mchld786f2cd/mac
[sup-dock]: https://support.apple.com/guide/mac-help/use-the-dock-mh35859/mac
[sup-dd]: https://support.apple.com/guide/mac-help/change-desktop-dock-settings-mchlp1119/mac
[sup-notif]: https://support.apple.com/guide/mac-help/mh40609/mac
[sup-nc]: https://support.apple.com/guide/mac-help/get-notifications-mchl2fb1258f/mac
[sup-nset]: https://support.apple.com/guide/mac-help/change-notifications-settings-on-mac-mchl205da693/mac
[sup-widg]: https://support.apple.com/guide/mac-help/mchl52be5da5/mac
[sup-cc]: https://support.apple.com/guide/mac-help/quickly-change-settings-mchl50f94f8f/mac
[sup-focus]: https://support.apple.com/guide/mac-help/mchl999b7c1a/mac
[sup-focus2]: https://support.apple.com/guide/mac-help/set-up-a-focus-to-stay-on-task-mchl613dc43f/mac
[sup-lock]: https://support.apple.com/guide/mac-help/change-lock-screen-settings-on-mac-mh11784/mac
[sup-tid]: https://support.apple.com/guide/mac-help/use-touch-id-mchl16fbf90a/mac
[sup-act]: https://support.apple.com/guide/mac-help/take-actions-and-shortcuts-in-spotlight-mchl4953dfeb/mac
[sup-spotk]: https://support.apple.com/guide/mac-help/spotlight-keyboard-shortcuts-mh26783/mac
[sup-spot]: https://support.apple.com/guide/mac-help/search-with-spotlight-mchlp1008/mac
[sup-shot]: https://support.apple.com/guide/mac-help/take-a-screenshot-mh26782/mac
[sup-mc]: https://support.apple.com/guide/mac-help/view-open-windows-spaces-mission-control-mh35798/mac
[sup-sm]: https://support.apple.com/guide/mac-help/use-stage-manager-mchl534ba392/mac
[sup-ss]: https://support.apple.com/guide/mac-help/change-system-settings-mh15217/mac
[sup-finder]: https://support.apple.com/guide/mac-help/organize-your-files-in-the-finder-mchle9f0a1b2/mac
[sup-stor]: https://support.apple.com/guide/mac-help/optimize-storage-space-sysp4ee93ca4/mac
[sup-gm]: https://support.apple.com/en-us/105118
[sup-a11y]: https://support.apple.com/guide/mac-help/change-display-settings-for-accessibility-unac089/mac
[sup-motion]: https://support.apple.com/guide/mac-help/reduce-screen-motion-mchlc03f57a1/mac
[6c26]: https://sixcolors.com/post/2025/09/macos-26-tahoe-review-power-under-glass/
[6c27]: https://sixcolors.com/post/2026/09/macos-27-golden-gate-review-bridging-the-tahoe-gap/
[ms26-2]: https://www.macstories.net/stories/macos-26-tahoe-the-macstories-review/2/
[ms26-3]: https://www.macstories.net/stories/macos-26-tahoe-the-macstories-review/3/
[ms26-beta]: https://www.macstories.net/stories/macos-tahoe-the-macstories-public-beta-preview/
[ms27-2]: https://www.macstories.net/stories/macos-27-the-macstories-review/2/
[ms27-3]: https://www.macstories.net/stories/macos-27-the-macstories-review/3/
[ms27-13]: https://www.macstories.net/stories/macos-27-the-macstories-review/13/
[ms27-beta]: https://www.macstories.net/stories/macos-27-golden-gate-the-macstories-public-beta-preview/
[ms-icons]: https://www.macstories.net/linked/wwdc-2026-macos-27-icon-refinements/
[mr26r]: https://www.macrumors.com/2025/08/01/macos-tahoe-review/
[mr26f]: https://www.macrumors.com/2025/09/24/all-the-new-macos-tahoe-features/
[mr261]: https://www.macrumors.com/2025/11/03/apple-releases-macos-tahoe-26-1/
[mr27-glass]: https://www.macrumors.com/2026/06/09/macos-golden-gate-liquid-glass/
[mr27-menu]: https://www.macrumors.com/2026/06/11/macos-27-golden-gate-menu-items/
[mr27-50]: https://www.macrumors.com/2026/09/17/new-features-changes-macos-27/
[mr272]: https://www.macrumors.com/2026/09/21/macos-27-2-slider-design/
[9to5-slider]: https://9to5mac.com/2026/09/22/macos-27-gives-you-more-control-over-liquid-glass/
[9to5-27new]: https://9to5mac.com/2026/09/14/macos-27-golden-gate-now-available-here-is-everything-new/
[9to5-bg]: https://9to5mac.com/2026/06/09/macos-27-golden-gate-makes-it-clear-when-apps-are-sneakily-running-in-background/
[9to5-263]: https://9to5mac.com/2026/02/11/macos-tahoe-26-3-fixes-two-annoying-design-problems/
[9to5-wall]: https://9to5mac.com/2026/07/06/macos-27-golden-gate-adds-these-new-wallpapers-and-screen-savers-to-your-mac/
[9to5-siri]: https://9to5mac.com/2026/09/17/macos-27-golden-gate-siri-ai-top-features-apples-assistant-finally-comes-to-life-video/
[9to5-scr]: https://9to5mac.com/2024/08/14/macos-sequoia-screen-recording-prompt-monthly/
[ars27]: https://arstechnica.com/gadgets/2026/09/macos-27-golden-gate-the-ars-technica-review/
[512-icons]: https://512pixels.net/2025/11/on-tahoes-terrible-icons/
[512-stop]: https://512pixels.net/2026/08/golden-gate-beta-6-stoplights-liquid-glass/
[df-squircle]: https://daringfireball.net/2026/07/eliminate_app_icon_squircle_jail
[df-menu]: https://daringfireball.net/2026/06/macos_27_golden_gate_removes_the_dumb_icons_from_menu_items
[df-icons27]: https://daringfireball.net/linked/2026/06/21/basic-apple-guy-macos-27-icons
[tsai-26ann]: https://mjtsai.com/blog/2025/06/10/macos-tahoe-26-announced/
[tsai-26]: https://mjtsai.com/blog/2025/09/15/macos-tahoe-26/
[tsai-corners]: https://mjtsai.com/blog/2025/10/16/tahoe-window-corners/
[tsai-onlg]: https://mjtsai.com/blog/2025/10/16/on-liquid-glass/
[tsai-icons]: https://mjtsai.com/blog/2025/11/11/tahoes-terrible-icons/
[tsai-disbelief]: https://mjtsai.com/blog/2025/12/29/liquid-glass-disbelief/
[tsai-gg-sb]: https://mjtsai.com/blog/2026/06/09/golden-gate-sidebars-and-toolbars/
[tsai-gg-menu]: https://mjtsai.com/blog/2026/06/09/golden-gate-menu-icons/
[tsai-gg]: https://mjtsai.com/blog/2026/09/14/macos-27-golden-gate/
[tsai-gg-glass]: https://mjtsai.com/blog/2026/09/18/golden-gate-liquid-glass/
[tonsky]: https://tonsky.me/blog/tahoe-icons/
[nng]: https://www.nngroup.com/articles/liquid-glass/
[macobs]: https://www.macobserver.com/news/users-say-liquid-glass-breaks-menu-focus-in-macos-tahoe/
[osx-rt]: https://osxdaily.com/2026/02/13/reduce-transparency-works-again-in-macos-tahoe-26-3/
[osx-lp]: https://osxdaily.com/2025/09/23/miss-launchpad-macos-tahoe-solutions/
[ecl]: https://eclecticlight.co/2025/09/15/appearance-matters-get-tahoe-looking-in-better-shape/
[wp-lg]: https://en.wikipedia.org/wiki/Liquid_Glass
[wp-tahoe]: https://en.wikipedia.org/wiki/MacOS_Tahoe
[wp-sonoma]: https://en.wikipedia.org/wiki/MacOS_Sonoma
[wp-dock]: https://en.wikipedia.org/wiki/Dock_(macOS)
[ai26]: https://appleinsider.com/articles/25/09/11/macos-tahoe-review----a-strong-update-just-not-for-the-reasons-you-expect
[mw-cc]: https://www.macworld.com/article/2859555/how-to-take-full-control-of-your-control-center-in-macos-tahoe.html
[mw-gg]: https://www.macworld.com/article/3161244/macos-golden-gate-in-pictures-5-design-upgrades-coming-to-your-mac.html
[macforce]: https://www.macforce.com/blog/control-center-is-improved-and-highly-customizable-in-macos-26-tahoe
[idb-spot]: https://www.idownloadblog.com/2025/08/06/how-to-use-new-spotlight-search-mac/
[idb-lock]: https://www.idownloadblog.com/2025/10/22/tips-to-customize-mac-lock-screen/
[idb-borders]: https://www.idownloadblog.com/2026/09/24/show-borders-ios-macos/
[awt]: https://appleworld.today/2025/10/macos-tahoe-adds-the-ability-to-change-your-macs-clock-screen-font/
[heise]: https://www.heise.de/en/news/macOS-27-Spotlight-search-removed-from-menu-bar-Siri-AI-button-no-substitute-11458298.html
[launchie]: https://www.launchie.app/blog/where-is-launchpad-on-mac
[lapcat]: https://lapcatsoftware.com/articles/2025/6/2.html
[crest]: https://crestnotch.app/macbook-notch-dimensions
[memo-notch]: https://memo.d.foundation/macbook-notch-macos-27
[devs-mb]: https://devsreviews.com/reviews/macos-27-menu-bar-managers/
[notchy]: https://notchy.dev/mac-notification-banner-position/
[volhud]: https://github.com/dannystewart/volumeHUD
[alttab]: https://github.com/lwouis/alt-tab-macos/issues/4658
[pc-dock]: https://github.com/ProfileCreator/Configuration-Profile-Reference/blob/master/markdown/Dock%20Payload.md
[stilldock]: https://stilldock.com/guides/mac-dock-defaults-commands
[zed]: https://github.com/zed-industries/zed/discussions/38233
[mrf]: https://forums.macrumors.com/threads/want-the-bigger-window-corner-radius-back-come-inside.2484898/
[lnsw]: https://forum.latenightsw.com/t/dialog-toolkit-in-macos-tahoe-buttons-overlap/5594

---

## Open

- **Dimensions Apple does not publish**: traffic-light diameter, spacing and
  inset; title-bar and toolbar heights; glass capsule heights; window corner
  radii in 26 and 27 (only community figures); window shadow blur, offset and
  opacity; sheet and alert sizes and radii; menu corner radius, row height,
  insets and highlight shape; status item size and spacing; Dock radius,
  padding and indicator geometry; Cmd+Tab icon size, panel radius and
  highlight; banner width, radius and timeout; Control Centre panel width,
  spot size and grid columns; OSD capsule size, position and duration; Mac
  widget sizes in points; Spotlight panel size; lock screen layout.
- **Menu bar height on notched Macs**: third-party figures of 32 and 39 pt
  depend on model and scaled mode; no Apple figure.
- **Control Centre grid**: 32 spots per Macworld, eight empty slots in edit
  mode per MacForce; how these relate (per page or not) is unclear. Its glass
  variant and open/close motion are undocumented.
- **The 27 slider's reach**: whether it covers the menu bar and Control
  Centre; whether it affects app icons (Apple's help page and the WWDC26 lab
  summary conflict); whether its clearest stop matches the 2025 glass; the
  medium default and the setup prompt rest on 9to5Mac alone.
- **26.1's control**: a two-way choice per Apple and MacRumors, a slider per
  Wikipedia.
- **Show Borders**: whether it replaced "Show toolbar button shapes" (sources
  disagree) and its exact place in Settings.
- **Motion values**: no spring parameters or durations for anything, including
  minimise, Dock bounce, auto-hide, Control Centre and banners. Whether Genie
  or Scale changed in 26/27.
- **Prompts**: the 26/27 Touch ID prompt, admin authentication sheet and
  privacy prompt layouts; any 27 alert change.
- **Not seen first-hand**: glass on Mission Control thumbnails and the Spaces
  bar in 26; Stage Manager and Split View in 26/27; Quick Look and Open/Save
  panels in 26/27; the Game Mode icon before 26 and any activation notice;
  27's Siri answer card in Spotlight; the screenshot thumbnail's duration.
- **Clipboard history length** in Spotlight: 8 hours or 24.
- **Accent colour list** in 26/27 under the renamed "Color" setting.
- **SF Symbols naming**: whether the 2026 release is "SF Symbols 8" or now
  year-numbered.
- **Reviews not read**: The Verge (unreachable); Ars Technica read in one pass
  only, its claims marked; Riccardo Mori's essays seen only as summaries;
  Apple's June 2026 newsroom announcement of 27 was not found.
- **Wikipedia-only claims**, left out of the body: the pointing-hand cursor
  returning to its earlier style in 27.
