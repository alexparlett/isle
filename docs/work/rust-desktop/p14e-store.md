# P14e · Store

Planned 2026-11-17 after the board (`p14e/store-1.html`, published) passed
with every recommendation; building. At the user's word, after P14d:
updates and installs in a storefront like CachyOS's Package Installer
2.0, and Software Update in Settings for the system alone.

P14d put every update in Settings › Software Update. The user wants it
the Mac's way: Software Update is the operating system, and a store is
for apps. Here the operating system is CachyOS (its kernel, graphics
driver, Mesa and tools) and Isle; apps come from four catalogues:
CachyOS's repositories, Flathub, the AUR and AppImages. CachyOS's
Package Installer 2.0, previewed 2026-11-13, sets the shape: a sidebar
of Discover, Browse, Installed and Updates, apps shown with icons,
descriptions and screenshots, every catalogue in one window.

What it must get right: what catalogue you are looking at is always
named; pacman never does a partial upgrade, from Store or Settings; what
an install brings and a removal takes is said before the password; and
every run keeps P14d's guarantees (one password, outlives its window,
Stop only outside a transaction, failure said in words).

## What it answers to

- Decisions: B29 (Isle built from git), B100, P14d's choices (the run,
  restart owed, the polkit action, Isle's PKGBUILD repository).
- Intent: `platform.md` Updates; `apps.md` Settings 12 (Software Update,
  revised) and a new Store entry; `shell.md` Menu bar 2.
- Earlier packets: P14d (the run machinery, `Updates1`, Settings' page,
  the module, glyph and menu), P13 (Settings' sub-pages), P15 (Files'
  window and sidebar as the app pattern), P19b (Cleaner keeps orphans and
  the package cache).
- Outside: CachyOS Package Installer 2.0 (preview, 2026-11-13: storefront,
  repositories with the AUR and Flatpak, sidebar tabs); AppStream
  (`archlinux-appstream-data`, Flathub's appstream); the AUR's RPC.

## Choices this plan makes

### The split

1. **Settings › General › Software Update is the system:** a CachyOS
   card and an Isle card (board, Settings 1–4). CachyOS names the
   kernel, the graphics driver, Mesa and CachyOS's tools found among
   what is waiting, and "Also updates {n} apps installed from CachyOS"
   with their icons and Show All; one Update Now runs `pkexec pacman
   -Syu`, since pacman upgrades all or nothing. Isle's card shows the
   build, the commit messages since it, and its own Update Now; both
   chosen, Isle's build follows CachyOS's upgrade in the same run
   (P14d's `paru -Syu --mode=rp isle-git`). The restart owed is said
   there. A footer opens Store.
2. **Store is apps** from four catalogues, all on: CachyOS, Flathub, the
   AUR and AppImages. Its Updates covers Flathub, the AUR and AppImages,
   each a section with Update All and an Update per app; Update
   Everything runs those three in turn. Apps from CachyOS are listed,
   badged, "With the system", the section linking to Software Update.
3. **The system menu:** "System Settings…" counts the system's updates
   (CachyOS, Isle), "Store…" the apps'. Control Centre's Updates module
   and the island's glyph count and show both.

### Catalogues

4. **Always named.** A sidebar section, Catalogues, with a page each
   saying what it is and who stands behind it (board, Store 3); a badge
   on every app and package; search grouped by catalogue with a pop-up
   to narrow it (Store 4); an app in more than one picks its catalogue on
   its page. Catalogues… turns one off, which hides it everywhere.
5. **CachyOS** from AppStream: `archlinux-appstream-data`
   (`/usr/share/swcatalog`) gives names, summaries, icons, screenshots
   and categories by package name; CachyOS's repositories rebuild Arch's
   packages under the same names. An app is a package with a desktop
   component; everything else is a library or tool and never listed as an
   app. A dependency of `isle-git`.
6. **Flathub** through the `flatpak` command: its appstream for the
   catalogue, `flatpak remote-ls --updates`, `flatpak update` and
   `install` per app, non-interactive, its own polkit for system-wide
   installs. `flatpak` is an optional dependency; without it the
   catalogue says how to add it.
7. **The AUR** through its RPC (`/rpc/v5` search and info) for the
   catalogue, with its own facts in place of AppStream's (votes,
   popularity, last updated, out of date, maintainer); installs and
   updates through paru as the user (`paru -S --mode=a … --sudo pkexec`).
   The page warns that nothing there is checked by CachyOS or Arch.
   Isle's own package is never shown here (choice 1).
8. **AppImages** in `~/Applications`, as Isle already registers them
   (`isle-apps::appimage`): updates from the image's embedded update
   information (zsync, GitHub releases) where it has some; one without
   says it updates itself or that its developer must be visited. No
   catalogue to browse: the page lists what is installed and how to add
   one (drop it in Applications).

### Installing and removing

9. **What an app brings is said before the password** (board, Store
   9–11): under Get on its page ("also installs 14 packages, 532 MB in
   all", from `pacman -S --print` as the user), and in a sheet listing
   them that Get opens from anywhere; an app that brings nothing new
   installs without the sheet. With system updates waiting, Get offers
   "Update and Install" (`pacman -Syu <pkg>`), since installing against
   a stale system is a partial upgrade.
10. **Remove takes what only it needed** (`pacman -Rns`, listed first
    from `--print`); changed settings are kept as `.pacsave`. An app
    another package needs is refused, naming it. Flatpak removal leaves
    its data unless asked; an AppImage goes to the Trash.
11. **Optional extras are offered, never installed for you:** "Works
    better with" on the app page from the package's optdepends, each with
    its own Get.
12. **Every run is P14d's run:** a transient unit, a progress file, one
    password (the pacman action keeps its authorisation for paru's
    calls), Stop only outside a transaction, failures said in words.
    Flatpak and AppImage runs need no root and can run beside a pacman
    run; two pacman runs never overlap.

### The app

13. **Store**, a Freya app of Isle's own (`isle-store`), on Files' window
    and sidebar pattern; "Store…" in the system menu. Discover's features
    are a list Isle ships; its rows and Browse come from AppStream
    categories across catalogues.
14. **Build order:** Settings' split and Store's Updates and Installed
    first (14e.1–14e.4), then catalogue pages and search, then Discover,
    Browse and app pages, then Get and Remove.

### Proof

15. **Headless tests:** the AppStream reader against a recorded slice of
    `archlinux-appstream-data`; app and library told apart; the AUR RPC
    against recorded JSON; flatpak's lines and AppImage update info
    against fixtures; Settings' cards from `Updates1`; Store's pages from
    a fake catalogue; Get's and Remove's lists from recorded `--print`.
16. **The VM walk** by real input: a system update from Settings with
    its kernel named; a Flathub app updated alone and by Update All; an
    AUR package found, installed and updated with one password; an
    AppImage updated; Krita got with its dependencies listed and removed
    with them; a removal refused.

## Phases

### 14e.0 · The evidence

In the VM: `archlinux-appstream-data`'s files, icons and how packages map
to components; `pacman -S --print` and `-Rns --print` as the user;
flatpak's appstream location, `remote-ls --updates` and `update`'s output
through a pipe; AppImage update information and a tool to apply it; the
AUR RPC's fields. Recorded in `docs/research/rust-desktop/store-2026.md`.

### 14e.1 · Software Update for the system

`Updates1` with the CachyOS and Isle parts and the highlights; Settings'
two cards; the menu's two counts; the module and glyph over both.

### 14e.2 · `isle-catalog`

The catalogues behind one model: AppStream for CachyOS and Flathub, the
AUR's RPC, AppImages; installed apps; updates per catalogue. Tests.

### 14e.3 · Store's window, Updates and Installed

The app, its sidebar, Updates by catalogue with each run, Installed with
Open and Remove….

### 14e.4 · Catalogue pages and search

A Catalogues section in the sidebar, a page each saying what it is and who
stands behind it: CachyOS's and Flathub's apps listed sixty at a time, the
AUR's searched (it has no pictures or categories), AppImages' installed
ones. Search in the toolbar over CachyOS and Flathub as typed and the AUR's
RPC after a pause, grouped by catalogue, narrowed by a pop-up.

### 14e.5 · Discover, Browse and app pages

Discover: Isle's featured list, then a row per category chosen afresh each
day among apps with an icon and a screenshot. Browse: the categories, each
a page. An app's page: icon, name, summary, developer, catalogue; Get or
Open, its size and what it also installs; Also in, for the same app in
another catalogue; screenshots fetched and kept in
`~/.cache/isle/store/shots`; About; Works better with; Information, with
the AUR's own facts. Catalogue icons in JPEG XL are made PNG in
`~/.cache/isle/store/icons` as a page first draws them.

### 14e.6 · Get and Remove

Get goes straight to its run when the app brings nothing else; otherwise a
sheet lists the packages with versions and sizes (`pacman -S --print`,
the AUR's info and `pacman -T`, `flatpak remote-info` and its runtime).
With CachyOS's updates waiting it says so and offers Update and Install
(`install:cachyos+updates:<pkg>`, `pacman -Syu --needed <pkg>`); the AUR
is refused until they are done. Remove lists what goes from `pacman -Rs
--print` and runs `pacman -Rns`, or names who needs it; Flathub's keeps the
app's data. Parts `install:<catalogue>[+updates]:<id>` and
`remove:<catalogue>:<id>` run through `Updates1.Install` like every other.

### 14e.7 · The package, the bars and the walk

### 14e.8 · The host and close

## Not in P14e

Ratings and reviews. Accounts. Paid apps. Snap. Downgrades. A kernel
chooser (CachyOS's Kernel Manager). Firmware (fwupd).

## Risks

- Package Installer 2.0 is only a preview; its released shape may move,
  and Store follows the board, not it.
- AppStream coverage of CachyOS's own packages (its tools, its kernels)
  is thin; they are named from a list Isle keeps, not from AppStream.
- The AUR runs community code on this computer; Store says so on every
  AUR page and Get, and never builds without the password asked for the
  install.
- `flatpak` and AppImage update tools are not installed by default.
