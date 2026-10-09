# Spotlight for Isle: research note (2026-10-09)

Scope: what the launcher ("Spotlight") needs underneath it, on this machine
(CachyOS, kernel 7.2.8, Isle's own Smithay compositor, Freya shell, zbus,
tokio on a worker thread). "Local:" marks evidence gathered on this host
today; everything else carries a URL. Statements marked *(unverified)* are
from prior knowledge and were not re-checked in this pass.

---

## §1 File-name search

### §1.1 What is on this machine (local)

- `plocate 1.1.25` is installed (`/usr/bin/plocate`, setgid `locate`);
  `plocate-updatedb.timer` is active, daily, next run Sat 00:22.
- `/var/lib/plocate/plocate.db` is 369 MB, mode `0640 root:locate`, last
  good build **Oct 8 07:29**. The Oct 9 00:00 run **failed**:
  `updatedb: fwrite(): No space left on device` (journal), although `df`
  now shows 625 GB free on `/`. So the db is ~30 h stale today and a
  transient full disk (or btrfs metadata pressure) broke the nightly run.
  Lesson: anything built on plocate must tolerate a stale or missing db.
- `/etc/updatedb.conf`: `PRUNENAMES = ".git .hg .svn"`, prunes `/tmp`,
  `/var/cache`, network and fuse filesystems; `$HOME` *is* indexed.
- plocate's permission model (`man plocate`): results are filtered to
  files "visible to the calling user (by virtue of having read and execute
  permissions on all parent directories)"; the index is read via setgid.
  A compositor running as the user can therefore exec `plocate` but cannot
  read the db file directly (no `locate` group membership).
- Latency (warm cache): `plocate -i -l 20 readme` = **28 ms**; 53,593 total
  matches for "readme".
- `localsearch` / `tracker3-miners` are **not installed**. `tinysparql
  3.11.1` is present only as a gtk3/gtk4 dependency (no indexer running).
- `$HOME` size: `fd -H -u --exclude .cache` = **1,093,502 entries**, 98,192
  dirs, avg full path 128.7 bytes (134 MB total), avg basename 19.8 bytes.
  Biggest subtrees: `.rustup` 314k, `.local` 300k, `Development` 167k,
  `.unsloth` 142k, `.cargo` 90k, `.config` 47k. With fd's defaults (skip
  hidden, honour .gitignore) it is **83,909 entries / 7,004 dirs**.
- Walk time (warm page cache, all cores): `fd -H -u` over 1.09 M entries
  **85 ms wall** (1.25 s CPU); default `fd` over 84k entries **17 ms**.
  Cold-cache numbers will be much worse; not measured (needs dropping
  caches as root).
- inotify limits: `max_user_watches = 524288`, `max_user_instances = 1024`.

### §1.2 Options compared

| Option | Freshness | Cost | Content? | Notes |
|---|---|---|---|---|
| plocate | daily (timer) | ~0 at query; nightly root walk | no | 28 ms queries; stale up to 24 h+; shells out (no lib API); substring/regex, no fuzzy ranking |
| localsearch 3 | live (inotify) | background miner + extractor sandbox | metadata, and plain text of documents *(unverified: `nie:plainTextContent`, capped by an extractor max-bytes setting)* | D-Bus/SPARQL; GNOME-centric; would be a new dependency stack |
| own index (Rust) | live | one walk + watches | names only | full control of ranking, exclusions, memory |
| on-demand walk (`fd`-style) | live | a walk per query | names only | fine for scoped dirs; 1 M-entry walk is ~85 ms warm, unbounded cold |

- localsearch: indexes `$HOME` recursively by default but skips hidden
  folders, mountpoints, Git repositories and dirs with `.nomedia`;
  configurable via `ignored-directories`, `ignored-directories-with-content`,
  `ignored-files`. Daemons `localsearch-3` (indexer + SPARQL endpoint) and
  `localsearch-extractor-3` (sandboxed metadata extractor). D-Bus name
  `org.freedesktop.LocalSearch3`, queried with SPARQL (`tinysparql query
  --dbus-service=org.freedesktop.LocalSearch3`).
  https://gnome.pages.gitlab.gnome.org/localsearch/indexed-data.html ,
  https://localsearch-ac0dff.pages.gitlab.gnome.org/commandline.html
- Own index sizing for this home: 1.09 M nodes x (basename ~20 B + parent
  u32 + name offset u32 + flags) is roughly **30-35 MB** as a parent-pointer
  tree, versus 134 MB if full paths are stored. A fuzzy matcher that needs
  owned UTF-32 haystacks (nucleo) multiplies the name bytes by 4 for
  non-ASCII only (ASCII stays bytes).
- Watching: inotify needs one watch per directory. 98k dirs fits the 524k
  limit, but most of them (`.rustup`, `.cargo`, `target/`, `node_modules`,
  `.local/share/Steam`) are noise; excluding hidden + gitignored dirs (as
  fd defaults do) brings it to ~7k watches. fanotify with filesystem/mount
  marks and `FAN_REPORT_DFID_NAME` would give one watch for the whole fs,
  but those mark types need `CAP_SYS_ADMIN`; unprivileged fanotify (5.13+)
  allows only inode marks *(unverified detail)*. The compositor runs as the
  user, so inotify is the practical choice. Crates: `ignore 0.4.33`
  (2026-08-04, parallel walker honouring .gitignore), `notify 8.2.0`
  (2026-08-30). (crates.io API, local query today.)

### §1.3 What other launchers use (2025-2026)

- **KRunner**: Baloo (KDE's indexer), file names and optionally contents
  ("Also index file content"), `balooctl6` in Plasma 6.
  https://docs.kde.org/stable_kf6/en/plasma-desktop/kcontrol/baloo/ ,
  https://wiki.archlinux.org/title/Baloo
- **GNOME Shell**: no own index; text goes to every
  `org.gnome.Shell.SearchProvider2` D-Bus provider (`GetInitialResultSet`,
  `GetSubsearchResultSet`); Files (Nautilus) is the file provider and sits
  on localsearch3.
  https://developer.gnome.org/documentation/tutorials/search-provider.html
- **COSMIC launcher (pop-launcher)**: the `find` plugin shells out to
  `fdfind`/`fd -i --full-path <query>` per query; `files` plugin is path
  browsing; also `recent`, `desktop_entries`, `calc`.
  https://github.com/pop-os/launcher/blob/master/plugins/src/find/mod.rs
- **Vicinae** (Raycast-like): own file indexer on Linux (Spotlight on
  macOS): full scan of `$HOME` at first start (CPU-heavy by design), then
  incremental; skips dot files/dirs and honours `.gitignore`.
  https://docs.vicinae.com/file-search
- **Walker**: providers live in the `elephant` daemon; the files provider
  can be configured to use `fd`.
  https://www.linuxlinks.com/walker-fast-application-launcher/ ,
  https://github.com/basecamp/omarchy/discussions/2835
- **Anyrun**: `kidex` plugin, which queries the separate `kidex` inotify
  file-index daemon; also `rink` (units), `symbols`, `applications`.
  https://github.com/anyrun-org/anyrun/tree/master/plugins

Pattern: the modern Raycast-likes (Vicinae, Anyrun/kidex) keep their own
names-only index with gitignore/dotfile exclusion; COSMIC and Walker walk
on demand with fd; only the DE-integrated ones (KDE, GNOME) lean on a
content indexer.

---

## §2 File content search

- **ripgrep as a library**: `grep-searcher 0.1.17` (2026-07-15) +
  `grep-regex` + `ignore` give rg's engine in-process: memory-mapped or
  buffered search, binary detection, gitignore-aware parallel walk. No
  index, so cost is proportional to bytes read per query; fine for a
  scoped "search in Documents / this project" action, too slow and too
  IO-heavy as a keystroke-by-keystroke global search over 1 M files.
  (crates.io, local query; https://github.com/BurntSushi/ripgrep/tree/master/crates)
- **tantivy 0.26.2** (2026-09-08): Lucene-like inverted index, BM25,
  segments, configurable indexing memory budget, incremental commits;
  index is typically much smaller than the source text. Costs: text
  extraction is yours (PDF/ODF/Office need extra crates), plus a writer
  thread and merge IO; reopening a reader after commits is cheap.
  https://www.paradedb.com/learn/tantivy/introduction ,
  https://fulmicoton.com/posts/behold-tantivy-part2
- **localsearch FTS**: SPARQL `fts:match` over extracted text and
  metadata *(unverified details)*; buys extraction for many formats for
  free, at the price of the GNOME indexer stack running continuously and
  being disabled-by-design for hidden dirs and Git repos (§1.2).
- Practical limit for any content index here: `$HOME` has 1.09 M files but
  most are toolchains/caches; a content index should be an allowlist
  (Documents, Desktop, Downloads, notes) rather than a denylist.

---

## §3 Fuzzy matching crates

Versions and dates from the crates.io API (local query today).

| Crate | Version / last release | Notes |
|---|---|---|
| `nucleo-matcher` | 0.3.1 / 2024-02-20 | Helix's matcher; Smith-Waterman with affine gaps, two matrices; Unicode pre-segmentation, ASCII fast path, prefilter, greedy fallback for long haystacks; `Config::match_paths()` path bonuses; ~7-8x faster than skim in its own bench. MPL-2.0. No release in 2.5 years but still 1.9 M recent downloads. https://github.com/helix-editor/nucleo |
| `nucleo` | 0.5.0 / 2024-04-02 | High-level threaded "injector + snapshot" picker on top of the matcher; fits a type-ahead UI with streamed items. |
| `frizbee` | 0.13.0 / 2026-08-13 | SIMD Smith-Waterman, typo tolerance (`max_typos`), fzf-style prefix/delimiter/capitalisation bonuses, matches UTF-8 bytes directly; claims ~4x nucleo and ~5x fzf with typos off, ~20x on Unicode; `match_list_parallel`. Used by blink.cmp, atuin, television, skim, fff. MIT. Actively released. https://github.com/saghen/frizbee |
| `fuzzy-matcher` (skim) | 0.3.7 / 2020-10-04 | Unmaintained; slow on large sets; skim itself now uses frizbee. |
| `sublime_fuzzy` | 0.7.0 / 2020-12-19 | Unmaintained; Sublime-style scoring, small inputs only. |

Fit: for ~150 app entries any of them is instant; the difference matters
for 1 M file names. frizbee is the actively maintained, fastest option and
adds typo tolerance (useful for app names: "frefox"); nucleo-matcher is the
proven, stable one with the best-documented scoring. Both expose match
indices for highlighting.

---

## §4 Desktop entries and icons

- **`freedesktop-desktop-entry 0.8.3`** (2026-09-12), maintained by
  Pop!_OS/COSMIC (mmstick, jackpot51), MPL-2.0: `default_paths()`,
  `desktop_entries(locales)`, `get_languages_from_env()`, `LocaleMap`
  localisation, `name()/generic_name()/keywords()/categories()`,
  `no_display`, `hidden`, `only_show_in/not_show_in`, actions, `dbus
  activatable`, Exec parsing with `ExecError`, `find_app_by_id()` and
  matching/score helpers. This is what cosmic-launcher uses.
  https://docs.rs/freedesktop-desktop-entry/latest/freedesktop_desktop_entry/
- **Icons**: `freedesktop-icons 0.4.0` (2025-04-16) does theme-inheritance
  lookup with a cache; COSMIC carries its own fork. `linicon` is stale
  (2022). (crates.io, local query.)
- **Spec rules the Apps view must honour** (Desktop Entry Spec 1.5,
  https://specifications.freedesktop.org/desktop-entry-spec/latest/ ):
  `NoDisplay=true` hides from menus but keeps the entry for MIME/launch;
  `Hidden=true` means "deleted", and must also mask same-ID entries lower
  in `XDG_DATA_DIRS`; `OnlyShowIn/NotShowIn` against `XDG_CURRENT_DESKTOP`
  (Isle should set its own name, e.g. `Isle`); `Keywords` (localised,
  `;`-separated) feed search; `[Desktop Action x]` groups listed in
  `Actions=` become secondary results; `DBusActivatable=true` means
  launch via `org.freedesktop.Application.Activate/ActivateAction` on the
  bus name equal to the desktop-file ID, falling back to Exec.
  Desktop-file ID: path relative to `applications/` with `/` replaced by
  `-`, first match in data-dir order wins.
- **Categories for an Apps grid filter**: map the spec's Main Categories
  (`AudioVideo/Audio/Video`, `Development`, `Education`, `Game`,
  `Graphics`, `Network`, `Office`, `Science`, `Settings`, `System`,
  `Utility`) to a short list of filters; Additional Categories refine.
  https://specifications.freedesktop.org/menu-spec/latest/category-registry.html
- **Watching for installs**: watch `applications/` (and its subdirs) under
  `$XDG_DATA_HOME` and every `$XDG_DATA_DIRS` entry, including
  `/var/lib/flatpak/exports/share` and
  `~/.local/share/flatpak/exports/share` (added by flatpak's
  profile.d script). pacman writes files in place; flatpak swaps the
  `exports` symlink tree, so watch the `exports/share/applications`
  directory and also its parent for create/move. Debounce (pacman
  transactions touch many files; `update-desktop-database` follows) and
  re-scan whole dirs rather than diffing events.
- Local: `/usr/share/applications` has 97 entries, `~/.local/share/
  applications` 22; 7 system entries declare `Actions=`, 3 are
  `DBusActivatable=true`. Flatpak is not installed via pacman (stale
  `~/.var/app` dirs for browsers exist). `XDG_DATA_DIRS` was empty in the
  agent's shell, so the session must set it (default
  `/usr/local/share:/usr/share`).

---

## §5 Calculator and unit conversion

| Engine | State | Units | Currency | Network |
|---|---|---|---|---|
| `fend-core 1.5.8` (2026-01-24) | pure Rust, tiny, no deps | yes, large set | via a host-provided `ExchangeRateFnV2` handler | **none in core**; the fend CLI fetches (UN treasury / ECB) |
| `numbat 1.24.0` (2026-08-23) | pure Rust language with dimensional types | yes, strict typing | `numbat-exchange-rates` (ECB XML) | `prefetch_exchange_rates()` is blocking network; `set_exchange_rates(xml)`, `load_currency_module_on_demand(bool)`, `use_test_exchange_rates()` let the host control it |
| `kalk 3.2.3` (2026-07-10) | pure Rust, math-focused | limited | no | none |
| `rink-core` *(not version-checked)* | pure Rust | very large (GNU units data) | from a host-supplied currency file | host-controlled; used by Anyrun |
| libqalculate 5.12.0 (local, with `qalc`) | C++ | very large | yes | updates exchange rates over the network when stale *(unverified default)*; no maintained Rust bindings on crates.io (`qalculate-sys` not found) |

- fend: `evaluate_with_interrupt` / `evaluate_preview_with_interrupt` with
  an `Interrupt` trait, so a runaway expression can be cut off per
  keystroke. https://docs.rs/fend-core/latest/fend_core/ ,
  https://newreleases.io/project/cargo/fend/release/1.3.0
- numbat: https://docs.rs/numbat/latest/numbat/struct.Context.html
- Fit for an always-running process with no network by default: **fend-
  core** (no deps, interruptible, currency only if Isle installs a handler
  that reads a cached rates file refreshed by an opt-in, rate-limited
  fetch). numbat is the alternative if typed-unit errors are wanted; it
  is heavier.

---

## §6 Emoji data

- `emojis 0.9.0` (2026-06-20): O(1) lookup, name, group/subgroup, CLDR
  order, `unicode_version()`, skin-tone variants (`with_skin_tone`), gemoji
  shortcodes; **Unicode 17.0** data. No CLDR keyword annotations, so
  "search by keyword" beyond name + shortcode needs extra data.
  https://docs.rs/crate/emojis/latest , https://lib.rs/crates/emojis
- Keywords: CLDR `common/annotations/<locale>.xml` and
  `annotationsDerived/` (keywords + TTS names per emoji, localised);
  `emoji-test.txt` gives fully-qualified sequences, groups and versions.
  https://github.com/unicode-org/cldr/tree/main/common/annotations ,
  https://unicode.org/Public/emoji/latest/emoji-test.txt
- Unicode 18.0 / Emoji 18.0 was published in September 2026 (nine new
  emoji: cracking face, leftward/rightward thumb, monarch butterfly,
  pickle, lighthouse, meteor, eraser, net, plus skin-tone variants), so
  the crate is one version behind; filter by the installed font's coverage
  (Noto Color Emoji version) rather than by the data's.
  https://www.iphone-ticker.de/neun-neue-emojis-unicode-18-0-offiziell-veroeffentlicht-286504/ ,
  https://blog.emojipedia.org/draft-emoji-list-for-2026-2027/
- Suggested approach: build-time codegen from `emoji-test.txt` + CLDR
  `annotations/en.xml` (+ en_GB) into a static table; skin tone as a user
  preference applied with the `emojis` modifier logic or the sequence
  rules from `emoji-test.txt`.

---

## §7 Clipboard history in the compositor

- **ext-data-control-v1** (wayland-protocols staging) is a straight port of
  `wlr-data-control-unstable-v1`, which is now deprecated; lets a
  privileged client watch and set the selection and primary selection.
  Supported by KWin, wlroots, niri, Hyprland, Wayfire.
  https://wayland.app/protocols/ext-data-control-v1 ,
  https://gitlab.freedesktop.org/wayland/wayland-protocols/-/merge_requests/336
- **In-compositor history (no protocol needed)**: Smithay's
  `wayland::selection` gives `SelectionHandler::new_selection(target,
  source, seat)` whenever any client sets the clipboard or primary
  selection, and `send_selection(...)` when the compositor itself owns
  the selection. Helpers set a compositor-owned selection
  (`set_data_device_selection`), read a client selection into an fd
  (`request_data_device_client_selection`), and query current userdata.
  Modules: `data_device`, `primary_selection`, `wlr_data_control`,
  `ext_data_control`. So Isle can: on `new_selection`, read the offered
  MIME list, skip if it contains `x-kde-passwordManagerHint` (value
  `secret`), otherwise pull the preferred types into the history store;
  to "paste" an old entry, set a compositor-owned selection carrying the
  stored bytes for each stored MIME. Implementing `ext_data_control` too
  keeps external tools (cliphist, wl-clipboard) working.
  https://docs.rs/smithay/latest/smithay/wayland/selection/index.html
  (smithay 0.7.0 is 2025-06-24; Isle may track git.)
- **Sensitive hint**: `x-kde-passwordManagerHint: secret` is the de facto
  convention (Klipper since 2018; KeePassXC sets it; CopyQ, Clipman,
  wl-clipboard honour it).
  https://phabricator.kde.org/R120:637cd0bf62e419b8038cb9b1565089fbd341ee7a ,
  https://gitlab.xfce.org/panel-plugins/xfce4-clipman-plugin/-/issues/72
- **cliphist** (0.7.0 installed locally): `wl-paste --watch cliphist store`
  per MIME family; stores byte-for-byte in a bbolt DB with
  `max-items`/dedupe; skips the entry when wl-paste sets
  `CLIPBOARD_STATE=sensitive` (wl-clipboard derives that from the hint).
  https://github.com/sentriz/cliphist (cliphist.go lines 83-84)
- **clipse**: Go TUI; history in `clipboard_history.json`, images written
  to a `tmp_files` dir, excludes by source app name ("1Password",
  "Password Safe"), optional auto-paste via `/dev/uinput`.
  https://github.com/savedra1/clipse
- Image storage: keep originals as files (PNG as offered) keyed by
  content hash under `$XDG_STATE_HOME`/`$XDG_DATA_HOME`, metadata (MIME
  list, source app id, time, size, text preview) in a small DB; cap by
  count and bytes; expire like macOS (§11).

---

## §8 Vault integration

What matters: list titles without decrypting secrets into Isle's address
space, then fetch one field (or TOTP) on demand.

- **rbw** (unofficial Bitwarden CLI, Rust): `rbw-agent` holds keys in
  memory with `lock_timeout` (default 3600 s), pinentry for unlock;
  `rbw ls/list` (optionally `--fields name,user,folder`), `rbw get <name>
  [--field X | --full | --raw]`, `rbw code <name>` for TOTP, `rbw search`.
  The local DB is encrypted, so listing names needs an unlocked agent but
  never prints passwords. Maintainer calls it feature-complete.
  https://github.com/doy/rbw
- **bw** (official Bitwarden CLI, Node): `bw list items` returns full
  item JSON *including* passwords, so a title list exposes secrets to the
  caller; `bw get totp <id>`; `bw serve` exposes a local REST API; slow
  per-invocation startup *(unverified timing)*. Not installed locally.
- **KeePassXC**: Secret Service (`org.freedesktop.secrets`) exposes items
  of the exported group(s) with attributes Title, UserName, URL, Notes,
  TOTP and non-protected custom attributes; the secret is a separate
  `GetSecrets` call, and KeePassXC can prompt per client. The browser
  protocol (NaCl over keepassxc-proxy) has `get-logins` (by URL) and
  `get-totp`. `keepassxc-cli show --totp` also works. Not installed.
  https://github.com/keepassxreboot/keepassxc-browser/blob/develop/keepassxc-protocol.md
- **1Password CLI**: `op item list --format json` returns id, title,
  vault, category, URLs (no secret values); `op read
  op://vault/item/field` reads one field; `op item get <id> --otp` for
  TOTP; unlock via desktop-app integration. Not installed.
  https://www.1password.dev/cli/reference
- None of these is installed on this host today (local `which`).

---

## §9 Game libraries

### Heroic (local: `heroic-games-launcher 2.22.3-2`, native package)
- Config root `~/.config/heroic/` (Flatpak:
  `~/.var/app/com.heroicgameslauncher.hgl/config/heroic/` *(path
  unverified this pass)*).
- `store_cache/legendary_library.json` = `{ "library": [ {app_name,
  title, runner:"legendary", art_cover, art_square, is_installed,
  install:{install_size,is_dlc}, developer, namespace, store_url,
  is_linux_native, canRunOffline, ...} ], "__timestamp": {...} }`.
  Local: 51 Epic titles, 0 installed.
- `store_cache/gog_library.json`, `nile_library.json` (Amazon) exist
  (empty here); `sideload_apps/library.json` = `{ "games": [...] }`
  (empty here); installed state for Epic is also in
  `legendaryConfig/legendary/installed.json`. Art cache in `images-cache/`
  and `icons/<appName>.jpg`; per-game settings `GamesConfig/<app>.json`.
- Launch: `heroic://launch?appName=<app>&runner=<legendary|gog|nile|
  sideload>[&arg=..][&gui=false]` (new style) or legacy
  `heroic://launch/<runner>/<appName>`; `gui=false|0|no` or the
  `hideWindowOnProtocolLaunch` setting suppresses the window. Pass it as
  an argument to `heroic` (or `xdg-open`).
  https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/blob/main/src/backend/protocol.ts

### Lutris (local: `lutris 0.5.22`)
- DB `~/.local/share/lutris/pga.db` (SQLite); tables `categories, games,
  games_categories, saved_searches, service_games, sources`.
  `games(id, name, sortname, slug, installer_slug, parent_slug, platform,
  runner, executable, directory, updated, lastplayed, installed,
  installed_at, year, configpath, has_custom_banner, has_custom_icon,
  has_custom_coverart_big, playtime, service, service_id, discord_id)`.
  Local: 14 games, 13 installed (wine, steam, ubisoft services).
- Art: `~/.local/share/lutris/coverart/<slug>.jpg` (portrait),
  `~/.local/share/lutris/banners/<slug>.jpg`, icons
  `~/.local/share/icons/hicolor/128x128/apps/lutris_<slug>.png` (local).
- Launch: `lutris lutris:rungameid/<id>` or `lutris:rungame/<slug>`;
  `lutris -l -j` lists games as JSON.
  https://manpages.debian.org/testing/lutris/lutris.1
- Flatpak: `~/.var/app/net.lutris.Lutris/data/lutris/` *(unverified)*.

### Steam (local, native package `steam 1.0.0.87`)
- `~/.local/share/Steam/appcache/librarycache/` is now **one directory
  per appid** (1,325 dirs here), not flat `<appid>_library_600x900.jpg`
  files. Inside: `header.jpg` (1,098), `library_hero.jpg` (565),
  `library_600x900.jpg` (507), `library_hero_blur.jpg`, `logo.png`,
  `library_header.jpg`, `library_capsule.jpg`, and a `<sha1>.jpg` (706,
  ~1.5 KB, likely the client icon named by its hash).
- Some assets sit one level deeper in **hash-named subfolders**
  (`librarycache/220/ac2f07.../library_600x900.jpg`; 434 such subdirs; 23
  of the 507 portraits are nested). Lookup must try
  `<appid>/library_600x900.jpg`, then `<appid>/*/library_600x900.jpg`,
  then fall back to `header.jpg`.
  (Local evidence; a CachyOS forum thread notes the split:
  https://discuss.cachyos.org/t/lnx-splits-steam-caches-into-obsurity/27501 )
- User-set art overrides live in
  `~/.local/share/Steam/userdata/<id>/config/grid/` as `<appid>p.jpg`
  (portrait), `<appid>_hero.jpg`, `<appid>.jpg`, `<appid>_logo.png`
  (local) and should win.
- Installed list: `steamapps/appmanifest_<appid>.acf` in each library in
  `steamapps/libraryfolders.vdf` (6 manifests in the default library
  here). Launch with `steam steam://rungameid/<appid>`.
- Flatpak Steam: `~/.var/app/com.valvesoftware.Steam/.local/share/Steam/`.

---

## §10 Typing text into a client from the compositor

- **wtype's trick**: for each distinct codepoint it calls
  `xkb_utf32_to_keysym`, appends the keysym to a private keymap
  (keycode = index + 1), uploads the whole XKB keymap with
  `zwp_virtual_keyboard_v1.keymap`, then sends press/release with ~2 ms
  gaps; for stdin it re-uploads every 100 new characters. Breaks under
  Xwayland, which caps keycode/keysym pairs.
  https://github.com/atx/wtype/blob/master/main.c ,
  https://dyn.manpages.debian.org/bullseye/wtype/wtype.1.en.html
- In Isle's compositor the same can be done without the protocol: Smithay
  `KeyboardHandle::set_keymap_from_string` (or a per-client keymap
  change) followed by synthesised key events to the focused surface, then
  restore the user keymap. Every keymap swap is broadcast to clients and
  some toolkits re-read it slowly, so batch the text into one keymap.
  *(Smithay API names unverified this pass.)*
- **text-input-v3 / input-method-v2**: the compositor can act as the input
  method and `commit_string` arbitrary Unicode to clients that enable
  text-input (GTK4, Qt6, Chromium/Electron with flags, Firefox); no keymap
  games, but terminals and games often lack text-input.
  https://wayland.app/protocols/text-input-unstable-v3 ,
  https://wayland.app/protocols/input-method-unstable-v2
- **Paste**: set a compositor-owned selection (§7) and send the client's
  paste chord (Ctrl+V; Ctrl+Shift+V for terminals, decided by app id).
  Most robust for long text and images; it clobbers the clipboard, so
  restore the previous selection afterwards.
- Recommended order: text-input commit when the focused client has
  text-input enabled; else selection + paste chord; keymap typing only for
  short strings into clients that reject paste.

---

## §11 macOS 26/27 Spotlight (reference behaviour)

- macOS 26 Tahoe adds browse modes: **Cmd-1** Applications (an app grid
  with category filters, replacing Launchpad), **Cmd-2** Files, **Cmd-3**
  Actions & Shortcuts, **Cmd-4** Clipboard history.
  https://support.apple.com/guide/mac-help/mh26783 ,
  https://eshop.macsales.com/blog/98115-spotlight-gets-supercharged-in-macos-tahoe-your-complete-guide-to-getting-more-out-of-the-default-mac-launcher/
- Clipboard history: Cmd-Space then Cmd-4 (or the Clipboard button);
  searchable; Cmd-C to copy an item; More > Clear History; first use asks
  to allow Clipboard results and warns sensitive data may appear.
  https://support.apple.com/guide/mac-help/mchl40d5b86b/mac
  Retention 8 h by default, 26.1 added 30 min / 7 days options
  (https://www.ithinkdiff.com/spotlight-macos-tahoe-quick-keys-clipboard/ ).
- Actions: hundreds of system and app actions (App Intents) runnable in
  place, with parameters filled inline; **Quick Keys** are short strings
  ("sm" Send Message, "ar" Add Reminder) assignable to actions.
  https://www.ithinkdiff.com/spotlight-macos-tahoe-quick-keys-clipboard/
- macOS 27 "Golden Gate" (WWDC 2026) mostly adds Siri conversations
  started from Spotlight and better photo/mail search; browse modes stay.
  https://macrumors.com/2026/06/08/wwdc-2026-recap

---

## Recommendations

1. **Files: own names-only index in-process**, built with `ignore`'s
   parallel walker over an allowlist-plus-gitignore scope (skip hidden,
   `target/`, toolchains), parent-pointer tree (~30 MB for the full home,
   a few MB for the default scope), kept live with inotify (~7k watches
   at fd-default scope) and a periodic cheap rescan for missed events.
   Use plocate only as an optional "everywhere" fallback via exec, and
   expect it to be stale (it failed last night here).
2. **Content search on demand, not indexed, in v1**: an explicit "search
   contents" action over a scoped folder using `grep-searcher` + `ignore`
   on the tokio worker, streamed results, cancellable. Revisit tantivy
   with an allowlist (Documents, notes) only if this proves too slow.
3. **Fuzzy: frizbee** for big lists (files, emoji, clipboard) for speed
   and typo tolerance; nucleo-matcher is the conservative fallback.
   Keep ranking boosts (frecency, app vs file kind) outside the matcher.
4. **Apps: freedesktop-desktop-entry + freedesktop-icons**, honour
   Hidden/NoDisplay/OnlyShowIn with `XDG_CURRENT_DESKTOP=Isle`, index
   Keywords and Actions, launch DBusActivatable apps over zbus, debounce
   inotify on every `applications/` dir including flatpak exports; derive
   the Apps grid filters from Main Categories.
5. **Calculator: fend-core** with no network; currency only from a
   cached rates file that an opt-in, daily, rate-limited fetch refreshes.
6. **Emoji: generate a static table** from `emoji-test.txt` + CLDR
   annotations at build time (or `emojis` plus a CLDR keyword table);
   skin tone as a preference; hide emoji the installed font lacks.
7. **Clipboard history in the compositor** via Smithay's
   `SelectionHandler::new_selection`, skipping `x-kde-passwordManagerHint`
   offers; files on disk for images, small DB for metadata, macOS-like
   retention choices (30 min / 8 h / 7 d); also expose
   `ext_data_control` for external tools.
8. **Vault: rbw first** (list names, `get --field`, `code` for TOTP, all
   via an unlocked agent, secrets fetched only on activation), with
   1Password `op` and KeePassXC Secret Service as later providers; avoid
   `bw list items`, which returns secrets.
9. **Games: read files, launch by URL**: Steam appmanifests + per-appid
   librarycache (with the nested-hash fallback and userdata grid
   overrides), Heroic `store_cache/*_library.json` +
   `heroic://launch?appName=..&runner=..&gui=false`, Lutris `pga.db` +
   `lutris:rungameid/<id>`; check both native and `~/.var/app` roots.
10. **Inserting text**: text-input commit when available, else
    compositor selection + paste chord (restore the old selection), keymap
    typing only as a last resort for short strings.
11. **Borrow Tahoe's shape**: Cmd-1..4 for Apps / Files / Actions /
    Clipboard, an app grid with category filters as the Apps mode, and
    user-assignable quick keys on actions.
