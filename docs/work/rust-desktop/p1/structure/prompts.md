# System prompts

Batch 2. Answers to: `shell.md` System prompts 1–9 and its design item
(dialog layout and copy); `platform.md` Polkit agent, Keyring prompter,
Network and Bluetooth agents (each prompt's layout and copy, the refusal
callers show), Login Items and session restore (the log-out
confirmation's copy), Supervisor and crash records (the crash notice, the
safe start); `switcher.md` Shared places (a prompt holding focus, left to
this sheet); `menu-bar.md` system menu (Force Quit…, Restart…, Shut Down…,
Log Out…); B12, B28, B30.

## Purpose

The system asking something only the user can answer (an administrator's
password, a keyring's, a network's, a code for a device) and the user
asking the system to end the session or an app. Every one is drawn by the
shell (B12), in one frame, with the screen dimmed and focus held so a
secret goes only where it is meant (`design-references.md` §12,
elementary). They are not app dialogs (sheets, `window-chrome.md`), not
notifications, and not the place a waiting request is announced (the
island, `island.md`).

## Content

### The frame

The four agent prompts (polkit, keyring, network, Bluetooth) share one
frame, top to bottom:

1. **Who asks**: the asking app's icon, found from the request's process
   through the compositor's app identity; else a symbol for the kind
   (padlock, key, Wi-Fi, Bluetooth).
2. **Title**: one line in Isle's words (Copy).
3. **Message**: the requester's own text where it sends one (polkit's
   action message, the keyring's, a VPN plugin's), as plain text, never
   markup (System prompts 5), wrapped to a few lines.
4. **Fields**, each labelled: the identity where there is a choice, then
   the secret, then its confirmation.
5. **Choices**: the checkboxes the requester offers, and "Show password"
   on a network secret.
6. **Status line**, its slot reserved so nothing moves when it fills: one
   message at a time, in this order: locked out, checking, wrong, Caps Lock,
   fingerprint.
7. **Buttons**: "Details" leading where there are details; Cancel, then
   the action trailing as the default, named for what it does (`copy.md`
   rule 4).
8. **Queue**: "1 more request" under the buttons while others wait.

Strings from requesters are shown as given, since Isle cannot know what
each one means; the frame's own words (title, buttons, status, errors)
follow `copy.md`.

**Where.** Centred across the screen of the asking app's window when it
has one, else the main screen (Shell runtime 14); at a fixed height in
the upper part of that screen, growing downward only (brief P5). Never a
sheet on the asking window: the asker is often a daemon or a command in a
terminal with no window of its own, and a sheet sits in a place the app
draws around, where a look-alike can be faked. A system prompt sits over a
scrim no client can draw above (Shell runtime 13), which is what makes it
trustworthy.

**Dim and hold.** While a prompt is on screen every screen is dimmed by a
scrim over windows and every other surface; the island's privacy form
stays above it (`competition.md`). The prompt holds keyboard focus. A click
on the scrim does nothing; it never dismisses (`design-v1-lessons.md` §8,
Do not repeat). Cmd+Tab sets the prompt aside (For the user 1).

**Armed after a moment.** For a short time after a prompt appears (stage
4), Return and Space do nothing, so a key meant for the window cannot
answer it. Typing into the field works at once.

**One at a time** (System prompts 5). Others wait in arrival order, except
that one the user started (a network or device chosen in Control Centre,
`pkexec` typed in the front terminal) goes to the front, since the user is
waiting for it. A requester that cancels (polkit's cancel, NetworkManager's
CancelGetSecrets, BlueZ's Cancel) removes its prompt, on screen or
waiting, without a word. A restarted agent's request in flight closes its
prompt (`platform.md` Polkit agent 6).

**Waiting.** A prompt not on screen is the island's "prompt waiting"
activity with Show and Cancel (`island.md`). A prompt waits:

- behind a full-screen game, always (brief P7);
- behind a full-screen window, unless that window's app asked (a `pkexec`
  in a full-screen terminal comes forward);
- while the session is locked (System prompts 7); a prompt on screen when
  the lock comes goes back to waiting with its field cleared;
- while set aside by Cmd+Tab;
- behind the prompt in front of it.

It comes forward by itself when what held it ends (the game or full-screen
window leaves the front, the session unlocks, the prompt ahead closes), or
at once from Show.

### Administrator authentication (polkit)

- **Who asks**: the subject's app; else the action's icon from its
  policy; else the padlock.
- **Title**: "{app} needs your password"; with no app known, "An
  administrator's password is needed".
- **Message**: polkit's message for the action, as given.
- **Identity**: with one administrator, the field is labelled "Password
  for {name}"; with several, a pop-up of them first, the user first if
  they are one (System prompts 1).
- **Details**: the action's id and vendor, and for `pkexec` the program and
  its arguments.
- **Buttons**: Cancel, Allow.
- **Fingerprint** (B12; System prompts 1), only where polkit's PAM stack
  has `pam_fprintd`, which is an opt-in set by `isle setup` (Settings,
  `settings.md`), as v1's sudo line was. PAM then asks for the finger
  first: the status line shows the reader's message, the field takes
  typing at once, and a typed password is held until PAM asks for it, as
  the greeter does (v1 D34). Return before then shows "Waiting for the
  fingerprint reader". The agent cannot run the reader beside PAM as the
  lock does, because polkit's answer must come from PAM.
- **Wrong password**: the field shakes, clears and keeps focus; "Wrong
  password. Try again."; the prompt stays until Cancel. A lockout is shown
  as on the lock and shares PAM's count with it (`lock.md`).
- **Checking**: "Checking"; Allow disabled; Cancel still ends the
  conversation.
- **Refused**: Cancel or Escape returns polkit's cancel (`platform.md`
  Polkit agent 5). Isle's own callers (Settings, Updates, Files) say
  "Permission was not given." where the action was asked for (`copy.md`);
  other programs report it their own way.
- **No "remember" choice**: how long an authorisation lasts is the
  policy's, and one privileged job asks once (v1 D77, D79).
- **No timeout**: polkit waits; so does the prompt.

### Keyring (SystemPrompter)

gnome-keyring and oo7 send the prompt's text and kind (`platform.md`
Keyring 1–2); the frame draws what it is given.

- **Unlock**: the key; the requester's title and message; a password
  field; its choice ("Automatically unlock…") where it sends one; its
  continue label as the action, else "Unlock".
- **New keyring** (a new password): Password and Confirm; while they
  differ the status line says "The passwords don't match." and the action
  is disabled. Choosing no password is the keyring's own follow-up
  question, shown as a confirm prompt.
- **Confirm**: message and the two buttons the requester names.
- **Wrong password**: the keyring asks again with its warning, shown in the
  status line.
- **No fingerprint**: the keyring is decrypted with the password itself.
- **When it appears**: when a keyring is asked for and locked; after a
  fingerprint sign-in, the first time the login keyring is needed
  (`lock.md`; `platform.md` Keyring 3).
- **Where**: the caller's window when the request names one, as above.

### Network secrets (NetworkManager)

- **Wi-Fi**: the Wi-Fi symbol; title "“{network}” needs a password";
  message the security ("WPA2 Personal"); a password field with "Show
  password"; buttons Cancel, Join. NetworkManager remembers the network as
  it remembers every one joined; forgetting is in Control Centre's Wi-Fi
  detail and Settings.
- **A wrong password**: NetworkManager asks again with its "new secrets"
  flag; the status line says "Couldn't connect to “{network}”. Check the
  password and try again." (`copy.md` rule 7) with the field selected.
- **Enterprise (802.1X)**: only the fields NetworkManager's request asks
  for (identity, password, a key's password); title "Sign in to
  “{network}”". Certificates are the packet's (`platform.md` Open).
- **VPN** (System prompts 3): NetworkManager's VPN plugins describe their
  fields through their auth dialogs' external-UI mode, and Isle draws them
  in this frame, as GNOME Shell does; title "“{vpn}” needs a password",
  action Connect. A plugin without that mode shows its own GTK window,
  outside Isle. VPN providers (B14) ask through their own contract.
- **From Control Centre.** Choosing a secured network in the Wi-Fi detail
  raises this prompt at the front of the queue, over the pinned panel,
  which stays and shows connecting, connected or the failure. One dialog
  for every secret, as macOS, in place of v1's password page in the panel.
- **After Join** the prompt closes at once; connecting shows in Control
  Centre and the island's glyph.
- **Timeout**: NetworkManager cancels after its own; the prompt closes,
  and the failure shows in the Wi-Fi detail, not as a new message.

### Bluetooth pairing (BlueZ Agent1, KeyboardDisplay)

| BlueZ asks | Title and body | Buttons |
|---|---|---|
| RequestConfirmation | "Pair with {device}?"; "Check that {device} shows this code."; the code, large | Cancel, Pair |
| RequestPasskey, RequestPinCode | "Pair with {device}"; a field for the code (six digits for a passkey; up to sixteen characters for a PIN) | Cancel, Pair |
| DisplayPasskey, DisplayPinCode | "Type this code on {device}"; the code, large, each digit marked as it is typed on the device | Cancel |
| RequestAuthorization | "{device} wants to pair" | Cancel, Pair |
| AuthorizeService | "Allow {device} to connect?"; the service's name | Don't Allow, Allow |

- **Who asks**: the device's kind icon and name from BlueZ; an unnamed
  device by its address.
- **Started from Control Centre's Bluetooth detail**: front of the queue,
  over the pinned panel, which shows the result.
- **Arrives with Settings closed** (B28; `platform.md` Network and
  Bluetooth agents 2).
- **Timeout**: BlueZ's Cancel closes the prompt; a failure shows in the
  Bluetooth detail ("Couldn't pair with {device}. Make sure it's on and
  nearby, then try again."). Pairing is bound by
  BlueZ's timeout, so one that waits behind a game or the lock usually
  lapses.

### The session dialog

Two forms, in the centre of the main screen, in the one-at-a-time group
with Spotlight, Mission Control and the capture tool (`competition.md`;
Shell runtime 10). A scrim dims every screen, since the action is the
whole session's; Escape cancels.

**The chooser**, from the power key, Ctrl and the power key (macOS), or a
desktop chord for keyboards without one (proposed for P5's table:
Ctrl+Cmd+Delete, which on a PC keyboard with the swap is the keys
labelled Ctrl+Alt+Delete).

- Five actions in a row: Lock Screen, Log Out, Sleep, Restart, Shut Down
  (System prompts 8), with Shut Down selected.
- The apps still open, as a row of icons with their names in the
  accessibility tree (System prompts 8).
- "Reopen windows when logging back in", checked as last left.
- Shut Down counts down with "If you do nothing, the computer will shut
  down in {n} seconds." (macOS; 60 s as the start). Any key or click stops
  the countdown.
- Choosing an action acts: the chooser is the confirmation.

**The confirmation**, from the system menu's Restart…, Shut Down… and Log
Out {user}… (Shift+Cmd+Q), and from "Restart to finish updating…"
(`menu-bar.md`):

- One question: "Are you sure you want to restart?", "…shut down?",
  "Are you sure you want to log out?" (`copy.md`); for an owed restart,
  "Restart to finish updating?" with the reasons (`island.md` Copy).
- The open apps' icons; "Reopen windows when logging back in"; the
  countdown line; Cancel and the action as the default.
- Option+Shift+Cmd+Q logs out with no confirmation, as macOS.

**Quitting.** After either form, apps are asked to quit as by Cmd+Q
(`compositor.md` Session 8), and the dialog shows the icons of those not
yet gone, each leaving as its app quits. An app that asks to save stops
the action: the dialog says "Couldn't restart because {app} didn't quit."
with "Show {app}", which closes the dialog and brings the app forward with
its question (macOS's interrupted restart). An app that does not answer
within the wait is marked "Not responding" and offered "Force Quit {app}",
after which the action goes on.

**Sleep and Lock Screen** act at once. The session dialog never opens over
the lock; the power key there goes to the lock's own row (`lock.md`).
Over a game the power key still opens the chooser, without transitions; the
chord goes to the game, since it is not in B30's escape set.

### Force Quit

- **Opens** from Cmd+Option+Escape and Force Quit… in the system menu. Over
  a full-screen game the chord is the game's (`gaming.md` Keys 2): Cmd+Tab
  out first.
- **A panel** in the centre of the main screen, in the one-at-a-time group,
  with no scrim, so the user can see what is stuck behind it. It holds focus
  while open; Escape or Cmd+W closes it.
- **The list**: every running app by name: its icon (a game's art), its
  name, "Not responding" where it has missed its ping (`window-chrome.md`),
  apps running in the background after quit included (`dock.md`), Isle's
  own apps included; never the shell, which is the compositor. A not
  responding app is selected on opening, else the front app. Several can
  be selected (Cmd-click, Shift-click), as macOS.
- **The list is live**: an app that quits leaves it, and the selection
  stays on its app.
- **Force Quit** (the default button) asks once, as a sheet on the panel:
  "Do you want to force {app} to quit? You will lose any unsaved changes."
  with Cancel and Force Quit. Then every process of the app is ended. One
  that cannot be ended is marked "Couldn't force quit".
- The not-responding alert on close (`window-chrome.md` For the user 5)
  force quits directly and does not open this panel.

### Crash notice and safe start

Neither asks anything, so neither is a prompt; their copy is here, their
form the notification's and the island's.

- **The crash notice** (`platform.md` Supervisor 4): after the compositor
  restarts from a crash, an Isle notification, drawn as `notifications.md`
  draws any (a banner once, then kept in Notification Centre until
  dismissed): "Isle quit unexpectedly." with "Your apps were reopened."
  when session restore ran, and "Report…", which opens the crash record. An
  app that crashes gets the same form with `copy.md`'s "{app} quit
  unexpectedly." and Reopen and Report…. A notification rather than macOS's
  dialog, because the user is already looking at a desktop rebuilt around
  them and a modal on top of it says nothing more (brief P1). The lock's
  minimal form (`lock.md`) leaves the same notice after unlock.
- **The safe start** (Supervisor 5): the island's configuration-error
  glyph shows while settings are ignored, and its notice row says
  `copy.md`'s "Isle started without your settings because they stopped it
  starting." with "Use my settings" (applies `settings.kdl` and
  `tokens.kdl` live; a crash then makes the next start safe again) and
  "Report…". A banner says the same once at start. This widens the glyph's
  meaning in `island.md` from "holds a rejected edit" to "is not in force".

## States

| State | What shows |
|---|---|
| Rest | Nothing. |
| One prompt | The frame on its screen, the scrim on every screen, focus in its first field (or on its default button when it has none) from the first frame; until armed, Return and Space on the button do nothing, and arming moves no focus. |
| Armed | After the moment, Return and Space answer. |
| Typing | Dots; Caps Lock in the status line while on. |
| Checking | "Checking"; the action disabled; Cancel works. |
| Wrong | Shake, field cleared and focused, the message. |
| Locked out | The field disabled and the time left, as on the lock. |
| Fingerprint | The reader's message; a typed password held until PAM asks. |
| Queued | "{n} more requests" under the buttons; the island counts nothing more, since the prompt is on screen. |
| Requester cancelled | The prompt closes, or leaves the queue, silently. |
| Agent restarted | The prompt in flight closes; nothing is said. |
| Waiting | Not on screen; the island's prompt activity, Show and Cancel. |
| Set aside | As waiting, after Cmd+Tab (For the user 1). |
| Loading | An app icon not yet resolved shows the kind's symbol; the prompt never waits for it. |
| Many screens | The prompt on one screen (the asker's, else main), the scrim on all. |
| Windowed game in front | As normal, without transitions or blur (`competition.md` For the user 1). |
| Full-screen game | Waits. |
| Full-screen window | Waits, unless that window's app asked. |
| Locked | Waits; comes forward after unlock, after the armed moment so the Return that unlocked cannot answer it. |
| Focus on | Unchanged: prompts are not notifications. |
| Session dialog: countdown | The line counting; stops on any input. |
| Session dialog: quitting | The icons of apps not yet gone. |
| Session dialog: interrupted | "Couldn't restart because {app} didn't quit.", Show {app}; or Not responding and Force Quit {app}. |
| Force Quit: open | The list, a not responding app selected, else the front app. |
| Force Quit: confirming | The sheet on the panel. |
| Reduce Motion | No shake; the frame and scrim fade. |
| Reduce Transparency | The frame solid. |
| Increase Contrast | An edge on the frame and the fields; the error and the selection in their stronger forms. |
| Largest text size | The frame grows downward; the message wraps to more lines before Details takes the rest. |

## Opening and closing

- A prompt opens where it belongs, fading in with its scrim, scaling from
  `motion-panel-scale` about its anchor in place, with no slide (brief P5). It closes faster, and focus returns to what had it (Shell
  runtime 11), Spotlight or a pinned Control Centre included, since a
  prompt closes nothing (Shell runtime 10).
- The next prompt in the queue opens in place of the last after a short
  pause, so two prompts never read as one.
- A prompt coming forward from waiting opens as a new one, on its screen.
- The session dialog and Force Quit fade in and out in place; opening
  either closes Spotlight, Mission Control or the capture tool.
- In Game Mode and under Reduce Motion everything here is a fade or a cut.
  Values are stage 4's.

## Keyboard

| Action | Key |
|---|---|
| Answer (the default button) | Return, once armed |
| Cancel | Escape |
| Move between fields, choices and buttons | Tab, Shift+Tab |
| Toggle a choice; press a button | Space |
| Open Details | Return or Space on it |
| Choose an administrator | Up and Down in the pop-up |
| Set the prompt aside | Cmd+Tab (For the user 1) |
| Bring a waiting prompt forward | Show in the island's Now row (Ctrl+F8, then Tab to Show and Return) |
| Session chooser | Left and Right move; L Lock Screen, O Log Out, S Sleep, R Restart; Return acts on the selection (Shut Down at first); Escape or the Cancel button closes it |
| Confirmation | Return the action, Escape Cancel |
| Open Force Quit | Cmd+Option+Escape |
| Force Quit list | Up and Down; Cmd or Shift with them to add to the selection; Return Force Quit; Escape or Cmd+W close |

The chooser's letters follow macOS's dialog (S Sleep, R Restart, Return
Shut Down) and add v1's L and O for the two actions macOS's lacks, since
`shell.md`'s "first letter" cannot tell Sleep from Shut Down.

## Pointer

- **Click** on a field focuses it; on a button, acts (once armed); on the
  scrim, nothing.
- **Hover** opens nothing, so no hover guards apply.
- **Drag**: the frame is not movable.
- **Force Quit**: click selects, Cmd-click and Shift-click extend, double
  click asks to force quit.
- **Session chooser**: a click on an action acts.

## Shared places

- **Over everything but the lock** (`competition.md`): adds that a prompt
  dims every screen with a scrim drawn above windows and every other
  surface; only the island's privacy form, the lock and the idle dim are
  above it. A click on the scrim does nothing.
- **Keyboard focus**: second in `competition.md`'s order, except that
  Cmd+Tab sets the prompt aside into the island and switches (gate 2). The menu bar and every other
  chord still cannot take focus from it. This settles `switcher.md`'s
  open row.
- **A prompt against a full-screen window or game**: it waits as the
  island's activity (`island.md`), except over a full-screen window whose
  own app asked.
- **A prompt against Control Centre**: a prompt the panel started (a
  network, a device) opens over the pinned panel; the panel stays under the
  scrim and shows the outcome. Over Control Centre or Spotlight the frame
  is solid, so glass never sits on glass.
- **A prompt against a sheet or an app's dialog**: the prompt wins focus;
  the sheet keeps its window's focus for after (`window-chrome.md`).
- **Prompts against the lock**: they wait; the lock wins (`lock.md`).
- **The session dialog and Force Quit** are in the centre group, one at a
  time with Spotlight, Mission Control and the capture tool; Force Quit is
  new to that group (adds to `competition.md` and `shell.md` Shell runtime
  10). Neither opens over the lock.
- **The power key** is the session dialog's: the compositor holds logind's
  power-key inhibitor so a press opens the chooser rather than powering off.
  Over the lock, the lock's row takes it.
- **The Return key**: the armed moment keeps a key meant for
  a window, or for the lock, from answering a prompt.
- **The crash notice** is a notification (`notifications.md`); **the safe
  start** is the island's configuration-error glyph and notice row (adds
  to `island.md`).

## Answers

- **System prompts, dialog layout and copy** (`shell.md`): one frame for
  every agent prompt (who asks, title, message, fields, choices, status
  line, buttons), centred on the asker's screen, the screen dimmed and
  focus held; per-prompt layouts and copy above.
- **Where a prompt appears**: **replaced** macOS's sheet on the asking
  window with a system-modal frame on the asker's screen, else the main
  screen, because the asker often has no window and a sheet can be
  imitated inside one.
- **Dim and hold** (`design-references.md` §12): **adopted**, with Cmd+Tab
  as the one way past it (For the user 1).
- **A prompt over a full-screen game** (`island.md`, System prompts 6):
  **adopted**, waiting as live activity; **changed** for a full-screen
  window whose own app asked, which comes forward.
- **While locked** (System prompts 7): **adopted**, and a prompt on screen
  at lock time goes back to waiting with its field cleared.
- **Polkit, keyring, network and Bluetooth: each prompt's layout and copy;
  the refusal callers show** (`platform.md`; proposal "Permission was not
  given"): layouts above; the refusal **adopted** as `copy.md` has it, said
  by Isle's callers in place.
- **Fingerprint for polkit** (B12): where polkit's PAM stack has it, the
  password is held until PAM asks (v1 D34's route), opt-in; the keyring
  takes no fingerprint.
- **Wi-Fi passwords in Control Centre** (v1's drill-down page): **replaced**
  by this prompt over the pinned panel, one dialog for every secret.
- **The log-out confirmation's copy** (`platform.md` Login Items; proposal
  "Reopen windows when logging back in"): **adopted** with `copy.md`'s
  question and "Log Out".
- **The session dialog** (System prompts 8): two forms; a countdown on Shut
  Down in the chooser and on every confirmation (60 s, macOS); open apps
  shown as icons; macOS's letters plus L and O.
- **Force Quit** (System prompts 9): macOS's panel, with background apps
  and multiple selection, a confirmation, no scrim.
- **The crash notice's form and copy; how the safe start is announced**
  (`platform.md` Supervisor): a notification for the crash; the island's
  configuration-error glyph and notice row, plus one banner, for the safe
  start.

## Copy

| Where | String |
|---|---|
| Polkit title | "{app} needs your password"; "An administrator's password is needed" |
| Polkit field | "Password for {name}" |
| Polkit buttons | "Cancel", "Allow"; "Details" |
| Refusal (callers) | "Permission was not given." |
| Keyring | The requester's own; "Unlock" when it names no action |
| Keyring mismatch | "The passwords don't match." |
| Wi-Fi | "“{network}” needs a password"; "Show password"; "Cancel", "Join" |
| Wi-Fi wrong | "Couldn't connect to “{network}”. Check the password and try again." |
| 802.1X | "Sign in to “{network}”"; "User name", "Password", "Private key password"; "Cancel", "Join" |
| VPN | "“{vpn}” needs a password"; "Cancel", "Connect" |
| Bluetooth | "Pair with {device}?", "Check that {device} shows this code." ("Cancel", "Pair"); "Pair with {device}" ("Cancel", "Pair"); "Type this code on {device}" ("Cancel"); "{device} wants to pair" ("Cancel", "Pair"); "Allow {device} to connect?" ("Don't Allow", "Allow"); the displayed code's accessible name "{n} of {total} digits typed" |
| Bluetooth failure (in Control Centre) | "Couldn't pair with {device}. Make sure it's on and nearby, then try again." |
| Status | "Wrong password. Try again.", "Checking", "Caps Lock is on", "Waiting for the fingerprint reader", "Too many attempts. Try again in {n} min." |
| Queue | "1 more request", "{n} more requests" |
| Chooser | "Lock Screen", "Log Out", "Sleep", "Restart", "Shut Down", "Cancel"; "Reopen windows when logging back in"; "If you do nothing, the computer will shut down in {n} seconds." |
| Confirmation | "Are you sure you want to restart?", "Are you sure you want to shut down?", "Are you sure you want to log out?", "Restart to finish updating?"; "Cancel", "Restart", "Shut Down", "Log Out"; "If you do nothing, the computer will restart in {n} seconds." (and "shut down", "log out") |
| Interrupted | "Couldn't restart because {app} didn't quit." ("shut down", "log out" likewise); "Show {app}"; "Not responding"; "Force Quit {app}" |
| Force Quit | "Force Quit" (title and button); "If an app doesn't respond for a while, select it and choose Force Quit."; "Not responding"; "Do you want to force {app} to quit? You will lose any unsaved changes."; "Couldn't force quit" |
| Crash notice | "Isle quit unexpectedly.", "Your apps were reopened.", "Report…"; "{app} quit unexpectedly.", "Reopen", "Report…" |
| Safe start | "Isle started without your settings because they stopped it starting."; "Use my settings", "Report…" |
| Island waiting row | As `island.md`: "{app} needs your password", "“{network}” needs a password", "{device} wants to pair"; "Show", "Cancel" |

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **Cmd+Tab while a prompt is up.** Hold focus strictly, as elementary
   does and `competition.md` has it now; or let Cmd+Tab set the prompt
   aside into the island as a waiting activity, lifting the scrim, with
   Show bringing it back. *Recommended: set aside.* A Wi-Fi or keyring
   password often has to be looked up in another app first, and a strict
   hold would mean cancelling to fetch it. The danger elementary guards
   against, typing a secret into the wrong window by accident, needs a
   deliberate Cmd+Tab to happen here, and the armed moment covers a stray
   Return.
