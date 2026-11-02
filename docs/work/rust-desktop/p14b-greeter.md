# P14b · The greeter

Planned 2026-11-01. Reviewed by the user the same day: every
recommendation taken, the console keys kept as the way out.

P14b was the switch: the greeter, the v1 import, theming installed apps,
the rest of `isle setup`, updates, a second GPU. At the user's choice
(2026-11-01) it is only the greeter now, built before the parity packets
end, and the rest is P14c.

Today greetd runs tuigreet (P14a choice 2), a text screen listing Isle and
Hyprland. The lock already draws the face P1 drew for both (`p1/surfaces/lock.md`),
but it only knows the user who is signed in: the name comes from
`getpwuid(getuid())` (`lock_face.rs:91`), the password goes to `isle-auth`,
which authenticates its own uid against the `isle` service
(`isle-auth/src/main.rs:29`), and the face reads notifications, media,
Focus and the settings store, none of which exist before anyone signs in.
Under greetd it is greetd that runs PAM; the greeter only answers its
questions over a socket.

The lock's field also drops every key held with Ctrl, Option or Cmd
(`lock.rs:436`), so Cmd+A, Cmd+V and Cmd+Backspace do nothing. The greeter
uses the same field, so P14b fixes it for both.

What P14b must get right is that the greeter can never be the reason the
machine cannot be signed into. The escape hatch is the console keys, which
Isle already maps from the physical chord (`binds.rs:274`), locked or not:
Ctrl+Option+F2 (with Fn while the Keychron's F-row sends media keys) gives
logind's text login on tty2, where `sudo isle setup --undo` brings tuigreet
back. A greeter that dies before it reads a key cannot take that chord, so
it also falls back to tuigreet on its own.

## What it answers to

- Decisions: B26 (the greeter is the compositor in a greeter mode, reusing
  the lock), B12 and B67 (Isle's PAM services), B66 (the lock), B100–B103
  (the package and `isle setup`).
- Intent: `platform.md` Session start and greeter 1–2 (3's fingerprint and
  autologin are P14c's), Lock 4; `compositor.md` Session 1.
- Design: `p1/surfaces/lock.md` (the greeter's states, the user list, the
  session capsule, Greeter: user chosen, Greeter to session),
  `p1/structure/lock.md`.
- Evidence: `session-2026.md` §1 (the protocol, the handoff, PAM services,
  how other greeters run, where a session can publish its look); the host
  read on 2026-11-01 (greetd 0.10.3, `/etc/pam.d/greetd` with the keyring
  lines, no `greetd-greeter` service, `greeter` uid 951 with home `/`,
  three session entries, one AccountsService user).

## Choices this plan makes

### Scope

1. **The greeter and the field.** P14b: the greeter mode, its greetd
   client, its PAM services, the look the session publishes for it, the
   fallback to tuigreet, the `greeter` setup step, and the field's
   editing chords on the lock and the greeter. P14c keeps fingerprint
   sign-in and autologin (they come with the PAM lines), the v1 import,
   theming, updates and the second GPU.

### How the greeter runs

2. **`isle-compositor --greeter`, run by greetd as `greeter` on tty1.** One
   binary (B26). greetd's `[default_session] command` runs it; the udev
   backend takes the seat through libseat from the logind session greetd
   opens for the greeter. No `isle-session` supervisor: it would export a
   session environment and start `isle-session.target` for a user who has
   no session. greetd restarts the greeter when it exits.
3. **The greeter is the lock and nothing else.** It starts locked and never
   unlocks. No Wayland socket, no Xwayland, no clients, no session bus, no
   D-Bus names, no portal, no settings store, no state under a home, no
   shortcuts but the layout switch and the console keys (Ctrl+Option+F1 to
   F12), which are the way out if the greeter misbehaves. The worker runs only the system-bus
   services the face needs: logind (power, the seat), UPower (the battery)
   and AccountsService (users). A `Mode` on `Isle` decides this at start;
   the lock's code paths that reach notifications, media, Focus or
   `end_session` take the greeter's branch instead of being stubbed in
   place.
4. **The greeter's own files.** `XDG_CACHE_HOME=/var/cache/isle-greeter`
   (fontconfig and the shader cache) and `/var/lib/isle-greeter/` for its
   state: the last user, each user's last session, and the crash count of
   choice 15. Both made by a `tmpfiles.d` file in the package, owned by
   `greeter`, mode 0700. The greeter sets the variable itself, so greetd's
   command line stays one word and a flag.

### Signing in

5. **Isle's own greetd client, on the worker.** A module in `isle-auth`'s
   library: the 32-bit native-endian length, serde JSON, the four requests
   and three responses (`session-2026.md` §1). Not the `greetd_ipc` crate,
   which is GPL-3.0-only and saves a few dozen lines. The conversation runs
   on the worker as `isle-auth`'s attempts do; the field's password crosses
   to it in `Zeroizing` and nothing logs it.
6. **PAM's conversation drives the face.** `create_session` with the
   chosen user. A `secret` message is the field, its text the placeholder
   (so "Password" and a changed-password prompt both read as PAM says); a
   `visible` message is the same field shown in clear; `info` and `error`
   go to the status line with their glyphs. `auth_error` is the lock's
   Wrong password (shake, count, the layout mark), then `cancel_session`
   and a fresh `create_session` for the next try; a faillock message reads
   as Locked out, as the lock's parse does today. Choosing another user or
   Escape cancels the session in flight.
7. **Two PAM services, both Isle's.** `/usr/lib/pam.d/isle-greetd` signs
   users in: greetd's packaged stack (securetty, nologin,
   `system-local-login`) with `-auth optional pam_gnome_keyring.so` and
   `-session optional pam_gnome_keyring.so auto_start`, so the login
   keyring still unlocks from the typed password, as the host's edited
   `/etc/pam.d/greetd` does now. `/usr/lib/pam.d/isle-greeter` is the
   greeter's own session: account and session from `system-local-login`,
   no keyring. greetd's config names them with `[general] service` and
   `[default_session] service`; greetd's own file is left alone
   (`session-2026.md` §1).

### Users and sessions

8. **Users from AccountsService.** `ListCachedUsers` on the system bus, each
   user's `UserName`, `RealName`, `IconFile`, `SystemAccount` and `Locked`;
   system and locked accounts are left out. If AccountsService does not
   answer within the system bus's second, passwd from `UID_MIN` to
   `UID_MAX` with a login shell. One user: the lock's face, that user's
   name and picture. Two or more: P1's user list, the last user selected.
   The picture is `IconFile` (under `/var/lib/AccountsService/icons/`,
   which the greeter can read), else P1's placeholder circle; `~/.face` is
   in a home the greeter cannot read.
9. **Sessions from `/usr/share/wayland-sessions`.** Each entry's `Name`,
   `Exec`, `DesktopNames`, `TryExec`, `Hidden` and `NoDisplay`, through the
   desktop entry parser `isle-apps` has. X sessions are not listed: there
   is no X server to start them on. The command is cosmic-greeter's:
   `XDG_SESSION_TYPE=wayland`, `XDG_CURRENT_DESKTOP` from `DesktopNames`,
   `XDG_SESSION_DESKTOP` its first name, and `Exec` split by the entry's
   quoting, sent as `cmd` and the variables as `env` too, so they reach
   `pam_systemd`. Isle is the default; each user's last choice is kept
   (choice 4). The session capsule shows when more than one entry is
   listed, as P1 drew it; here that is Isle, Hyprland and Hyprland (uwsm).

### The look

10. **The session publishes the greeter's look to a file the greeter can
    read.** `/var/lib/isle/greeter/<user>/look.kdl` and, for an image
    wallpaper, `wallpaper.png` beside it, scaled to the main screen and at
    most 16 MB. The worker writes both atomically, half a second after the
    last change to what they hold. What `look.kdl` holds:
    - appearance: mode (Auto kept as Auto, resolved by the greeter the same
      way), accent, Increase Contrast, Reduce Motion, Reduce Transparency,
      text size;
    - the wallpaper: a colour, or the image beside it;
    - the clock's 24-hour choice;
    - the keyboard section: layouts and the per-keyboard profiles, so a
      Mac keyboard types `gb(mac)` with Cmd and Option as in the session;
    - the main screen's identity, and each output's scale and mode by
      identity.
    Glass is the shipped `material-glass` (P1: the greeter has no glass
    position). AccountsService's extension properties were the other way;
    they cannot carry an image, so the file would be needed anyway.
11. **The greeter reads only that file, and checks it.** Its own schema,
    parsed with the settings derive; any field that does not parse takes the
    shipped default, and a file that does not parse at all gives the
    shipped look. `tokens.kdl` and `settings.kdl` are never read (Session
    start 2), so a broken hand edit cannot reach the greeter. The look shown
    is the selected user's; it changes when the selection does, crossfading
    over `motion-fade-ms` (P1's Greeter: user chosen).
12. **The directory is made by `isle setup`.** `/var/lib/isle/greeter/` is
    root's, 0755; each user's directory is theirs, 0755, made by the
    `greeter` step (choice 16). A session without it publishes nothing and
    says so once at debug.

### The face

13. **The lock's face in its Greeter state.** P1's table: no column, no
    island, the user list and the session capsule leading the utilities;
    date, time, the field, the status line, the power row, the layout
    capsule (cycling, as the lock's does today), the battery. The minimal
    face stands in after repeated panics, as on the lock. Many screens: the
    face on the published main screen, else the pointer's; the others
    show the backdrop, date and time.
14. **Power at the greeter goes straight to logind.** Sleep, Restart and
    Shut Down through the system bus (the greeter's session is the seat's
    active one, so polkit allows them). Restart and Shut Down keep the
    confirm in place; there are no apps, so no "Quitting apps…".
15. **The greeter falls back to tuigreet.** For a greeter that never gets
    as far as reading the console keys. Each start counts itself in
    `/var/lib/isle-greeter/`; after three starts within a minute that never
    drew a face, the greeter execs tuigreet with P14a's arguments, if it is
    installed, and logs why. A face drawn clears the count. A start that
    follows a session start within ten seconds is "Greeter, session
    failed": "Couldn't start {session}. Choose another session and try
    again."

### The handoff

16. **Greeter to session.** On `success` after `start_session` the face
    fades out as Unlock does, the last frame (the softened backdrop) stays
    on the screen, and the greeter exits; greetd starts the session. Isle
    started by the greeter (the greeter adds `ISLE_FROM_GREETER=1` to
    `env`) draws its first frame as the softened backdrop with the lock's
    wash and dissolves it as Unlock does, so signing in and unlocking look
    alike. greetd sets the console to text mode between the two
    (`session-2026.md` §1); if that shows as a flash, the VM and the host
    say so and it is recorded, not worked around in P14b.

### `isle setup`

17. **The `greeter` step names Isle's greeter.** Its check is Due while
    greetd's `[default_session] command` is anything else, tuigreet
    included; its apply writes the command, `[general] service =
    "isle-greetd"` and `[default_session] service = "isle-greeter"`, and
    makes the look directories; its undo puts back the lines it replaced,
    which on the host is tuigreet's. A config changed by hand since is left
    alone, as today's step does.

### The field

18. **The field takes a macOS password field's chords, on the lock and the
    greeter.** By the physical chord as the typing keyboard's profile names
    it, read before translation: Cmd is a Mac keyboard's Cmd and a PC
    keyboard's left Alt (P5):
    - Cmd+A selects everything typed: the dots take `selection`; the next
      character replaces them, Backspace or Delete clears them.
    - Cmd+Backspace and Option+Backspace clear the field (macOS's secure
      field deletes all before the caret, and the caret is always at the
      end).
    - Cmd+V pastes the clipboard's text into the field on the lock, cut at
      the field's limit, line breaks dropped; at the greeter there is no
      clipboard.
    - Cmd+C and Cmd+X do nothing: a password is never copied out of the
      field, as on macOS.
    - Characters typed with Option (`#` is Option+3 on `gb(mac)`) reach the
      field; only chords with Cmd or Ctrl are taken as chords.
    - Left and Right do nothing; there is no caret to move.

### Proof

19. **Tested headless with a fake greetd, installed in the VM, used on the
    host.** Headless: the client against a fake greetd socket scripting
    PAM's conversations (one secret, a wrong one, an info and a second
    secret, a faillock message, a cancel, a session that fails to start);
    the greeter mode starting with no session bus and no home; the look
    written by a session and read by a greeter, a broken look giving the
    shipped one, a missing wallpaper; the session list from fixture entries;
    the fallback after three faceless starts; the setup step's check, apply
    and undo; every chord of choice 18 on the lock, and a password typed
    with Option. The VM: greetd running the greeter, sign in to Isle and to
    Hyprland, a wrong password, the keyring unlocked on sign-in, power,
    the look following a change made in the session, Ctrl+Option+F2 from
    the greeter to tty2's login and back with Ctrl+Option+F1, the fallback
    forced by a greeter that panics at start. The host, the user's: the same, and
    `isle setup --undo` back to tuigreet.

## The workspace

```
crates/
  isle-auth/src/greetd.rs        the greetd client (new)
  isle-compositor/src/
    greeter.rs                   the mode, its users, sessions, look, fallback (new)
    greeter_look.rs              the session's side: writing the look (new)
    lock.rs                      the greeter's branch; the field's chords
    lock_face.rs                 the greeter's face model
    main.rs                      --greeter
  isle-shell/src/lock/           the user list, the session capsule, the selection
  isle-settings/                 the look's schema
  isle-cli/src/setup.rs          the greeter step
data/
  pam/isle-greetd, isle-greeter  (new)
  tmpfiles/isle-greeter.conf     (new)
pkg/arch/PKGBUILD                the new files
```

## Phases

Each phase ends with its tests passing headless and a line in
`progress.md`.

### 14b.0 · The record

This sheet reviewed; the roadmap's P14b and P14c (done with the split).

### 14b.1 · The field

Choice 18 on the lock, with its tests. Shippable alone.

### 14b.2 · The greetd client

The protocol, the fake greetd in `isle-fakes`, the conversation's tests.

### 14b.3 · The greeter mode

`--greeter`, the mode's cuts, the worker's three services, the cache and
state directories, starting with no session bus and no home.

### 14b.4 · Users, sessions and signing in

AccountsService and passwd, the session entries, the face's Greeter state
with the user list and session capsule, the conversation on the face,
power through logind.

### 14b.5 · The look

The session's writer, the greeter's reader, the shipped fallback, the
keyboard profiles and outputs from it.

### 14b.6 · The handoff and the fallback

Greeter to session, `ISLE_FROM_GREETER`, the session-failed state, the
fallback to tuigreet.

### 14b.7 · The package and the setup step

The PAM files, `tmpfiles.d`, the PKGBUILD, the `greeter` step.

### 14b.8 · The VM

Choice 19's VM run.

### 14b.9 · The host and close

The user's run. ARCHITECTURE gains How the greeter signs in; decisions
recorded from B140; `platform.md`'s open points (the greetd client, where
the session publishes) answered; this sheet ends with its outcome.

## Not in P14b

Fingerprint sign-in and autologin (P14c, with the PAM lines). The v1
import, theming installed apps, the rest of `isle setup`, updates, a
second GPU, v1's units (P14c). The layout menu P1 drew in place of the
cycling capsule (the lock has the capsule today; both change together
later). The on-screen keyboard at the greeter, if the lock does not have
it by then. X sessions at the greeter.

## Depends on, and feeds

- Needs P9 (the lock, its face, the minimal face, the power row), P5 (the
  keyboard profiles), P14a (the package, the setup table, tuigreet's line).
- P14c builds fingerprint sign-in and autologin on this greeter's PAM
  services and setup step.

## Done when

- `cargo test` passes headless with choice 19's tests.
- In the VM, greetd's greeter is Isle's, and signs into Isle and Hyprland.
- On the host, the user signs into Isle from Isle's greeter, the login
  keyring is unlocked, and `isle setup --undo` brings tuigreet back.

## Reviewed

The user reviewed the plan on 2026-11-01 and took every recommendation:
the greeter alone, the rest in P14c; the greeter mode as the lock and
nothing else; Isle's own greetd client; two PAM services of Isle's; the
look published to `/var/lib/isle/greeter/<user>/`; the fallback to
tuigreet; Cmd+C and Cmd+X never copying the password. The console keys,
which Isle already maps, are the way out of a misbehaving greeter
(the user's point).

## Risks

- **Locked out of the machine.** A greeter that cannot draw on the host
  (an NVIDIA driver updated without a restart, as P14a found) may die
  before it reads Ctrl+Option+F2. Choice 15's fallback covers that; the VM
  forces it before the host ever runs the greeter, and the host run tries
  the console keys from the greeter first.
- **The greeter on NVIDIA as `greeter`.** The driver's shader cache and
  `/dev/nvidia*` access for a system user are untested; choice 4's cache
  directory and the `video` group the user already has are the guess. The
  VM cannot show this (llvmpipe); the host run must.
- **The text-mode flash.** greetd's console reset between greeter and
  session may show; choice 16 records it rather than chasing it.
- **PAM messages beyond a password.** An expired password asks for a new
  one twice; choice 6 handles any number of prompts, but the face was drawn
  for one. The fake tests the shape; the copy is PAM's.

## Outcome

Built 2026-11-01 and 02 in phases 14b.0–14b.8; the host run, part of done,
is the user's, with its checklist in `docs/research/rust-desktop/p14b-greeter.md`.
B140–B143 record the decisions. Differences from the plan:

- **No arrival from the greeter's backdrop.** Choice 16's
  `ISLE_FROM_GREETER` and the session's softened first frame are not built:
  the lock's unlock is a cut, not P1's dissolve, so signing in and
  unlocking already look alike. The greeter hides its face and exits after
  `motion-exit-fade-ms`.
- **The look is a settings file.** Choice 10's `look.kdl` is written in
  `settings.kdl`'s own form by `isle_settings::text_of` and read through
  the settings store, so the greeter needs no schema of its own; per-device
  keyboard entries are written even when they hold their defaults.
- **The console keys changed.** The VM run found that a PC-profile
  keyboard's Ctrl+Alt+F-key did nothing (Alt read as Cmd, since P5); they
  now act with Ctrl and Option or Cmd (B142).
- **The session menu closes** on a choice, Escape or its capsule, not on a
  click elsewhere on the face.
- **The field's chords** are read by the Cmd role, which a PC keyboard puts
  on its left Alt, not Ctrl as choice 18 first said.

Found on the way and fixed: two `isle-package` tests failing before P14b
(`isle-provider-init`'s crate, and `tests.rs` modules counted as shipped
source); a PKGBUILD quoting slip of P14b's own. The guest kept five
hand-copied helper and policy files from P19's walks, overwritten by name.
