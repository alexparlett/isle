# Widgets and providers, 2026

Evidence for P17 (`docs/work/rust-desktop/p17-widgets.md`): how a provider
is sandboxed on this host, how the registry and v1's store carry over, where
each built-in widget's data comes from, and what the tree has today.
Gathered 2026-10-28 on the host (CachyOS, kernel 7.2.10) and from the tree at
`c447b0c`.

## 1. The sandbox

B28 left bubblewrap or Landlock open; `platform.md` Providers 5 asks for
network hosts, paths and bus names, each granted or revoked by the user.

**On the host.** `bwrap` 0.13.1 (already pulled in by glycin, loupe and
webkit2gtk), `xdg-dbus-proxy` 0.1.9 and `pasta` are installed; the LSMs are
`capability,landlock,lockdown,yama,bpf`. Unprivileged user namespaces work:

```
bwrap --unshare-all --die-with-parent --new-session --clearenv \
  --ro-bind /usr /usr --tmpfs /usr/bin \
  --ro-bind /usr/bin/python3 /usr/bin/python3 --ro-bind /usr/bin/sh /usr/bin/sh \
  --symlink usr/lib /lib --symlink usr/lib64 /lib64 --symlink usr/bin /bin \
  --proc /proc --dev /dev --tmpfs /tmp  python3 -c …
```

ran Python with `/usr/bin` holding only `sh` and `python3` (a symlink bound
as its target), no `ls` on the path, and a loopback interface to bind on.

**What each grant needs.**

| Grant | Landlock | bubblewrap |
|---|---|---|
| Read a path | Yes (`LANDLOCK_ACCESS_FS_READ_*`) | `--ro-bind` |
| Read and change a path | Yes | `--bind` |
| Connect to a host | No: ABI 4+ filters TCP ports, never names or addresses | No by itself: `--unshare-net` is all or nothing |
| Use a bus name | No | No by itself; `xdg-dbus-proxy --filter --talk=<name>` on a socket bound in, as Flatpak does |
| Run a command | Only as execute rights on paths | `--tmpfs /usr/bin` and bind the commands granted |

Neither filters hosts. The two ways to give "Connect to api.open-meteo.com"
meaning:

- **The host fetches** (v1 D31): the provider never has a network and asks
  Isle to fetch. A provider is then not "a program in any language" but one
  that speaks a fetch protocol; curl and `requests` would not work in it.
- **A filtering proxy.** The sandbox has no network but loopback. The runner
  serves an HTTP CONNECT and plain-HTTP proxy on a Unix socket bound in,
  allowing only granted hosts on 80 and 443; inside, a relay on
  `127.0.0.1:<port>` forwards to that socket, and `HTTPS_PROXY`,
  `HTTP_PROXY` and `ALL_PROXY` point at it. curl, Python's `urllib` and
  `requests`, Node's fetch (with `NODE_USE_ENV_PROXY`), Go's `net/http` and
  git honour these. The proxy resolves names, so the sandbox needs no DNS. A
  program that ignores the variables gets no network: denied, as a revoked
  grant is (Providers 5). This keeps D31's intent (Isle decides every
  connection) with the provider free to use its own client.

The relay must be a process inside the sandbox's network namespace. The
smallest form is a helper of Isle's own (`isle-provider-init`) as the
sandbox's first process: it opens the relay, starts the provider as its
child and exits with it. pasta could join the namespace instead but filters
nothing, so it would need the same proxy behind it.

Landlock adds nothing bubblewrap's mount namespace does not already give for
paths, cannot express the other three, and is a restriction the process
applies to itself, so a launcher is needed anyway.

**Limits.** `platform.md` Providers 4: a time limit and an output cap per
call, killing the process group. A stream has no call to time; its cap is
per line. Memory: a transient user scope (`systemd-run --user --scope -p
MemoryMax=`) per provider, or `RLIMIT_AS`, which breaks runtimes that
reserve address space (Node, Go). Systemd scopes are already how Isle
launches apps (B102).

**Paths never granted.** Isle's own config and state (`~/.config/isle`,
`~/.local/state/isle`) and the provider folders, writable: a provider that
could write them could grant itself.

## 2. v1's store and the registry

From `isle-v1-inventory.md` §6 and the registry checkout
(`~/Development/isle-widgets`):

- Sources are git repositories with `index.json` (`{name, widgets:[{id,
  path} | {id, repo, ref, …}]}`), cloned shallow under
  `~/.cache/isle/widget-sources/`, pulled hourly; installs copy into staging
  and swap in, leaving `.isle-install.json` (source, version, ref); updates
  compare semver. v1 ran the `git` CLI (`shell/scripts/widgetstore.py`).
- The registry lists six widgets: analog-clock, system-rings, transport,
  battery, focus-timer (QML), uptime (a `source` command into the `text`
  view, the one provider-shaped entry). Sizes are on v1's 12 × 8 grid
  ("3x2").
- **v1 still reads the registry.** v1 is the daily desktop until M2 and its
  Browse reads the registry's `index.json`; entries it cannot load would
  show there as broken. Isle 2's index has to sit beside it until P21.
- Manifest fields worth keeping: `id, name, glyph, category, description,
  version, author, license, homepage, sizes, default, min, max, multiple,
  permissions, settings[{key,label,type,min,max,default}], screenshots`.
  Gone with QML: `editor`, `trust`, `requires`, `view` (now `template`).

## 3. The built-ins' data

| Widget | Source today | Missing |
|---|---|---|
| Clock | jiff, the system zone (`night_light.rs`), tzdata via `isle-sun` | City names for zones (tzdata's own names) |
| Calendar | The month grid (`centre.rs`) | Calendars: P20 |
| Weather | None; v1 used Open-Meteo with curl | A fetch on the worker (`reqwest` is already in `isle-media`); the place from the zone's coordinates (`isle-sun`) |
| Media | `isle-media`, the Now Playing module | Nothing |
| Timer | `ActivityKind::Timer`, `EventKind::TimerDone` in the island's model, posted only by `Test1` | A timer service |
| System | None (Settings › General reads `/proc/meminfo` once) | A sampler: `/proc/stat`, `/proc/meminfo`, `/proc/net/dev`, `/proc/uptime`, hwmon; the GPU through NVML on NVIDIA, sysfs on amdgpu and i915. Monitor (P19) needs the same |
| Devices | `isle-power`, `isle-bluetooth`, the Batteries module | Nothing |
| Storage | `isle-disks` (UDisks2), removable disks only, no used or free | The system volume; `statvfs` for used and free (as `isle-fs` does) |
| Desktops | `Isle::mission_screens()` | Nothing |
| Controls | `Module` data (`island/panel.rs`), the catalogue (`control_edit.rs`) | The `Tile` component is private to the panel and sends `PanelAction`s |
| Session | `prompts/session.rs` `Choice`, `end_session`, `isle-power::Profiles` | Nothing |
| Notifications | `NotificationsState` entries and senders | Nothing |

NVML: `libnvidia-ml.so` ships with the driver; `nvml-wrapper` loads it at
run time, so the build does not link it and a machine without it shows no
GPU series.

## 4. The tree today

Nothing of the dashboard exists. What does:

- The shortcut `dashboard` (`isle-keys/src/shortcuts.rs:75`, marked P17) and
  `shortcuts.dashboard` = Ctrl+Option+Cmd+W (`isle-settings/src/schema.rs:459`),
  running `Action::Reserved`.
- Tokens: `size.widget-cell`, `type.style.figure`, the `series` colours,
  `softened` (the wallpaper under the lock and the dashboard).
- `softened_elements` (`wallpaper/mod.rs:319`) and `render_locked`
  (`lock.rs:184`), which draws the island's privacy form over the lock: the
  dashboard's pass follows it.
- Control Centre's edit mode: packing (`island/grid.rs`), the gallery and
  tile drags (`island/panel.rs`), the edits in the compositor
  (`control_edit.rs`), opened from Settings through `Island1.EditControls`.
- Keyed settings by output identity (`wallpaper_output`), list values as
  encoded strings (`control_centre.modules` as `id:size`). `Value` holds
  only Bool, Int, Float, Str and lists of strings, and a keyed map needs a
  typed `Section`, so a provider's own settings have no home yet.
- No overlay exclusion in one place: each opener closes the others by hand
  (`spotlight.rs:414`, `mission.rs:502`, `switcher.rs:139`,
  `force_quit.rs:197`, `session_end.rs:176`, `toolbar.rs:446`).
- No wallpaper menu, no `Corner::Widgets`, no Spotlight action, no Desktop
  rows, no Providers page (Privacy & Security has the two portal rows).
- No sandbox, runner, manifest parser or grant store; no git crate. `gix`
  0.90 has a blocking HTTP transport on reqwest with rustls
  (`blocking-http-transport-reqwest-rust-tls`), the stack `isle-media`
  already pulls in, and shallow clones.

## 5. macOS

macOS's widgets (interactive since macOS 14) sit on the desktop and in
Notification Centre, act through App Intents (buttons and toggles that run
one intent without opening the app), and refresh on a timeline the system
budgets. Where they sit is P1's answer for Isle; what carries over is that a
widget's interaction stops at buttons and toggles running one action, with
no free input, and that refresh is the host's to schedule.
