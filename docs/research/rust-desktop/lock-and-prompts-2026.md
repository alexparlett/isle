# Lock and prompts, 2026-10

Evidence for P9: the compositor's own lock with PAM, the polkit agent, the
keyring prompter, the NetworkManager secret agent, the BlueZ pairing agent,
and idle, dim and DPMS with `org.freedesktop.ScreenSaver`, as they stand on
2026-10-08. Read against B11 (zbus, no libdbus), B12 (the desktop takes the
prompt roles and locks itself) and B28 (no `ext-session-lock` served; the
Bluetooth agent). Extends `services-2026.md` §NetworkManager and §BlueZ.

Every claim carries a source and a date. Repositories were read through a
clone or the GitHub API on 2026-10-08, at the commits given; Arch packages
through archlinux.org's JSON and file lists the same day. Anything not
verified is under **OPEN** at the end and nowhere else.

---

## 0. The host (read 2026-10-08)

| What | Reading | Consequence |
|---|---|---|
| PAM | `pam` 1.7.3; `/etc/pam.d/system-auth` runs `pam_faillock preauth`, `-pam_systemd_home`, `pam_unix try_first_pass nullok`, `pam_faillock authfail` (`[default=die]`), `authsucc`; `/etc/security/faillock.conf` all defaults | deny 3, fail_interval 900 s, unlock_time 600 s (`pam_faillock.c:78-80`) |
| PAM vendor dir | `/usr/lib/pam.d/` holds `polkit-1`, `systemd-user`, `systemd-run0` | Isle can ship `/usr/lib/pam.d/isle`; `/etc/pam.d/isle` overrides it (§1.3) |
| `unix_chkpwd` | `/usr/bin/unix_chkpwd` `-rwsr-sr-x root`; `/etc/shadow` `0600 root` | Unprivileged `pam_unix` verifies through the helper (§9) |
| faillock tally | `/run/faillock/alexparlett` `-rw-rw---- alexparlett:root` | The user's own process can write its tally: lock-screen failures count (§1.4) |
| polkit | 127-3.1 (CachyOS rebuild of Arch 127-3); `/usr/lib/polkit-1/polkit-agent-helper-1` `0755`, not setuid; `polkit-agent-helper.socket` static and active at `/run/polkit/agent-helper.socket` | Agents must use the socket; the old spawn path fails (§2.2) |
| Keyring | gnome-keyring 1:50.0 owns `org.freedesktop.secrets`; gcr 3.41.2 ships `/usr/lib/gcr-prompter` and the activation files for `org.gnome.keyring.SystemPrompter` and `.PrivatePrompter`; gcr-4 4.4.1 ships `gcr-ssh-agent` (socket enabled, `SSH_AUTH_SOCK=/run/user/1000/gcr/ssh`); kwallet 6.30 ships `ksecretd` activatable only as `org.kde.secretservicecompat`; oo7 not installed (Arch `extra/oo7` 0.6.0-5, 2026-10-04) | §3 |
| pinentry | 1.3.3 with `pinentry-gnome3`, `-qt`, `-gtk`, `-curses`; gnupg 2.4.9; openssh 10.5p1, no `ssh-askpass` installed | §3.5 |
| NetworkManager | 1.58.1; VPN plugins `nm-openvpn-service.name` (networkmanager-vpn-plugin-openvpn 1.12.5; auth-dialog `/usr/lib/nm-openvpn-auth-dialog`, external UI mode and hints) and `nm-protun.name` (python-proton-vpn-api-core 5.6.20; `/usr/lib/nm-protun-auth-dialog`, external UI mode, no hints) | §4.3 |
| BlueZ | 5.87; `/etc/bluetooth/main.conf` all defaults (`AlwaysPairable=false`); the D-Bus policy lets any user call `org.bluez` | §5 |
| logind | systemd 262; `InhibitDelayMaxUSec` 5 s, `IdleAction` ignore, `IdleActionUSec` 30 min, `HandleLidSwitch` suspend; session methods include `Lock`, `Unlock`, `SetIdleHint`, `SetLockedHint`, `SetDisplay`, signals `Lock`, `Unlock` | §6 |
| NVIDIA | open kernel module and `nvidia-utils` 615.71.09 on `linux-cachyos` 7.2.8; nvidia-utils ships `systemd-suspend.service.d/nvidia-no-freeze-session.conf` (`SYSTEMD_SLEEP_FREEZE_USER_SESSIONS=false`) | The compositor is not frozen during suspend entry and must stop rendering itself (§6.4) |

### 0.1 What PAM loads into a process (measured)

A test program in the scratchpad called `pam_start_confdir` and
`pam_acct_mgmt` as the user, under `LD_DEBUG=libs`, with a stack of
`pam_faillock`, `pam_fprintd`, `pam_systemd_home`, `pam_unix`,
`pam_gnome_keyring` and `pam_systemd`. Loaded: libpam, libaudit, libcap-ng,
libcrypt, libtirpc, libnsl, the Kerberos libraries (through pam_unix),
**libsystemd.so.0** (through pam_fprintd) and the modules. **No
libdbus-1.** `ldd` over `/usr/lib/security/*.so`: only `pam_snapper.so`
links libdbus-1; `pam_fprintd`, `pam_issue`, `pam_limits` and
`pam_timestamp` link libsystemd (sd-bus); `pam_systemd.so` and
`pam_systemd_home.so` link nothing beyond libc and libgcc_s (systemd-shared
is static). Linux-PAM dlopens every module of every type named in the
service file at `pam_start`; an `include` pulls only the lines of the
including type (`libpam/pam_start.c`, `pam_handlers.c:141`, linux-pam
v1.7.3, 2026-09-24). An auth-only service file therefore never loads
`pam_systemd`.

---

## 1. PAM from Rust

### 1.1 Crates

| Crate | Latest | Licence | Conversation | Handle | Credentials |
|---|---|---|---|---|---|
| `pam` (1wilkens) | 0.8.0, 2023-11-01; repo last push 2023-12-28 | MIT or Apache-2.0 | `Conversation { prompt_echo, prompt_blind, info, error }` | Send if the conversation is; not Sync | `authenticate()` always runs `pam_acct_mgmt` too; `setcred` only inside `open_session`; no `PAM_REFRESH_CRED` |
| `pam-client` (gitlab cg909) | 0.5.0, 2022-07-30; last activity 2024-10-18 | MPL-2.0 | `ConversationHandler { prompt_echo_on, prompt_echo_off, text_info, error_msg }` | `unsafe impl Send` (`context.rs:747`); not Sync | `authenticate`, `acct_mgmt`, `reinitialize_credentials`, `chauthtok`, `open_session` |
| `pam-client2` (LeChatP fork) | 0.5.5, 2026-03-27; push 2026-06-12 | MPL-2.0 | as pam-client | Send, not Sync (compiled probe) | as pam-client |
| `nonstick` (code.pfish.zone, Mercurial) | 0.1.2, 2026-03-13 | MIT | richest: `Prompt`, `MaskedPrompt`, `Error`, `Info`, `RadioPrompt`, `BinaryPrompt` | **not Send** (`*mut LibPamHandle`); build it on the worker thread | authenticate, account management, change authtok; no setcred ("future release") |
| `pam-sys` | 1.0.0-alpha5, 2023-11-01 | MIT or Apache-2.0 | raw FFI; bindgen at build time (libclang) | | |
| `libpam-sys` (under nonstick) | 0.2.0, 2025-08-03 | | raw, no bindgen | | |
| `pam-bindings` (lvkv) | 0.3.0, 2026-06-05 | MIT | module side only; not usable by an app | | |

`pam-sys`, `pam-sys2` and `libpam-sys` all declare `links = "pam"`, so only
one can be in the graph. sudo-rs keeps its own wrapper in-tree
(`src/pam/`, pre-generated bindings, a `Converser` trait with
`handle_error`/`handle_info`, secure memory; pushed 2026-10-02): a reference,
not a crate. Of the maintained client crates only `pam-client2` is Send,
has the full conversation and a credentials API.

### 1.2 How lockers and greeters do it

| Program | Crate or language | Where PAM runs | Calls | Messages |
|---|---|---|---|---|
| cosmic-greeter locker (98df07d, 2026-10-07) | `pam-client` 0.5.0 | `spawn_blocking` thread in the locker, a separate ext-session-lock client of cosmic-comp; a new context per attempt | `authenticate`, `acct_mgmt`; no setcred | `text_info` shown as a prompt, `error_msg` as an error |
| greetd (d6733e9, 2026-07-21) | `pam-sys` 0.5.6 and its own wrapper | forked session worker, `execv(/proc/self/exe --session-worker)` (`session/interface.rs:112`) | authenticate, acct_mgmt (chauthtok on NEW_AUTHTOK_REQD), setcred ESTABLISH, open_session | all four |
| swaylock (44b82de, 2026-07-09) | C | child forked at the start of `main()` (`comm.c:95`); password over a pipe | authenticate loop, `setcred(PAM_REFRESH_CRED)`; no acct_mgmt | ignores ERROR_MSG and TEXT_INFO |
| waylock (Zig, 88673f0, 2026-05-21) | C ABI | forked child | authenticate, `setcred(PAM_REINITIALIZE_CRED)` | ignored |
| hyprlock (0332e40, 2026-08-11) | C++ | `std::thread` in the locker | authenticate only (acct_mgmt is issue #691, open since 2025-02-21) | string-matches "left to unlock" (faillock) and "Place your finger" (fprintd) |
| kscreenlocker (360c8fd, 2026-10-06) | C++ | to 6.7.5: one QThread per service in the greeter process. Master, MR !318 merged 2026-08-18: each `pam_authenticate` in its own `kscreenlocker_worker` process, peer-to-peer D-Bus, `PR_SET_PDEATHSIG SIGKILL`, not dumpable; cancel, then terminate, then kill | `setcred(PAM_REFRESH_CRED)` on success | shown |
| GNOME | | GDM's root session worker; gnome-shell opens a reauthentication channel (`js/gdm/userVerifier.js:326`) | | |

swaylock moved PAM into a child in 2019 (PR #19, commit 0c7932a3,
2019-01-16) after issue #10: a crash on repeated Enter, with pam_mount,
left the desktop open. KDE's stated reason for !318 is cancellation: "we
also need them to cancel … isolating the actual pam_authenticate calls in
standalone processes". `pam_start` appears nowhere in kwin, smithay, niri or
cosmic-comp. No surveyed compositor runs PAM in its own process.

### 1.3 Service files

| Package (Arch, 2026-10) | File | Content |
|---|---|---|
| swaylock 1.8.6 | `/etc/pam.d/swaylock` | `auth include login` |
| hyprlock 0.9.6 | `/etc/pam.d/hyprlock` | `auth include login`; hyprlock falls back to `su` when `/etc/pam.d/hyprlock` is missing and never looks in `/usr/lib/pam.d` |
| waylock 1.6.0 | `/etc/pam.d/waylock` | `auth include system-auth` |
| kscreenlocker 6.7.5 | `/usr/lib/pam.d/kde`, `kde-fingerprint`, `kde-smartcard` | `kde` includes system-local-login; master adds `kde-face` and `kde-u2f` |
| gdm 51.0 | `/usr/lib/pam.d/gdm-password`, `gdm-fingerprint`, `gdm-smartcard`, … | |
| cosmic-greeter 1.9.1-1 (2026-09-27) | none | Upstream's `debian/cosmic-greeter.pam` (ef1253aa, 2024-09-06) is reverted in the PKGBUILD; the locker uses `login` |

libpam reads `/etc/pam.d/<svc>` first, then `/usr/lib/pam.d/<svc>`
(since Linux-PAM 1.2.0; `pam_private.h` `PAM_CONFIG_DIST_D`). Arch builds
1.7.3 with `-Dvendordir=''`, which does not affect that path.

### 1.4 Fingerprint and faillock

- **pam_fprintd in the password stack blocks it.** Defaults `max-tries=3`,
  `timeout=30` s (`pam/pam_fprintd.c:76-77`, fprintd HEAD 2026-07-24); the
  password prompt arrives only after the fingerprint gives up (swaylock #61,
  2019-03-19; cosmic-greeter #173, 2025-01-09, and #507, 2026-08-05). It
  can be cancelled only by SIGINT through a signalfd in its sd-bus loop
  (lines 473-533), which works in a process of its own and not on a thread.
- **Desktops run fingerprint as a second, parallel conversation**: KDE `kde`
  and `kde-fingerprint`, GDM `gdm-password` and `gdm-fingerprint`. Arch's
  fingerprint files run `pam_faillock preauth` (a locked account blocks the
  finger too) and no `authfail`, so finger failures do not count. hyprlock
  skips PAM and calls fprintd's `net.reactivated.Fprint` D-Bus API directly.
- **faillock messages** come as `PAM_TEXT_INFO`, not `PAM_ERROR_MSG`: "The
  account is locked due to %u failed logins." and "(%d minutes left to
  unlock)" (`pam_faillock.c` `faillock_message`, lines 421-455), sent at
  `preauth` when already locked unless `silent` or `PAM_SILENT`. Preauth
  then returns `PAM_AUTH_ERR` and no password prompt is ever sent.
- **Lock-screen failures count and can lock the user out of the lock
  screen.** The tally file is chowned to the user (`faillock.c:90`), so an
  unprivileged locker writes it. swaylock #286 (2023-02-09, open): the tally
  is not reset after a swaylock unlock. hyprlock #1074 (2026-09-20, open):
  aborting an in-flight conversation after a fingerprint unlock is counted as
  `authfail` and locked the account. KDE bug 481808 (2024-02-25): suspending
  from the lock screen records a failure. Lesson: never end a pending
  password conversation by returning a conversation error; end the
  transaction without answering, or kill the worker.
- On Fedora/RHEL (authselect `with-faillock`) the reset is in `account
  required pam_faillock.so`, so a locker that skips `pam_acct_mgmt` never
  clears the tally. Arch resets in `auth` (`authsucc`).

### 1.5 Thread or helper process

Separate processes are the norm (swaylock, waylock, greetd, GDM, KDE since
2026-08). Reasons found in code and commits:

1. A module crash takes the host process down (swaylock #10). For a
   compositor that is the session.
2. `pam_authenticate` cannot be cancelled; pam_fprintd honours only SIGINT.
3. Process-wide side effects. Unprivileged `pam_unix` sets `SIGCHLD` to
   `SIG_DFL` for the whole process, forks `unix_chkpwd`, waits for that pid,
   then restores the old handler (`modules/pam_unix/support.c:515-629`).
   Isle's compositor spawns with `tokio::process` (`launch.rs`,
   `menubar.rs`), whose reaper is driven by a SIGCHLD handler; a SIGCHLD
   that arrives during the window is dropped. `pam_gnome_keyring` changes
   `SIGPIPE`/`SIGCHLD` dispositions and may fork
   (`gnome-keyring pam/gkr-pam-client.c:388-458`).
4. Hygiene: a non-dumpable worker with `PR_SET_PDEATHSIG`; the password
   never sits in the compositor's heap beyond the field.
5. libdbus: none in today's stack (§0.1), but any module a user adds
   (`pam_snapper`) would bring it into the compositor.

**Recommendation.** Run PAM in a small `isle-auth` helper binary, forked per
attempt (or per lock) by the compositor, speaking a pipe protocol modelled
on polkit's helper lines (§2.2): prompts and messages up, responses down,
`SUCCESS`/`FAILURE` last. Run the password service and a fingerprint path
side by side; fingerprint through fprintd's D-Bus API from the compositor's
zbus worker (as hyprlock does), or a second helper on an `isle-fingerprint`
service without `authfail`. Use `pam-client2` inside the helper. Ship
`/usr/lib/pam.d/isle` as `auth include system-local-login` and `account
include system-local-login` only (no session lines), call `acct_mgmt` after
`authenticate`, and `setcred(PAM_REFRESH_CRED)` on success. Show every
`TEXT_INFO` and `ERROR_MSG` on the lock.

---

## 2. polkit agent

### 2.1 The D-Bus side (polkit d3ba690, 2026-10-07)

- The agent exports `org.freedesktop.PolicyKit1.AuthenticationAgent` at a
  path of its choice:
  - `BeginAuthentication(s action_id, s message, s icon_name, a{ss}
    details, s cookie, a(sa{sv}) identities)`. polkitd calls with timeout
    `G_MAXINT` (`polkitbackendinteractiveauthority.c:2696-2703`), so the
    call stays open while the dialog is up. Return `()` once the helper
    reported SUCCESS; return `org.freedesktop.PolicyKit1.Error.Cancelled`
    when the user dismisses (polkitd then sets `polkit.dismissed=true`).
    `details` carries `polkit.caller-pid` and `polkit.subject-pid`;
    `message` is localised to the registered locale.
  - `CancelAuthentication(s cookie)`.
- Identities arrive as `("unix-user", {"uid": u})`; groups and netgroups are
  expanded by polkitd (l.2583-2600). COSMIC and GNOME choose the user's own
  uid first, then root, then the first entry.
- `Authority.RegisterAuthenticationAgent((sa{sv}) subject, s locale, s
  object_path)`; `…WithOptions` adds only `fallback: b` (pkttyagent). The
  subject must be `unix-session` (`{"session-id": s}`) equal to the caller's
  own session, or `unix-process`; one agent per subject.
- polkitd finds the caller's session by `sd_pidfd_get_session`, then
  `sd_pid_get_session`, then `sd_uid_get_display`
  (`polkitbackendsessionmonitor-systemd.c:344-430`). A compositor started as
  a `systemd --user` service has no session of its own, so it should resolve
  the session the same way: `XDG_SESSION_ID`, logind `GetSessionByPID`,
  then `User.Display` (hyprpolkitagent f2bdf10, 2026-06-05).
- `AuthenticationAgentResponse2(u uid, s cookie, identity)` (polkit 0.114,
  CVE-2015-4625) and `AuthenticationAgentResponse3(s cookie, identity,
  subject)` (socket mode) are called by the helper as root, never by the
  agent.

### 2.2 The helper

**Line protocol** (`src/polkitagent/polkitagenthelper-pam.c`,
`polkitagentsession.c`). The helper runs `pam_start("polkit-1", user)`,
`pam_authenticate`, `pam_acct_mgmt`, checks `PAM_USER` is unchanged, calls
Response2 or Response3, then prints the result. It writes
`PAM_PROMPT_ECHO_OFF <prompt>`, `PAM_PROMPT_ECHO_ON <prompt>` (the agent
answers one line), `PAM_ERROR_MSG <m>`, `PAM_TEXT_INFO <m>`, and finally
`SUCCESS` or `FAILURE`. Messages are `g_strescape`d and libpolkit-agent
undoes it with `g_strcompress` (`polkitagentsession.c:479`); cosmic-osd,
soteria and zbus-polkit-agent do not unescape. A newline cannot be part of
a password. One run is one attempt; a retry starts again with the same
cookie.

**Setuid mode**: `argv = [helper, user]`, cookie on the first stdin line; it
refuses with "needs to be setuid root" when `geteuid() != 0`.

**Socket mode**: commit c007940 (authored 2024-09-06, committed
2025-03-25), first released in **polkit 127** (2025-12-17); not in 125
(2024-08-08) or 126 (2025-01-13). The agent connects to
`/run/polkit/agent-helper.socket`, writes `user\n` then `cookie\n`, and the
line protocol follows. The helper identifies the agent by `SO_PEERPIDFD` and
`SO_PEERCRED` and calls Response3 with a `unix-process` subject built from
the pidfd. libpolkit-agent-1 uses the socket when it exists and falls back
to spawning the binary. Later fixes: CVE-2026-4897 (7e122c8, 2026-03-27,
getline overflow), CVE-2026-85498 (eea1729, 2026-08-27, empty cookie
underflow), the `polkit-agent-helper(8)` man page (5195b3a, 2026-07-03),
which notes that PAM modules needing devices or the home directory need a
drop-in under `polkit-agent-helper@.service.d/` because of the unit's
sandbox. No 128 tag exists.

**Arch** 127-3 (2025-12-28) "Remove suid and statically activate agent
helper": the helper is installed `0755` and the socket is wanted by
`sockets.target`. On Arch the spawn fallback cannot work. The Arch
PKGBUILD carries neither 2026 CVE fix.

### 2.3 Rust agents

| Project | Licence, date | Bus | Helper | Notes |
|---|---|---|---|---|
| cosmic-osd `src/subscriptions/polkit_agent*.rs` | GPL-3.0-or-later; 9b14ee0, 2026-10-06 | zbus 5.14 | socket first (cb9698c, 2026-04-25), else spawn | `unix-session` from `login1 /session/auto`; locale hard-coded `en_US`; 3 retries; no unescape |
| soteria | Apache-2.0; v0.3.2, 2026-07-21; GTK4 | zbus 5.7 | socket, else spawn | answers only a prompt that reads `Password:`; re-registers when polkitd's owner changes |
| hyprpolkitagent | BSD-3; 0.2.0, 2026-09-09 | sdbus-c++ (polkit-qt dropped 2026-06-03) | own socket client plus spawn fallback | PR #59 (2026-06-23): a PAM failure before any prompt is answered as Cancelled, because Failed produced an endless dialog loop |
| `zbus-polkit-agent` (waycrate) | GPL-3.0-or-later; 0.4.3, 2026-05-07 | zbus | socket only; blocking `read_exact` inside a hand-rolled future | used by psst |
| `polkit-agent-rs` | MIT; 0.3.0, 2025-05-24 | none | binds the C libpolkit-agent-1 (GIO) | |
| psst (phisch) | MPL-2.0; 2026-06-22 | zbus | through zbus-polkit-agent | one project: pinentry, Gcr prompter and polkit agent |

**Recommendation.** Write the agent on Isle's zbus worker: register a
`unix-session` subject resolved as polkitd does; speak the socket protocol
directly and drop the spawn path; unescape lines; one connection per
attempt; `()` on SUCCESS, `Error.Cancelled` on dismiss and on a failure with
no prompt; `CancelAuthentication` closes the sheet and answers the pending
call; re-register when polkitd restarts; offer an identity picker when
several admins are offered. The helper runs PAM as root in its sandbox, so
the polkit agent needs no PAM of Isle's own.

---

## 3. Keyring prompter

### 3.1 The Gcr system-prompter protocol (gcr 52bfb6a, 2026-10-05, 4.4.1)

- Session bus name `org.gnome.keyring.SystemPrompter`; prompter object
  `/org/gnome/keyring/Prompter`; client callbacks at
  `/org/gnome/keyring/Prompt/p<N>`. Errors
  `org.gnome.keyring.Prompter.InProgress` and `.Failed`.
- Interface `org.gnome.keyring.internal.Prompter`: `BeginPrompting(o
  callback)`, `PerformPrompt(o callback, s type, a{sv} properties, s
  exchange)` with `type` `password` or `confirm`, `StopPrompting(o
  callback)`. All return at once.
- Callback interface `org.gnome.keyring.internal.Prompter.Callback`:
  `PromptReady(s reply, a{sv} properties, s exchange)` with reply `""`,
  `"yes"` or `"no"`, and `PromptDone()`.
- Sequence: BeginPrompting; when free the prompter calls `PromptReady("",
  {}, <its public key>)` (other clients queue); the client calls
  PerformPrompt with its properties and public key; the prompter shows the
  sheet and answers `PromptReady("yes", changed, <encrypted secret>)` or
  `"no"`; the client may PerformPrompt again with a `warning` (retry) or
  StopPrompting, after which the prompter calls `PromptDone`. The prompter
  watches the caller's name.
- Properties (`gcr-prompt.c:104-228`): strings `title`, `message`,
  `description`, `warning`, `choice-label`, `caller-window`,
  `continue-label`, `cancel-label`; booleans `password-new`,
  `choice-chosen`; int `password-strength`.
- Wire quirk: in `PromptReady` the prompter wraps each value twice,
  `v(v(...))` (`gcr-system-prompter.c:468`), and clients unwrap with
  `g_variant_get_variant` (`gcr-system-prompt.c:621-623`). Isle must do the
  same. MR !169 to stop it is open (updated 2025-11-08).
- Secret exchange `sx-aes-1` (`gcr-secret-exchange.c`): a GKeyFile group
  `[sx-aes-1]` with base64 `public`, `secret`, `iv`; DH over
  `ietf-ike-grp-modp-1536`, generator 2; key HKDF-SHA256 of the shared
  secret, no salt or info, 16 bytes; AES-128-CBC, PKCS#7. Rust
  implementations: oo7 `server/src/gnome/secret_exchange.rs` (MIT) and psst
  `crates/keyring-prompter/src/secret_exchange.rs` (MPL-2.0).
- Documentation: only the in-tree XML ("an internal interface, and not a
  public API. It can change between releases") and the GcrSystemPrompter
  API reference.
- gcr 4 keeps `GcrSystemPrompter` and `GcrSystemPrompt` but ships no
  prompter program; `/usr/lib/gcr-prompter` (GTK 3) and the activation
  files come from gcr 3 (branch gcr-3-41, 2024-01-12). gcr-prompter takes
  the name with `REPLACE` and exits after 10 s idle.
- In-process prompters: GNOME Shell `js/ui/components/keyring.js` (29380c2,
  2026-10-06; `ALLOW_REPLACEMENT`), Cinnamon, elementary's
  pantheon-agent-polkit, Phosh. A D-Bus replace succeeds only against an
  owner that allowed it, so Isle should own the name without
  `AllowReplacement`; a stray gcr-prompter then cannot take it, and the
  activation file only fires when Isle is not running.

### 3.2 Other desktops

- **COSMIC** starts `gnome-keyring-daemon --components=pkcs11,secrets,ssh`
  (`cosmic-session/data/start-cosmic`), sets `Secret=oo7-portal;gnome-keyring;`
  for portals, and ships no prompter (code search of the pop-os org for
  `SystemPrompter`): its prompts are the GTK 3 gcr-prompter window.
- **KDE**: kwallet master (cf41da3, 2026-10-07) has `ksecretd` serve
  `org.freedesktop.secrets` with its own Qt dialogs, and `kwalletd6` can
  proxy to any Secret Service provider. A new `org.kde.secretprompter`
  interface exists in personal repos (mart/ksecretprompter, 2025-12-19) and
  oo7 uses it on Plasma.
- **Hyprland**: hyprpolkitagent but no keyring prompter; prompts fall to
  gcr-prompter. psst and bb-auth fill the gap.

### 3.3 gnome-keyring and oo7

- gnome-keyring 51.1 (tag 2026-09-17; Arch `extra` 1:51.1-1, 2026-10-07)
  still builds on gcr 3; 48.0 disabled its own ssh agent. Its unlock,
  create, change and PKCS#11 prompts all use `GcrSystemPrompt`.
- GNOME OS replaced it: gnome-build-meta 528af16d "Replace gnome-keyring
  with oo7" (committed 2026-06-16, after a revert on 2026-06-14) removes
  gnome-keyring and gcr 3 and adds `oo7-daemon`, `oo7-pam`, `oo7-portal`.
- oo7 (linux-credentials/oo7, MIT, 5284d64, 2026-10-08): 0.6.0
  (2026-02-21), 0.7.0.alpha (2026-07-17), 0.7.0.beta (2026-09-21).
  oo7-daemon chooses a prompter per caller (`server/src/service/mod.rs:
  536-557`): Plasma's `org.kde.secretprompter` when `XDG_CURRENT_DESKTOP=kde`
  and it is activatable; otherwise the Gcr protocol to
  `org.gnome.keyring.SystemPrompter` (`server/src/gnome/prompter.rs`);
  otherwise a CLI prompter for non-graphical callers. "Graphical" is the
  caller's logind session type, then the peer's `WAYLAND_DISPLAY`/`DISPLAY`,
  then the systemd user manager's environment; the last two came in
  f6a8624 (2026-08-25) for compositors run as user services. `pam_oo7.so`
  hands the login password to `$XDG_RUNTIME_DIR/oo7/pam.sock`. It migrates
  gnome-keyring and KWallet keyrings.
- Arch `extra/oo7` 0.6.0-5 (2026-10-04): provides `org.freedesktop.secrets`,
  conflicts with gnome-keyring, ships `oo7-daemon`, `oo7-portal`,
  `pam_oo7.so`, user units and `oo7-cli`. Arch has 0.6, not the 0.7 betas
  with the user-service fix.

### 3.4 What goes through the Gcr prompter

Through it: gnome-keyring (unlock, create, change, PKCS#11 login),
oo7-daemon outside Plasma, gcr-4's `gcr-ssh-agent` (key unlock with
"Automatically unlock this key whenever I'm logged in";
`gcr-ssh-agent-interaction.c`), and `pinentry-gnome3`
(`gcr_system_prompt_open`; falls back to curses without a prompter). Arch's
`/usr/bin/pinentry` script picks gnome3 first on any graphical session
unless `XDG_CURRENT_DESKTOP` is KDE or LXQt, so gpg-agent's prompts reach
it too (pinentry 15b7e8a, 2026-07-03).

Not through it: pinentry-qt and -gtk, `SSH_ASKPASS` programs, kwallet,
polkit. OpenSSH (4079f38, 2026-10-08, `readpass.c`) runs `$SSH_ASKPASS`
when a display is set and there is no tty, or with
`SSH_ASKPASS_REQUIRE=force|prefer`; argv[1] is the prompt, the answer comes
on stdout; `SSH_ASKPASS_PROMPT=confirm` for `ssh-add -c`, `none` for a
notice.

### 3.5 Recommendation

1. Serve `org.gnome.keyring.SystemPrompter` (single prompt, queued),
   `password` and `confirm`, `password-new` with a confirm field and
   strength, the choice checkbox, sx-aes-1 and the double-wrapped reply.
   This one service covers gnome-keyring, oo7, gcr-ssh-agent and
   pinentry-gnome3. `PrivatePrompter` has no caller found; skip it.
2. Keep gnome-keyring on the host for now; plan the move to oo7 when Arch
   ships 0.7 (the user-service session detection). Keep kwallet's
   `ksecretd` off `org.freedesktop.secrets`.
3. Optionally ship a small `SSH_ASKPASS` program that forwards to the
   shell, honouring `SSH_ASKPASS_PROMPT`.
4. The remaining password surfaces are the NetworkManager and BlueZ agents
   (§4, §5) and GIO's `org.gtk.MountOperationHandler` (GNOME Shell serves it
   at `/org/gtk/MountOperationHandler`, `js/ui/shellMountOperation.js:
   551-566`) for network mounts.

---

## 4. NetworkManager secret agent

Sources: NetworkManager ee56e17 (2026-10-08, 1.59.2-dev; the API files are
unchanged from 1.58), gnome-shell 29380c2, network-manager-applet ff50029
(2026-04-10), libnma 513be68 (2026-09-22), NetworkManager-openvpn 7fb52f4
(2026-09-10), plasma-nm 6b4da9e (2026-10-08).

### 4.1 Interfaces

- `org.freedesktop.NetworkManager.SecretAgent` at the fixed path
  `/org/freedesktop/NetworkManager/SecretAgent` (`nm-dbus-interface.h:91`);
  only root may call it (system bus policy).
  - `GetSecrets(a{sa{sv}} connection, o path, s setting_name, as hints, u
    flags) → a{sa{sv}}`; the reply must hold the setting with at least one
    key. New agent-owned secrets are saved by the agent itself.
  - `CancelGetSecrets(o path, s setting_name)`, `SaveSecrets(connection,
    path)`, `DeleteSecrets(connection, path)` (without secrets).
- Flags (`nm-dbus-interface.h:891-927`): `ALLOW_INTERACTION` 0x1,
  `REQUEST_NEW` 0x2, `USER_REQUESTED` 0x4, `WPS_PBC_ACTIVE` 0x8; the
  internal `ONLY_SYSTEM` and `NO_ERRORS` are masked before the call.
- Errors `org.freedesktop.NetworkManager.SecretAgent.{Failed,
  PermissionDenied, InvalidConnection, UserCanceled, AgentCanceled,
  NoSecrets}` (`nm-errors.h:209-237`).
- `AgentManager` at `/org/freedesktop/NetworkManager/AgentManager`:
  `Register(s id)`, `RegisterWithCapabilities(s id, u caps)` (VPN_HINTS
  0x1), `Unregister()`. Identifier: 3-255 characters of `[A-Za-z0-9_.-]`,
  no leading, trailing or doubled dot. One agent per identifier per UID,
  not per session (`nm-agent-manager.c:475-480`). Agents are dropped when
  their bus name goes and are not persisted; re-register on NM restart.
- NM tries agents in turn, not all at once: an agent in the requesting
  process first, then agents in active sessions, then the newest. A
  connection's `permissions` ACL restricts the users; a user-started
  activation uses only that user's agents. `UserCanceled` ends the request;
  any other error moves to the next agent. GetSecrets times out at 120 s
  (`nm-secret-agent.c:432`) without a CancelGetSecrets, so the agent should
  drop its sheet before then.
- System-owned secrets reach the agent only if polkit grants
  `settings.modify.own` or `.system`; otherwise they are stripped and marked
  not required in the copy sent (`nm-agent-manager.c:1015-1036,
  1130-1175`).
- Secret flags (`nm-setting.h:41-65`): 0 NM stores it (root-only keyfile);
  0x1 agent-owned; 0x2 not saved (ask every time); 0x4 not required. VPN
  flags are `<secret>-flags` items in `vpn.data`.
- Storage used by GNOME Shell and nm-applet (`shell-network-agent.c`,
  `applet-agent.c:17-27`): libsecret schema
  `org.freedesktop.NetworkManager.Connection` with `DONT_MATCH_NAME`,
  attributes `connection-uuid`, `setting-name`, `setting-key`, one item per
  key, label "Network secret for <id>/<setting>/<key>"; only secrets whose
  flag is exactly agent-owned are saved; a save first deletes every item of
  that uuid. plasma-nm uses its own QtKeychain format and does not
  interoperate.
- Registrations: GNOME Shell `org.gnome.Shell.NetworkAgent`, nm-applet
  `org.freedesktop.nm-applet`, plasma-nm `org.kde.plasma.networkmanagement`,
  all with VPN_HINTS.

### 4.2 Settings and fields

| Setting | Secret keys |
|---|---|
| `802-11-wireless-security` | `psk`, `wep-key0`..`3`, `leap-password` |
| `802-1x` | `password`, `password-raw`, `private-key-password`, `phase2-private-key-password`, `pin`, the CA and client certificate passwords (PKCS#11 PINs) |
| `gsm` | `pin`, `password`, `initial-eps-bearer-password` |
| `cdma`, `pppoe`, `adsl` | `password` |
| `macsec` | `mka-cak` |
| `wifi-p2p` | `wps-pin` |
| `wireguard` | `private-key`, peers' `preshared-key` (reply `peers: aa{sv}` with `public-key`) |
| `vpn` | `secrets` (a{ss}), reply `{"vpn": {"secrets": …}}` |

GNOME Shell's `networkAgent.js` asks: `psk` for `wpa-psk`, `sae` and
`wpa-none` (8-63 characters or 64 hex); the WEP key by `wep-tx-keyidx`;
`leap-password`; for 802.1X the hinted fields among `identity`,
`password`, `private-key-password`, or by EAP method (username and password
for md5, leap, ttls, peap, fast; identity and key password for tls); pppoe
username, service, password; the gsm PIN when hinted; cdma and Bluetooth
password; a WPS button hint with `WPS_PBC_ACTIVE`. It has no WireGuard,
macsec, adsl or wifi-p2p case. Without `USER_REQUESTED` it posts a
notification first and dismissing it answers `UserCanceled`. A second
request for the same path and setting replaces the first.

### 4.3 VPN secrets

- VPN_HINTS is all or nothing: if any agent for the user lacks it, NM uses
  the plugin's old `Connect()` and interactive hints (2FA, challenges)
  cannot flow (`nm-vpn-connection.c:1722-1748`). Hints include
  `x-vpn-message:<text>`.
- Plugin `.name` files are read from `$NM_VPN_PLUGIN_DIR`,
  `/usr/lib/NetworkManager/VPN`, `/etc/NetworkManager/VPN`; `[GNOME]
  auth-dialog`, `supports-hints`, `supports-external-ui-mode`.
- Command line (GNOME Shell `VPNRequestHandler`, nm-applet
  `applet-vpn-request.c:530-551`): `<auth-dialog> -u <uuid> -n <name> -s
  <service> [--external-ui-mode] [-i] [-r] [-t <hint>]…`; `-i` allows
  interaction, `-r` asks anew, `-t` once per hint when hints are supported.
- Stdin: `DATA_KEY=<k>` / `DATA_VAL=<v>` for each `vpn.data` item,
  `SECRET_KEY=` / `SECRET_VAL=` for each secret, then `DONE`. The agent
  fills agent-owned secrets from its own store first (NM does not have
  them).
- External UI mode (version 2): the dialog prints a key file
  `[VPN Plugin UI] Version=2 Description= Title=` and a group per secret
  with `Value`, `Label`, `IsSecret`, `ShouldAsk` (openvpn adds
  `ForceEcho`), then exits. The desktop draws the fields itself; groups
  with `ShouldAsk=false` and a value go straight into the reply; empty
  output means nothing to ask.
- Legacy mode: the dialog draws its own GTK window, prints `key\nvalue\n`
  pairs and a blank line, and waits up to 20 s for `QUIT`. Exit 0 is
  confirmed, non-zero is cancelled.
- Cancel: GNOME closes its own sheet or writes `QUIT`; nm-applet sends
  SIGTERM, then SIGKILL after 2 s. KDE uses its own Qt VPN plugins instead.
- Both plugins on this host support external UI mode, so Isle can draw
  every VPN prompt itself and never show a GTK window.

### 4.4 COSMIC and other Rust agents

- cosmic-applet-network (406d1b7, 2026-10-06) uses `nmrs` 3.5.5 (zbus,
  MIT; 3.5.6 out 2026-10-07): identifier with a random UUID to dodge the
  one-per-UID rule, VPN_HINTS; Wi-Fi requests answered `NoSecrets` and the
  password re-sent through its own connect flow; VPN gets one password
  field copied to every hinted key; no auth-dialog. cosmic-settings
  registers more agents through `nmrs::agent`.
- `nmrs::agent` gives `&self` interface methods, pending requests keyed by
  path and setting, a 120 s timeout, save and delete forwarded as events.
- zbus 5.19 runs each method call as its own task; a `&self` method holds
  only a read lock while awaiting, a `&mut self` method the write lock
  (`object_server/mod.rs:337-363`). Isle's agents must use `&self` (or no
  lock across the await) so `CancelGetSecrets` and BlueZ `Cancel` can run
  while a prompt is pending.
- Others: wayle's `wayle-network` (zbus, own VPN prompts), vibepanel (gio;
  implements external UI and legacy auth-dialogs).

**Recommendation.** One agent on the zbus worker, identifier `org.isle.Shell`
(or similar) with VPN_HINTS; a sheet per setting by the table above,
including 802.1X hints and WireGuard; store agent-owned secrets in the
Secret Service with the nm-applet schema so they interoperate; VPN through
the plugin's auth-dialog in external UI mode with Isle's sheet, legacy mode
only as a fallback; respect `USER_REQUESTED` (a notification when the
request came from autoconnect); drop the sheet at ~110 s.

---

## 5. BlueZ pairing agent

Sources: bluez.git 866b0b8 (2026-10-08; 5.87 is 65d11ede),
`doc/org.bluez.Agent.rst`, `doc/org.bluez.AgentManager.rst`; kernel
`net/bluetooth/hci_event.c`, `smp.c` at master 2026-10-08.

### 5.1 Interface

| Method | Use |
|---|---|
| `Release()` | after bluetoothd has dropped the agent |
| `RequestPinCode(o) → s` | legacy PIN, 1-16 alphanumerics |
| `DisplayPinCode(o, s)` | legacy keyboards: show a 6-digit PIN to type; ended by Cancel |
| `RequestPasskey(o) → u` | 0-999999 |
| `DisplayPasskey(o, u passkey, q entered)` | repeated as keys are typed; zero-pad to 6 |
| `RequestConfirmation(o, u passkey)` | numeric comparison; empty reply is yes, an error is no |
| `RequestAuthorization(o)` | incoming Just Works pairing |
| `AuthorizeService(o, s uuid)` | a service connection from an untrusted device |
| `Cancel()` | |

Errors `org.bluez.Error.Rejected` and `.Canceled`; bluetoothd treats any
error as no. Each request times out after 60 s with a `Cancel`
(`agent.c:40, 330-350`); one request at a time per agent.

`AgentManager1` at `/org/bluez`: `RegisterAgent(o, s capability)` (one per
sender; `""` means KeyboardDisplay), `UnregisterAgent(o)`,
`RequestDefaultAgent(o)`. Default agents form a stack; the default's
capability becomes the adapter's IO capability. With no default agent and
`AlwaysPairable=false` the adapter is not bondable. `Device1.Pair()` uses
the calling sender's agent; incoming pairing and every `AuthorizeService`
go only to the default agent (`device.c:7850-7854`, `adapter.c:8050-8110`).
Trusted devices are authorised without asking (`adapter.c:8073-8076`).

With KeyboardDisplay: a phone or computer gets `RequestConfirmation`; a
keyboard `DisplayPasskey` with `entered` updates; a display-only peer
`RequestPasskey`; headsets and mice pair Just Works, silently when Isle
starts it and with `RequestAuthorization` when the device does. bluetoothd's
autopair plugin answers most legacy PINs itself (0000, 1234, 1111 for
audio; a random 6-digit `DisplayPinCode` for keyboards); only the rest
reach `RequestPinCode`.

### 5.2 How desktops present it

- **GNOME** (gnome-bluetooth 7bfd9ed, 2026-09-19): the agent lives in the
  Settings panel's widget, registered as DisplayYesNo; with Settings
  closed nothing answers. A PIN database (35 entries) picks fixed PINs,
  keyboard PINs and iCade arrows; `RequestConfirmation` is a match dialog;
  `RequestAuthorization` yes/no; `RequestPasskey` rejected;
  `AuthorizeService` trusts a device the remote paired, rejects any non-HID
  service from an unpaired device, and otherwise asks. Local pairing sets
  `Trusted`.
- **KDE** (bluedevil b9fc331, 2026-10-06): the kded agent is default;
  every prompt is a persistent notification ("Verify PIN", "Trust and
  Authorize" / "Authorize Only" / "Deny"); the pairing wizard registers its
  own non-default agent for the `Pair()` it starts.
- **blueman** (a57cb92, 2026-07-20): KeyboardDisplay, default; entry dialogs
  for PIN and passkey, notifications for display and confirmation,
  `AuthorizeService` with "Always accept", "Accept", "Deny".
- **COSMIC**: cosmic-settings (zbus through dbus-settings-bindings) is
  KeyboardDisplay and default while open, accepts `AuthorizeService` and
  `RequestAuthorization` without asking, fails `RequestPasskey` and
  `RequestPinCode`, uses `&mut self` methods. The applet uses `bluer`
  0.17.4, which links libdbus through the `dbus` crate, and is not default.

**Recommendation.** Register one agent as KeyboardDisplay and default at
session start on the zbus worker (B28; `services-2026.md` §BlueZ). Sheets:
confirm a 6-digit code; show a passkey with typed-digit progress; enter a
PIN or passkey; yes/no for an incoming pairing. `AuthorizeService` for a
paired, untrusted device asks with "Always" (sets Trusted), "Allow once",
"Deny". Set `Trusted` after pairing the user started. `Cancel` and the 60 s
timeout close the sheet. Answer `org.bluez.Error.Rejected` or `.Canceled`.

---

## 6. Idle, inhibitors, ScreenSaver and logind

### 6.1 Smithay (cf43a02, 2026-10-08; last release 0.7.0, 2025-06-24)

- `wayland::idle_notify`: `IdleNotifierState` makes the
  `ext_idle_notifier_v1` global at version 2, one calloop timer per
  notification object; `notify_activity(&Seat)` resumes and re-arms them;
  `set_is_inhibited(bool)` resumes idle notifications and stops their
  timers, skipping `get_input_idle_notification` ones as the protocol
  requires.
- `wayland::idle_inhibit`: `IdleInhibitHandler { inhibit, uninhibit }`; the
  doc says the compositor must ignore inhibiting surfaces that are
  invisible or dead itself.
- Smithay keeps no idle timer for the compositor. Isle's dim, lock and
  screen-off timers are its own, fed by the same activity hook and the same
  inhibited flag.
- niri (2c82b77, 2026-10-08) and cosmic-comp (e259901, 2026-10-08) count an
  inhibitor only while `surface_primary_scanout_output` is set, i.e. the
  surface was shown on an output, and prune dead ones; niri ORs in its own
  `org.freedesktop.ScreenSaver` cookies and notifies activity at most once
  per loop iteration. Neither applies inhibitors per output, though the
  protocol says so. sway counts only the lock surface's inhibitor while
  locked.
- cosmic-idle (21e983c, 2026-10-06) is the policy client: fade to black,
  `zwlr_output_power_v1` off, lock 500 ms later by `loginctl lock-session`,
  a separate suspend timer.

### 6.2 `org.freedesktop.ScreenSaver`

Interface (kscreenlocker `dbus/org.freedesktop.ScreenSaver.xml`): `Lock`,
`SimulateUserActivity`, `GetActive → b`, `GetActiveTime → u`,
`GetSessionIdleTime → u`, `SetActive(b) → b`, `Inhibit(s app, s reason) →
u cookie`, `UnInhibit(u)`, `Throttle`, `UnThrottle`, signal
`ActiveChanged(b)`.

- KDE serves it at both `/ScreenSaver` and `/org/freedesktop/ScreenSaver`;
  `GetActive` is the lock state, `SetActive(true)` locks and
  `SetActive(false)` is ignored. GNOME's gnome-settings-daemon proxies
  `Inhibit` to the session manager. niri, hypridle and cosmic-idle serve
  Inhibit and UnInhibit at both paths. All drop a caller's cookies on
  `NameOwnerChanged`.
- Callers:

| Caller | Order | Path |
|---|---|---|
| Chromium and Electron (`power_save_blocker_linux.cc`, 2026-10-07) | `org.gnome.SessionManager` if owned, else `org.freedesktop.ScreenSaver` for display locks; also a `zwp_idle_inhibitor_v1` on the focused window; no portal | `/org/freedesktop/ScreenSaver` |
| Firefox (`WakeLockListener.cpp`, 2026-07-30) | ScreenSaver, FreeDesktop PowerManagement, portal, GNOME, XScreenSaver, Wayland idle-inhibit | **`/ScreenSaver`** |
| SDL3 (`SDL_dbus.c`, 2026-09-14) | portal when sandboxed, else ScreenSaver; Wayland inhibitor as fallback; inhibits by default at video init; also `SimulateUserActivity` | `/org/freedesktop/ScreenSaver` |
| mpv (2026-09-03) | Wayland idle-inhibit only | |
| xdg-desktop-portal-gtk `inhibit.c` | GNOME session manager, else ScreenSaver (Idle flag only); calls `GetActive` and watches `ActiveChanged` | `/org/freedesktop/ScreenSaver` |

Isle must serve both paths, `GetActive` and `ActiveChanged`, and expect the
portal backend to hold cookies for sandboxed apps. Isle's own portal (B13)
will implement `org.freedesktop.impl.portal.Inhibit` directly.

### 6.3 logind (systemd 262, released 2026-09-22)

- `Session.SetIdleHint(b)` (owner or root), `SetLockedHint(b)`, `Lock`,
  `Unlock`; signals `Lock` and `Unlock` from `loginctl lock-session` and
  `unlock-session`, emitted only on the real session path, not on
  `/session/auto`. `CanIdle` and `CanLock` since v257.
- `Manager.Inhibit(what, who, why, mode) → h`: `what` among shutdown,
  sleep, idle, handle-power-key, handle-suspend-key, handle-hibernate-key,
  handle-lid-switch, handle-reboot-key; modes `block`, `block-weak` (v257),
  `delay` (shutdown and sleep only). `PrepareForSleep(b)`.
- `IdleAction` uses the idle hint and is held off by `idle` inhibitors; a
  desktop that wants to honour `systemd-inhibit --what=idle` watches
  `BlockInhibited` (hypridle does).
- NEWS: v257 `block-weak`, `CanLock`/`CanIdle`; v258 sessions tracked by
  pidfd and stopped when the leader exits; v260 `CanSuspend()` gained
  inhibitor-related answers; v262 `pam_systemd` `inhibit=`/`inhibit-why=`.
  Nothing new for idle or lock in 262.
- swayidle (a959e59, 2026-08-27) holds a sleep delay lock, runs
  `before-sleep` on `PrepareForSleep(true)` and releases it; it waits for
  the command only with `-w`. hypridle (aa958ed, 2026-08-11) `inhibit_sleep
  = 3` holds the lock until `hyprland-lock-notify-v1` reports locked;
  `InhibitDelayMaxSec` (5 s) caps both.

### 6.4 Sleep with NVIDIA

nvidia-utils turns off systemd-sleep's freezing of user sessions, so the
compositor runs through suspend entry. cosmic-comp #2078 (open, 2026-02-10)
shows page-flip retry loops and a 60 s freeze without `PrepareForSleep`
handling; #2522 (open) loses the DRM device on resume.

**Recommendation.** Isle holds a sleep delay inhibitor from start. On
`PrepareForSleep(true)`: lock in the core, commit a locked frame on every
active output (outputs that are off count as locked, as in niri), set
`LockedHint`, stop rendering, close the fd; within 5 s. Re-take it on
`PrepareForSleep(false)`. Answer the session's `Lock` and `Unlock` signals,
set `IdleHint` at the idle step and `LockedHint` with the lock, honour logind
`idle` block inhibitors, and serve the ScreenSaver interface with
`SetActive(false)` refused.

---

## 7. DPMS

- Smithay `DrmCompositor::clear()` (`compositor/mod.rs:4085`) calls
  `AtomicDrmSurface::clear_state()`: one `ALLOW_MODESET` commit that resets
  the used planes, the connectors and the CRTC. It is a full disable, not
  `ACTIVE=0`; the next `queue_frame` modesets back. There is no
  ACTIVE-only call, and the kernel's guarantee that `ACTIVE` 0→1 never
  fails a test (`drm_crtc.c:203`) does not apply to a full disable.
  `reset_state` re-reads kernel state after a VT switch. Smithay has no
  wlr-output-power-management.
- niri `Tty::set_monitors_active(false)` calls `compositor.clear()` on every
  surface; `true` does nothing and the next frame re-enables; `redraw()`
  skips while inactive; key, button, motion, axis, gesture, touch and
  tablet events wake. PR #3910 (2026-08-23) stopped the redraw loop and
  frame callbacks while off after #3295 (VRAM growth while off).
- cosmic-comp `set_dpms(false)` clears on the surface thread and cancels
  queued renders and vblank timers; any input but device removal wakes.
- KWin also fully disables connector, CRTC and planes.
- NVIDIA reports: niri #4461 (closed 2026-09-28): a DP monitor in standby
  drops hotplug about every 6.5 s, leaking VRAM on each reconnect (also on
  sway and cosmic-comp); niri #4306 (fixed 2026-08-20) black screen when an
  output was turned off at start; cosmic-comp #2428 (open, 2026-05-31)
  displays wake right after blanking; cosmic-comp #2559 (open,
  2026-07-11) RTX 5090 with DSC: GSP crash on wake, avoided with direct
  scanout off.

**Recommendation.** Use `DrmCompositor::clear()` for screens off, stop the
redraw loop and frame callbacks entirely, treat hotplug while off as
reconfiguration and not as activity, wake on the input set above, and make
the first frame after wake a composited one (no direct scanout); when
locked, the lock frame. Test the 5070 Ti (Blackwell, like #2559) with the
monitor's DP standby.

---

## 8. Security of an in-process lock

### 8.1 What others do

- **niri**: `LockState::{Unlocked, WaitingForSurfaces, Locking, Locked}`
  plus per-output render state. `locked` is sent only once every output
  has a locked frame; a failed render unlocks. While locked it draws the
  lock surface or a solid colour and nothing else. If the locker dies the
  session stays locked and a new locker may replace the dead lock (Smithay
  `LockStatus::Defunct`, fea33c8f16, 2026-08-11). Keyboard focus is the
  lock; window lookups return none; popup grabs are dismissed; binds run
  only if `allow-when-locked` or in a fixed list (VT switch, suspend,
  monitors on and off, layout, shortcuts-inhibit toggle). Output
  screencopy and casts show the locked frame; window casts
  (`render_windows_for_screen_cast`) have no lock check.
- **cosmic-comp**: sends `locked` at once, before a locked frame. Shows
  only layers flagged by `cosmic-session-lock-layer-v1` (2026-09-11). Open
  bugs: #2702 (2026-08-06) an input-method keyboard grab survives the lock,
  so the password reaches the IME client; #1946 grabs not cleared.
- **KWin**: links kscreenlocker in-process; the greeter is a separate
  process. Only lock-screen, input-method and lock-overlay windows are
  visible while locked; a `LockScreenFilter` filters input, after the VT
  filter (VT switching works). A crashed greeter is restarted up to three
  times, then a black emergency window tells the user to run `loginctl
  unlock-session`. KWin locks on start when logind's `LockedHint` is set and
  on `PrepareForSleep(true)`.

### 8.2 Guarantees Isle needs

1. The lock state lives in the compositor core, not in the shell's state.
   Rendering and input routing test it before anything else. The shell
   draws the lock but cannot unlock: only the PAM helper's `SUCCESS` (or
   fprintd's match) flips it, plus logind's `Unlock` signal.
2. If the shell's lock view fails (panic caught, no frame within a
   deadline), the core draws a solid lock frame of its own, with no client
   content. If the compositor itself dies the session ends (v258), and the
   lock marker in `$XDG_RUNTIME_DIR` plus `LockedHint` make a restart begin
   locked (platform.md, Lock 4).
3. While locked: no client surface drawn on any output (windows, layers,
   popups, subsurfaces, client cursors; the compositor's own cursor);
   keyboard and pointer focus on the lock; popup and keyboard grabs
   revoked, including input-method grabs (cosmic-comp #2702), pointer
   constraints and Xwayland grabs; virtual-keyboard, virtual-pointer and
   EI input ignored; client idle inhibitors ignored.
4. Capture: output screencopy, image-copy-capture and portal output casts
   render the lock frame; window and toplevel casts pause or send black
   frames; the screenshot tool refuses.
5. Shortcuts: only VT switch, media, brightness, volume, layout and power.
   VT switching stays allowed: another VT needs its own login.
6. Notifications on the lock show app and title only unless the user
   chooses previews (a setting), never actions that open an app.
7. Sleep and DPMS: the first frame after wake is the lock frame; the sleep
   order in §6.4.
8. Failures: a lock-screen attempt counts toward faillock (§1.4); the lock
   shows faillock's "locked … minutes left" text and the fingerprint path
   is still offered within its own stack's rules.

---

## 9. Lock-screen PAM as the user

- Unprivileged `pam_unix` cannot read `/etc/shadow`; `get_pwd_hash` returns
  `PAM_UNIX_RUN_HELPER` and `_unix_run_helper_binary` forks the setuid
  `unix_chkpwd`, writes the password to a pipe and waits for that pid
  (`support.c:515-629`). `unix_chkpwd` lets a non-root caller check only its
  own account: when the name differs from the real uid's, it drops to the
  real uid before reading shadow (`unix_chkpwd.c:100-128`), so the check
  fails.
- Measured on the host with a stack of `auth required pam_unix.so nodelay`:
  a wrong password returns `PAM_AUTH_ERR` (7), not `AUTHINFO_UNAVAIL`; the
  helper ran. `pam_acct_mgmt` through `pam_unix` (the helper's `chkexpiry`
  mode) returns success, logging a harmless "setuid failed".
- A changed password is read from shadow on every attempt, so the lock
  accepts the new password at once; nothing is cached.
- `pam_unix` waits 2 s after a failure unless `nodelay` (`pam_fail_delay`);
  Isle's service inherits it from `system-auth`.
- `pam_systemd_home` (in Arch's `system-auth`) authenticates homed users
  over its own varlink and D-Bus code without libdbus; irrelevant for this
  user (not homed) but kept by including `system-local-login`.

---

## Recommendations for P9, in short

1. PAM in a forked `isle-auth` helper (pam-client2), service
   `/usr/lib/pam.d/isle` with auth and account only; acct_mgmt and
   `PAM_REFRESH_CRED`; all messages shown; fingerprint in parallel through
   fprintd's D-Bus API; never abort a pending conversation with an error.
2. polkit agent on zbus, socket protocol only, session subject resolved as
   polkitd does.
3. `org.gnome.keyring.SystemPrompter` in the compositor with sx-aes-1; it
   covers gnome-keyring, oo7, gcr-ssh-agent and pinentry.
4. NetworkManager agent with VPN_HINTS, nm-applet's keyring schema and VPN
   auth-dialogs in external UI mode.
5. BlueZ agent, KeyboardDisplay, default, all eight methods.
6. `&self` zbus interface methods so cancels arrive while a prompt waits.
7. Idle timers of Isle's own on Smithay's activity hook; inhibitors only
   when visible; ScreenSaver at both paths; logind hints, signals and a
   sleep delay lock released after the locked frame.
8. DPMS by `DrmCompositor::clear()`, with rendering fully stopped while off.
9. Lock state in the core, a core fallback frame, grabs and IME revoked,
   captures blanked.

---

## Sources

linux-pam v1.7.3 (2026-09-24) and master 52b33a5 (2026-10-08); crates.io
pages for pam, pam-client, pam-client2, nonstick, pam-sys, libpam-sys,
pam-bindings; gitlab.com cg909/rust-pam-client and LeChatP fork;
github.com/trifectatechfoundation/sudo-rs; pop-os/cosmic-greeter,
cosmic-osd, cosmic-applets, cosmic-settings, cosmic-session,
cosmic-comp, cosmic-idle, dbus-settings-bindings; kennylevinsen/greetd;
swaywm/swaylock, swayidle, sway; hyprwm/hyprlock, hypridle,
hyprpolkitagent; codeberg ifreund/waylock; invent.kde.org kscreenlocker,
kwin, kwallet, bluedevil, plasma-nm, mart/ksecretprompter;
gitlab.freedesktop.org libfprint/fprintd, polkit/polkit (github
polkit-org/polkit), NetworkManager, NetworkManager-openvpn,
xdg-desktop-portal; gitlab.gnome.org gcr, gnome-keyring, gnome-shell,
gnome-bluetooth, gnome-settings-daemon, libsecret, libnma,
network-manager-applet, gnome-build-meta; github linux-credentials/oo7,
imvaskel/soteria, phisch/psst, waycrate zbus-polkit-agent,
freedesktop-rs/nmrs, blueman-project/blueman, Smithay/smithay,
niri-wm/niri, systemd/systemd (v262 NEWS), gpg/pinentry,
openssh/openssh-portable, chromium/chromium, mozilla-firefox/firefox,
libsdl-org/SDL, mpv-player/mpv, torvalds/linux; git.kernel.org bluez;
gitlab.archlinux.org packaging for polkit and cosmic-greeter; archlinux.org
package pages for oo7, gnome-keyring, swaylock, hyprlock, waylock,
kscreenlocker, gdm, cosmic-greeter. Issues: swaylock #10, #61, #156, #286;
hyprlock #691, #1036, #1074; cosmic-greeter #173, #507; KDE bug 481808;
kscreenlocker MR !318; hyprpolkitagent PR #59; gcr MR !169; niri #3295,
#3910, #4306, #4461, #4613, #2201; cosmic-comp #1946, #2078, #2428, #2522,
#2559, #2702. All read 2026-10-08.

---

## OPEN

Not verified; do not treat as fact.

- **A tokio SIGCHLD lost during pam_unix's window** leaving a child
  unreaped, and a `waitpid(-1)` reaper stealing `unix_chkpwd`'s status:
  inferred from the code, not reproduced. Moot if PAM runs in a helper.
- **A successful unlock through the helper as this user** was not run (no
  password used); only the failure and account paths were measured.
- **Concurrent PAM handles on threads in one process**: KDE up to 6.7 did
  it; individual modules may not be thread-safe.
- **Which Plasma release ships kscreenlocker !318** (expected 6.8).
- **nonstick's repository activity** (Mercurial; only the crate read).
- **CachyOS's polkit 127-3.1** may or may not carry the 2026 CVE fixes.
- **Who calls `org.gnome.keyring.PrivatePrompter`**: no caller found.
- **Whether `org.kde.secretprompter` has shipped** in a Plasma release.
- **That COSMIC ships no keyring prompter**: default-branch code search only.
- **The BR/EDR SSP association table** (chosen by the controller); only the
  kernel's host side and the LE tables were read.
- **BluezQt's default capability** for bluedevil's agent.
- **That no GNOME component other than Settings registers a BlueZ agent.**
- **The protun auth-dialog's behaviour** beyond its strings.
- **GNOME Shell on a WireGuard request** and nm-applet's lack of a WireGuard
  prompt: inferred from code.
- **Whether an ACTIVE=0-only DPMS commit behaves better on NVIDIA 615**
  than Smithay's full clear, and whether a raw commit can coexist with
  DrmCompositor's state.
- **niri window casts while locked** and cosmic-comp toplevel capture while
  locked: from code reading only.
- **How Steam and Proton games inhibit idle** on Xwayland, and whether
  Xwayland forwards `XScreenSaverSuspend` to a Wayland inhibitor.
- **Whether KDE PowerDevil holds a sleep delay inhibitor.**
- **Virtual-keyboard, virtual-pointer and EI input reaching an in-process
  password field**: an inference; no project documents it.
- **Notification previews on the lock**: no source surveyed; §8.2 item 6 is
  a design proposal, not a finding.
