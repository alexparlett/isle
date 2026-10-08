# Passkeys on Linux, and what Isle can do: state as of 2026-10-03

Research for the question "can Isle make passkeys work like on a Mac":
Touch ID-style passkeys (synced, or at least device-bound), plus a phone and
security keys. Read against B12 (the desktop owns the polkit agent, keyring
prompter, secret agents and its own lock), B13 and B26 (Isle's own portal
backend, owned by the compositor's service worker), B17 (Mac expectations)
and B28 (the Bluetooth agent).

Every claim carries a source and a date. Git repositories were read through
a clone or the GitHub API on 2026-10-03; commit and release dates are given.
Anything not verified is under **OPEN** at the end and nowhere else.

---

## 0. The machine (read from the host, 2026-10-03)

| What | Reading | Consequence |
|---|---|---|
| systemd | 262 | `60-fido-id.rules` runs `fido_id` on every hidraw node and sets `ID_SECURITY_TOKEN`; `70-uaccess.rules` line 52 tags `ID_SECURITY_TOKEN` with `uaccess`. A FIDO key's hidraw node is handed to the active seat session by logind. No extra udev rule is needed. |
| libfido2 | not installed; Arch's `libfido2` 1.17.0 (2026-04-18) ships no udev rules ([file list](https://archlinux.org/packages/extra/x86_64/libfido2/files/)) | systemd's rules are the only ones; the old `70-u2f.rules` is not part of Arch anymore. |
| Browsers | `chromium` 153.0.8010.52, `vivaldi` 8.2; no Firefox, no Google Chrome | Neither installed browser can sign in to Chrome Sync (§5), so neither has Google Password Manager passkeys. |
| Bluetooth | BlueZ 5.87, `hci0` powered, central and peripheral roles; the D-Bus policy lets any session process call `org.bluez` (`/usr/share/dbus-1/system.d/bluetooth.conf`, `<policy context="default">`) | Hybrid's prerequisites are met without configuration. |
| TPM | `/dev/tpm0`, `/dev/tpmrm0` present, `root:tss 0660`; user not in `tss`; `tpm2-tss` 4.2.0 installed, `tpm2-tools` not | A TPM-backed authenticator needs either `tss` membership or a privileged daemon, or `systemd-creds --user` (§3.6). |
| Fingerprint | fprintd 1.94.5 installed; `fprintd-list` reports "No devices available"; `lsusb` shows no reader | This desktop has no fingerprint sensor. "Touch ID" here means a libfprint-supported USB reader or a biometric security key; without one, user verification is the login password or a key's PIN. |
| `/dev/uhid` | `root:root 0600` | A virtual-HID authenticator (§3.5) needs a privileged daemon or a udev rule. |
| Password managers | Proton Mail and Proton VPN installed; no Proton Pass, 1Password, Bitwarden or KeePassXC package | `platform.md` already lists Proton Pass as a vault provider at parity. |

---

## 1. Roaming authenticators (security keys)

### 1.1 Browsers

- **Chromium and Chrome**: USB HID keys work with Chromium's own UI for touch,
  PIN entry, PIN setting and credential choice. Chromium's `device/fido/`
  tree (main, read 2026-10-03,
  [gitiles](https://chromium.googlesource.com/chromium/src/+/main/device/fido/))
  has `hid/` and `cable/` but no NFC transport and no BLE security-key
  transport; its platform-authenticator directories are `mac/`, `win/` and
  `cros/` only. On Linux: USB keys and hybrid; nothing else.
- **Firefox**: USB FIDO2/CTAP2 keys including PIN since Firefox 114
  ([release notes](https://www.firefox.com/en-US/firefox/114.0/releasenotes/)),
  through Mozilla's `authenticator-rs` with Firefox's own PIN and touch UI.
  Current release is 157 (Arch, 2026-09-29). Firefox stores no passkeys of
  its own (meta bug [1792433](https://bugzilla.mozilla.org/show_bug.cgi?id=1792433),
  NEW, 2026-07-31).
- **WebKitGTK (Epiphany)**: no usable WebAuthn in any release. Igalia landed
  test gardening for "[WPE][GTK] Support WebAuthn" on 2026-07-24
  ([WebKit bug 205350](https://bugs.webkit.org/show_bug.cgi?id=205350),
  [PR #70116](https://github.com/WebKit/WebKit/pull/70116)); "[GLib] Allow
  building Webauthn support" is open and routes every request to a stub that
  rejects with NotSupportedError
  ([PR #72235](https://github.com/WebKit/WebKit/pull/72235), opened
  2026-08-23). A hybrid and GLib HID transport series is filed, not merged
  ([bug 320957](https://bugs.webkit.org/show_bug.cgi?id=320957), 2026-08-04).
- **Flatpak browsers** need `--device=all` to reach hidraw; the FOSDEM 2026
  talk calls this the current workaround and a sandbox break
  ([slides](https://archive.fosdem.org/2026/events/attachments/838A8N-credentials-for-linux-bringing-passkeys-to-linux/slides/267430/fosdem_ravbuub.pdf)).

### 1.2 Electron

Electron's WebAuthn delegate leaves account choice and PIN collection to the
embedding app. On Linux, a request that needed a key's PIN crashed the
browser process (CachyOS report, Electron 43.4.0,
[electron#54317](https://github.com/electron/electron/issues/54317)); the fix
makes it reject instead and was merged to main, 43, 44 and 45 on 2026-09-25
([#54355](https://github.com/electron/electron/pull/54355),
[#54404](https://github.com/electron/electron/pull/54404)). A
`collect-webauthn-pin` session event
([#53351](https://github.com/electron/electron/pull/53351)) and a hybrid
event ([#53733](https://github.com/electron/electron/pull/53733)) are open,
not merged. Electron merged macOS Touch ID and system passkeys
([#51412](https://github.com/electron/electron/pull/51412), 2026-05-01;
[#51563](https://github.com/electron/electron/pull/51563), 2026-08-06), with
no Linux counterpart. In practice: touch-only keys work in Electron apps on
Linux; PIN keys fail; phones do not work.

### 1.3 Native apps

Native programs use libfido2 over hidraw directly (OpenSSH `-sk` keys,
`pam_u2f`, `systemd-cryptenroll --fido2-device`). They draw their own
prompts or none: OpenSSH asks for the PIN and shows the touch reminder
through `SSH_ASKPASS`. Isle could serve that role the way it serves the
polkit agent, but no D-Bus API exists for it.

### 1.4 PIN and biometric keys

CTAP2 keys with a PIN work in Chromium and Firefox, which draw the PIN
field. Biometric keys (on-key fingerprint, "built-in UV") verify on the key
and the browser asks only for a touch. Neither browser exposes an API that
lets the desktop draw the PIN or touch prompt.

---

## 2. Hybrid transport (a phone as the authenticator)

- **Chrome and Chromium on Linux** support it: Google's support table states
  Chrome on every platform supports cross-device sign-in
  ([developers.google.com](https://developers.google.com/identity/passkeys/supported-environments),
  page updated 2025-05-19). It needs a powered Bluetooth adapter (BLE
  proximity) and internet on both ends (the tunnel service, CTAP 2.3
  [hybrid section](https://fidoalliance.org/specs/fido-v2.3-ps-20260226/fido-client-to-authenticator-protocol-v2.3-ps-20260226.html#sctn-hybrid)).
  No pairing is involved, so Isle's `org.bluez.Agent1` (B28) plays no part.
  Android and iPhone both work as the phone.
- **Known Linux failures**: Chromium powers the adapter through
  `org.bluez.Adapter1.Powered`; if Bluetooth was turned off with an rfkill
  soft block, BlueZ refuses and the browser cannot undo it. Omarchy's fix
  turns Bluetooth off with `Powered=false` instead
  ([omarchy PR #13867](https://github.com/omacom/omarchy/pull/13867), open
  2026-10-02). A community repo blames iPhone failures on a stale BlueZ
  discovery filter and Intel AX211 firmware scanning
  ([fix-iphone-passkey-linux](https://github.com/shawnmbrooks85/fix-iphone-passkey-linux),
  unvetted). Chromium's tracker has an open "Passkey from an Android phone
  never completes" report
  ([41485480](https://issues.chromium.org/issues/41485480)).
- **Firefox**: not shipped. Bug
  [1543674](https://bugzilla.mozilla.org/show_bug.cgi?id=1543674) (hybrid
  for platforms on authenticator-rs, i.e. Linux) is ASSIGNED, and Michael
  Farrell is implementing it: the Noise channel and handshake and the Base10
  QR encoder landed in mozilla-central on 2026-09-17 (target 158,
  [2055392](https://bugzilla.mozilla.org/show_bug.cgi?id=2055392),
  [2057377](https://bugzilla.mozilla.org/show_bug.cgi?id=2057377)); the
  BLE advertisement listener
  ([2059618](https://bugzilla.mozilla.org/show_bug.cgi?id=2059618)) and the
  authenticator-rs adapter
  ([2059619](https://bugzilla.mozilla.org/show_bug.cgi?id=2059619)) are
  WIP; one piece is blocked on an `nss-rs` release (comment 2026-09-18).
  Firefox has no Bluetooth stack; the bug says it will target the WebSocket
  path first.
- **Electron**: none (§1.2). **WebKitGTK**: none (§1.1).
- **Outside the browser**: `libwebauthn` (§3.2) implements hybrid, including
  "remember this phone" linking, and `credentialsd` exposes it over D-Bus.

---

## 3. A platform authenticator, or OS credential API, on Linux

### 3.1 The situation

Linux has no OS WebAuthn API. Chromium has no Linux platform authenticator
and no OS-bound key on Linux: `crypto/user_verifying_key.cc` returns a
provider only for Windows and macOS, and `crypto/unexportable_key.cc` only
for Windows and Apple (main, read 2026-10-03,
[gitiles](https://chromium.googlesource.com/chromium/src/+/main/crypto/)).
Firefox's maintainer wants the OS to own this: in bug 1543674 (2025-01-08)
John Schanck asks for "something like Windows Hello" on Linux that Firefox
could call. The cross-desktop answer under way is Credentials for Linux.

### 3.2 Credentials for Linux: who and what

- **Organisation**: [github.com/linux-credentials](https://github.com/linux-credentials).
  Core contributors named at FOSDEM 2026: Isaiah Inuwa (Bitwarden), Martin
  Sirringhaus (SUSE), Alfie Fresta (speaker)
  ([slides](https://archive.fosdem.org/2026/events/attachments/838A8N-credentials-for-linux-bringing-passkeys-to-linux/slides/267430/fosdem_ravbuub.pdf),
  2026-02-01). No institutional funding is stated anywhere found. The org
  also now hosts `oo7` (the Rust Secret Service that GNOME 51 adopted, §3.7).
- **libwebauthn**: a Rust FIDO2/U2F client library. Transports: USB HID,
  BLE, NFC (pcsc or libnfc), hybrid; TPM 2.0 platform authenticator
  "planned" (slides; [issue #4](https://github.com/linux-credentials/libwebauthn/issues/4),
  open since 2020, updated 2026-03-09). Releases roughly every two weeks:
  v0.5.0 (2026-05-18, CTAP 2.3 hybrid over BLE L2CAP), v0.7.0 (06-07,
  largeBlob), v0.9.0 (07-01, origin-authorisation and attestation fixes),
  v0.10.0 (2026-07-14); last push 2026-09-19
  ([releases](https://github.com/linux-credentials/libwebauthn/releases)).
- **credentialsd**: the D-Bus service and the proposed portal. LGPL-3.0,
  Rust, zbus 5 on tokio. Releases: v0.1.0 2025-08-15 (USB and hybrid,
  "pre-production"), v0.2.0 2026-02-21 (NFC, PRF, window handles), v0.3.0
  2026-08-28 (restructured as a portal handler; needs a patched
  xdg-desktop-portal), v0.3.1 2026-09-07
  ([releases](https://github.com/linux-credentials/credentialsd/releases)).
  Main at `1f2c8ea` (2026-09-28) is adding cancellation and transport
  restarts. Packages: RPMs on OBS for Fedora and openSUSE; AUR
  `credentialsd` 0.3.1 (2026-09-12), whose PKGBUILD does not pull in the
  patched xdg-desktop-portal that 0.3.x needs
  ([AUR PKGBUILD](https://aur.archlinux.org/cgit/aur.git/tree/PKGBUILD?h=credentialsd)).

### 3.3 credentialsd's architecture (clone of main, 2026-10-03)

```
browser or app
  -> org.freedesktop.portal.experimental.Credential     (xdg-desktop-portal, patched fork)
  -> org.freedesktop.handler.portal.experimental.Credential   (credentialsd: Gateway)
       credentialsd drives USB / NFC / hybrid through libwebauthn
  -> org.freedesktop.impl.portal.experimental.Credential      (the desktop's UI)
```

- **Frontend** (in xdg-desktop-portal): `CreateCredential(parent_window,
  origin, type, options) -> handle` and `GetCredential(parent_window, origin,
  options) -> handle`, Request-object pattern, plus capability properties
  (`HybridTransport`, `PasskeyPlatformAuthenticator`,
  `UserVerifyingPlatformAuthenticator`, `ConditionalGet`, `RelatedOrigins`
  and the signal* methods) (`doc/xyz.iinuwa.credentialsd.Credentials.xml`).
- **Handler**: xdg-desktop-portal delegates to credentialsd. credentialsd
  accepts calls only from a trusted caller: today, xdg-desktop-portal at a
  known path in its own mount namespace (`credentialsd/src/gateway/mod.rs`,
  `should_trust_app_id`). The trusted caller is meant to vouch for the app's
  identity and allowed origins; currently any client may ask for any origin
  after explicit user confirmation (SECURITY.md; slides).
- **UI backend, which is what a desktop implements**: credentialsd calls
  `org.freedesktop.impl.portal.experimental.Credential` at
  `/org/freedesktop/portal/desktop` on the bus name
  `xyz.iinuwa.credentialsd.UiControl`. The name is a hard-coded
  `default_service` in `credentialsd/src/dbus/ui_control.rs`, so a desktop
  that replaces the reference UI today must own that name. The interface, as
  served by the reference UI (`credentialsd-ui/src/dbus.rs`, about 1,100
  lines of Rust and zbus):
  - `CreateSession(session_handle, parent_window, origin, type, devices,
    app_id, app_pid, options)`, which opens the sheet; plus
    `org.freedesktop.impl.portal.Session.Close` and the `Closed` signal.
  - Calls into the UI: `NotifyNeedsPin(attempts_left)`, `NotifyPinNotSet`,
    `NotifyNeedsUserVerification`, `NotifyNeedsUserPresence`,
    `NotifySelectingCredential`, `NotifyHybridStarted` (QR payload passed
    over a memfd), `NotifyHybridConnecting`, `NotifyHybridConnected`,
    `NotifyUsbConnected`, `NotifyNfcConnected`, `Notify*Restarting`,
    `NotifyCeremonyCompleted`, `NotifyErrorOccurred`.
  - Signals back: `DiscoveryRequested`, `ClientPinEntered(pin_fd)`,
    `SetDevicePin(pin_fd)`, `CredentialSelected(id)`. PINs travel as file
    descriptors, never as D-Bus strings.
  - The XML under `doc/` and parts of `doc/api.md` describe the older
    `UiControl` and `FlowControl` shapes and are stale against the code.
- **Where credentials live**: nowhere yet. credentialsd mediates external
  authenticators (USB, NFC, phone). There is no platform authenticator in
  the code: `credential_service/` has `usb.rs`, `nfc.rs` and `hybrid.rs`
  only. The README's "internal platform authenticator (device PIN)" flow is
  a mockup. Issue
  [#8](https://github.com/linux-credentials/credentialsd/issues/8) ("Implement
  platform authenticator", open) wants it in a separate process; the FOSDEM
  roadmap lists a "TPM-backed platform authenticator, PIN or biometrics" and
  "remember this phone" storage as next.
- **Third-party providers** (Bitwarden, 1Password, KeePass): designed but not
  built. [#26](https://github.com/linux-credentials/credentialsd/issues/26)
  carries a draft (2025-11-15) for `CredentialProvider` and
  `CredentialProviderManager` interfaces, with registration only through a
  trusted settings app and a signed or Noise-protected channel. 1Password
  gave input; the Bitwarden contributor says Bitwarden will implement it
  once the spec settles
  ([#1](https://github.com/linux-credentials/credentialsd/issues/1),
  2025-05-25).

### 3.4 Upstream status of the portal

- **xdg-desktop-portal**: "Introduce Credentials portal (experimental)"
  ([PR #1889](https://github.com/flatpak/xdg-desktop-portal/pull/1889)),
  opened 2026-01-23, ported to libdex 2026-08-05, not merged. Sebastian Wick
  (2026-07-10) questions letting credentialsd call the impl directly instead
  of the usual frontend-to-impl shape, so the backend interface may still
  move. Merging waits on an "experimental portals" mechanism
  ([issue #2066](https://github.com/flatpak/xdg-desktop-portal/issues/2066);
  doc PR #2129 opened 2026-09-04) whose naming (path
  `/org/freedesktop/portal/desktop/experimental`, interface suffix) was still
  under debate on 2026-09-24. Until then the Credentials portal lives in
  [linux-credentials/xdg-desktop-portal](https://github.com/linux-credentials/xdg-desktop-portal)
  (three commits on top of upstream, 2026-06-26), enabled by
  `XDG_DESKTOP_PORTAL_ENABLE_EXPERIMENTAL`. The original request is
  [issue #989](https://github.com/flatpak/xdg-desktop-portal/issues/989)
  (2019).
- **Firefox**: [bug 1983767](https://bugzilla.mozilla.org/show_bug.cgi?id=1983767)
  "Implement support for xdg-credentials-portal (credentialsd) on Linux",
  ASSIGNED to Martin Sirringhaus;
  [D261536](https://phabricator.services.mozilla.com/D261536) is "Needs
  Revision" with merge conflicts, last updated 2026-07-22; reviewers asked
  for the C++/Rust split to be reworked. A patched Firefox Flatpak (153.0.3)
  ships with credentialsd releases.
- **Chromium**: no upstream work found (OPEN). The credentialsd web
  extension has a Chromium manifest
  ([PR #138](https://github.com/linux-credentials/credentialsd/pull/138)),
  which the maintainer says is "broken again" (issue
  [#25](https://github.com/linux-credentials/credentialsd/issues/25),
  2026-08-06).
- **Web extension (the stopgap)**: overrides `navigator.credentials.create`
  and `get` in every page and forwards them through a Python
  native-messaging helper to the portal frontend
  (`webext/app/credential_manager_shim.py`). It ignores conditional mediation
  (autofill-style passkey offers). It replaces the browser's own key and
  hybrid handling for every site.
- **WebKitGTK**: Isaiah Inuwa offered credentialsd to WebKit in bug 205350
  (2025-12-05). PR #72235's stub says "a platform specific implementation
  will come next"; which one is OPEN.

### 3.5 Software authenticators that pose as security keys (uhid)

A different route that needs no browser change: a daemon creates a virtual
USB FIDO device through `/dev/uhid`. systemd's `fido_id` tags it like any
key, and every browser, Electron and libfido2 sees a security key.

| Project | Lang | Keys | User verification | State |
|---|---|---|---|---|
| [psanford/tpm-fido](https://github.com/psanford/tpm-fido) | Go | TPM P-256 per registration | pinentry presence | U2F-era; last push 2024-05-11 |
| [matejsmycka/linux-id](https://github.com/matejsmycka/linux-id) | Go | TPM; resident keys in `~/.config/linux-id/creds.json` | pinentry, or `--auth fprintd` which sets UV | CTAP2; AUR 0.2.4 (2026-09-20) |
| [pando85/passless](https://github.com/pando85/passless) | Rust (soft-fido2) | `pass`, TPM (experimental) or file | desktop notification; optional PIN | AUR 0.20.0 (2026-09-23); needs user in a `fido` group with rw on `/dev/uhid` |
| [baseman70/cable-uhid-bridge](https://github.com/baseman70/cable-uhid-bridge) | Rust | none; bridges hybrid to a virtual key | phone | 9 stars, 2026-09 |
| llavero, tpm-fido2-thinkpad-linux, gaze-fido | Go, Rust | TPM-sealed | fprintd or face | weeks old, single authors |

Properties of this route, which follow from CTAP and the browsers' behaviour:
the credential reports `authenticatorAttachment: cross-platform`;
`isUserVerifyingPlatformAuthenticatorAvailable()` stays false, so sites do
not offer "create a passkey on this device"; the browser's sheet says
"security key". `/dev/uhid` can create any HID device, keyboards included,
so granting a user or group write access to it hands out input injection.

### 3.6 TPM and user verification building blocks

- `systemd-creds --user` (systemd 256+, man page on the host) encrypts a
  credential to the calling user with the TPM and host key without the
  caller touching `/dev/tpmrm0`. The key is decrypted into the daemon's
  memory when used, so this seals a credential store; it does not give
  non-exportable keys. Keys that never leave the TPM need direct TPM access
  (`tss` group or a system service).
- fprintd's D-Bus interface is what B12's lock already uses; linux-id's
  `--auth fprintd` shows it serving as WebAuthn UV.

### 3.7 GNOME, KDE, freedesktop

- **GNOME 51** (2026-09-16) adds FIDO2/passkey login to GDM's new
  multi-mechanism login, aimed at corporate setups
  ([OMG! Ubuntu](https://www.omgubuntu.co.uk/2026/09/gnome-51-released);
  [release notes](https://release.gnome.org/51/) speak only of "multiple
  authentication mechanisms"). That is OS login, not WebAuthn for browsers.
  GNOME 51 also moves to oo7 for passwords and keys (release notes), which
  B12's prompter already expects. No GNOME Shell UI for the Credentials
  portal was found (OPEN).
- **KDE**: only a brainstorm thread about storing FIDO2 keys in KWallet
  ([KDE Discuss](https://discuss.kde.org/t/feature-store-fido2-keys-in-kwallet/21278)).
  No Plasma credentialsd backend found (OPEN).
- **freedesktop specs**: none beyond the portal PR. oo7 has no passkey work
  (its only related issue, #121, is about fingerprint unlock, closed).

---

## 4. Password-manager passkeys on Linux

All of them work through a browser extension, in any browser the extension
supports. None can act as a system platform authenticator on Linux, because
no system API exists to register with (§3.3).

| Manager | Passkeys on Linux | Unlock on Linux | Notes |
|---|---|---|---|
| 1Password | browser extension ([support](https://support.1password.com/save-use-passkeys/), 2026-09-11) | "unlock using system authentication", a polkit action; with `pam_fprintd` on `polkit-1` the polkit dialog takes a fingerprint ([nicolasauler/nixos PR #28](https://github.com/nicolasauler/nixos/pull/28), merged 2026-09-24) | AUR 8.12.38 (2026-10-01). Desktop passkey export is not available ("only … iOS and Android"). |
| Bitwarden | browser extension | desktop app unlocks through polkit ([biometrics help](https://bitwarden.com/help/biometrics/); [clients PR #4586](https://github.com/bitwarden/clients/pull/4586)); the extension's biometric unlock needs the vault unlocked once per session | Credential-provider integration exists on Windows (Entra login, [2026-03-04](https://bitwarden.com/blog/log-into-windows-with-a-bitwarden-passkey/)); a Bitwarden engineer leads credentialsd. |
| Proton Pass | browser extension; Android 14+ and iOS 17+ as the system provider ([support](https://proton.me/support/pass-use-passkeys)) | OPEN | Passkeys sync across Proton devices. The Linux desktop app (AUR 1.38.2) is not listed for passkeys. |
| KeePassXC | KeePassXC-Browser since 2.7.7 ([2024-03-10](https://keepassxc.org/blog/2024-03-10-2.7.7-released/)); passkeys export and import as `.passkey` JSON | polkit "quick unlock" lands in 2.8.0 (beta1 2026-09-23, [releases](https://github.com/keepassxreboot/keepassxc/releases); [PR #8983](https://github.com/keepassxreboot/keepassxc/pull/8983)) | Sync only through whatever syncs the database file. |
| Google Password Manager | Google Chrome only (§5) | GPM PIN | |
| gnome-keyring, oo7, KWallet | none | — | |

The pattern that matters for Isle: 1Password, Bitwarden and KeePassXC 2.8
unlock through **polkit**, and B12 makes Isle the polkit agent, so their
"Touch ID" prompt is already an Isle-drawn sheet with fprintd.

---

## 5. Synced passkeys

- **Google Password Manager**: Chrome "with a signed in profile on macOS,
  Windows, Linux" creates, stores and syncs GPM passkeys
  ([Chrome blog](https://developer.chrome.com/blog/passkeys-gpm-desktop),
  2024-09-19), shared with Android and protected end to end by the GPM PIN.
  It needs **Google Chrome**: Google cut third-party Chromium builds off the
  Sync API on 2021-03-15
  ([BleepingComputer](https://www.bleepingcomputer.com/news/google/google-to-kill-chrome-sync-feature-in-third-party-browsers/)),
  which rules out Arch's `chromium` and Vivaldi. Chromium has no Linux
  user-verifying or TPM key provider (§3.1). So on Linux the device holds no
  OS-bound key, and user verification cannot be OS biometrics; that it falls
  to the GPM PIN is an inference from the code (OPEN for the exact UI).
  `google-chrome` is in the AUR (154.0.8037.97, 2026-10-02).
- **iCloud Keychain**: no Linux client. An iPhone serves its passkeys to
  Linux only as the phone in hybrid (§2).
- **Password managers** (§4) are the cross-platform sync route: Proton
  Pass, 1Password and Bitwarden sync to the phone and serve passkeys there
  natively.
- **CXF/CXP**: CXF 1.0 is a FIDO Proposed Standard (errata 2026-03-09); CXP
  is still a Working Draft dated 2024-10-03
  ([FIDO downloads](https://fidoalliance.org/download-credential-exchange-specifications/)).
  Apple ships CXF same-device transfer in iOS and macOS 26; Google Password
  Manager on Android imports and exports through Credential Exchange from
  Play services 26.21 (June 2026,
  [Android Authority](https://www.androidauthority.com/google-password-manager-import-export-support-3673351/));
  Bitwarden Android imports from GPM. Transfers are app-to-app on one device
  through OS APIs. Linux has no such OS mediator, and no Linux desktop app
  was found offering CXF import or export (OPEN).

---

## 6. What it would take for Isle

Isle controls the compositor, the portal backend (B13, B26), the polkit
agent, the keyring prompter and the lock (B12), and Bluetooth (B28). Ranked
by feasibility.

### (a) Security keys and phones out of the box: feasible now, small

What works today with no patches: Chromium, Chrome and Firefox with USB
keys including PIN keys; Chromium and Chrome with Android and iPhone
through hybrid; Electron with touch-only keys; libfido2 tools.

What Isle has to get right:
1. **A logind seat session.** uaccess on the key's hidraw node needs Isle's
   session, started by greetd (B26), to be the active session on `seat0`
   (§0). Check in the VM with `loginctl show-session` and `getfacl
   /dev/hidraw*` while a key is plugged in.
2. **Bluetooth "off" is `Powered=false`, never an rfkill soft block**, so a
   browser can power the adapter for hybrid (omarchy's failure, §2). The
   Mac shape is "Chrome wants to turn on Bluetooth", answered once. This
   belongs in the Bluetooth service crate and Control Centre.
3. Optional: an `SSH_ASKPASS` that draws OpenSSH's PIN and touch prompts as
   Isle sheets; `pam_u2f` as an extra factor in Isle's PAM service for the
   lock and greeter.

What Isle cannot do here: draw the browser's PIN, touch or QR prompt.
Chromium and Firefox draw their own, and no API hands them to the desktop.
The risk is BlueZ flakiness on some adapters (§2), outside Isle's control.

### (d) Synced passkeys through a password manager: feasible now

What works today: Proton Pass, 1Password, Bitwarden or KeePassXC passkeys
in Chromium, Vivaldi, Chrome and Firefox through their extensions, synced
to the phone. Unlock goes through polkit for 1Password, Bitwarden and
KeePassXC 2.8, so the fingerprint or password sheet is Isle's (B12) and
reads like Touch ID. Proton Pass fits the user's existing Proton accounts
and `platform.md`'s vault provider, but its Linux unlock path is unverified
(OPEN). Google Chrome adds GPM passkeys shared with Android (PIN-verified).

Needs no patches. Risks: the extension's own UI sits in the page rather than
in a system sheet; Electron and native apps get nothing; this machine needs
a USB fingerprint reader for the "touch" half (§0).

### (b) credentialsd as the platform layer, Isle as its UI: feasible to prototype, not to ship

What Isle would build: its portal backend owns
`xyz.iinuwa.credentialsd.UiControl` on the service worker's connection
(B26) and implements `org.freedesktop.impl.portal.experimental.Credential`
and the Session interface (§3.3) in zbus. That is the same stack as
credentialsd-ui's 1,100 lines, which serve as the reference. The shell
draws one Mac-style sheet for "touch your key", the PIN field, the phone QR
code with its connecting states, and the account picker, parented to the
requesting window through the portal's `parent_window` handle. Isle's app
index supplies the app name that credentialsd-ui reads from `.desktop`
files. The packages would be `credentialsd`, the patched
xdg-desktop-portal fork in place of Arch's, and the extension or patched
Firefox.

What works through it today: USB, NFC and hybrid in Firefox with the
extension or the patched Flatpak, and in native apps that call the portal.
Chromium's extension is currently broken (§3.4), and Chromium and Vivaldi
are this machine's browsers.

Risks: the backend interface is explicitly experimental and the
xdg-desktop-portal reviewers may change its direction (PR #1889); replacing
the distro's xdg-desktop-portal with a fork; the extension intercepts every
page's WebAuthn calls; the Firefox patch needs a rework; no platform
authenticator or provider API exists yet, so it adds a system sheet but not
"passkeys on this Mac". The upside is that this is the one route on which
GNOME, KDE, Firefox and Bitwarden converge, and its stack (Rust, zbus 5,
tokio, portal pattern) is Isle's own (B11, B13).

### (c) Isle's own platform authenticator: feasible, but the wrong place to spend effort

Two forms.
- **c1, a uhid virtual key** (§3.5): an Isle daemon, TPM-sealed keys, UV
  drawn by Isle (fprintd or the login password through PAM, B12) as a Touch
  ID sheet, and resident credentials. Works in every browser, Electron app
  and libfido2 tool today, unpatched. But it presents as a security key
  (cross-platform attachment, the browser's "security key" wording, no
  platform-authenticator signal to sites); it is device-bound with no sync;
  it needs a privileged helper for `/dev/uhid`, which is a keyboard
  injection primitive; and Isle would own CTAP2 authenticator code and key
  custody. Existing implementations are young (passless, linux-id).
- **c2, a TPM platform authenticator inside credentialsd**: what upstream
  plans (§3.3). Done there, it becomes the shared answer rather than Isle's
  private one.

### Recommendation

1. Do (a) now, in the packets that already own the pieces: a seat-active
   session (greeter), `Powered=false` for Bluetooth off (service crates and
   Control Centre), and the polkit agent drawn with fprintd. Verify a key
   and a phone in Chromium in the VM with USB and Bluetooth passed through.
2. Rely on (d) for the synced, Touch ID-like experience: a password manager
   whose unlock is Isle's polkit sheet, with Proton Pass first if its Linux
   unlock path checks out (OPEN), otherwise 1Password or Bitwarden. Google
   Chrome for GPM passkeys is the alternative for an Android-centred user.
   This machine needs a libfprint USB reader or a biometric key for the
   fingerprint half.
3. Design the credential sheet now as a portal-backend role (b): one Freya
   sheet for key, PIN, phone QR and account choice, behind an off-by-default
   flag, against credentialsd's pinned v0.3.x interface. Ship it on by
   default when the Credentials portal merges in xdg-desktop-portal and
   Firefox or Chromium calls it. Record it as a role beside B12's when
   adopted.
4. Do not build (c1). If device-bound platform passkeys are wanted before
   upstream has them, contribute the TPM and fprintd platform authenticator
   to credentialsd (c2) rather than shipping an Isle-only uhid daemon.

---

## OPEN

Not verified; do not treat as fact.

- **How Chrome on Linux verifies the user for GPM passkeys.** The code shows
  no OS user-verifying or TPM key provider on Linux; that it prompts for the
  GPM PIN each time, or accepts the Google account password, is inferred, not
  seen.
- **Proton Pass on Linux**: whether the desktop app or extension can unlock
  through polkit or system authentication, and whether the desktop app
  serves passkeys outside the extension.
- **Chromium upstream plans** for a Linux credentials portal or credentialsd
  backend. Nothing found on the issue tracker, which needs JavaScript to read.
- **Which platform implementation WebKitGTK will use** after PR #72235
  (credentialsd, its own HID and hybrid series in bug 320957, or both).
- **GNOME Shell or KDE Plasma backends** for the experimental Credential
  portal, and GNOME 51 GDM's FIDO2 login mechanism (SSSD passkey, pam_u2f,
  or its own).
- **Reliability of Chromium hybrid with this host's adapter** (Foxconn
  0489:e10a on USB, BlueZ 5.87), Android and iPhone. Not tried.
- **Whether greetd-started Isle sessions get uaccess ACLs** on hidraw in the
  VM. Expected from logind's rules, not yet checked.
- **Whether credentialsd's backend interface keeps its shape** if the
  xdg-desktop-portal reviewers move it to the usual frontend-to-impl
  pattern (PR #1889, 2026-07-10 comment).
- **CXF import or export in any Linux desktop password manager.** None found.
- **Bitwarden's macOS desktop passkey provider.** A forum thread says it was
  not shipped as of 2026-06; not checked against release notes.
