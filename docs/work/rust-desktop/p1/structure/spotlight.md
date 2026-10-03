# Spotlight

Batch 2. Answers to: `shell.md` § For the design process (Spotlight; Shell
runtime, Spotlight's screen) and its Spotlight 1–11 with the open
assistant item; `shell.md` No Dock 1, 3, 5, 7, 8; `gaming.md` § For the
design process (Games as apps, Spotlight's part, which `dock.md` pointed
here); `platform.md` § For the design process (Notifications and badges,
the Apps view's tiles) and Clipboard history, emoji and symbols; the items
`dock.md` and `menu-bar.md` left to this sheet. B17, B19, B20, B21, B26,
B27, B30.

## Purpose

macOS Spotlight on Cmd+Space: one field that finds something and does it.
It finds apps, shell actions, Settings rows, open windows, the front app's
menu items, files and their contents, sums and conversions, emoji,
clipboard history and the vault, with the web as the last resort, and it
carries every v1 launcher prefix. With no Dock (`dock.md`) it is the only
launcher: its Apps view is the grid that replaced Launchpad, with the
user's favourites at the top. It is not a window picker for every app
(Mission Control and App Exposé are), not a file browser (Files is), not
where running apps are seen (the switcher is), and not a chat window (For
the user 1).

## Content

Spotlight is two parts on one axis: the **field**, a single bar, and under
it the **results**, a panel that appears only when there is something to
show (macOS 26's form, `design-macos.md` § 3.12). Both are centred on the
screen; the field's place never changes while Spotlight is open, and
everything below it grows downward from it (brief P5).

**The field**

| Part | What | Data |
|---|---|---|
| Scope token | At the field's leading edge while a scope is on: its name ("Clipboard", "Firefox"). Absent in the general search | quick key, prefix or Tab (Keyboard) |
| Query | The text typed. On reopening, the last general query returns selected, so typing replaces it and Return repeats it, as on macOS; a scoped query (clipboard, emoji, vault, run) is never kept | |
| Activity mark | A small still mark at the trailing end while a slow source is still searching; nothing otherwise | |
| Quick keys | Four buttons at the trailing end while the query is empty: Apps, Files, Actions, Clipboard, each naming its chord on hover. They give way to the query once typing starts | Cmd+1 to Cmd+4 |

No magnifier glyph and no prefix hints in the field (v1 `0e062e2`); the
placeholder names the scope.

**Scopes.** A quick key, a prefix typed as the first character, or Tab on
an app narrows the search to one source. They are one model: each sets the
scope token, and Backspace in an empty query clears it.

| Scope | Quick key | Prefix | Holds |
|---|---|---|---|
| Apps | Cmd+1 | | The Apps view (below) |
| Files | Cmd+2 | `/` names, `?` contents | Files by name, or by content with Contents on; a Names / Contents switch under the field |
| Actions | Cmd+3 | | Shell actions, the front app's menu items, apps' desktop-entry actions and quicklists |
| Clipboard | Cmd+4 | `:` | Clipboard history and pins |
| Calculator | | `=` | Calculation and unit conversion |
| Emoji | Ctrl+Cmd+Space opens it | `.` | Emoji and symbols, as a grid |
| Windows | | `@` | Every open window |
| Passwords | | `*` | The vault's items |
| Run | | `>` | A command to run |
| One app | Tab on an app result (in the Apps view, Tab moves to the category pop-up instead) | | That app's windows, desktop-entry actions, quicklist and recent files |

**The general search.** With no scope, results are grouped by source under
a small header, in a fixed order so the eye learns where each kind lands:

1. **Calculation**, first, only when the query reads as a sum or a
   conversion (a cheap test as it is typed, such as a leading digit). Its
   row is placed at once and filled when the calculator answers, so
   nothing is inserted above the selection later.
2. **Top hit**: the one best match among the fast sources (apps, actions,
   Settings, windows, menu items). A name that starts with the query beats
   one matched inside a word, so typing "lock" puts Lock Screen first and
   "fi" puts Firefox first, keeping v1's actions first where they match
   (`design-v1-lessons.md` § 5 Launcher). The top hit is not repeated in
   its group.
3. **Apps**, ranked by launch count and recency (`shell.md` Spotlight 4);
   games among them.
4. **Actions**: shell actions, each with its chord at the trailing end, and
   apps' desktop-entry actions ("Firefox: New Private Window").
5. **Menu items** of the app that was in front when Spotlight opened: its
   enabled items, each with its path ("Edit › Find") and its hint
   (`menu-bar.md` Menu items in Spotlight).
6. **Settings**: rows by label, with their pane ("Settings › Displays"),
   from the index installed with Settings (`apps.md` Settings 4).
7. **Windows**: by title, with the app's name and its desktop.
8. **Files**, by name: at most a handful, then "Show all in Files".
9. **Search the web** for the query: always the last row while the query is
   not empty.

Clipboard, emoji, the vault and run commands are reached only through
their scopes, so a general search never shows a copied password's
neighbour, a secret's title or a stray emoji beside an app (For the user
3). Contents search runs only in the Files scope with Contents on.

**A result row.** Every row is the same height, so the list can promise
eight rows (`design-v1-lessons.md` § 5 Launcher). Leading: the icon, a
game's cover, a file's thumbnail or a clipboard image's thumbnail, in the
icon's own shape (brief, Left out). Then the title, and a second line only
where the title leaves something to say (a path, an app name, a desktop, a
snippet of matching text). Trailing: a chord for an action or menu item, a
pin mark for a pinned clipboard item, nothing otherwise. No row shows its
kind in words: the group header says it.

**Results panel.** A list the height of eight rows that scrolls past them,
group headers scrolling with it. It appears at that height with the first
result and keeps it until the query is cleared, so arrivals and narrowing
never resize it (v1 lesson, brief P5). With fewer results the space below
them is empty panel, not a shorter panel.

**Clipboard.** Pinned items first, in the order pinned, under "Pinned";
then the rest newest first under "Recent". A text item shows its first
line, with the line count when there are more; an image shows a thumbnail
in the icon slot and its size and source app ("Image · 1920 × 1080 ·
Firefox"); copied files show the first file's icon and name. Cmd+Y opens a
preview of the selected item over the list: the whole text, scrollable, or
the image fitted. Secret offers are never recorded (`platform.md`
Clipboard 2).

**Emoji.** A grid, not a list: recent picks first, then Unicode's groups
under their names, and symbols by their names. The selected glyph's name
shows in a line under the grid. Variants (skin tones) are in the glyph's
menu.

**Passwords.** One row per vault item: its title and the provider's name,
never a field's value (v1 D42). A locked provider shows one row saying so,
whose action opens Keychain to unlock it.

**Run.** The command as typed, then recently run commands. Return runs it
detached in the session scope; Cmd+Return runs it in the user's terminal.

**Calculator.** The answer as the row's title, the expression normalised
under it. Return copies the answer; the island shows "Copied"
(`island.md` Events).

**Files.** Name, the folder it is in (home shortened to ~), and a
thumbnail where Files' thumbnailer has one. Contents rows add the matching
line. Contents search is debounced and capped at twenty files (v1's 250 ms
and 20, inventory § 4 Launcher), on the worker. The last row of the Files
scope is "Show all in Files", which opens a Files window searching for the
query. An empty Files scope lists recent files.

**The Apps view.** Cmd+1, the bottom-right hot corner by default, or the
Launchpad key opens it. The field stays where it is with the Apps token;
the results panel becomes a larger panel below it, on the same axis, that
holds a grid of tiles (gate 2, `dock.md` For the user 1):

| Part | What | Data |
|---|---|---|
| Favourites | The user's apps in the user's order, first, under "Favourites"; as many rows as the user fills. Absent until one is added | `settings.kdl`, Spotlight's section |
| All apps | Every installed app and game, alphabetical by name, under "All apps". Favourites appear here too, so every app is where its name puts it | the app index (`platform.md` App index), the game library (`gaming.md` Game library) |
| Category filter | A pop-up button at the panel's top trailing corner: "All apps" or one category. A category shows only its apps, alphabetical, with no favourites row | desktop entries' main categories, under Mac-style names (stage 5); Games from the game library |
| Tile | The icon in its own shape, the name under it on one line, cut at its end with the full name on hover | desktop entry |
| Badge | A count at the icon's top trailing corner, "999+" past three digits, as on the switcher's cards; a status mark, not the accent (brief P6). No progress, attention or running mark: those are the switcher's | LauncherEntry, kept per desktop entry (`platform.md` Notifications and badges 6) |

- **No folders and no pages** (gate 2): the grid scrolls vertically and its
  column count is fixed while it is open, so the arrows always move the
  same way. The count is stage 4's.
- **Typing in the Apps view** narrows the grid to matching apps in rank
  order, best first and selected; the section headers and favourites row
  go while a query is typed.
- **Games** are tiles like any app: the cover, fitted to the tile's icon
  slot by height and keeping its own shape, and the game's name (`gaming.md`
  Games as apps). A desktop entry Steam, Heroic or Lutris wrote for a
  library game folds into that game, so it is one tile, launched through
  its launcher (`gaming.md` Game library 2).

**The tile's menu** (secondary click, or Cmd+K on the selected tile),
taking the duties the Dock's app menu had (`dock.md`):

| Group | Item | Notes |
|---|---|---|
| 1 | Open | Or the app's name in front, if it is running |
| 2 | Desktop-entry actions, then the LauncherEntry quicklist | Only if the app has them |
| 3 | Add to favourites, or Remove from favourites | |
| 4 | Options ▸ | Open at login (checked when on); Keep running after last window (checked when on; not for games; the B20 policy, also in Settings › Apps); Assign to ▸ All desktops, This desktop, None |
| 4 | Show in Files | The desktop entry or AppImage, selected |
| 4 | Show in {launcher} | Games only: the game's page in Steam, Heroic or Lutris |
| 5 | Quit | Only while it is running |

`menu-bar.md` did not take "Keep running after last window" into the app
menu, so this menu and Settings are its homes.

## States

| State | What shows |
|---|---|
| Rest | Nothing. Spotlight draws nothing and runs nothing while closed (shell runtime 7). |
| Open, empty field | The field with the quick keys, and nothing below it (For the user 2); or the last general query, selected, with its results. |
| Typing, general | Groups as in Content; the selection on the first row (the calculation or the top hit). |
| Scoped | The token in the field and that scope's own form: list, grid (Apps, Emoji) or Clipboard's two groups. An empty scoped query shows what the scope holds: recent files, every action by use, the whole clipboard, recent emoji, every window, every vault item, recent commands. |
| Apps view | Favourites, then all apps; the first tile selected. |
| Apps view, filtered | Matching tiles in rank order; headers and favourites hidden. |
| One app (Tab) | That app's windows, actions, quicklist and recent files, grouped; Backspace in the empty field returns to the general search with the query as it was. |
| Loading: fast sources | Apps, actions, Settings, windows and menu items answer from memory within the 50 ms bar (acceptance bar 6). Return pressed before they answer acts on the top hit once they have, never on the previous query's list. At session start the app index answers from its last saved state while it rebuilds. |
| Loading: slow sources | Files, contents, the calculator and the vault arrive after the fast ones. They only add rows below the selected row; a result that would land above it waits for the next keystroke. The field's activity mark shows while any is searching. |
| No results | Only "Search the web for “{query}”" in the general search. In a scope without the web: one line, "No results for “{query}”". |
| Empty sources | Clipboard: "No copied items". Windows: "No open windows". Passwords with no provider: "No password manager", with "Open Keychain". |
| Error | A source that fails or is missing (no file search tool, the calculator absent, a provider erroring) leaves its group out, never a stalled frame (shell runtime 8). A provider's error is one row with its reason, as the provider gave it (`copy.md` rule 7). |
| Vault locked | One row per locked provider: "{source} is locked", acting to unlock it in Keychain. |
| Many results | The list scrolls; the selection stays in view. Each group shows every match except Files, capped with "Show all in Files". The Apps view scrolls. |
| Nothing to paste into | No window had focus when Spotlight opened: a clipboard pick or an emoji is copied instead, and the island shows "Copied". |
| Second screen | Opens on the screen holding the focused window, or under the pointer if no window has focus; from a hot corner, on that corner's screen. Never on two screens. |
| Full-screen app in front | Opens over it as normal; choosing an app on another Space moves the view there (`compositor.md` § Apps, hide, quit 3). |
| Windowed game in front | Cmd+Space and Ctrl+Cmd+Space are the game's: they are outside B30's escape set (`competition.md` Keys). Spotlight is reached by Cmd+Tab out of the game, or by the hot corner, which works beside a windowed game. Opened, it is as normal, in its solid form and without transitions (Game Mode 9). |
| Full-screen game in front | Not opened: the keys are the game's, and hot corners never fire over a full-screen game (`shell.md` Mission Control 8). Cmd+Tab out of the game first. |
| A system prompt | The prompt takes focus over Spotlight and closes nothing (shell runtime 10); Spotlight keeps its query and gets focus back when the prompt is answered. Cmd+Space does not open while a prompt holds focus. |
| Locked | Closed. Locking closes Spotlight and discards a scoped query. |
| Shortcuts inhibited | Cmd+Space and Ctrl+Cmd+Space go to the inhibiting window; the hot corner still opens the Apps view. |
| Focus on | Unchanged. |
| Reduce Motion | Appears and goes by fading only; the results panel and the Apps view appear at their size; the selection moves without sliding. |
| Reduce Transparency | Field and panels in their solid form (brief P4). |
| Increase Contrast | An edge around the field and panels; the selection and the badge in their stronger forms. |
| Largest text size | Rows grow and the list still shows eight; the panel is taller. Tiles' names scale; the Apps view's column count is set again on the next open, never while open. |

## Opening and closing

- **Cmd+Space** opens Spotlight with the field focused; Cmd+Space again,
  or Escape in an empty field, closes it. Keys typed from the moment
  Cmd+Space is pressed reach the field, through the opening (`shell.md`
  Spotlight 1, acceptance bar 6): the compositor buffers them until the
  field exists.
- **Ctrl+Cmd+Space** opens it in the Emoji scope.
- **Cmd+1** in Spotlight, the **Launchpad key**, or the **bottom-right hot
  corner** after a rest (`dock.md` For the user 3) open the Apps view. The
  hot corner fires once per visit and closes a Spotlight it opened if it
  fires again.
- **Opening** is a quick fade in place, scaling from `motion-panel-scale`
  about the field's centre: no drop from the top and no
  growth from a hot corner, since the panel opens where it belongs (v1
  D91). The results panel fades in under the field with the first result.
- **Closing** is faster than opening (brief P5). On a choice, Spotlight
  goes at once, focus returns to the window that had it, and only then is
  the action delivered: the raise, the paste, the typed emoji, the menu
  item (shell runtime 11, v1 D63).
- **Choosing a running app raises it**, as committing in the switcher does,
  never starting a second copy (v1 D63); an app that is not running starts
  with an activation token and the pointer's starting form (`dock.md`
  Launch feedback).
- **Other surfaces.** Spotlight closes Control Centre, Notification Centre,
  the menu bar and its menus (`island.md`). It and Mission Control, the
  switcher, the capture tool, the session dialog and the widgets dashboard
  are one at a time: opening one closes the other, and Cmd+Tab closes
  Spotlight and switches (`competition.md`).
- A click outside Spotlight closes it, as on macOS.

## Keyboard

Focus stays in the field throughout; the arrows move the selection in the
list or grid beneath it, so typing never stops working.

| Action | Key |
|---|---|
| Open or close | Cmd+Space |
| Open in Emoji | Ctrl+Cmd+Space |
| Open the Apps view | The Launchpad key; Cmd+1 when open |
| Apps, Files, Actions, Clipboard scope (again to leave it) | Cmd+1, Cmd+2, Cmd+3, Cmd+4 |
| Other scopes | Their prefix as the first character: `/` `?` `=` `.` `:` `@` `*` `>` |
| Into an app's windows, actions and recent files | Tab on an app result |
| Leave a scope | Backspace in an empty query |
| Move the selection | ↑ and ↓ in a list; the four arrows in a grid. No wrapping |
| Previous or next group | Cmd+↑, Cmd+↓ |
| A page up or down | Page Up, Page Down |
| Do it: open, switch to, run, paste, type, copy the answer, copy the password | Return |
| Copy instead of pasting or typing (clipboard, emoji); the username (vault) | Shift+Return |
| The one-time code (vault) | Ctrl+Return |
| Show in Files (file, app); run in the terminal (run) | Cmd+Return |
| Quick Look a file; preview a clipboard item | Cmd+Y; again or Escape closes it |
| The selected result's or tile's menu | Cmd+K |
| Copy the result (a file, a path, the answer, an item) | Cmd+C |
| Pin or unpin a clipboard item | Cmd+P |
| Delete a clipboard item | Cmd+Delete |
| Move a favourite left or right | Option+Cmd+← and → |
| Clear the query; with none, close | Escape |
| Close and switch apps | Cmd+Tab |

Shortcuts show as ⌘ ⌥ ⌃ ⇧ on a Mac-layout keyboard and as words on a PC
one (`copy.md` rule 11). Every action a menu offers has a key above or is
in the menu Cmd+K opens; nothing is reached only by pointer.

## Pointer

| Input | On | Does |
|---|---|---|
| Hover | A row or tile | Selects it, but only when the pointer moves, so a pointer resting where Spotlight opens never takes the selection (`design-v1-lessons.md` § 3.5) |
| Click | A row or tile | Does it, as Return |
| Click | A quick-key button, the Names / Contents switch, the category filter, "Show all in Files" | Sets the scope, switch or filter; follows the link |
| Secondary click | A row or tile | Its menu (the tile's menu above; a result's: Open, Show in Files, Quick Look, Copy, Pin, Delete, its alternates) |
| Scroll | The list, the grid | Scrolls; the selection does not follow until the pointer moves |
| Drag | A file or clipboard item out of the panel | Spotlight hides when the drag leaves it, so the target is visible, and closes after the drop |
| Drag | A tile within Favourites | Reorders; the other tiles make room |
| Drag | A tile from All apps into Favourites | Adds it at the drop point |
| Drag | A favourite out of its row into All apps | Removes it from favourites |
| Click | Outside Spotlight | Closes it |
| Rest | The bottom-right screen corner | Opens the Apps view (the default hot corner) |

Nothing opens on hover.

## Shared places

| Contender | Proposal |
|---|---|
| Centre of the screen, overlay layer | Spotlight sits in the overlay layer, centred on its screen, the field in the upper part of the screen (stage 4 sets the height; v1 put its card at 30%). One of the one-at-a-time surfaces with the switcher, Mission Control, the capture tool, the session dialog and the widgets dashboard; Cmd+Tab closes it and switches (`competition.md`). |
| The island's panels and the menu bar | Opening Spotlight closes Control Centre, Notification Centre, the revealed bar and its menus (`island.md`, `menu-bar.md`). Spotlight sits below the island's strip, so the island stays visible and its privacy marks are never covered. |
| Banners | Banners keep their place and keep arriving while Spotlight is open; Spotlight is centred, so on the ultrawide they do not meet. Where banners sit is `notifications.md`'s. |
| The island's "Copied" event | Spotlight's copies (the answer, a vault field, a clipboard or emoji pick with nowhere to paste) raise it after Spotlight has closed. |
| Bottom-right corner | The Apps view's hot corner after a rest; quarter zones while a window is dragged (`competition.md`); never over a full-screen game. The capture thumbnail and picture-in-picture sit inside the work area there; their hit areas stop short of the corner point, so clicking them never rests in the corner (for `capture.md`). |
| Keyboard focus | Level 4 of `competition.md`'s order: below the lock, a prompt and the held switcher; above menu-bar surfaces and windows. A prompt takes focus over Spotlight and gives it back. |
| Cmd+Space, Ctrl+Cmd+Space, the Launchpad key | Desktop chords, taken before Cmd translation (B21), so an app never sees them; Ctrl+Space stays the app's (`macos-model.md` § 3, the key table). Over a game, windowed or full screen, and under shortcuts-inhibit they go to the window. They go in P5's chord table. |
| Paste and typing into the previous window | Through Isle's own input source, untranslated, after focus has returned (B21, `compositor.md` § Keyboard-shortcuts-inhibit and injected input 2). |
| Menu items | Spotlight's Menu items group reads the same index as menu search; choosing one acts without revealing the bar (`menu-bar.md`). |
| Quick Look | Cmd+Y calls Quick Look in the Files process (B26); its window appears over Spotlight, which stays open behind it. |
| The lock | Locking closes Spotlight. |

`competition.md` gains: the bottom-right corner's hit-area rule for the
capture thumbnail and picture-in-picture, and Spotlight closing on lock.

## Answers

- **Result layout and grouping by source** (`shell.md` Spotlight, proposal:
  grouping by source). **Adopted**, with a calculation row and a top hit
  above the groups, a fixed group order, uniform rows in a list the height
  of eight that scrolls, and caps only on files. Clipboard, emoji, the
  vault and run commands live in their scopes rather than among the
  groups, so the general search stays short and never shows a secret's
  title.
- **How clipboard images and pins show** (proposal: thumbnails).
  **Adopted**: an image's thumbnail in the row's icon slot with its size
  and source, Cmd+Y for the whole item; pins as a group of their own
  first, in the order pinned, with a mark.
- **The Apps view's layout** (proposal: a grid by category under the
  favourites row). **Changed** as gate 2 decided (`dock.md` For the user
  1): favourites in the user's order, then every app alphabetically, with
  no folders and no pages; categories are a filter, not sections, so an
  app has one place in the grid.
- **Badges on tiles** (`dock.md`, `platform.md` Notifications and badges).
  **Confirmed**: the count at the icon's top trailing corner, as on the
  switcher's cards. Progress, attention and running marks stay the
  switcher's.
- **The tile's menu** (`dock.md`: Open at login, Show in Files, Assign
  to, desktop-entry actions and the quicklist). **Adopted**, with Add to
  and Remove from favourites, Keep running after last window (which
  `menu-bar.md` did not take), and Quit while running. dock.md's "listed
  under the app's result" becomes Tab into the app and the result's menu,
  so the list never grows under the selection.
- **Games** (`gaming.md` Games as apps, Spotlight's part). **Confirmed**:
  the cover in the icon slot, the game's name, launched through its
  launcher, raised when running; a launcher's desktop entry for a library
  game folds into the game.
- **Quick keys** (`shell.md` Spotlight 3). **Adopted**: Cmd+1 to Cmd+4 as
  on macOS, shown as buttons while the field is empty; v1's prefixes as
  the other scopes, one model with the quick keys.
- **Quick Look on Space** (`shell.md` Spotlight 10). **Changed** to Cmd+Y,
  macOS's Quick Look chord: the field holds focus, so Space is part of the
  query. Cmd+Return still shows the file in Files. `shell.md` Spotlight 10
  is updated at the gate.
- **The vault in the general search** (v1 listed vault items beside apps,
  inventory § 4). **Changed**: only under `*` (For the user 3).
- **The assistant slot** (`shell.md` Spotlight, open; macOS 27's "Search or
  Ask"). Not now; the web row is the last-resort slot an assistant would
  take (For the user 1).
- **Which screen** (`shell.md` Shell runtime, proposal: Spotlight on the
  main screen). **Adopted**, with the main screen following focus: the
  screen of the focused window, as the switcher, or the hot corner's own
  screen.
- **Full screen and a game in front.** Spotlight is not in B30's escape
  set, which `competition.md` and `gaming.md` Keys and pointer 2 settle:
  over any game its keys are the game's. Over a full-screen app it opens
  as normal.
- **Ranking** (`shell.md` Spotlight 4, 5). **Adopted**: launch count and
  recency for apps; the top hit drawn only from fast sources; slow results
  never move the selected row.

## Copy

| Where | String |
|---|---|
| Field placeholder | "Search"; in a scope: "Search apps", "Search files", "Search actions", "Search clipboard", "Calculate", "Search emoji", "Search windows", "Search passwords", "Run a command" |
| Scope tokens | "Apps", "Files", "Actions", "Clipboard", "Calculator", "Emoji", "Windows", "Passwords", "Run", or the app's name |
| Quick-key buttons (hover) | "Apps {shortcut}", "Files {shortcut}", "Actions {shortcut}", "Clipboard {shortcut}"; {shortcut} in `copy.md` rule 11's form: ⌘1 to ⌘4 on a Mac keyboard, the words on a PC one |
| Group headers | "Calculation", "Top hit", "Apps", "Actions", "Menu items", "Settings", "Windows", "Files", "Pinned", "Recent" |
| Files | "Names", "Contents", "Show all in Files" |
| Web row | "Search the web for “{query}”" |
| Apps view | "Favourites", "All apps"; category filter "All apps" |
| Tile menu | "Open", "Add to favourites", "Remove from favourites", "Options", "Open at login", "Keep running after last window", "Assign to", "All desktops", "This desktop", "None", "Show in Files", "Show in {launcher}", "Quit" |
| Result menu | "Open", "Show in Files", "Quick Look", "Copy", "Copy path", "Pin", "Unpin", "Delete", "Paste", "Run in terminal", "Clear clipboard history…" |
| Clear history | "Clear clipboard history?" with "Clear {n} items" and "Cancel"; "Pinned items are kept." |
| Clipboard item | "Image · {w} × {h} · {app}"; "{n} more lines" |
| Empty and errors | "No results for “{query}”", "No copied items", "No open windows", "No password manager" with "Open Keychain", "{source} is locked" with "Unlock in Keychain" |
| Island | "Copied" (`island.md`) |

Words follow `copy.md`.

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **The assistant slot.** macOS 27's field is "Search or Ask", sending
   unmatched text to Siri. (a) No assistant: the web row stays the last
   resort, and its place is where an assistant row would go later, so
   adding one changes no layout; (b) an "Ask" row under the web row,
   backed by a provider the user installs (any CLI or API, sandboxed as
   providers are, B28), answering inside the panel; (c) 27's form: "Search
   or Ask" in the field and Return on an unmatched query asking.
   **Recommended: (a).** There is no Siri to copy, the user works with
   assistants in a terminal and an editor already, and a row that answers
   in prose turns a calm, fixed-height list into a reading pane (brief
   P1). (b) is the way in if wanted later.
2. **The empty field.** (a) Nothing under the field but the quick keys,
   as macOS 27, with the last query returning selected on reopening;
   (b) a row of recent apps under the field; (c) suggestions (apps,
   files and actions by time of day and use). **Recommended: (a).**
   It is calm (brief P1), the field is the fastest route to anything, the
   last query covers "do that again", and Cmd+1 is one chord from the
   grid of everything.
3. **Clipboard and vault in the general search.** (a) Only in their
   scopes (Cmd+4 or `:`, and `*`); (b) as v1, with vault titles and
   clipboard matches among the groups. **Recommended: (a).** The general
   search is what is on screen in a shared or recorded session, and a
   copied text or a vault title beside an app is noise and a leak; the
   scopes are one key away.
