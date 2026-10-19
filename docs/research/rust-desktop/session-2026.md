# Session and install, 2026

Evidence for P14 (`roadmap.md`, Session and install): greetd and a
compositor greeter, a second GPU's connectors, autostart, the session
environment, the package, and theming Qt and GTK apps from tokens. Read
2026-10-19 from sources and from this host (CachyOS, AMD Granite Ridge
card1/renderD128 with nothing connected, RTX 5070 Ti card2/renderD129
driving DP-4, nvidia-open 615.71.09). Sections are cited from the plan as
§n.

## 1. greetd and a compositor greeter

**The protocol** (`greetd-ipc(7)`, greetd 0.10.3 installed; source at
`d6733e9`, 2026-07-21). A Unix socket named by `GREETD_SOCK`, which greetd
puts only into the greeter's PAM environment. Each message is a 32-bit
native-endian length and a UTF-8 JSON object tagged by `type`.

- Requests: `create_session {username}`, `post_auth_message_response
  {response?}` (omitted for info and error), `start_session {cmd: [..],
  env: [..]}`, `cancel_session`.
- Responses: `success`; `error {error_type: auth_error|error,
  description}`, where `auth_error` is a wrong answer and not fatal;
  `auth_message {auth_message_type: visible|secret|info|error,
  auth_message}`, which must be answered by a response or a cancel. The
  number and order of messages are PAM's, not fixed.
- `start_session`'s `env` is put into the PAM environment before
  `pam_open_session`, so `XDG_SESSION_TYPE=wayland` and
  `XDG_CURRENT_DESKTOP=Isle` reach `pam_systemd` and the session. greetd
  adds `XDG_SEAT`, `XDG_SESSION_CLASS`, `USER`, `HOME`, `SHELL` and
  sources `/etc/profile` and `~/.profile` unless `source_profile = false`.

**`greetd_ipc` crate** 0.10.3 (2024-06-07, the protocol's version):
`Request`/`Response` serde enums; features `sync-codec` (blocking
read/write on a stream) and `tokio-codec`. GPL-3.0-only. The protocol is a
length prefix and serde_json, which Isle already has; a client of its own
is a few dozen lines and avoids the licence question if Isle is ever
published under another.

**Handoff.** After a successful `start_session` the greeter must exit;
greetd starts the session when it does (`context.rs`). Five seconds after
scheduling it sends SIGTERM every second, and SIGKILL once ten seconds
have passed. A greeter that exits without creating a session is treated
as an error. The session runs on the same VT; greetd sets the VT to text
mode and clears it before the session starts, so the screen goes from the
greeter to blank to the session *(a visible flash, unsure)*.

**PAM.** greetd's services are configurable: `[general] service`
(default `greetd`, used for user sessions), `[default_session] service`
(default `greetd-greeter`, falling back to the general one when no such
file exists) and `[initial_session] service` (default the general one).
greetd checks for a service in `/etc/pam.d` and `/usr/lib/pam.d`
(`server.rs`). The packaged `/etc/pam.d/greetd` is a pacman backup file
and on this host is already edited (keyring lines; `pacman -Qkk`). Setting
`[general] service = "isle-greetd"` in the greetd config `isle setup`
writes, with the package shipping `/usr/lib/pam.d/isle-greetd`, leaves
greetd's own file alone; cosmic-greeter does the same with
`service = "cosmic-greeter"`.

**Autologin.** `[initial_session] command = "isle-session"`, `user = …`;
run once per boot, guarded by the runfile `/run/greetd.run`; after logout
the greeter runs. No password is typed, so `pam_gnome_keyring` cannot
unlock the login keyring.

**How the greeters run.**

| Greeter | How it runs | Users | Its own state |
|---|---|---|---|
| tuigreet | in greetd's VT, no compositor | passwd, UID 1000–60000 | `/var/cache/tuigreet/lastuser`, `lastsession`, `-path` |
| gtkgreet | under cage or sway | typed | none *(unsure)* |
| ReGreet | under cage or sway | AccountsService `ListCachedUsers` (UserName, RealName, Shell) | `/var/lib/regreet/state.toml` (tmpfiles, owned by `greeter`) |
| cosmic-greeter | `cosmic-comp cosmic-greeter` as user `cosmic-greeter` | passwd filtered by `login.defs` UID_MIN/MAX, shell, systemd-homed | cosmic-config under the user's home `/var/lib/cosmic-greeter` |

cosmic-greeter builds the command from the session's desktop entry:
`/usr/bin/env XDG_SESSION_TYPE=wayland XDG_CURRENT_DESKTOP=<DesktopNames>
XDG_SESSION_DESKTOP=<first> <Exec split>`, and sends the same variables
as `env`. The users' appearance reaches it through
`cosmic-greeter-daemon`, a root system-bus service
(`com.system76.CosmicGreeter`, callable by group `cosmic-greeter`) that
drops to each user's uid and reads their theme, wallpaper bytes, xkb and
clock settings from their home; icons come from
`/var/lib/AccountsService/icons/<user>`.

**Where Isle's session can publish the greeter's choices** (platform.md,
open for P14):

- AccountsService (26.27.3 here) serves per-user extension interfaces: an
  XML in `/usr/share/dbus-1/interfaces/` with a symlink from
  `/usr/share/accountsservice/interfaces/` exactly
  `../../dbus-1/interfaces/<name>`. Properties only, stored in
  `/var/lib/AccountsService/users/<user>`; any caller may read by default;
  the user sets their own under polkit's
  `org.freedesktop.accounts.change-own-user-data`, which is yes for
  active, inactive and any. Good for scalars (light or dark, accent,
  layouts, main screen). The built-in `Session`, `SessionType`,
  `IconFile` and `Language` are there already.
- A wallpaper is a file in a home the greeter cannot read. Either a
  directory `isle setup` makes per user (for example
  `/var/lib/isle/greeter/<user>/`, owned by the user, world-readable,
  under a root-owned parent) holding a scaled copy, or a root helper like
  cosmic's. The directory needs no daemon.

## 2. A second GPU's connectors

**Smithay at the pin** (`10e6b3c`, 2026-10-10;
`backend/renderer/multigpu/`).

- `GpuManager<A: GraphicsApi>` holds a device per DRM node;
  `GbmGlesBackend` is the provided API (`add_node(render_node, gbm)`).
  `single_renderer(node)` gives a `MultiRenderer` on one GPU at no extra
  cost; `renderer(render, target, copy_format)` renders on one and binds
  framebuffers on the other.
- Copy path per frame: allocate on the render GPU a buffer whose
  modifiers both sides accept (render GPU's bind formats ∩ target's
  import formats, excluding `Invalid`), render into it, import it on the
  target and draw it there as a texture. If no such buffer can be made,
  "Falling back to cpu-copy": `ExportMem` readback on the render GPU and
  upload on the target, up to three merged damage rects, else the whole
  frame. `can_do_cross_device_imports` is false only for software EGL.
- Client dmabufs go the other way through `ImportDma`, with shadow copies
  (dma, then memory) when the buffer cannot be imported where it is drawn;
  `early_import` starts them on commit.

**niri** (`f51d761`, 2026-10-19). One render GPU: `debug {
render-drm-device }`, else `udev::primary_gpu` (boot_vga); `ignore-drm-device`
skips nodes. Every primary node is opened and added to the `GpuManager`
(software EGL rejected); each output renders with `renderer(primary,
device's own render node, format)`. A device without a render node gets
LINEAR-only render formats. Dmabuf feedback: the render tranche is the
render GPU; a scan-out tranche for the output's device, restricted to
LINEAR when that device is not the render GPU ("AMD iGPU + dGPU systems …
produce a glitched scanout if the modifier is not Linear"). Elements are
generic over a renderer trait with `as_gles_renderer()` for custom
shaders.

**cosmic-comp** (`742b800`). `GbmGlowBackend`. The primary GPU is
`COSMIC_RENDER_DEVICE`, else one with a connected internal panel, else the
boot GPU. Each output renders on the GPU that drives it, unless the
visible windows' buffers come from the primary GPU, in which case the
primary renders and copies (`render_node_for_output`).

**NVIDIA rendering, AMD or Intel scanning out.**

- niri #4374 (2026-07, NVIDIA render, Intel panel; a commenter has
  NVIDIA + AMD on a desktop): cross-vendor import fails for every format
  (NVIDIA block-linear against the other's tiling), so every frame takes
  the CPU copy, which ran a 1080p panel at 144 Hz. 10-bit failed: XB30
  readback is refused by NVIDIA's GLES, XR30/AR30 failed to allocate;
  niri moved to AB30 and Smithay #2123 (closed) touched the fallback's
  format. Isle's `COLOUR_FORMATS` are 8-bit only, which this path handles.
- The other direction works with LINEAR: KWin MR 3859 notes that into
  NVIDIA only LINEAR imports, and that NVIDIA cannot render to LINEAR
  ("external only"), which is why NVIDIA → AMD finds no shared buffer
  *(not tried on 615.71)*.
- A 4K ARGB frame read back is 33 MB; at 60 Hz about 2 GB/s across PCIe
  plus a CPU copy *(estimate, unmeasured)*. Damage keeps most frames
  smaller.

**Isle today** (`backend/udev.rs`). One `GlesRenderer`, built on the first
GPU opened; only the render GPU's primary node is opened ("not driving
this GPU's outputs"). `IsleElement`, `WaylandSurfaceRenderElement<GlesRenderer>`
and `GlesTexture` appear in 13 files.

**What it takes.**

- A. niri's shape: `GpuManager` instead of the one renderer, every
  primary node opened with its own `DrmOutputManager`, `renderer(nvidia,
  amd, format)` per AMD output, render code generic over a renderer trait
  (or typed to `MultiRenderer`, textures `MultiTexture`), `early_import`
  on commit, per-surface feedback as niri's. On this hardware the frame
  still takes the CPU copy.
- B. Smaller: keep the NVIDIA `GlesRenderer` and the element types; for an
  AMD output render offscreen on NVIDIA, read back the damage
  (`ExportMem::copy_framebuffer`), and give the pixels to a second
  `GlesRenderer` on AMD whose `DrmOutputManager` draws one uploaded
  texture. The same copy A falls back to, with no change through the
  render code; it gives up direct scan-out of client buffers on that
  output.
- B fits this host, where the AMD ports are spare. A is the route only if
  the iGPU is to render its own outputs.

## 3. XDG autostart through systemd

`systemd-xdg-autostart-generator` (systemd 262; `xdg-autostart-service.c`
at v262) reads the autostart directories (user overriding system by file
id) at manager start and on every `daemon-reload`, and writes
`app-<id>@autostart.service` into `generator.late/`, wanted by
`xdg-desktop-autostart.target`.

- Skipped: `Hidden=true`, `X-systemd-skip=true`, a missing or
  non-executable `TryExec`.
- `OnlyShowIn`/`NotShowIn` become `ExecCondition=systemd-xdg-autostart-condition`,
  checked against the manager's `XDG_CURRENT_DESKTOP` when the unit
  starts.
- `X-GNOME-Autostart-Phase`: the man page says no unit is made; the
  source removes GNOME from `OnlyShowIn`, adds it to `NotShowIn`, and
  skips the entry only when `OnlyShowIn` was GNOME alone. On this host
  `gnome-keyring-secrets.desktop` (Phase PreDisplayServer) has a unit
  with `ExecCondition=… "Unity:MATE:Cinnamon" "GNOME"`.
- `AutostartCondition=` and `X-KDE-autostart-condition=` call GNOME's and
  KDE's helpers.
- Each unit: `PartOf=` and `After=graphical-session.target`, `Type=exec`,
  `ExitType=cgroup`, `Restart=no`, `Slice=app.slice`.

`xdg-desktop-autostart.target` has `RefuseManualStart=yes` and
`StopWhenUnneeded=yes`: a desktop opts in with `Wants=` from one of its
own units. niri.service has `Wants=` and `Before=xdg-desktop-autostart.target`;
uwsm binds it from `wayland-session-xdg-autostart@.target`, started after
the compositor is up; Arch's sway snippet does not start it.

**Against platform.md.**

- Login Items would run as user units in `app.slice`, outside the logind
  session, which D17 measured: NetworkManager, udisks and logind refuse or
  ask an admin password from there. platform.md (Session start 4) has
  every app Isle launches in the session scope.
- After a compositor crash the supervisor keeps `graphical-session.target`
  up, so units whose apps died with their Wayland connection stay
  inactive (`Restart=no`) and the target never re-runs them.
  `systemctl --user start` of each generated unit would restart only the
  dead ones (a running unit is a no-op), but they would still be user
  units.
- A Login Item added from Settings needs a `daemon-reload` before a unit
  exists.
- Conditions follow the manager's `XDG_CURRENT_DESKTOP`, which §6 may
  make `Isle:GNOME`, so GNOME-only entries would start.

**Recommendation:** Isle's own launcher. Read the same directories with
the generator's rules (Hidden, X-systemd-skip, TryExec, OnlyShowIn and
NotShowIn against `Isle` alone, Phase treated as the generator does),
launch through `launch.rs` into the session scope, and keep each item's
pid and start time under the runtime directory so a compositor start
relaunches only items no longer running. Do not pull
`xdg-desktop-autostart.target`, or every item starts twice.

## 4. The session environment and the shared user manager

- **niri-session** (`resources/niri-session`): refuses if `niri.service`
  is active; `systemctl --user reset-failed`;
  `dbus-update-activation-environment --systemd --all` (the comment notes
  bare `import-environment` is deprecated); `systemctl --user --wait start
  niri.service`; on exit starts `niri-shutdown.target` and `unset-environment
  WAYLAND_DISPLAY DISPLAY XDG_SESSION_TYPE XDG_CURRENT_DESKTOP NIRI_SOCKET`.
- **uwsm** (`0727985`): refuses if `graphical-session-pre.target`,
  `graphical-session.target`, any `wayland-session*@` target or a
  `wayland-wm@` service is active or activating. It exports the
  difference the login shell made plus `PATH`, `XDG_CURRENT_DESKTOP`,
  `XDG_SESSION_*`, never seat or session ids, and records what it set in
  a cleanup list under the runtime directory, unset on stop (never the
  SSH agent variables).
- **gnome-session** (`faba9ab`, `leader-systemd.c`): exits with "A graphical
  session is already running!" if `graphical-session.target` is active.
  It uploads with systemd's `UnsetAndSetEnvironment`, which in one call
  clears a list (`DISPLAY`, `WAYLAND_DISPLAY`, `XAUTHORITY`, every `LC_*`
  and others) and sets its own, so a previous session's values never
  leak; it does not unset at logout.
- **Arch's sway** (`50-systemd-user.conf`): `set-environment
  XDG_CURRENT_DESKTOP=sway`, `import-environment` and
  `dbus-update-activation-environment --systemd` of `DISPLAY`, `SWAYSOCK`,
  `WAYLAND_DISPLAY`, `XDG_CURRENT_DESKTOP`; never unsets.
- systemd: one graphical session per user ("systemd only supports
  running one graphical session per user at a time",
  systemd.io/DESKTOP_ENVIRONMENTS). dbus-daemon cannot unset activation
  variables (`dbus-update-activation-environment(1)`, Limitations); with
  dbus-broker (37, this host) services are activated as systemd units, so
  the manager's environment is the one that counts.

No tool found checks who owns the variables before unsetting; all of
them refuse the second session instead. For Isle, whose dev loop runs
beside v1:

- At start, if `graphical-session.target` is active and the manager's
  `WAYLAND_DISPLAY` (`org.freedesktop.systemd1.Manager.Environment`) is
  set and is not ours, another session owns the manager: export nothing,
  start no target, say so in the log. Otherwise export with
  `UnsetAndSetEnvironment`.
- At exit, read `Environment` again and unset only the names whose value
  is still the one Isle set. Not atomic; the window is small.

## 5. The package

**A -git PKGBUILD** (`/usr/share/pacman/PKGBUILD-vcs.proto`, the Rust
package guidelines, archived 2026-04).

- `pkgname=isle-git`, `provides=(isle)`, `conflicts=(isle)`.
  `source=("isle::git+https://github.com/alexparlett/isle.git#branch=feature/rust-desktop")`
  (public); a `git+file://` URL to the checkout builds committed HEAD
  only. The repository has no tags, so `pkgver()` is `printf "r%s.%s"
  "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"` (today
  `r469.4070241`), which names the commit as platform.md wants.
- `prepare()`: `cargo fetch --locked --target "$(rustc --print
  host-tuple)"`. The guidelines' `export RUSTUP_TOOLCHAIN=stable` is
  "not required" with a `rust-toolchain.toml`, and here would override
  its pin (1.97.1); with the `rust` package instead of rustup the file is
  ignored and the system compiler must meet `rust-version = "1.97"`.
- `build()`: `export CARGO_TARGET_DIR=target`; `cargo build --frozen
  --release`. `--frozen` stops cargo's network use only: the Skia build
  script downloads a prebuilt archive with curl from
  `github.com/marc2332/rust-skia/releases` unless `SKIA_BINARIES_URL`
  (accepts `file://`) points at one listed in `source=()`.
  `check()` is `cargo test --frozen --workspace` if wanted.
- makepkg's `lto` (on in CachyOS's `OPTIONS`) only adds `-flto=auto` to
  `CFLAGS`/`LDFLAGS`; it reaches the C that `cc` builds in ring,
  libspa-sys, ffmpeg-sys-next, freya-skia-bindings, smithay and
  wayland-backend. Whether that breaks this link is untried; `options=(!lto)`
  is the safe default, and Rust's own LTO belongs in the Cargo profile.
  CachyOS's `RUSTFLAGS` add `-C target-cpu=native`, fine for this
  machine. `!debug` is already in CachyOS's `OPTIONS`.
- A rebuild in paru's clone directory reuses `src/isle/target`
  *(makepkg updates an existing git checkout in place, unsure)*.

**Split packages** (`pkgbase` with `package_a()` per name). Nothing here
installs on its own: the compositor, session, record and settings
binaries talk to each other and must come from one commit. One package
until something is optional.

**Where files go.**

| What | Path |
|---|---|
| user units (`isle-session.target`) | `/usr/lib/systemd/user/` |
| PAM (`isle`, `isle-greetd`) | `/usr/lib/pam.d/`; libpam here reads `/etc/pam.d/%s` then `/usr/lib/pam.d/%s`, and polkit and systemd already ship there; an `/etc` file replaces the whole service |
| session entry | `/usr/share/wayland-sessions/isle.desktop`, `DesktopNames=Isle` (greeters derive `XDG_CURRENT_DESKTOP` from it) |
| D-Bus session services | `/usr/share/dbus-1/services/<name>.service` |
| portal backend | `/usr/share/xdg-desktop-portal/portals/isle.portal` |
| portal preferences | `/usr/share/xdg-desktop-portal/isle-portals.conf` |
| NVIDIA profile | `/etc/nvidia/nvidia-application-profiles-rc.d/50-isle`, in `backup=()` |

- The NVIDIA driver searches `~/.nv/nvidia-application-profiles-rc{,.d}`,
  `/etc/nvidia/nvidia-application-profiles-rc{,.d}` and
  `/usr/share/nvidia/nvidia-application-profiles-<version>-rc` only
  (`profiles.html`); a `/usr/share/nvidia/…-rc.d/` directory is never
  read. Earlier files and rules win. `procname` matches the executable's
  basename, so `data/nvidia/50-isle.json` (`isle-compositor`,
  `GLVidHeapReuseRatio` 0) is right in form and covers greeter mode too.
- The driver's own file gives "No VidMem Reuse" to cosmic-comp, Hyprland,
  Xwayland, plasmashell and the DSOs libkwin, libmutter, libwlroots and
  libweston; CachyOS's nvidia-utils adds `limit-vram-usage` for niri,
  browsers and terminals. Nothing matches a Smithay compositor named
  `isle-compositor`. niri ships no profile; its wiki has users write the
  same JSON (`"Limit Free Buffer Pool On Wayland Compositors"`, value 0).
- v1 left an unowned `/usr/share/xdg-desktop-portal/portals/isle.portal`
  (FileChooser, `UseIn=Hyprland`); pacman refuses the package while it
  exists ("exists in filesystem").

**Portal preferences.** `portals.conf(5)` and `xdp-portal-config.c` (1.22.1,
installed, and main): locations in order `~/.config`, `$XDG_CONFIG_DIRS`
(`/etc/xdg`), `/etc`, `~/.local/share`, `$XDG_DATA_DIRS`, `/usr/share`. In
each, the first of `<desktop>-portals.conf` for each `XDG_CURRENT_DESKTOP`
entry lowercased (`Isle` → `isle-portals.conf`), else `portals.conf`.
In 1.22.1 and main every location's file is kept (1.19's notes first
mention the fallback rework), and per interface the first file whose
interface key, or else `default`, names an installed backend that
implements it wins. So a user `portals.conf` outranks the packaged
`isle-portals.conf` for every interface it can answer. v1's file
(`default=hyprland;gtk`, `FileChooser=isle`) would hand ScreenCast to
`hyprland.portal`, which is installed. Renaming it to
`~/.config/xdg-desktop-portal/hyprland-portals.conf` keeps it for v1's
session (`XDG_CURRENT_DESKTOP=Hyprland`) and out of Isle's.

**paru** (2.1.0). `Devel` is on in CachyOS's `/etc/paru.conf`. `--devel`
compares `git ls-remote` of each -git source (URL and branch) with the
commit recorded at install, without fetching the PKGBUILD; `--gendb`
records the current commit of installed -git packages, for migrating
from another helper. A PKGBUILD repository (`[isle]` with `Url =` the
repository, `Path =` a subdirectory) is scanned up to three levels,
refreshed with the sync databases (`paru -Sya` alone), outranks the AUR,
and needs a committed `.SRCINFO` or `GenerateSrcinfo`.

## 6. Qt apps and the portal's file panels

qtbase 6.11.0 and 5.15.2 sources; Qt 6.11.2 and 5.15.19 installed, with
the `gtk3` and `xdgdesktopportal` theme plugins for both and qt6ct 0.11.

- `QT_QPA_PLATFORMTHEME` names one plugin; Qt adds it ahead of the
  platform integration's names, which come from `XDG_CURRENT_DESKTOP`
  (`QGenericUnixTheme::themeNames`): `KDE` alone gives `kde`; GNOME,
  X-Cinnamon, Pantheon, Unity, MATE, XFCE and LXDE give `gtk3` then
  `gnome`; any other name gives itself lowercased; `generic` is last.
- `QT_QPA_PLATFORMTHEME=gtk3`: palette from the GTK 3 theme (D39); file
  dialogs are `gtk_file_chooser_dialog_new`, GTK's in-process widget, so
  `GTK_USE_PORTAL` (read by `GtkFileChooserNative`) does nothing.
- `xdgdesktopportal`: builds its base theme from the platform
  integration's names, skipping its own; with `XDG_CURRENT_DESKTOP=Isle`
  that is `isle` (no such plugin) then `generic`, which has no palette, so
  Qt's default Fusion palette. Palette, fonts, icons and hints come from
  the base; the FileChooser portal handles file dialogs whenever the portal
  answers, with the base's dialog kept as a fallback in 6.x (in 5.15 only
  for directories on FileChooser before version 3); colour scheme comes
  from the Settings portal. No `QT_QPA_PLATFORMTHEME_FALLBACK` exists.
- qt6ct reads its palette from a colour-scheme file and has
  `standard_dialogs=xdgdesktopportal`, but answers only for Qt 6 (qt5ct
  is a second plugin, key and file), the reason for D39.
- `kde`, from qtbase's `QKdeTheme` or plasma-integration, reads
  `kdeglobals`; it is chosen only for `XDG_CURRENT_DESKTOP=KDE` or by
  name, and its file dialogs are Qt's or KIO's in-process ones.

**Recommendation:** `XDG_CURRENT_DESKTOP=Isle:GNOME` with
`QT_QPA_PLATFORMTHEME=xdgdesktopportal`. Qt 5 and 6 then take `gtk3` as
the base, so the palette still comes from the GTK 3 theme Isle renders
(one source, D39), and file dialogs go to the portal, where Isle's panels
answer. To test in the VM, for side effects of `GNOME` in the list:

- xdg-desktop-portal still finds `isle-portals.conf` first in each
  location.
- Autostart entries with `OnlyShowIn=GNOME` would match the generator's
  condition, which §3's own launcher avoids by matching `Isle` alone.
- Chromium's password store picks libsecret for GNOME and the plain store
  for an unknown desktop *(unsure)*.
- GTK and Electron may take GNOME-only paths *(unsure)*.
- Whether the gtk3 base follows a live light/dark change under the
  portal theme *(unsure)*.

If GNOME in the list does harm, the fallback is the same pair set only in
the environment of processes the launcher sees linking `libQt5Gui` or
`libQt6Gui`.

## 7. GTK user CSS and light and dark

GTK 4.22 and 3.24.52 `gtksettings.c`, libadwaita 1.9.0
`adw-style-manager.c` and `adw-settings-impl-portal.c`; installed GTK
4.22.5, GTK 3.24.52, libadwaita 1.9.4.

- GTK 4.20 added `@media (prefers-color-scheme)`, `prefers-contrast` and
  `prefers-reduced-motion`. The user `~/.config/gtk-4.0/gtk.css` provider
  has its `prefers-color-scheme` bound to `gtk-interface-color-scheme`, so
  `@media` blocks there switch live. On Wayland GDK takes that setting
  from the Settings portal's `color-scheme`; libadwaita sets it from its
  own dark state, so it follows an app that forces dark too.
- Both GTKs read the user `gtk.css` once per process (a static provider
  made at the first settings init) and never again. Changing
  `gtk-theme-name` reloads only the theme provider (and that theme's
  imports); on Wayland GTK 4 takes it from the portal's
  `org.gnome.desktop.interface gtk-theme`. libadwaita draws its own
  stylesheet whatever the theme name.
- libadwaita's accent comes from the portal's `accent-color` mapped to the
  nearest of its named accents, live. A `@define-color accent_bg_color`
  in the user `gtk.css` (priority USER, above libadwaita's THEME) wins but
  is fixed until the app restarts.
- GTK 3 has no media queries *(an unknown at-rule is dropped, unsure)*;
  light and dark there is the theme name (`adw-gtk3` or `adw-gtk3-dark`).

**How others reload.** Omarchy sets `color-scheme` and `gtk-theme`
(Adwaita or Adwaita-dark) and lists GTK among apps that change "on their
next refresh or launch". matugen-themes writes `colors.css` imported from
`gtk.css`, and for GTK 3 sets `gtk-theme` to `""` then `adw-gtk3-{mode}`,
for GTK 4 flips `color-scheme` and back; matugen's own `reload_gtk_theme`
was removed. wallust does nothing for GTK. By the source, those toggles
reload the theme and re-evaluate `@media`, not the user `gtk.css`
*(whether a user file's new colours then show is untested)*.

**For Isle.** GTK 4 and libadwaita: one `gtk.css` with light and dark in
`@media` blocks; light and dark live, other token changes on the app's
next start. GTK 3 and GTK 4 without libadwaita: put the colours in
themes Isle owns (`~/.local/share/themes/Isle-light` and `Isle-dark`,
each importing adw-gtk3 and defining Isle's colours) and switch
`gtk-theme` between them, which reloads them live; an accent change in
the same mode toggles through a second name *(unsure until tried)*.

## 8. Rendering templates

crates.io, 2026-10-19:

| Crate | Version | Licence | Kind |
|---|---|---|---|
| minijinja | 3.0.1 (2026-10-15) | Apache-2.0 | runtime Jinja2; strict undefined; no required dependency beyond serde as listed |
| tera | 2.4.0 (2026-09-11) | MIT | runtime Jinja-like |
| askama | 0.16.1 (2026-09-04) | MIT/Apache-2.0 | templates compiled into the binary |
| upon | 0.11.0 (2026-07-25) | MIT/Apache-2.0 | small runtime engine, little used |
| tinytemplate | 1.2.1 (2021) | MIT/Apache-2.0 | unmaintained |

- askama does not fit platform.md's templates-as-data under
  `/usr/share/isle/templates/`.
- v1's seven templates use only `{{name}}` (about 40 names), no
  conditions or loops. v1's regex replacer failed on an unknown name but
  passed a malformed placeholder through: this host's
  `~/.config/qt6ct/qt6ct.conf` still has a literal `"{{font_mono},10,…`.

**Recommendation:** Isle's own replacer: `{{name}}` from the token map
(dotted names for the dark set of §7's `@media` blocks), an error on an
unknown name and on any `{{` or `}}` left after rendering, so a
half-written file is never installed. minijinja with strict undefined
is the step up if a template needs a condition or a loop.

## Sources

Read 2026-10-19. greetd `d6733e9` (git.sr.ht/~kennylevinsen/greetd:
`greetd/src/{server,context}.rs`, `session/worker.rs`, `config/mod.rs`,
`greetd_ipc/`), `greetd-ipc(7)`, `greetd(5)`; cosmic-greeter `5b912ba`;
ReGreet `290e6f8`; tuigreet `5199ce5`; accountsservice `88e3de5`
(`src/user.c`, `daemon.c`, `user-classify.c`) and `accounts-daemon` 26.27.3
here. Smithay fork `10e6b3c` (`~/.cargo/git/checkouts`); niri `f51d761`
(`src/backend/tty.rs`, `resources/`, `docs/wiki/`), niri issues #4374,
#4708, #550, Smithay #2123; cosmic-comp `742b800`
(`src/backend/kms/`); KWin MR 3859 (invent.kde.org). systemd 262
`systemd-xdg-autostart-generator(8)`, `systemd.special(7)`,
`src/xdg-autostart-generator/xdg-autostart-service.c` at v262,
systemd.io/DESKTOP_ENVIRONMENTS; uwsm `0727985`; gnome-session `faba9ab`;
Arch sway packaging `50-systemd-user.conf`;
`dbus-update-activation-environment(1)`. `PKGBUILD-vcs.proto`,
`PKGBUILD-split.proto`, Arch Rust package guidelines (wiki archive
2026-04), `paru(8)`, `paru.conf(5)`, `/etc/makepkg.conf{,.d/rust.conf}`,
`/usr/share/makepkg/buildenv/lto.sh`; freya-skia-bindings 0.100.0-beta.1
`build_support/binary_cache/`; libpam strings; NVIDIA `profiles.html`
and `nvidia-application-profiles-615.71.09-rc`; xdg-desktop-portal
`8eba842` and 1.22.1 `src/xdp-portal-config.c`, `portals.conf(5)`. qtbase
v6.11.0 (`qxdgdesktopportaltheme.cpp`, `qgenericunixtheme.cpp`,
`qdesktopunixservices.cpp`, `qguiapplication.cpp`, `qgtk3theme.cpp`,
`qgtk3dialoghelpers.cpp`) and v5.15.2; qt6ct `qt6ctplatformtheme.cpp`.
GTK 4.22.0 and 3.24.52 (`gtk/gtksettings.c`,
`gdk/wayland/gdksettings-wayland.c`), libadwaita 1.9.0, GTK issue 6064;
Omarchy `81fd2d2` (`bin/omarchy-theme-set{,-gnome}`), matugen `9f8c7fd`
(`CHANGELOG.md`), matugen-themes `db7dcfe`, wallust `a09480b`. crates.io
API for template crates and greetd_ipc.
