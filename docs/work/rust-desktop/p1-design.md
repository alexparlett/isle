# P1 · Design

Planned 2026-10-03. Not started.

The design process for the new Isle (B3, B17). It answers every
presentation question the intent docs leave open (each doc's "For the
design process" list, and B24) and produces `DESIGN.md` and the first
`tokens.kdl`. It runs in six stages, each ending at a gate where the user
decides; nothing moves to the next stage until the gate is passed.

Runs beside P2 and P3 and blocks neither. Building packets take its output
in waves (stage 5): a surface whose design is not through its gate is built
as structure only (roadmap, P1).

## What it answers to

- Decisions: B3 (fresh design), B7 (compositor-driven open and close, Skia
  on the compositor's context), B10 (tokens as data, live), B17–B21 (the
  macOS model), B24 (attention), B25 (window chrome), B30 (Focus, Game Mode).
- Intent: the "For the design process" lists in `docs/design/shell.md`,
  `apps.md`, `compositor.md`, `gaming.md` and `platform.md`. Each item is
  answered once, in `DESIGN.md`, and the list item is replaced with a
  pointer to the answer.
- Evidence: `isle-v1-inventory.md` (§4 surfaces, §14 v1's design
  language, §16 lessons), `macos-model.md` §3 (macOS 27 behaviours
  against v1), `shell-ui-spike.md` (what Freya draws cheaply).
- v1's `DESIGN.md`, `docs/canvas` and `docs/screenshots` on `main`, as
  reference only (B3).

## How it is made

- **Mock-ups in HTML, then Freya.** Stages 3 to 5 are drawn as HTML and
  CSS pages on a shared harness. Once P3's preview mode exists, the
  surfaces still to be designed are prototyped in Freya instead, and the
  ones already drawn in HTML are rebuilt there before their building packet
  starts, so the motion and the materials are judged as Skia draws them.
- **The harness** (`p1/harness/`): a 3440×1440 stage scaled to fit the
  window, with a 1920×1080 second screen beside it; a wallpaper picker;
  light and dark; reduced motion, reduced transparency and increased
  contrast; a state picker per surface; an actual-size view. It reads
  `tokens.kdl` with a small reader of its own and maps tokens to CSS
  custom properties, reloading when the file changes, so mock-ups use the
  token file the shell will read. `harness/serve.py` serves the boards
  locally in the same wrapper the Artifact host adds. Motion runs on a small JS spring with the same parameters the
  compositor will take, not on CSS easing.
- **Materials are approximated.** CSS `backdrop-filter` stands in for the
  compositor's blur. Each material is specified in terms the compositor
  can draw (blur radius, passes, saturation, tint, noise), and any that
  costs too much at 3440×1440 is cut when P3 or P4 measures it.
- **Fonts and images are free to ship.** Typefaces under OFL or similar,
  since Isle installs them; wallpapers the repo may carry. No SF Pro or
  Apple artwork in the mock-ups.
- **Where things go.** Working files in `docs/work/rust-desktop/p1/`
  (committed); research in `docs/research/rust-desktop/design-*.md`; the
  record in `docs/DESIGN.md` and `tokens.kdl`. Each gate's board is also
  published as a private Artifact, so it can be reviewed away from the
  desk; the repo copy is the source.
- **The user decides at every gate**, in chat, from the board. Questions
  are asked as choices with a recommendation, never as an open "what do
  you think".

## Stages

### 1. Research

What macOS does today, what others do better, what v1 learned, and what
the user wants. Four notes, then a brief.

- **1a · macOS 27.** Per surface in the structure list (stage 2): what
  macOS 27 shows, its states and sizes, its materials and motion, the
  HIG's rules for it, what changed in 26 and 27 (Liquid Glass). From
  Apple's HIG and support pages, and reviews with screenshots. Into
  `design-macos.md`.
- **1b · Others worth learning from.** GNOME 49, KDE Plasma 6, COSMIC,
  elementary, Otto, Windows 11, and the macOS menu-bar tools (Ice, iStat
  Menus, Raycast, Rectangle). Only what one of them does better than macOS
  on a named surface, with why. Into `design-references.md`.
- **1c · v1's lessons.** What v1's design got right and wrong in use: its
  principles, the island's states and timings, the dashboard, the
  materials in game mode, the decisions the user reversed (v1 DECISIONS on
  `main`), and §14's values as a point of comparison. Into
  `design-v1-lessons.md`.
- **1d · The user.** An interview in short rounds of choices: daily use
  (gaming and coding on the 3440×1440 ultrawide, the second screen, the
  KVM), apps lived in, light or dark or both, density, how much glass,
  colour (one accent or several, from the wallpaper or fixed), type,
  motion appetite, what about macOS they want and what they don't, what
  they miss from v1. Reference images they like are welcome. Into
  `p1/brief.md`.

**Gate 1:** the brief. `p1/brief.md` states the principles to design to,
the user's answers, and what Isle takes from macOS as is, changes, or
leaves out. The user accepts or edits it.

### 2. Structure

What every surface is, before what it looks like. One sheet per surface in
`p1/structure/`, each with: what it is for; its content; its states (empty,
loading, error, full, overflow, many screens, game in front, locked);
how it opens and closes; its keyboard map; what it does on the second
screen; and the answers to its "For the design process" items, with the
proposal adopted, changed or replaced. Wireframes in the harness in grey,
no tokens.

The surfaces, in build order:

- **Window chrome**: server-side title bars, the zoom menu and the zone
  preview, the window menu; the unified toolbar for Isle's apps; sheets.
- **Menu bar** with the system menu, app menus, fallback app menu, menu
  search, status items, overflow and the clock; the **island**; the
  **wallpaper**.
- **App switcher** (Cmd+Tab, Cmd+`); launching without a Dock (revision 1).
- **Notification banners**, **OSDs**, **Control Centre** and its modules;
  **Focus** and **Game Mode** state.
- **Lock** and the **greeter**; **system prompts** (polkit, keyring,
  network, Bluetooth pairing); idle dim.
- **Spotlight** with the Apps view and the prefixes.
- **Capture** toolbar, thumbnail and the screen-share **picker**.
- **App conventions** and **Settings**.
- **Files** with Open, Save and Quick Look; **Mission Control**, Spaces,
  App Exposé, Split View, Stage Manager, hot corners; **Notification
  Centre** and **widgets** with the editor and the gallery; **Monitor**,
  **Keychain**, **Cleaner**; the **dropdown terminal**,
  **picture-in-picture**, the **on-screen keyboard**, clipboard history,
  emoji; crash notices, the safe start, a rejected hand edit.

Two cross-cutting sheets:

- **Competition.** What wins each shared place: the top edge (menu bar,
  island, banners, OSDs, the game-mode reveal), the island's queue
  (privacy first, then the order B24 proposes or a better one), the right
  edge (banners against Notification Centre), and keyboard focus between
  surfaces. One table, every pair.
- **Copy.** The rules for words (sentence case, "desktop" not
  "workspace", how buttons are named, how errors read), with the
  system's recurring strings.

**Gate 2:** the structure, in three batches so the early surfaces are not
held up: chrome, menu bar, island, switcher and wallpaper with the competition
sheet; then the rest of milestone M1's surfaces; then the parity surfaces.

### 3. Directions

Two or three visual directions that differ in kind, not in accent colour:
for example how much glass against how much solid, density, the
character of the type, how loud motion is. Each is a draft `tokens.kdl`
with any CSS it cannot express, over the same harness, drawing the same
sample:

- the island at rest and showing media and a recording, and the menu bar
  revealed;
- the switcher with a badged, an attention and a minimised entry;
- Control Centre open with a module drilled into;
- a notification banner over a full-screen window;
- a Settings window and a third-party window with Isle's title bar, side
  by side;
- the island's morph and Control Centre's open, animated;

each on two wallpapers (one light, one dark), in light and dark if the
brief asks for both, and on the ultrawide and the 1080p screen.

**Gate 3:** the user picks a direction, or names what to take from each.
If combining leaves something unresolved, a merged board is drawn and the
gate is repeated once.

### 4. System

The chosen direction made into a system. Into `tokens.kdl` and a
component board in `p1/system/`.

- **Principles**, the few rules the rest follows (v1 had five).
- **Tokens:** colour by role, light and dark, with increased contrast;
  materials (each with its blur, tint, saturation, noise, its fallback
  with reduced transparency, and its form in game mode); typography
  (families, scale, weights, figures, a text-scale factor); spacing;
  radii, with the rule for nested radii; strokes; shadows and depth;
  motion (springs as stiffness and damping, the durations for anything
  not on a spring, and what reduced motion turns each into); icon sizes.
- **The token schema.** The KDL node names and shape are fixed here and
  handed to P3, which reads them. Until then P3 works from a stub.
- **Icons and cursor:** the symbol set for the shell (Lucide, Phosphor,
  Material Symbols or another, chosen on the board), the app icon theme
  and how app icons sit in the switcher and the Apps grid, and the cursor theme.
- **Components** shared by the shell and the apps: buttons, toggles,
  sliders, segmented controls, steppers, text and search fields, pop-up
  buttons, menus and their items (checked, radio, disabled, icon,
  submenu), popovers, sheets, alerts, tooltips, list and grouped rows, a
  sidebar, a toolbar with the traffic lights, tabs, badges, progress, a
  notification card, a Control Centre module. Each with hover, pressed,
  focus, disabled and selected, and its keyboard behaviour.
- **Installed apps:** which tokens feed GTK, Qt, the terminal and the
  icon and cursor themes. P14 writes the templates.

**Gate 4:** the system board and `tokens.kdl`. `DESIGN.md` gets its first
draft from this stage: principles, tokens, components.

### 5. Surfaces

Each surface designed in the chosen direction from its structure sheet:
every state from the sheet, its motion (what the compositor animates and
with which spring), its keyboard map and its copy, on both screens and in
both themes. Its answers go into `DESIGN.md`.

In waves that stay ahead of the building packets:

- **Wave A** (P4, P6, P7): window chrome, menu bar and menus, island,
  wallpaper, switcher.
- **Wave B** (P8 to P13): banners, OSDs, Control Centre, Focus and Game
  Mode state, lock, greeter, prompts, idle dim, Spotlight, capture and the
  picker, Settings.
- **Wave C** (P15 to P20): Files and its panels, Mission Control and the
  rest of the window model, Notification Centre and widgets, Monitor,
  Keychain, Cleaner, and the remaining v1 surfaces.

Wave A is drawn in HTML. From P3's preview mode on, surfaces are
prototyped in Freya with fixture data, and the HTML ones are rebuilt there
before their packet starts. A Freya prototype is written as the surface's
view, so its building packet connects it to the real services rather than
redrawing it.

**Gate 5:** one per wave; within a wave, a surface the user is unsure of
can be held back without holding the rest.

### 6. Review

Per wave, before its gate, and once over the whole at the end.

- **Critique:** each surface against the brief's principles and its
  structure sheet; consistency across surfaces (the same thing looks and
  behaves the same everywhere).
- **Accessibility:** contrast checked by a script over every text and
  background pair in `tokens.kdl`, light, dark and increased contrast,
  against WCAG AA; a keyboard map for every surface with nothing reached
  only by pointer; a reduced-motion form for every animation and a
  reduced-transparency form for every material; a text-scale check at the
  largest setting; focus visible everywhere.
- **Copy:** every string against the copy sheet.

**Gate 6:** the user signs off `DESIGN.md` as the record. Design choices
that are decisions rather than looks (the island's order, where banners
sit, where widgets live) get B entries in `DECISIONS.md`, and each intent
doc's "For the design process" list points to its answers.

## Output

- `docs/DESIGN.md`: principles, tokens, components, and each surface's
  look, states and motion.
- `tokens.kdl`: the first token file, at the path P2's workspace layout
  gives it (drafted in `p1/system/` until then).
- `docs/research/rust-desktop/design-macos.md`, `design-references.md`,
  `design-v1-lessons.md`.
- `docs/work/rust-desktop/p1/`: `brief.md`, `structure/`, `directions/`,
  `system/`, `surfaces/`, `harness/`. Working files; nothing builds on
  them except the Freya prototypes, which move into the shell when their
  packet starts.
- New B entries in `DECISIONS.md`; the intent docs' design lists answered.

## Depends on, and feeds

- Starts now; needs nothing else to be built.
- P3 takes the token schema from stage 4 and provides the preview mode
  stage 5 moves to. The Freya fork at rc.8 holds up P3, and so the move to
  Freya, not P1.
- P4, P6 and P7 wait for wave A's structure; they may start on
  structure only if its look is late. The same holds for later waves.
- P14 takes the installed-apps mapping from stage 4.

## Done when

- Gates 1 to 6 are passed and `DESIGN.md` is signed off.
- Every item in the five "For the design process" lists has an answer in
  `DESIGN.md`.
- `tokens.kdl` passes the contrast script in every theme.
- Every surface has a keyboard map, a reduced-motion form and its copy.

## Risks

- **macOS 27 is weeks old.** Its HIG and reviews may be thin on details;
  where they are, the note says so and the design decides rather than
  guessing at Apple's.
- **Glass costs.** A blurred material over a 3440×1440 output may not fit
  the frame budget beside a game. The system defines a solid fallback for
  every material from the start, so cutting blur changes values, not
  layouts.
- **HTML lies about Skia.** Text rendering, blur and spring feel differ.
  Rebuilding in Freya before each packet is the check; the gate for a
  surface rebuilt in Freya is repeated if it reads differently.
- **Scope.** Twenty-odd surfaces and five apps. The waves keep the
  building packets fed in order, and wave C can run beside M2.
