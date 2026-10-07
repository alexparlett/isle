# Services, 2026-10

Evidence for P8's plan: the crates and daemons behind network, Bluetooth,
power, audio, brightness, media, the notification server and the tray, as
they stand on 2026-10-07. Extends `ui-and-services.md` §3, which it corrects
where marked.

## The host

NetworkManager 1.58.1, BlueZ 5.87, UPower 1.91.4, power-profiles-daemon
0.30, PipeWire 1.6.9 with WirePlumber 0.5.18, ddcutil 3.0.2. There is no
`/sys/class/backlight`: the desktop's only brightness path is DDC/CI.
`i2c_dev` is loaded, ddcutil's `60-ddcutil-i2c.rules` tags display buses
`uaccess`, and the user is in `i2c`.

Isle has no licence file and publishes nothing (`publish = false`).

## Correction to §3: libpulse brings libdbus

Arch's `libpulse.so.0` links `libdbus-1.so.3` through
`libpulsecommon-17.0.so`, so `libpulse-binding` puts a second D-Bus stack in
the process, which B11 forbids. `libpipewire-0.3.so` does not link it, but
the default `client.conf` loads `libpipewire-module-rt`, which does; its
condition is `module.rt = !false`, so the context property `module.rt =
"false"` should keep it out (untested).

In P8 8.2 the client context also needed `support.dbus = false`, and the
compositor turned out to carry libdbus already: the NVIDIA EGL driver dlopens
it when EGL starts (seen with `LD_DEBUG=libs`, headless on the 5070 Ti). So
the rule holds for what Isle links and loads itself, not for the GPU driver.

Routes to audio without libdbus:

| Route | State | Notes |
|---|---|---|
| `pulseaudio` (colinmarc) 0.3.1, 2025-11-30, MIT | Pure-Rust client of the Pulse native protocol; async, runtime-agnostic; repo pushed 2026-07 | Subscribe, sink, source, sink-input, source-output info, volume, sample upload and play are in its protocol layer; its high-level `Client` does not wrap volume or subscribe yet. `cpal`'s `pulseaudio` feature uses it |
| `pipewire` (pipewire-rs) 0.10.1, 2026-08-19 | FFI to libpipewire; its own main loop on a thread | COSMIC now does audio this way (`cosmic-pipewire`, MPL, inside the GPL `cosmic-settings-audio-server`). Volume through device Route params is what `wpctl` does; setting a node's `channelVolumes` directly is not the same (OPEN in §3) |

pipewire-pulse implements the whole subscription mask, so per-app streams
(sink inputs) and the microphone in use (source outputs) are visible over
the Pulse protocol. Peak meters (pavucontrol, GNOME's) are source outputs
too and are skipped by `application.id`.

wayle-audio runs a tokio port of libpulse's main loop inside
`spawn_blocking`; COSMIC's old code ran libpulse's own loop on a thread.

## Camera in use

No Pulse-level signal: cameras are PipeWire video nodes. GNOME Shell 45
(MR !2840) watches the registry for nodes with `media.role == "Camera"` and
marks the camera in use while one's state is `running`, clearing after a
short delay; Plasma's camera indicator watches PipeWire too. `media.class
== "Video/Source"` alone also matches screencast nodes. Apps that open
`/dev/video*` directly (Chromium and Electron by default, OBS's V4L2 source,
Zoom) are invisible to every desktop; there is no kernel signal (a sysfs
`streaming` attribute was proposed in 2023, unmerged). A best effort is an
inotify open and close watch on `/dev/video*` followed by a scan of the
user's `/proc/*/fd`, ignoring PipeWire's own probes.

## Brightness

- DDC/CI crates are unmaintained: `ddc-hi` 0.4.1 (2021; 0.5.0 unpublished,
  last push 2023), `ddc-i2c` 0.2.2 (2023), `ddc` 0.3.0 (2024), MIT. They need
  read and write on `/dev/i2c-N`.
- `ddcutil-service` 1.0.15 (2026-06, GPL-2.0) serves `com.ddcutil.DdcutilService`
  on the session bus (GetVcp, SetVcp): a separate daemon.
- A DDC write takes tens of milliseconds, a read with its retries up to about
  a second (v1 inventory §3.7): writes are debounced and serialised per bus.
- A laptop backlight is set without root by
  `org.freedesktop.login1.Session.SetBrightness("backlight", name, value)`
  (systemd 243+), on the caller's own session.

## Notification sounds

No Rust binding to libcanberra exists. `freedesktop-sound` 0.1.0 (2025)
resolves theme names; the lookup is small (`$XDG_DATA_DIRS/sounds/<theme>/
index.theme`, `Inherits`, then `freedesktop`, `.oga`, `.ogg`, `.wav`).
cosmic-notifications advertises `sound` and plays nothing; Otto shells out
to `pw-cat`. Without libpulse the choices are the Pulse protocol's sample
upload and play, a PipeWire stream with `media.role = "Notification"`, or
rodio over cpal.

## NetworkManager

- Scan with `Device.Wireless.RequestScan`; follow `LastScan` and
  `AccessPointAdded`/`Removed`. Access points carry `Ssid`, `Strength`,
  `Flags`, `WpaFlags`, `RsnFlags`, `Frequency`.
- A known network: `ActivateConnection(conn, device, ap)`. A new one:
  `AddAndActivateConnection2(settings, device, ap, {"persist": "disk"})`
  with `key-mgmt` `wpa-psk` (or `sae`); without a `psk` an agent is asked.
- With no secret agent registered, activation fails with NO_SECRETS. The
  agent (`SecretAgent` at `/org/freedesktop/NetworkManager/SecretAgent`,
  registered with `AgentManager.RegisterWithCapabilities`) gets
  `GetSecrets` with `ALLOW_INTERACTION` and `REQUEST_NEW` after a wrong
  password.
- 1.58: `settings.modify.system` is `auth_admin_keep`, with Arch's polkit
  rule letting local `wheel` through; Wi-Fi PSK prompts only on the last
  retry; a WPA3-SAE mismatch re-prompts.

## BlueZ

`Adapter1.StartDiscovery` is one session per client and ends on
`StopDiscovery` or when the client leaves the bus. Devices arrive through
ObjectManager. `Battery1.Percentage` is absent while unknown; headset levels
come from PipeWire through `BatteryProviderManager1`. Pairing is
`Device1.Pair()`; a code needs an `Agent1` registered with
`AgentManager1.RegisterAgent(path, "KeyboardDisplay")` and
`RequestDefaultAgent`.

## UPower and power profiles

- `DisplayDevice` is the system's batteries only. Peripherals come from
  `EnumerateDevices` and `DeviceAdded`/`Removed`, by `Type`: 5 mouse, 6
  keyboard, 12 gaming input, 13 pen, 14 touchpad, 17 headset, 18 speakers,
  19 headphones, 21 other audio, 22 remote, 26 wearable, 27 toy, 28
  Bluetooth generic. UPower reads BlueZ's `Battery1` and the kernel's HID
  batteries itself. Coarse devices report `BatteryLevel`, not `Percentage`.
- power-profiles-daemon 0.30 at `org.freedesktop.UPower.PowerProfiles`:
  `HoldProfile(profile, reason, app_id) → cookie` for `performance` or
  `power-saver` (power-saver wins), ended by `ReleaseProfile`, by the
  holder leaving the bus, or by the user setting a profile, which emits
  `ProfileReleased`. `ActiveProfileHolds` lists them.

## MPRIS

Players are `org.mpris.MediaPlayer2.*` (browsers append `.instanceN`),
found with `ListNames` and `NameOwnerChanged` on that namespace. `Position`
is never in PropertiesChanged: it is read on a status, metadata or rate
change and on `Seeked`, and extrapolated by `Rate` while playing; some
players fail the read for live streams. `mpris:artUrl` is `file://`
(Chromium's temporary files), `https://` (Spotify) or occasionally `data:`.

## The notification server

Spec 1.3 adds `ActivationToken(id, token)`, sent before `ActionInvoked`.
Hints: `action-icons`, `category`, `desktop-entry`, `image-data`,
`image-path`, `resident`, `sound-file`, `sound-name`, `suppress-sound`,
`transient`, `x`, `y`, `urgency`, and the legacy `image_data`,
`image_path`, `icon_data`. A timeout of -1 leaves it to the server, 0 means
never. KDE's extensions: `Inhibited` with `Inhibit`/`UnInhibit`; the
`inline-reply` action with `NotificationReplied(id, text)`;
`x-kde-origin-name`.

- Chromium wants `body` and `actions` or falls back to the portal; sends -1
  when `persistence` is advertised (else 25000); `requireInteraction` sends 0
  and critical urgency; sends `desktop-entry`, `image-path`,
  `suppress-sound`; uses `inline-reply` and `ActivationToken` when present.
- Electron, through libnotify: -1, or 0 for `timeoutType: 'never'`;
  `desktop-entry` and `sender-pid`.
- Firefox, through libnotify: -1, or 0 for `requireInteraction`;
  `suppress-sound`, `desktop-entry`.

## The tray

- `system-tray` 0.8.9 (2026-08, MIT, zbus 5) is host and watcher, taking
  over the watcher name when another drops; its DBusMenu proxy is the one
  B50 rejected.
- Items export `org.kde.StatusNotifierItem`, registered by unique name and
  path or by well-known name (Chromium 152+ uses an
  `org.freedesktop.StatusNotifierItem-*` bus name). Electron 43.3.0 broke
  registration by unique name (fixed in 43.4.1, 2026-08-19).
- Discord's Electron tray is SNI, falling back to XEmbed with no watcher.
  Steam uses libappindicator, so SNI with a DBusMenu menu (unconfirmed).
  Wine and Proton apps are XEmbed only (Wine's SNI driver, MR !2808, is
  unmerged); `xembedsniproxy` bridges them.

## Bindings

`pop-os/dbus-settings-bindings` (git only, last commit 2026-06-02, zbus
5.14) holds zbus proxies for NetworkManager, NM's AgentManager, BlueZ
(Adapter1, Device1, Agent1, AgentManager1, Battery1), UPower, MPRIS,
accounts, hostname1, locale1, timedate, geoclue2 and switcheroo; MPL-2.0
except `upower_dbus` (MIT). The crates.io names of the same crates are stale
publishes from 2023 and 2024, and `bluez-zbus` on crates.io is an unrelated
crate.

## Sources

github.com/pop-os/dbus-settings-bindings; github.com/pop-os/cosmic-settings-daemon;
github.com/wayle-rs/wayle-services; github.com/colinmarc/pulseaudio-rs;
gitlab.gnome.org/GNOME/gnome-shell MR 2840; github.com/arcnmx/ddc-hi-rs;
github.com/digitaltrails/ddcutil-service; freedesktop.org login1, MPRIS and
notification specifications; networkmanager.dev API and NEWS; bluez/bluez
`doc/`; gitlab.freedesktop.org upower and power-profiles-daemon;
github.com/jakestanger/system-tray; Chromium's
`notification_platform_bridge_linux.cc`, Electron's
`libnotify_notification.cc`, Firefox's `nsAlertsIconListener.cpp`;
gitlab.winehq.org wine MR 2808. Read 2026-10-07. Steam's tray mechanism and
pipewire-pulse's subscription coverage are from memory, not checked.
