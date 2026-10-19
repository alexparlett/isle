# P14a · Installed, beside Hyprland

Planned 2026-10-19. Reviewed by the user the same day: every
recommendation taken.

P14 is split in two (the user's choice, 2026-10-19). Isle is not yet a
replacement for v1: Files, Mission Control, widgets, session restore, the
dropdown terminal and the vault come with P15 to P20. But the host runs that
keep finding what headless and the VM miss (xremap, the console switch, Cmd+Q
in a game, the slider flooding the compositor) are hard to make from isle-tty
beside a running Hyprland. P14a makes Isle an installed package and a session
chosen at sign-in, so an evening in Isle is an evening on the host with
nothing of Hyprland's underneath, and Hyprland stays one choice away. P14b,
just before P21, is the switch itself: Isle's greeter mode, the one-time v1
import, theming installed apps, `isle setup` in full, the update path and a
second GPU's connectors.

Today nothing installs Isle. There is no PKGBUILD, no session entry, no
`/usr/share` path that the binaries look in but one (Settings' row index,
whose fallback is the build machine's checkout), and the portal, PAM, NVIDIA
and systemd files in `data/` are copied by nobody. v1's greeter starts
Hyprland and nothing else (`/etc/greetd/hyprland.lua`). And Isle and v1
collide wherever both use a name: the session log, the lock marker, the
portal file and the session variables in the user manager, and v1's xremap,
which a graphical session of either kind starts.

What P14a must get right is that installing Isle and signing into it changes
nothing about signing into Hyprland: v1 keeps its keys, its file chooser, its
log and its lock, and every change P14a makes on the host can be undone.

## What it answers to

- Decisions: B9 (the supervisor), B12 and B67 (Isle's PAM service), B26
  (where things run; Settings in software), B29 (a PKGBUILD from git), B35,
  B83 (the portal backend on the worker), B66 (the lock marker).
- Intent: `platform.md` Session start and greeter 4–6 (the session scope, the
  start order, the environment), Supervisor and crash records 2 and 6,
  Packaging and setup 1, 3–5 and 7, Login Items and session restore 1–3, Lock
  4; `compositor.md` Session 1.
- Evidence: `session-2026.md` (new: §3 autostart, §4 the shared user
  manager, §5 the package and portal precedence; §1, §2 and §6–§8 are
  P14b's), `smithay-and-gaming.md` §2.1
  (the NVIDIA profile), `isle-v1-inventory.md` §3 and §12, the host's own
  files read on 2026-10-19 (greetd, PAM, portals, user units, autostart).

## Choices this plan makes

### Scope

1. **P14 splits at the switch.** P14a: the package, the session entry,
   tuigreet as the greeter, the coexistence fixes, Login Items' engine, the
   steps of `isle setup` it needs, and the dev loop. P14b: the greeter mode
   (B26), the v1 import (B27), theming installed apps with Qt's file
   dialogs, the rest of `isle setup` (keyring and fingerprint PAM lines,
   autologin, groups), the update path and restart owed, and a second GPU's
   connectors, which the host cannot test today (only `card2-DP-4` is
   connected). The roadmap's P14 entry becomes the two.
2. **tuigreet is the greeter until P14b.** greetd runs
   `tuigreet --time --remember --remember-session --sessions
   /usr/share/wayland-sessions` as `greeter` on tty1, so Hyprland and Isle
   are both on the list and the last choice is kept. One line of
   `/etc/greetd/config.toml`; v1's greeter files stay where they are and the
   old line comes back with `isle setup --undo`. The cost is a text screen
   for a few seconds at boot; the lock inside Isle is already Isle's.

### The package

3. **One package, `isle-git`, from `pkg/arch/PKGBUILD`.** The greeter mode
   will be the same binary, and the binaries talk to each other and must
   come from one commit, so nothing calls for a split (`platform.md`
   Packaging's open point, answered). The source is
   `github.com/alexparlett/isle` on `feature/rust-desktop` (`main` after
   P21), so paru can build it in P14b; the dev loop points it at the
   checkout (choice 15). The build follows `rust-toolchain.toml` (no
   `RUSTUP_TOOLCHAIN` exported, which would override its pin), turns
   makepkg's C `lto` off, and lists Skia's prebuilt archive in `source=()`
   with `SKIA_BINARIES_URL` pointing at it, since its build script fetches
   it with curl even under `--frozen` (`session-2026.md` §5).
4. **The version is the commit.** `pkgver` is `r<count>.g<short hash>`, and
   `ISLE_COMMIT` (the compositor's `build.rs`) is the same hash, so `isle
   version` and `pacman -Q isle-git` name one build (Packaging 7). A package
   is never built from a dirty tree, so never reports `-dirty`.
5. **What is built.** A release build with `--locked`: `isle-compositor`,
   `isle-session`, `isle`, `isle-auth`, `isle-record` and `isle-settings`
   (software, no `gpu` feature, B26). No `test-hooks`, no test binaries, no
   `isle-preview`.
6. **What is installed.**

   | Path | From |
   |---|---|
   | `/usr/bin/` the six binaries | the build |
   | `/usr/share/wayland-sessions/isle.desktop` | `data/session/` (new) |
   | `/usr/lib/systemd/user/isle-session.target` | `data/systemd/` |
   | `/usr/share/xdg-desktop-portal/portals/isle.portal` | `data/portal/` |
   | `/usr/share/xdg-desktop-portal/isle-portals.conf` | `data/portal/` |
   | `/usr/share/applications/io.github.alexparlett.Isle.Settings.desktop` | `data/apps/` |
   | `/usr/share/dbus-1/services/io.github.alexparlett.Isle.Settings.service` | `data/apps/` |
   | `/usr/lib/pam.d/isle` | `data/pam/` |
   | `/etc/nvidia/nvidia-application-profiles-rc.d/50-isle`, in `backup=()` | `data/nvidia/`; the driver reads no `/usr/share` directory |
   | `/usr/lib/udev/rules.d/70-isle.rules` | `data/udev/` (new): i2c devices `uaccess`, for DDC/CI brightness (B65) |
   | `/usr/share/licenses/isle-git/` | Phosphor's licence |

   A test in the workspace checks that every path the PKGBUILD installs
   exists in `data/`, so a renamed file fails the build here, not on the
   host.
7. **The binaries carry their data.** Tokens, translation tables, actions,
   emoji and glyphs are compiled in already. Settings' row index joins them
   (`include_str!`, as the others): it is committed and tested against the
   panes, so reading it at run time from `/usr/share` or from the build
   machine's checkout buys nothing. No release build reads a path under
   `CARGO_MANIFEST_DIR`; a test greps for it. `/usr/share/isle/` stays empty
   until P14b's templates.
8. **Dependencies by evidence.** `depends` is what `ldd` finds for the six
   binaries, mapped to packages with `pacman -Qo`, plus every program Isle
   spawns (`Xwayland`, `ffmpeg`, `systemctl`,
   `dbus-update-activation-environment`, `xdg-desktop-portal` and its gtk
   backend, `gnome-keyring` or `gcr` for the agent and keyring, AccountsService
   for the lock's picture), found by reading every `Command::new`. Fonts and
   cursor named by the tokens (Inter, JetBrains Mono, Noto Color Emoji,
   Bibata) are optional, since Isle falls back without them. `namcap` passes
   on the PKGBUILD and the package.

### Side by side with v1

9. **One portal file for both desktops, chosen per desktop.** v1's shell and
   Isle's compositor own the same name,
   `org.freedesktop.impl.portal.desktop.isle`, in different sessions; v1
   serves only FileChooser. The packaged `isle.portal` lists the union (Isle's
   five and FileChooser), with no `UseIn`. `isle-portals.conf` chooses for
   Isle (FileChooser from gtk until P15); the user's
   `~/.config/xdg-desktop-portal/portals.conf`, which outranks it and today
   prefers Hyprland's backend, becomes `hyprland-portals.conf`, read only
   under Hyprland (`session-2026.md` §5). v1's root-installed copy of
   `isle.portal` belongs to no package, so pacman refuses the first install
   while it exists; `tools/package.sh` passes `--overwrite` for that one
   path.
10. **xremap runs only outside Isle.** It is wanted by
    `graphical-session.target`, which Isle's target binds. A user drop-in,
    `xremap.service.d/isle.conf`, adds
    `ConditionEnvironment=!XDG_CURRENT_DESKTOP=Isle`; the negative form so
    that it still runs under Hyprland whether or not Hyprland's session
    exports the variable before the target starts. The target's
    `Conflicts=xremap.service` stays for a session started beside one.
11. **The session's variables are unset only where they are still Isle's.**
    On a clean exit the supervisor reads the user manager's environment;
    if its `WAYLAND_DISPLAY` is still Isle's, it unsets each name whose value
    is the one it exported, and otherwise nothing, since a session started
    since owns them, even values both share (`QT_QPA_PLATFORMTHEME=gtk3`).
    A session that never exported unsets nothing (found in P4's host run). At start Isle
    still exports whatever else holds the manager: isle-tty depends on
    Isle's apps opening in Isle, and puts Hyprland's values back after.
    niri, uwsm and gnome-session refuse a second session instead
    (`session-2026.md` §4); with tuigreet the sessions are one after the
    other, so that case is isle-tty's alone.
12. **The lock marker belongs to a logind session.** It becomes
    `$XDG_RUNTIME_DIR/isle/locked-<XDG_SESSION_ID>` (`locked` when there is
    no session id, as nested). A compositor restarted by the supervisor is
    in the same session and starts locked; a new sign-in, which has just
    authenticated, does not; and v1, whose marker is
    `$XDG_RUNTIME_DIR/isle/locked`, never sees Isle's.
13. **Isle's logs move to `~/.local/state/isle/logs/`.** `session.log` and
    `session.log.old` there; v1's script keeps
    `~/.local/state/isle/session.log` and its `.1`. Crash records stay in
    `crashes/`, which v1 does not use. The lines also go to the user
    journal under `SYSLOG_IDENTIFIER=isle-compositor` (Supervisor 6), since
    greetd's session has no terminal to show them.
14. **The environment gains what daily use needs.** `SSH_AUTH_SOCK` for
    `gcr-ssh-agent.socket` (`$XDG_RUNTIME_DIR/gcr/ssh`, when the socket
    exists) and `XCURSOR_THEME` and `XCURSOR_SIZE` from the tokens, so X11
    clients and toolkits that do not use cursor-shape show Isle's cursor
    (Session start 6).

### The dev loop

15. **The host runs the installed package; the checkout is packaged, not
    synced.** `tools/package.sh` builds the package from the checkout's
    `HEAD` (a `git+file://` source at that commit; uncommitted changes are
    left out and said), with a persistent `CARGO_TARGET_DIR` under
    `~/.cache/isle/` so a rebuild is incremental, and installs it with
    `pacman -U`. A running Isle keeps the binary it started with; the next
    sign-in runs the new one. `tools/isle-tty.sh` stays for trying a build
    beside Hyprland without installing it. `isle-dev-sync` is v1's and is
    left to v1 (Packaging 5).
16. **The helpers are pinned to the running build.** The compositor opens
    `isle-auth` and `isle-record` when it starts and spawns them through
    that descriptor, so a package installed while Isle runs cannot hand an
    old compositor a new helper. A helper speaking a changed protocol would
    otherwise fail the unlock and leave the session locked.

### Login Items

17. **Isle starts XDG autostart entries itself, at every compositor
    start.** The host has eight (Claude, Proton Mail, KDE Connect, the
    update tray, the keyring's and others'). Isle reads
    `$XDG_CONFIG_HOME/autostart` over `$XDG_CONFIG_DIRS/autostart` by file
    id, with systemd's generator's rules (`Hidden`, `X-systemd-skip`,
    `TryExec`, `OnlyShowIn` and `NotShowIn` against `Isle`,
    `X-GNOME-Autostart-enabled=false`; `X-GNOME-Autostart-Phase` only
    changes GNOME's lists), and starts each through the launch path every
    app takes, in a scope of its own with an activation token. A
    compositor restart starts only items no longer running: one whose
    `app-isle-<id>-*.scope` still has processes, or whose bus name is
    owned when it is `DBusActivatable`, is left (v1 D53). The generator is
    not used and `xdg-desktop-autostart.target` never pulled in, or every
    item would start twice: its units start once per target, not per
    compositor, and with `Restart=no` stay dead after a crash
    (`session-2026.md` §3). The start order is Session start 5's: Login
    Items wait on the worker until the user manager names Isle's display
    and `graphical-session.target` is active (30 seconds at most), so a
    session that does not carry the manager starts none; isle-tty turns
    them off with `ISLE_LOGIN_ITEMS=0`. The Login Items pane is P20's.

### `isle setup`

18. **`isle setup` is built now with P14a's steps, in its final shape.**
    Each step has a check, an apply and an undo, and records what it changed
    in `/var/lib/isle/setup.json` (root's steps) or
    `~/.local/state/isle/setup.json` (the user's), so `--undo` reverses only
    what it did. `isle setup --check` lists what is due and asks nothing;
    `isle setup` applies the due steps with one authorisation, sudo with a
    terminal and pkexec without (Packaging 3 and 4). P14a's steps:
    - greetd's greeter is tuigreet (root; the old line kept);
    - the user's `portals.conf` becomes `hyprland-portals.conf` (user);
    - the xremap drop-in (user).
    P14b adds its own steps to the same table.

### Proof

19. **Tested headless, installed in the VM, used on the host.** Headless:
    the PKGBUILD's paths against `data/`, no checkout paths in a release
    build, the unset by value against a fake manager environment, the
    marker by session id, the autostart rules against fixtures, each setup
    step's check, apply and undo in a temporary root. The VM: the package
    built and installed in the guest, tuigreet listing both sessions, Isle
    signed into and out of, Hyprland after it with xremap running and v1's
    file chooser answering. The host, the user's: the same, then P12's and
    P13's pending host runs on the installed build.

## The workspace

```
pkg/arch/PKGBUILD         the package (new)
tools/package.sh          the checkout's HEAD packaged and installed (new)
data/
  session/isle.desktop    the wayland-sessions entry (new)
  udev/70-isle.rules      i2c uaccess (new)
  portal/isle.portal      FileChooser added for v1
crates/
  isle-session/           the unset by value, logs/, the journal, the new
                          variables
  isle-compositor/src/
    lock.rs               the marker by session id; the helper pinned
    recording.rs          the helper pinned
    spotlight.rs          the row index compiled in
    autostart.rs          Login Items' engine (new)
  isle-apps/              autostart entries read (the desktop entry parser
                          it already has)
  isle-cli/src/setup/     `isle setup`, its steps and their record (new)
```

## Phases

Each phase ends with its tests passing headless and a line in
`progress.md`.

### 14a.0 · The note and the roadmap

`docs/research/rust-desktop/session-2026.md`; the roadmap's P14 split into
P14a and P14b.

### 14a.1 · The binaries stand alone

The row index compiled in; the helpers pinned to the running build; the test
that no release build reads a checkout path.

### 14a.2 · The session beside v1

The session entry; the logs in `logs/` and the journal; the marker by session
id; the unset by value; `SSH_AUTH_SOCK` and the cursor variables.

### 14a.3 · Login Items

The autostart engine, its rules and its place in the start order.

### 14a.4 · The package

The PKGBUILD, its dependencies from `ldd` and the spawned programs, the
paths test, `namcap`, `tools/package.sh`.

### 14a.5 · `isle setup`

The step table with its record, `--check`, `--undo`, one authorisation; the
three steps.

### 14a.6 · The VM

The package installed in the guest; tuigreet with both sessions; Isle in and
out; Hyprland after it, intact.

### 14a.7 · The host and close

The user's run: install, `isle setup`, sign out of Hyprland, Isle from
tuigreet, P12's and P13's checklists, back to Hyprland. ARCHITECTURE gains
How Isle is installed; decisions recorded; `platform.md`'s answered points;
the roadmap's P14a and P14b; this sheet ends with its outcome.

## Not in P14a

The greeter mode, its published look and the session menu (P14b). The v1
import (P14b). Theming installed apps, fonts, Qt's route to the portal's
file dialog (P14b). `isle setup`'s keyring and fingerprint PAM lines,
autologin and groups (P14b). Updates through paru, `--devel`, restart owed
(P14b). A second GPU's connectors (P14b). The Login Items pane and session
restore (P20). Isle and Hyprland running at once on one user bus, beyond what
isle-tty does today: tuigreet makes them one after the other.

## Depends on, and feeds

- Needs P2 (the supervisor, the session target), P9 (the lock, its marker,
  `isle-auth`, the PAM file), P10 (the launch path), P11 (the portal files,
  `isle-record`), P13 (Settings' entry and service file).
- P15 installs Files' D-Bus services and FileChooser through this package;
  P20's Login Items pane reads this engine's list; P14b builds on the
  package, the setup table and the session.

## Done when

- `cargo test` passes headless, with the paths, checkout-path, unset,
  marker, autostart and setup tests.
- In the VM, the installed package gives a session chosen at tuigreet, and
  Hyprland after it keeps xremap, its file chooser and its log.
- On the host, the user signs into Isle from tuigreet on the installed
  package, and `isle setup --undo` puts greetd, the portal file and xremap
  back as they were.

## Risks

- **Skia's archive follows Freya.** Its URL and name are set by the pinned
  `skia-bindings`; a Freya bump that moves it fails the package build
  until `source=()` follows. The paths test reads the expected name from
  `Cargo.lock` so the failure shows here, not in makepkg.
- **makepkg's flags.** CachyOS adds `-C target-cpu=native` to `RUSTFLAGS`:
  right for this machine, wrong for a package built for another, which
  P14a does not do.
- **Typing a password at tuigreet.** The console's keymap is
  `/etc/vconsole.conf`'s, not Isle's per-keyboard profiles; a password with
  symbols on a Mac layout may type differently. 14a.6 and the host run
  check it.
- **xremap's condition.** If it is wrong, Hyprland loses its key remapping.
  The negative condition is the safe side; the VM checks Hyprland after
  Isle, and the step's undo removes the drop-in.
- **A release build's time.** A clean build in `makepkg` is long; the
  persistent target directory makes later ones short.

## Reviewed

The user reviewed the plan on 2026-10-19 and took every recommendation:
P14 split at the switch, tuigreet until the greeter mode, one `isle-git`
package with the version its commit, the coexistence fixes, the helpers
pinned, Isle's own Login Items launcher, `isle setup` in its final shape,
and the host run part of done.

## Outcome

Built 2026-10-19 in phases 14a.0–14a.7; the host run, part of done, is the
user's, with its checklist in `docs/research/rust-desktop/p14a-session.md`.
B100–B103 record the decisions. Differences from the plan:

- **The launch path's scopes are user scopes.** Choice 17 said Login Items
  run in the logind session's scope; every app Isle launches runs in a
  scope of its own under the user manager (P6), and polkit finds their
  session through logind's display session. `platform.md` says so.
- **The unset is by owner, then by value.** Choice 11 compared values
  alone; a session started since may hold the same values
  (`QT_QPA_PLATFORMTHEME=gtk3`), so nothing is unset unless the manager's
  `WAYLAND_DISPLAY` is still Isle's.
- **Login Items wait for the manager.** They start once the user manager
  names Isle's display and `graphical-session.target` is up, so a session
  that does not carry the manager starts none; isle-tty turns them off.
- **The compositor's Skia is built from source.** Its feature set has no
  prebuilt archive; Settings' does. The PKGBUILD builds the two apart,
  makes `gn`, `ninja` and `python` build dependencies, and keeps symbols
  (debug info stripped) so crash records name functions.
- **Additions.** `i2c-dev` loaded at boot beside the `uaccess` rule;
  `XCURSOR_THEME` and `XCURSOR_SIZE` sent by the compositor through the
  ready pipe from its tokens; the compositor's log lines without colour
  codes when not on a terminal; Settings' `--write-index` in debug builds
  only, since a release build has no checkout.

Found on the way: the capture test `a_shot_from_the_toolbar_holds_none_of_it`
fails four runs in five with or without P14a (offered as a separate task);
the guest kept a hand-copied `isle-portals.conf` from P11's run.

