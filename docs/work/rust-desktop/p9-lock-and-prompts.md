# P9 · Lock and prompts

Planned 2026-10-08. Reviewed by the user the same day, choice by choice:
every recommendation taken but two, fingerprint left out of P9 (choice 7)
and logind's `Unlock` honoured (choice 9); one packet; the host run
optional.

The roadmap's brief: the compositor's own lock with PAM off-thread and v1's
locker lessons, with no ext-session-lock served (B28); the polkit agent,
keyring prompter and NetworkManager secret agent (B12) and the Bluetooth
pairing agent (B28) with their dialogs, behind the Wi-Fi and Bluetooth rows
that fail in P8 for want of them; idle, dim and DPMS, with idle and the lock
among the holds on banners. Earlier packets left P9 more: the session
dialog, log out, restart and shut down behind the system menu's disabled
items (P6 choice 21), Force Quit and the not-responding alert (P4 choice
15, P7), a prompt set aside by Cmd+Tab (P7), the island's locked form and
the softened wallpaper (P6), the lock's panic threshold (P3) and the
lock-after-crash rule (P2).

P8 left a desktop that knows the machine but cannot keep it private or ask
the user anything. Nothing locks: the `lock` chord and the system menu's
Lock Screen, Restart…, Shut Down… and Log Out… are reserved and do nothing.
No idle is measured, no screen dims or turns off, no client is told it is
idle, and a playing video holds nothing because nothing would be held. A
secured Wi-Fi network fails in its row with "needs a password", a device
that needs a code cannot pair, `pkexec` finds no agent, and a locked
keyring falls to gcr's own GTK prompter. P9 closes all of it. What it must
get right is that the lock is the compositor's in the strictest sense: no
frame of the session reaches a screen while locked or on resume, whatever
the shell, PAM or a daemon does; that a secret goes only into Isle's own
field and from there only to the program that asked; and that the loop
never waits on PAM or an agent's caller.

## What it answers to

- Decisions: B9 (blocking work off the loop, panic boundaries), B12 (the
  prompt roles and the lock), B17, B26 (the greeter reuses the lock, in
  P14), B28 (no ext-session-lock; the Bluetooth agent), B30 (the lock chord
  in the escape set; no dim, lock or sleep over a game), B33 and B58
  (glass), B37 (surfaces, keyboard handover, the panic count), B44 (one
  focus path), B46 (the game flag), B47 (the island's model), B51 (the main
  screen), B55 (apps by their processes), B57 (centre surfaces), B60 (the
  service shape), B63 (holds on banners), B64 (Focus).
- Intent: `shell.md` Shell runtime 4, 10, 13 and its open point, Lock
  screen 1–10 and its open points, System prompts 1–9, Wallpaper and idle
  dimmer 3–4, acceptance bars 10, 11, 14; `platform.md` Polkit agent 1–6,
  Keyring prompter 1–4 and its open points, Network and Bluetooth agents
  1–2 and their open points, Lock, PAM and idle 1–10 and its open point,
  Session start 5 (the roles' start order), Supervisor 4 (the minimal
  lock's notice), acceptance bars 3, 4, 5 (for these roles), 11;
  `compositor.md` Idle, DPMS and sleep 1–5 and its open point, Session and
  robustness 4 and 8, the protocol table's idle-inhibit and ext-idle-notify;
  `gaming.md` Game Mode 8 and its open point.
- Left to P9 by earlier packets: listed above; the `locked` flag on P5's
  shortcut table, which nothing reads yet; `Holds::away`, which is sleep
  alone today (P8 choice 14); the notification server's `Inhibited`
  property, which is never sent (P8).
- Design, binding: `p1/structure/lock.md`, `prompts.md`, `competition.md`
  (Over everything, Keyboard focus), `island.md` (Locked, the prompt
  waiting activity), `wallpaper.md` (the softened form),
  `window-chrome.md` (the not-responding alert), `notifications.md`
  (Away, Locked); `p1/surfaces/lock.md`, `prompts.md`, `wallpaper.md`,
  `window-chrome.md`; `review/accessibility.md` and `copy.md`; DESIGN.md's
  Lock and Prompts entries.
- Evidence: `lock-and-prompts-2026.md` (new; the § numbers below are its
  sections unless another note is named), `ui-and-services.md` §3,
  `passkeys.md` §0 (the host's fingerprint and PAM), `design-v1-lessons.md`
  §5 Lock, auth and power, `isle-v1-inventory.md` §4 Lock and idle.

## Choices this plan makes

1. **The lock is a state of the core, drawn by its own path.**
   `isle-compositor`'s new `lock` module holds whether the session is
   locked; the shell only draws the face. While locked an output's frame is
   built by `render_locked`, a function separate from the desktop's that is
   given no windows, layer surfaces, Xwayland surfaces or desktop shell
   surfaces, only a fixed list: the idle dim, the island in its locked
   form, the lock face, the backdrop. So no client frame can be scanned out
   while locked by construction, not by a filter that could miss one
   (`platform.md` Lock 1, acceptance bar 10). The backdrop (the softened
   wallpaper and the lock's wash) is drawn by the compositor itself, so the
   first locked frame never waits for a Freya tree; the face fades in when
   its tree has drawn. A screen added while locked gets the same list in
   its first frame.
2. **Locking takes everything back.** On lock: keyboard focus moves through
   the one focus path to the lock face (B44); pointer focus, grabs,
   pointer constraints and keyboard-shortcuts inhibitors are released;
   Xwayland's keyboard grabs and input-method grabs are refused, and
   virtual-keyboard and virtual-pointer input from clients is dropped, so
   no client can read or type into the field (`lock-and-prompts-2026.md`
   §8.2: an input method once received a locker's password on
   cosmic-comp); the centre surface, menus,
   Control Centre, Notification Centre and the window menu close (B57); a
   prompt on screen goes back to waiting with its field cleared; banners
   are held. While locked only shortcuts marked `locked` in P5's table act
   (media, volume, brightness, layout) and the power key, which focuses the
   lock's Sleep. Clients keep running and playing sound; their frame
   callbacks are throttled as for a hidden window. Unlocking gives focus
   back to the window that had it, so a game takes its pointer lock again
   as after the switcher.
3. **The marker comes first.** `$XDG_RUNTIME_DIR/isle/locked` is written
   before the first locked frame is queued and removed once the unlock's
   transition ends. A compositor that starts and finds it starts locked,
   before lighting any output, so a crash, `kill -9` or the safe start
   comes back to the lock's backdrop as its first frame (`platform.md`
   Lock 4, acceptance bar 4). This is P2's lock-after-crash rule.
4. **PAM runs in a helper process, `isle-auth`.** B12 and ARCHITECTURE
   planned a PAM thread in the compositor. Every locker and greeter
   surveyed keeps PAM out of its main process, KDE since August 2026
   (`lock-and-prompts-2026.md` §1.5), and a thread would put every module
   the stack names into the compositor: unprivileged `pam_unix` sets
   `SIGCHLD` to its default for the whole process while it waits for
   `unix_chkpwd`, which can drop the signal `tokio::process` reaps the
   compositor's launched apps by; `pam_gnome_keyring` changes `SIGPIPE`
   and `SIGCHLD` and may fork; `pam_authenticate` cannot be cancelled; and
   a module that crashes ends the session. (No libdbus is loaded by the
   host's stack, measured, §0.1; only `pam_snapper` would bring it.)
   `isle-auth` is a small binary in the workspace on `pam-client2` (the one
   maintained crate with the whole conversation and credentials, §1.1),
   non-dumpable, with the compositor as its parent-death signal, started
   when the lock comes and ended at unlock, restarted if it dies. It
   speaks a line protocol over a socketpair, modelled on polkit's helper:
   PAM's prompts and messages up, answers down, the result last. One
   transaction per attempt: `pam_authenticate`, `pam_acct_mgmt`, and
   `pam_setcred(PAM_REFRESH_CRED)` on success, with the helper's locale
   set to C so `pam_faillock`'s info messages ("The account is locked due
   to…", "(n minutes left to unlock)") are read as the lockout and its
   minutes; the face shows Isle's words (`structure/lock.md` Copy), and
   any other message as given. A pending attempt is never ended by
   answering it with an error, which `pam_faillock` counts as a failure
   (hyprlock #1074): when logind's `Unlock` ends the lock mid-attempt,
   the helper is killed. A dead helper is "Couldn't check the password. Try again." and
   a new one. ARCHITECTURE's PAM thread becomes this helper. Rejected: a
   thread (above).
5. **Isle's PAM service.** `data/pam/isle`, installed as
   `/usr/lib/pam.d/isle` by P14's package (an `/etc/pam.d/isle` overrides
   it) and by hand in the VM until then: `auth include
   system-local-login`, `-auth optional pam_gnome_keyring.so`, `account
   include system-local-login`, and no session lines, so `pam_systemd` is
   never loaded (§1.3, §0.1). On Arch that brings `pam_faillock`, so the
   lock and polkit share one count as `structure/lock.md` asks, and since
   the tally file is the user's own, failures on the lock count (§1.4);
   `pam_unix` through the setuid `unix_chkpwd` (§9); and
   `pam_gnome_keyring`'s auth step, which unlocks the login keyring from
   the password where it is locked (`shell.md` Lock 9). No `pam_fprintd`,
   which would block the password until the finger gives up (choice 7).
   With no `isle` service in either directory
   the helper uses `login` and says so in the log, since a missing service
   falls to `other`, which denies everything and would leave the lock
   unopenable. This answers `platform.md`'s open point.
6. **The typed password lives in the compositor, not in a tree.** Keys
   reach the lock's field from the key engine, and the field is a
   zeroizing buffer on the loop; the face draws its dots by count. So a key
   typed while dozing, after waking a dark screen, or while the face is
   rebuilt after a panic is kept, and nothing in Freya's text editing
   holds the secret. The prompts' secret fields are `isle-ui`'s new
   `SecureField`, which keeps its text in a zeroizing buffer, draws dots,
   refuses copy and cut, takes paste, and hands its value to the loop on
   the action; the worker clears it after the requester has it (`platform.md`
   acceptance bar 11). Rejected: the Freya fork's `Input` in its hidden
   mode, whose text is an ordinary string kept across frames.
7. **No fingerprint in P9.** The host has no reader (`passkeys.md` §0),
   so the lock's fingerprint mark, its states and copy (`structure/lock.md`
   PAM and fingerprint) wait until there is one; the field's placeholder
   is "Enter password". When it comes, it runs beside the password through
   fprintd's D-Bus API, never `pam_fprintd` in the lock's stack (§1.4), and
   a match ends a pending attempt by killing the helper (choice 4). The
   polkit agent still holds a typed password until PAM asks for it, which
   is plain conversation handling for any stack that asks for something
   first (choice 20). Reviewed: the recommendation was to build it against
   a fake fprintd now; the user left it out.
8. **The minimal lock after the second panic.** A panic in the face
   rebuilds it under B37's boundary with the dots kept (choice 6). A
   second within a minute replaces it with `lock-minimal`, a separate small
   surface (`surfaces/lock.md` The minimal lock), until unlock. If that one
   panics too, the compositor keeps drawing a solid backdrop and keys still
   reach the field and PAM, so Return still unlocks, and `isle shell
   restart lock` brings the face back from another console. After an
   unlock from the minimal lock, the desktop posts the crash notice's form
   ("The lock screen had a problem.", Report…). This answers `shell.md`'s
   open point: two, where B37's boundary allows six for other surfaces.
9. **What locks, and what does not unlock.** The `lock` chord
   (Ctrl+Cmd+Q, in the escape set), the system menu's Lock Screen, the
   session chooser's L, `Session1.Lock` and `isle lock`, logind's `Lock`
   signal on Isle's session (`loginctl lock-session`), the ScreenSaver
   interface's `Lock`, idle (choice 12) and sleep (choice 10). Hot corners
   come with P16. Locking and unlocking set logind's locked hint.
   logind's `Unlock` signal on Isle's session unlocks, as KDE's does
   (`loginctl unlock-session`), so a session whose face cannot be used is
   recoverable over ssh or from another console; logind checks the caller
   against polkit's `org.freedesktop.login1.lock-sessions`. Nothing else
   ends the lock: ScreenSaver's `SetActive(false)` is refused (choice 14),
   and B28 still holds, since no client can take the lock or serve one.
   Reviewed: the recommendation was to ignore `Unlock`; the user chose to
   honour it.
10. **The lock is on every screen before sleep.** Isle holds a logind
    `sleep` delay inhibitor from start. On `PrepareForSleep(true)` it locks
    (the `lock-screen` setting `lock-on-sleep`, on by default; lid close
    reaches here as sleep, the lid itself staying logind's) and releases
    the inhibitor once every lit output has presented a locked frame,
    known from presentation feedback, and it has stopped drawing until
    resume, since NVIDIA's package keeps systemd from freezing the session
    during suspend (§6.4); an output that is off counts as done, since it
    will be lit with the lock (choice 15). On
    `PrepareForSleep(false)` it takes the inhibitor again and counts the
    resume as activity. logind's `InhibitDelayMaxSec` bounds a stuck
    frame; Isle logs it (`shell.md` Lock 4, acceptance bar 10). Isle also
    holds `handle-power-key` as a block inhibitor, so the power key opens
    the session chooser rather than powering off (`structure/prompts.md`
    Shared places).
11. **The softened wallpaper is made on the worker.** Beside the scaled
    copy P6 makes, the worker makes the softened form per screen and
    appearance (`surfaces/wallpaper.md`: blur, saturation and wash from
    the tokens) at a quarter of the screen's size, since shapes do not
    read in it, uploaded as a small texture and drawn scaled up with
    linear filtering. It is ready before any lock, so the first locked
    frame draws it at once; about 1.2 MiB at 3440 × 1440. The dashboard
    (P17) reads the same texture.
12. **Idle is one clock and one timer.** Every input from any device
    stamps the last activity; a resume, a connector returning after a
    real absence (choice 15) and `SimulateUserActivity` count as activity
    too. Smithay has no idle timer of its own (§6.1). A pure `idle::policy`
    takes the idle time, the timeouts, whether locked and what holds, and
    answers the current step (none, dim, lock, off) and the next edge; the
    loop arms one timer and rebuilds it on any change, so a changed
    timeout counts from the current idle time and one already past waits
    for the next idle period, and zero is off, never at once (D18). The
    timeouts are the new `lock-screen` section's `dim-after`, `lock-after`
    and `display-off-after` (5, 10 and 15 minutes, `structure/lock.md`
    Idle); while locked, the tokens' `timing-lock-dim-ms` and
    `timing-lock-off-ms` apply and nothing holds them. Dimming sets
    logind's idle hint; activity clears it.
13. **What holds idle, and the protocols.** All three steps are held by:
    idle-inhibit from a surface mapped on a lit output and not hidden or
    minimised; ScreenSaver inhibitors (choice 14); logind's block
    inhibitors of `idle`, read from `BlockInhibited`; and a game in front,
    windowed or full screen (B46, `gaming.md` Game Mode 8). P11's portal
    Inhibit and P18's pad input feed the same holds. `idle-inhibit` and
    `ext-idle-notify` v2 are served through Smithay's states: activity is
    reported on every input, the holds set the notifier's inhibited state,
    and input-idle notifications ignore it as the protocol asks. The holds
    are listed by `isle idle` and named in Control Centre's Display page
    ("Kept awake by {app}", `structure/control-centre.md`); nothing shows
    at rest.
14. **ScreenSaver is the worker's name, the loop's state.**
    `org.freedesktop.ScreenSaver` is owned on the session bus before the
    socket is announced, at `/org/freedesktop/ScreenSaver` and
    `/ScreenSaver` (Firefox's path): `Inhibit` and `UnInhibit` with
    cookies per sender, dropped when the sender leaves the bus; `Lock`;
    `GetActive`, `GetActiveTime`, `GetSessionIdleTime`,
    `SimulateUserActivity` and `ActiveChanged` (active meaning locked);
    `SetActive(true)` locks and `SetActive(false)` is refused, so no
    program can end the lock (§6.2). The worker forwards each call
    to the loop, which keeps the holds; an inhibitor's app comes from its
    sender's pid (B55). This answers `compositor.md`'s open point, and
    `gaming.md`'s: Steam and SDL games inhibit through this interface, and
    rule 8's game in front covers those that do not.
15. **The dim and display off are the compositor's.** The dim is a solid
    element of `colour-dim` over everything on each output, its alpha
    rising over `motion-dim-ms`; any input cuts it, and that input is
    delivered. Display off clears each output's DRM compositor
    (`DrmCompositor::clear()`, a full disable of planes, connector and
    CRTC, as niri, cosmic-comp and KWin do, §7) and stops that output's
    redraws and frame callbacks entirely; waking queues a frame, which
    modesets the output back, and that first frame is composited, never
    direct scanout (an RTX 5090 crashes waking into it, cosmic-comp #2559),
    and is the lock's when locked (`shell.md` Lock 5). The input that
    wakes a dark screen is consumed; on the lock, keys after it reach the
    field. The off state is applied again whenever the outputs change
    (D76). A connector that drops and returns within ten seconds while the
    screens are off is the monitor's standby flapping (niri #4461 on
    NVIDIA) and stays off; one that was gone longer is a KVM's return,
    counts as activity and lights, as `compositor.md`'s KVM bar asks.
    Nested and headless backends simulate it. `isle displays
    sleep` turns them off now; the corner action comes with P16.
16. **Banners and the island while away.** `Holds::away` becomes sleep,
    dimmed, displays off or locked; the server's `Inhibited` property
    follows every hold at last (P8). While locked, what arrives goes to the
    lock's list by `notifications.lock-screen`, which P8 added, and to
    Notification Centre; after unlock the summary is P8's ("While you were
    away"). The island's model takes `locked`: only the privacy form and
    OSDs draw, events are dropped and activities resume at unlock
    (`structure/island.md` Locked). Its locked form draws above the lock
    face in `render_locked`.
17. **The face is a surface fed from the loop.** `isle-shell`'s `lock`
    surface (every output, the face on the pointer's screen, date and time
    on the others) draws a `lock::Face` model the loop builds: the lock's
    clock, the Focus glyph, Now Playing from `isle-media`, the list, the
    user's name and picture from AccountsService (else the login name),
    the dot count, the status line's one message by `structure/lock.md`'s
    order, Caps Lock, the layouts, the battery from
    `isle-power`. Dozing after `timing-doze-ms` with the field empty; any
    printable key wakes it and is typed. The bottom row's Sleep is
    logind's `Suspend`; Restart and Shut Down confirm in place and go
    through choice 22.
18. **One prompt queue, a pure model.** `isle-shell`'s `prompts::queue`
    takes requests (kind, asker, the asker's output, whether the user
    started it, arrival) and the holds (locked, a full-screen game in
    front, a full-screen window in front unless its app asked, set aside,
    the prompt ahead) and answers which prompt shows, which wait and why,
    and what the island's prompt activity shows, as the banners' model
    does (B63). The user started a request when Control Centre asked for
    that network or device a moment before, or when the asker's pid
    descends from the front app's processes (`pkexec` in the front
    terminal, B55); those go to the front. A requester's cancel removes
    its request silently, on screen or waiting.
19. **The prompt is one surface over a compositor scrim.** `prompt` is a
    placed surface on the asker's screen, else the main screen, at a fifth
    of its height, above every client and every shell overlay and below
    the lock; it takes the keyboard while it shows and nothing takes it
    back but Cmd+Tab, which sets it aside and switches (gate 2). The scrim
    is a compositor element on every output under it, as the dim is, so no
    client can draw above it (`shell.md` Shell runtime 13). The armed
    moment is the tree's: for `timing-prompt-arm-ms` the action ignores
    Return, Space and clicks while fields take typing. A prompt waiting is
    the island's prompt activity with Show and Cancel. Over Control Centre
    or Spotlight, and with a windowed game in front, the frame is solid.
20. **The agents are worker roles, registered before the socket.** Each is
    on the worker under the supervisor, registered before the compositor
    announces its socket (`platform.md` Session start 5) and again when
    its daemon's name returns or the role restarts (acceptance bar 5).
    Each turns a call into a prompt request with a reply channel; the
    D-Bus method awaits it, so the caller waits and the loop does not.
    Every agent method takes `&self`, so a cancel is served while a
    prompt's call is still waiting (with `&mut self` zbus queues the cancel
    behind it, §4.4).
    - **Polkit** (`isle-polkit`, new): the `AuthenticationAgent` object,
      registered for Isle's logind session, found from `XDG_SESSION_ID`,
      else logind's session of Isle's pid. `BeginAuthentication` becomes
      the polkit prompt: the asker from `polkit.subject-pid`, the
      identities (the user first), Details from the action and, for
      `pkexec`, its program and command line. Each attempt is a
      conversation over `/run/polkit/agent-helper.socket` (polkit 127,
      B12; the host's helper is not setuid, so the old spawned helper
      fails, §2.2), its lines unescaped: PAM's prompts are answered from
      the field, its messages go to
      the status line, and with `pam_fprintd` in polkit's stack a typed
      password is held until PAM asks for it (v1 D34). Failure is "Wrong
      password" and a new conversation; Cancel returns polkit's
      cancellation; `CancelAuthentication` closes the prompt. cosmic-osd's
      agent is the reference.
    - **Keyring** (`isle-prompter`, new): owns
      `org.gnome.keyring.SystemPrompter` and serves
      `org.gnome.keyring.internal.Prompter` at `/org/gnome/keyring/Prompter`
      (`BeginPrompting`, `PerformPrompt`, `StopPrompting`), answering on
      the caller's `Callback` with `PromptReady` and `PromptDone`. The
      secret goes back through the `sx-aes-1` exchange (Diffie–Hellman,
      HKDF-SHA256, AES-128-CBC) in RustCrypto crates, each reply value
      wrapped twice as gcr does, with oo7's prompter as reference (§3.1).
      The name is owned without allowing replacement, so gcr's GTK
      prompter is neither activated nor able to take it. Unlock, new
      keyring with its confirmation and strength, and confirm prompts, as
      `structure/prompts.md` draws them. This one service is what
      gnome-keyring 50, oo7, gcr's SSH agent (a key's passphrase, with its
      "Automatically unlock" choice) and `pinentry-gnome3`, Arch's default
      for gpg, all call (§3.4), which answers `platform.md`'s open points.
    - **NetworkManager** (in `isle-network`): the `SecretAgent` object,
      registered with the AgentManager with VPN hints, without which no
      agent of the user's can pass a VPN's two-factor prompt (§4.3).
      `GetSecrets` for `802-11-wireless-security` (PSK and WEP), `802-1x`
      (identity, password, private key password) and `vpn`; without
      `ALLOW_INTERACTION` it answers from the keyring or with no secrets;
      `REQUEST_NEW` is the wrong password's message with the old one
      selected; `USER_REQUESTED` goes to the front. NetworkManager gives up
      after 120 s without a cancel, so the prompt closes then too. Isle
      keeps no secret: a connection Isle makes keeps its secret in
      NetworkManager (`psk-flags` 0, as the host's two connections do);
      an agent-owned secret of a connection made by GNOME's or KDE's
      tools is read from and saved to the Secret Service under
      nm-applet's attributes, through the `oo7` client crate, so those
      connections keep their secrets (§4.1; `platform.md` Network agents
      1). Certificates are not prompted; a connection that needs one is
      made in Settings (P13). The Wi-Fi row's "needs a password" goes:
      choosing a secured network now raises the prompt over the pinned
      panel.
    - **VPN**: a plugin whose `.name` file says
      `supports-external-ui-mode` (both of the host's, OpenVPN and
      Proton's) has its auth dialog run as GNOME Shell does (`-u`, `-n`,
      `-s`, `--external-ui-mode`, `-i`, `-t` per hint), fed the
      connection's data and saved secrets on stdin; the key file it prints
      describes the fields, which the frame draws, and those it says need
      not be asked go straight into the reply. Others run their own GTK
      dialog, outside Isle. This answers `platform.md`'s VPN point.
    - **Bluetooth** (in `isle-bluetooth`): `org.bluez.Agent1` registered
      with `KeyboardDisplay` and made the default. Each method is a prompt
      by `structure/prompts.md`'s table; `DisplayPasskey`'s typed count
      marks the code's cells; `Cancel` closes; refusal is
      `org.bluez.Error.Rejected`. A pairing started from Control Centre
      goes to the front over the pinned panel.
21. **Who asks.** A request's pid becomes an app through the registry
    (B55) and its window's output; with none, the kind's symbol and the
    main screen. Requesters' strings are plain text, never markup
    (`shell.md` System prompts 5).
22. **Ending the session is one engine.** `session_end` takes an action
    (log out, restart, shut down), asks every app to quit as Cmd+Q does,
    keep-running ones included (B30), and waits. An app that asks to save
    stops it ("Couldn't restart because {app} didn't quit.", Show {app});
    one that misses its ping is "Not responding" with Force Quit {app},
    after which the action goes on. Then log out ends the compositor
    cleanly, which ends the session (P2's clean exit), and restart and
    shut down call logind's `Reboot` and `PowerOff`; a block inhibitor
    held by another program is named in the same line. It serves the
    session chooser (the power key, `session-chooser`), the confirmation
    (the system menu's Restart…, Shut Down…, Log Out… and `log-out`),
    `log-out-now` with no confirmation (replacing P5's plain quit), and
    the lock's power row. "Reopen windows when logging back in" is not
    shown until session restore exists (P20), so it promises nothing.
23. **Force Quit and the not-responding alert.** Force Quit joins the
    centre surfaces (B57) on Cmd+Option+Escape and the system menu: every
    running app from the registry, background apps and Isle's own
    included (B55), each window pinged as it opens so "Not responding" is
    current, multiple selection, the sheet with Cancel as default. Force
    quitting sends `SIGKILL` to every process of the app; one refused is
    "Couldn't force quit". The not-responding alert is an attached surface
    on the frozen window, as title bars are (B41): "{app} is not
    responding", Force Quit and Wait (`surfaces/window-chrome.md`).
24. **Components join `isle-ui`.** `SecureField`, the text field, the
    checkbox row, the pop-up button for identities, the code cells, the
    Details box, the destructive well button and the prompt frame, each in
    the gallery and `isle-preview`.
25. **The tests gain the daemons' other halves.** `isle-fakes` gains
    logind's `Lock` and `Unlock` signals, `SetIdleHint`, `Inhibit` (with
    the descriptor watched for close), `BlockInhibited`, `Reboot` and
    `PowerOff`; polkit's authority with an agent-helper socket of
    its own; NetworkManager's AgentManager, which calls the agent when a
    test asks; BlueZ's AgentManager1, likewise. A test keyring client does
    gnome-keyring's side of the exchange. PAM runs for real: in
    `test-hooks` builds `isle-auth` takes a configuration directory
    (`pam_start_confdir`) holding a service of `pam_exec expose_authtok`
    over a script that accepts one password and counts failures. The test
    client gains `idle-inhibit` and `screensaver`. `Test1` gains the lock's
    state, the element kinds in each output's last frame, an idle clock
    override, each output's power state and the prompt queue.

## The workspace

```
crates/
  isle-auth/             the PAM helper binary (new)
  isle-polkit/           the polkit agent and the helper-socket conversation (new)
  isle-prompter/         org.gnome.keyring.SystemPrompter, sx-aes-1 (new)
  isle-network/          the secret agent; VPN auth dialogs in external-UI mode
  isle-bluetooth/        Agent1
  isle-fakes/            logind's additions, polkit, NM and BlueZ agent managers
  isle-ui/               SecureField, text field, checkbox, pop-up button, code cells,
                         details box, destructive well, prompt frame
  isle-settings/         `lock-screen`
  isle-shell/src/
    lock/                the face model and surface, the minimal lock
    prompts/             the queue model, the prompt surface, the session dialog,
                         Force Quit, the not-responding alert
  isle-compositor/src/
    lock.rs              lock state, the marker, the field, render_locked, the helper
    idle.rs              the policy, the timer, holds, idle-inhibit, ext-idle-notify,
                         the dim, display power
    screensaver.rs       org.freedesktop.ScreenSaver on the worker
    prompts.rs           requests from the agents, the queue on the loop, the scrim
    session_end.rs       log out, restart, shut down; the chooser and confirmation
    force_quit.rs        the panel and the alert
    services/login.rs    Lock, sleep and power-key inhibitors, hints
    wallpaper.rs         the softened form
  isle-ipc/              Session1.Lock, Idle1, Prompts1, Test1 additions
  isle-cli/              isle lock, isle idle, isle displays sleep, isle prompts
data/pam/isle            Isle's PAM service
```

## Phases

Each phase ends in a check run by script or in the VM and lands as its own
commits.

### 9.0 · The locked core

- The lock's state, the marker and starting locked, `render_locked` with
  the compositor's backdrop, focus and grabs taken back, the `locked`
  shortcuts, the softened wallpaper on the worker (choices 1–3, 11);
  `Session1.Lock`, `isle lock`, the chord and Lock Screen; a test-hooks
  unlock; `Test1`'s frame element kinds.

**Check:** headless: locking with a client window, a layer-shell overlay
and an X11 window up, every later frame on every output holds no client
element; the first frame after the request is the backdrop; a screen added
while locked starts locked; a keyboard-shortcuts inhibitor and a pointer
lock are released and come back at unlock; only the `locked` chords act;
`kill -9` while locked restarts locked with no unlocked frame (acceptance
bar 4).

### 9.1 · Authentication

- `isle-auth`, the line protocol, the field in the compositor, PAM's
  results and lockout, `data/pam/isle` and the fallback (choices 4–6).

**Check:** headless with the test PAM directory: the right password
unlocks, a wrong one counts, the third locks out and the minutes are read,
a killed helper is "Couldn't check the password" and a new one serves the
next attempt; the loop logs no stall while PAM sleeps. In the VM
with `/etc/pam.d/isle`, the real password unlocks and `faillock` counts.

### 9.2 · The face

- The `lock` surface and `lock::Face` (choice 17): date and time, Now
  Playing, the list by setting, the user, the field's dots, the status
  line, Caps Lock, the layout menu and its mark, the battery, Sleep,
  dozing, many screens, every Accessibility form; the minimal lock and the
  threshold (choice 8); the island's locked form (choice 16).

**Check:** screenshots against `p1/surfaces-b.html`'s lock crops; a typed
key while dozing reaches the field; the face follows the pointer to another
screen with its dots; 100 injected panics in the face never show session
content and the second in a minute brings the minimal lock (acceptance bar
10); a keyboard-only walk reaches every control (acceptance bar 14).

### 9.3 · logind: sleep, the signals and the power key

- The sleep delay inhibitor and the lock before sleep, `Lock` from logind,
  `Unlock` honoured, the locked and idle hints, the power-key inhibitor
  (choices 9, 10).

**Check:** headless against the fake: `PrepareForSleep(true)` locks and the
inhibitor is released only after every output presented a locked frame;
`lock-session` locks; `unlock-session` unlocks and kills a pending
attempt's helper without a failure counted; the hint follows. In
the VM, `systemctl suspend` with a window up resumes to the lock.

### 9.4 · Idle, the dim and display power

- `idle::policy`, the timer, the `lock-screen` section, the holds,
  idle-inhibit and ext-idle-notify, the dim, display off and on, waking
  rules, the away hold on banners and `Inhibited` (choices 12, 13, 15, 16);
  `isle idle`, `isle displays sleep`, the Display page's holder line.

**Check:** unit tests of the policy with a fake clock (a timeout changed
below the current idle time, zero, locked timeouts, holds); headless: the
dim, lock and off steps in order with an idle clock override, a click
during the dim delivered and one in the dark consumed, an idle-inhibit
surface holding all three and losing its hold when minimised, a game in
front holding them, ext-idle-notify's idled and resumed events reaching the
test client, a banner held while dimmed and summarised after.

### 9.5 · ScreenSaver

- The interface on the worker, cookies by sender, `Lock`, the active
  signal (choice 14).

**Check:** headless: an inhibitor holds idle and is dropped when its client
leaves the bus; `Lock` locks; killing the role owns the name again within a
second. In the VM, Chromium and Firefox playing a video hold idle.

### 9.6 · The prompt frame and queue

- `isle-ui`'s new components (choice 24), `prompts::queue`, the `prompt`
  surface, the scrim, the armed moment, waiting and the island's prompt
  activity, Cmd+Tab setting aside, solid over Control Centre and a
  windowed game (choices 18, 19, 21); a test prompt through `Test1`.

**Check:** unit tests of the queue (arrival order, user-started first,
cancel on screen and waiting, every hold); headless: Return within the
armed moment does nothing, after it answers; a prompt waits while locked
and comes forward after unlock, armed afresh; Cmd+Tab sets it aside and
Show brings it back; no client draws above the scrim; screenshots against
`p1/surfaces-b.html`'s prompt crops.

### 9.7 · The polkit agent

- `isle-polkit` (choice 20): registration, the helper socket conversation,
  identities, Details, a password held until PAM asks, wrong and lockout,
  cancel both ways.

**Check:** headless against the fake authority and helper: the prompt names
the asker, the right password completes the call, a wrong one asks again,
`CancelAuthentication` closes it, Cancel returns the cancellation, the role
killed registers again. In the VM, `pkexec true` from kitty shows Isle's
prompt (acceptance bar 3) and a wrong password counts with the lock.

### 9.8 · The keyring prompter

- `isle-prompter` (choice 20): the interface, the exchange, the three kinds.

**Check:** headless with the test keyring client: unlock, new with the
mismatch message, confirm, the secret decrypted by the client. In the VM,
`secret-tool` against a locked keyring shows Isle's prompt and the keyring
opens.

### 9.9 · Network secrets

- The secret agent, Wi-Fi and 802.1X fields, `REQUEST_NEW`, VPN external-UI
  dialogs; Control Centre's Wi-Fi row raising the prompt (choice 20).

**Check:** headless against the fake AgentManager: a secured network from
the Wi-Fi page raises the prompt in front over the pinned panel, Join
returns the secret, a wrong one is asked again with its message, a cancel
from NetworkManager closes it. In the VM, a WPA2 network from
`hostapd` on a virtual radio, and an OpenVPN connection's password through
its plugin's external UI.

### 9.10 · Bluetooth pairing

- The agent (choice 20), each method's prompt, the code cells, Control
  Centre's pair row.

**Check:** headless against the fake: each of the five kinds shown and
answered, `DisplayPasskey`'s count marking cells, `Cancel` closing, the
agent registered again when BlueZ returns.

### 9.11 · Ending the session

- `session_end` (choice 22), the chooser with its countdown, the
  confirmation, the lock's confirm in place and "Quitting apps…", log out
  with and without confirmation, the power key; the system menu's Lock
  Screen, Restart…, Shut Down… and Log Out… enabled.

**Check:** headless: the chooser from the power key and the chord, its
letters, the countdown stopping on a key; restart with a test client that
quits and one that refuses (Show {app}) and one that does not answer
(Force Quit {app}); log out exits the compositor cleanly; the fake logind
records `Reboot` and `PowerOff`; screenshots against the session dialog's
crops.

### 9.12 · Force Quit and the not-responding alert

- The panel, its sheet, the kill, the alert on close (choice 23).

**Check:** headless: a test client that stops answering pings is marked and
selected, Force Quit ends every process of its app, a background app is
listed, the alert on close force quits or waits.

### 9.13 · The measure, the VM and the host

- Release build at 3440 × 1440: the face and a prompt under 2 ms p95; the
  time from a lock request to the first locked frame; the softened form's
  VRAM; nothing drawn or timed while idle at rest; the shell inside its
  budgets.
- The VM: lock and unlock, suspend and resume, idle steps, `pkexec`,
  `secret-tool`, Wi-Fi and VPN prompts, `loginctl lock-session`. The host
  run, on tty8, is where DPMS on the NVIDIA driver, the KVM's return and a
  real keyring are seen; it is the user's choice, as for P6 to P8.

**Check:** the measures in `docs/research/rust-desktop/p9-lock-and-prompts.md`;
the VM's screenshots.

### 9.14 · Close

- ARCHITECTURE: how the lock holds (the core's state, the locked render
  path, the marker, the helper), how prompts are asked (the roles, the
  queue, the scrim), how idle decides; the PAM thread replaced by
  `isle-auth` in the process layout.
- DECISIONS for what later packets keep: the locked render path and the
  marker; PAM in a helper; the minimal lock's threshold; idle's one
  policy and its holds; prompts as one queue over a compositor scrim; the
  agents' secrets kept nowhere; one session-ending engine.
- The intent docs' open points; the roadmap's P9 entry, P11 (the lock in
  captures and the portal's Inhibit), P14 (the PAM service in the package,
  the greeter on the face), P16 (the lock and display-sleep corners), P18
  (pad input as activity), P20 (Reopen windows); `state.md`,
  `progress.md`.

## Not in P9

Fingerprint on the lock, until there is a reader (choice 7); the greeter
and its fingerprint sign-in (P14, on this face and greetd's PAM); the
package installing `/usr/lib/pam.d/isle` (P14); hot corners' Lock
Screen and Put Display to Sleep (P16); a capture of a locked screen showing
the lock and a window share holding its frame, and the portal's Inhibit
(P11); controller input as activity (P18); Settings › Lock Screen,
Notifications' lock row and the Network and Bluetooth panes (P13); session
restore and "Reopen windows when logging back in" (P20); the on-screen
keyboard on the lock (P20); the crash notice for apps ("{app} quit
unexpectedly", P20 with session restore); certificate selection for 802.1X
(P13); passkeys and WebAuthn (`passkeys.md`, not planned); an
`SSH_ASKPASS` program for OpenSSH outside gcr's agent, and GIO's
`MountOperationHandler` for network mounts (with Files, P15).

## Depends on, and feeds

- Needs P3 (surfaces, panic boundaries, springs), P4 (quit, pings, the
  registry), P5 (the key engine, the `locked` flag, inhibitors, the game
  flag), P6 (the island's model, the wallpaper's scaled copy, the system
  menu), P7 (processes and the centre group), P8 (`isle-bus`, logind,
  `isle-media`, `isle-power`, the notification server and holds, the Wi-Fi
  and Bluetooth rows).
- P11 keeps captures of a locked screen to the lock and feeds the portal's
  Inhibit into idle's holds. P13 draws Settings' Lock Screen pane over the
  `lock-screen` section. P14 installs the PAM service and runs the face as
  the greeter. P16 adds the corners. P18 adds pad input as activity.

## Done when

- `cargo test` passes headless: the locked render path, the marker, PAM
  through the test service, the face and its minimal form, sleep
  and the inhibitors, the idle policy and its steps, ScreenSaver, the queue
  and the prompt, each agent against its fake, the session's end and Force
  Quit.
- In the VM, the lock unlocks with the real password and survives `kill -9`
  locked; `pkexec true` asks in Isle's prompt; a locked keyring asks in
  Isle's prompt; idle dims, locks and turns the display off; a video holds
  it.

## Risks

- **A frame already in flight.** A page flip queued before the lock request
  can still scan out once after it. The request queues the locked frame
  at once and the check counts frames after the next vblank; if the
  in-flight frame is seen, the lock waits for it and draws over it before
  reporting the lock as up, and the sleep inhibitor waits the same way.
- **DPMS on NVIDIA.** Displays waking right after blanking (cosmic-comp
  #2428), standby hotplug flaps (niri #4461) and a crash on waking with
  direct scanout on a Blackwell card (cosmic-comp #2559) are all open
  elsewhere; the 5070 Ti is Blackwell. The ten-second flap rule and the
  composited first frame are guesses until the host run; the fallback is
  to keep the CRTC and scan out black.
- **The `sx-aes-1` exchange.** A wrong padding or key derivation fails
  silently as a wrong password. Checked against gnome-keyring 50 in the VM
  as well as the test client.
- **VPN external UI.** The protocol is the plugins' own, documented by GNOME
  Shell's code rather than a specification; a plugin that diverges falls
  back to its own dialog.
- **The test PAM service.** `pam_start_confdir` needs the PAM crate to
  expose it; if none does, `isle-auth` calls `pam-sys` for that one
  function.

## Reviewed

The user went through the choices one by one on 2026-10-08 and took every
recommendation but two: fingerprint is left out of P9 until the host has a
reader (choice 7), and logind's `Unlock` signal unlocks the session
(choice 9), where the plan had ignored it. P9 stays one packet, and the host
run is optional, as for P6 to P8.

## Outcome

Built 2026-10-08 to 10-09 in phases 9.0–9.14, as planned, with these
differences:

- **Idle's state is on `Session1`** (`isle idle`), not a separate `Idle1`
  interface.
- **The queue never puts a request ahead of the prompt on screen**, the
  user's own included; it goes first among those waiting.
- **`isle-auth` binds libpam itself**, a few functions with
  `pam_start_confdir` for the tests, rather than through `pam-client2`.
- **The asker is the nearest process with a window**, walking up from the
  asking pid, found in the VM where `pkexec` in kitty named no app.
- **Force Quit pings every window as it opens** so its marks are current,
  and its double click is the compositor's two presses.
- **The session dialog's chooser keeps its form while apps quit**; Show
  {app} or Force Quit {app} comes under its countdown line.
- **A session restarted while locked starts locked**, from the marker, as
  tty1's autologin would otherwise open the desktop.

The measures are in `docs/research/rust-desktop/p9-lock-and-prompts.md`:
the lock reaches every screen in about 20 ms, the face and a prompt draw
under 2 ms p95 in quiet runs (three earlier runs had single slow frames up
to 3.5 ms that did not come back), and nothing draws at rest. The VM
checked the lock through logind and the real PAM stack (a wrong password;
the right one was not typed, with no test password), `kill -9` while locked,
pkexec, the keyring, an OpenVPN password, the chooser with a real restart,
the idle steps and a logind idle hold. Not checked: a video holding idle,
DPMS on the NVIDIA driver and the login keyring on the host, and Wi-Fi (no
adapter in the guest). B66–B72 record the decisions; B12 is revised.
