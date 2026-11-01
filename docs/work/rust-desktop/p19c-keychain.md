# P19c · Keychain

Planned 2026-10-29. Reviewed by the user on 2026-11-01: every
recommendation taken. The last of P19's three packets (the user's choice,
2026-10-29), after P19a Monitor and P19b Cleaner.

`apps.md` Keychain 1–12 says what Keychain gives: the Passwords app with
Keychain Access's reach. That is saved logins, Wi-Fi passwords, app
secrets and SSH keys under one search, with nothing fetched until asked,
a generator, and copies that clipboard history skips and that are
withdrawn later. D42 and D20 are v1's record of how. P1 answered every
presentation question in `p1/structure/keychain.md` (binding) and
`p1/surfaces/keychain.md` with its Wave C board, and gate 2 settled what
a reveal asks for. The tokens are in `tokens.kdl`. What is open is how
it is built.

Today nothing lists a secret. Some of what Keychain sits on exists:
- the compositor serves the keyring prompter (`isle-prompter`, B12) and
  the polkit agent (`isle-polkit`);
- its NetworkManager agent keeps agent-owned Wi-Fi secrets in the
  keyring under `org.freedesktop.NetworkManager.Connection` (P8);
- clipboard history already skips an offer that carries
  `x-kde-passwordManagerHint` (`clipboard.rs`, P10).

`isle-app`'s data device sets selections of any type (P15). The island
has its Copied line, but only the compositor posts it.
`docs/research/rust-desktop/apps-2026.md` §6–7 has the evidence:
- gnome-keyring 51.1 owns the bus name, and oo7 0.6.0 is a plain Secret
  Service client;
- the host's keyring holds twelve items and every one is an app secret:
  no website logins at all;
- the host's Wi-Fi profiles are system-owned, so NetworkManager's
  `GetSecrets` passes silently for a wheel user;
- gcr's agent is a proxy that lists keys from `~/.ssh/*.pub` whether or
  not they are loaded;
- polkit has `auth_self`.

What P19c must get right is restraint. Keychain is the one Isle app that
handles other people's secrets for the user. A secret is fetched only
for the click that asks, and none is ever logged, kept or drawn on
hover. Every copy is marked and withdrawn. A walked-up session asks for
the password before anything shows.

## What it answers to

- Decisions: B2, B9, B12, B17, B19, B20, B26, B27, B94 (`isle-app`), B104
  (`isle-fs`'s Trash), B119 (work in flight); D20, D42.
- Intent: `apps.md` Keychain 1–12, its v1 lessons and open points,
  Conventions 1–25, bars 1–8 and 11, the Parity map's Keychain rows;
  `platform.md` Clipboard 2 and bar 11 (no secret in logs, state or
  caches), the keyring and secret-agent sections.
- Design, binding: `p1/structure/keychain.md` and its For the user 1,
  `p1/surfaces/keychain.md` and its board, `structure/conventions.md` and
  `surfaces/conventions.md` (window anatomy, sidebar rows, two-line rows,
  popovers, the settings window), `surfaces/window-chrome.md` (sheets and
  alerts), `surfaces/prompts.md` (the polkit frame, the keyring form),
  `surfaces/island.md` (the Copied line).
- Evidence: `apps-2026.md` §0 and §6–7, `isle-v1-inventory.md` §5 Keychain,
  `passkeys.md`, `lock-and-prompts-2026.md` §2–4.

## Choices this plan makes

### Scope

1. **One packet. Two things are cuttable** (19c.10): "Change password…"
   for a keyring, and dragging a login out.
2. **Providers, Spotlight's `*` scope and verification codes stay in
   P20,** with the vault (the roadmap). The Sources group holds the Secret
   Service's collections only, and no row shows a verification code,
   since only a provider gives one. The reading and the gate are a crate
   of their own (choice 5), so P20's vault and the `*` scope use the same
   fetch and the same gate.
3. **The host has no logins to show, and the plan builds Logins anyway.**
   The keyring holds twelve app secrets. Chromium-family browsers keep
   their site logins inside the browser, under the storage key the
   keyring holds, and Firefox never uses the Secret Service. So on the
   host, Logins is nearly empty until P20 brings Proton Pass; Apps, Wi-Fi
   and SSH keys carry the window.
   - **Logins is still built as P1 draws it,** against the fake (choice
     25) and against gnome-keyring and oo7-daemon in the VM. New login
     saves into the login keyring, so logins can be kept there, and git
     and other libsecret tools can find them (choice 9).
   - **Importing a browser's logins is not in,** nor reading Firefox's
     `logins.json`: each is a browser's store, not the user's keyring.
4. **No gate.** P1 drew every view, state and motion. Three things P1 did
   not write are added to `p1/structure/keychain.md` at 19c.0:
   - **A delete line for a storage key.** Deleting a browser's or
     Electron app's storage key (the `*_libsecret_os_crypt_password_v2`
     items) costs more than a sign-in: the saved passwords and cookies it
     encrypted can no longer be read. The alert's informative text says
     "{app} will lose the passwords and sign-ins it keeps." in place of
     P1's line.
   - **The key generation bar is indeterminate.** Generating a key gives
     no progress to show, so P1's determinate bar becomes
     `conventions.md`'s indeterminate one.
   - **The gate's message,** the polkit dialog's line under P1's title:
     "Keychain wants to show or copy a saved password." (choice 16).

### Reading secrets

5. **A new crate, `isle-secrets`,** for Keychain's sources: the Secret
   Service, NetworkManager's saved networks, `~/.ssh` and the agent. It
   also holds the kinds, the fetch, the generator and the gate (choice
   16). The app is `isle-keychain`. P20's vault provider and Spotlight's
   `*` scope (in the compositor) depend on `isle-secrets`, not on the app.
6. **The Secret Service through `oo7::dbus`** (0.6, on the workspace's
   zbus 5), not `oo7::Keyring`, which picks the file backend when it
   thinks it is sandboxed.
   - **An encrypted session** (`dh-ietf1024-sha256-aes128-cbc-pkcs7`,
     oo7's `native_crypto`), so a secret never crosses the session bus in
     the clear. A service that offers only `plain` is used with it.
   - **Lists carry no secrets.** A list is built from `SearchItems` over
     every collection, plus each item's label, attributes, created and
     modified (apps.md Keychain 3). `GetSecret` is called in one place
     only, `isle_secrets::fetch`, for one item, and only from Show, Copy
     or Edit.
   - **What it fetches** is held in `Zeroizing` buffers and dropped when
     the click's work ends: after the copy, or when the shown value
     hides.
   - **Changes** come from `ItemCreated`, `ItemChanged`, `ItemDeleted`
     and `CollectionChanged`, so a list follows edits made elsewhere.
     The window does not poll.
7. **Kinds by schema and attributes,** from the research's table, as one
   function tested over the host's twelve items' shapes:
   - **Logins:** `org.gnome.keyring.NetworkPassword`,
     `org.epiphany.FormPassword`, `org.remmina.Password`; also an
     `org.freedesktop.Secret.Generic` item with `username` and a
     `service` that reads as a URL or host name (Python keyring and
     go-keyring logins: "service — username"). Anything else with a user
     and a server or URL attribute counts too, as P1's rule says.
   - **Wi-Fi:** `org.freedesktop.NetworkManager.Connection` items whose
     connection is Wi-Fi are not listed on their own. They are the
     network's password, shown on the network's item (choice 11).
   - **Apps:** every other item, a VPN's or wired 802.1X connection's NM
     secrets among them. Each is named by the app that stored it:
     - the `application` attribute;
     - a portal secret's `app_id`;
     - the schema's known owner (Chromium-family storage keys,
       gnome-online-accounts, gcr's SSH passphrases);
     - otherwise the item's label.
     The app's icon and name come from `isle-apps` when it is installed.
   - **Rules are code, not a file.** Unlike Cleaner's rules, the set of
     schemas is short and changes with the Secret Service's clients, not
     with the user.
8. **Locks and the keyring's own actions.**
   - **Unlock** calls the service's `Unlock` and completes its prompt,
     which brings up the compositor's keyring form (B12). Keychain draws
     no password field.
   - **Lock** calls the service's `Lock`.
   - **Change password…** calls gnome-keyring's
     `org.gnome.keyring.InternalUnsupportedGuiltRiddenInterface.ChangeWithPrompt`,
     the only change-password route either daemon offers, through the
     same prompter. It is disabled where the service lacks it (cuttable,
     choice 1).
   - **A locked collection** lists whatever titles the service still
     gives, under P1's locked row.
9. **New and edited logins.**
   - **New login** writes `org.gnome.keyring.NetworkPassword` with
     `user`, `server`, `protocol` and `port` from the website field, so
     libsecret's network lookups (git's credential helper among them)
     find it. Notes go in an `isle:notes` attribute, and the label is
     the title. "Save in" offers the unlocked collections, Login first.
   - **Editing a login** may change its title, user, website, notes and
     password.
   - **Editing an Apps item** changes its title and secret only. Its
     attributes are what the app looks it up by, so they are never
     rewritten.
   - **A secret that is not text** (a portal app's binary key) shows in
     base64 and copies as text, so the ten dots stay true to P1.
10. **Delete** calls the item's `Delete` after P1's alert, with the
    storage-key line of choice 4 for those items. Nothing is soft-deleted
    or kept: the Secret Service has no Trash, as P1's "You can't undo
    this." says.

### Wi-Fi

11. **Saved networks from NetworkManager's settings,** through
    `isle-network`: each Wi-Fi connection's name (`id`), SSID and
    security (`key-mgmt`). The password is read when Show or Copy asks,
    by where the profile keeps it:
    - **system-owned** (`psk-flags` 0, the host's profiles):
      `Settings.Connection.GetSecrets("802-11-wireless-security")`. NM's
      own polkit check applies, silent for wheel on the host and the
      desktop's dialog elsewhere. Refused, the field stays dots (P1);
    - **agent-owned** (`psk-flags` 1): the keyring item Isle's agent saved
      for that connection's UUID, through the same fetch as any item;
    - **not saved** (`psk-flags` 2): the field says nothing is saved, and
      Show and Copy are disabled;
    - **WPA-Enterprise:** the `802-1x` setting's identity as the user
      name and its password the same way.
    The research's open point, what Isle's agent does when NetworkManager
    asks it on Keychain's behalf, does not arise: Keychain reads an
    agent-owned secret from the keyring and never asks NetworkManager
    for it.
12. **Read-only, as P1 draws it.** "Wi-Fi Settings…" opens
    `isle-settings://wifi`, where a network is forgotten or edited.

### SSH keys

13. **Keys from `~/.ssh`, read by `ssh-key`.**
    - **What is a key:** every file that parses as an OpenSSH private
      key, paired with its `.pub`, and every `.pub` whose private half is
      in another format (shown with what the `.pub` gives).
    - **Read without a passphrase:** type, fingerprint (SHA256), comment
      and whether it is encrypted.
    - **When it is read again:** when the window gains focus and after
      Keychain's own changes. No watch is kept.
14. **The agent's truth is behind gcr's proxy.**
    - **Which socket:** when `SSH_AUTH_SOCK` is gcr's
      (`$XDG_RUNTIME_DIR/gcr/ssh`) and its inner agent's socket
      (`gcr/.ssh`) exists, "In agent" is read from the inner agent. Read
      through the proxy, every `.pub` would look loaded. Any other
      `SSH_AUTH_SOCK` is read directly. Both are read with
      `ssh-agent-lib` 0.6.
    - **Add to agent** decrypts the key in process with `ssh-key`'s
      `encryption` feature and the passphrase from P1's sheet, then sends
      `ADD_IDENTITY` to the agent. No `ssh-add` runs, so the passphrase is
      never in an argument list, a pipe to another program, or a file
      (apps.md Keychain 8). A wrong passphrase leaves the sheet open with
      its field cleared.
    - **Remove from agent** sends `REMOVE_IDENTITY`.
15. **New SSH key and delete.**
    - **Generating:** `ssh-key` generates Ed25519 (the default) or RSA
      3072, encrypts it with the passphrase (bcrypt-pbkdf, as
      `ssh-keygen`), and writes the pair. The private key is 0600, the
      folder 0700 if it is made, and an existing name is refused. RSA is
      the slow case: the sheet's bar and the switcher card show it, and
      the process stays until it is done (B119).
    - **Delete** moves both files to the Trash through `isle_fs`, with
      Undo, and removes the key from the agent if it is there. Undo
      brings back the files, not the agent's copy.

### The gate

16. **The reveal gate is Isle's own polkit action, at `auth_self`,** and
    Keychain holds the grant itself.
    - **The action:** `io.github.alexparlett.Isle.Keychain.reveal` in
      `io.github.alexparlett.Isle.Keychain.policy`, with `allow_any` no,
      `allow_inactive` no and `allow_active` `auth_self`. Its icon is
      Keychain's, and its message is "Keychain wants to show or copy a
      saved password."
    - **The check:** Keychain calls `CheckAuthorization` with its own
      process as the subject (`unix-process` with its pidfd) and
      `AllowUserInteraction`. The desktop's polkit dialog asks, drawn as
      P1's "Keychain needs your password", and fingerprint comes through
      polkit's PAM stack when P14b adds it.
    - **The research recommended `auth_self_keep`.** polkit keeps that
      grant for five minutes from when it was given, not from the last
      use, so it would ask again in the middle of use. Keychain instead
      keeps its own grant from one `auth_self` check.
    - **When the grant ends:**
      - after five minutes with no input in Keychain's windows;
      - when the session locks (Shell1's `LockChanged`);
      - when the process ends.
    - **What it gates:** Show, Copy and Edit of a secret field, Wi-Fi's
      included, which then also passes NetworkManager's own check. A user
      name, a public key and every list pass freely.
    - **When it asks:** at the first such click after Keychain opens, not
      at launch. P1's "once when Keychain opens" is read as "once per
      opening", so opening Keychain to read a fingerprint asks nothing.
    - **What it is:** a gate against a walked-up session, not a security
      boundary. Any process of the user's can read an unlocked keyring,
      as on a Mac. The decision records it.

### Reveal and copy

17. **Show** fetches the one field and draws it in the mono face. Only one
    field shows at a time. A shown value hides:
    - after "Hide shown passwords after";
    - when the window is not shown (P19a's `use_shown()`: minimised,
      fully covered or locked);
    - at the session's lock, before the lock draws;
    - when the window closes.
    Hiding drops the value. Freya's text layout holds its own copy, which
    Keychain cannot zero (Risks).
18. **Copy marks, withdraws and stays.**
    - **The offer:** `text/plain;charset=utf-8`, `text/plain` and
      `x-kde-passwordManagerHint` = `secret`, so Isle's history and
      Klipper-style managers skip it (platform.md Clipboard 2).
    - **`isle-app` gains a secret selection:** its contents are zeroed
      when the source is cancelled or withdrawn, and it reports when
      another selection has replaced it.
    - **Withdrawn:** after "Clear copied passwords after", if no other
      copy has replaced it, Keychain destroys its source, and the
      compositor clears the selection. A test checks that the compositor
      does this.
    - **The process stays** while a copy is waiting to be withdrawn, after
      the window closes and after ⌘Q, so a password copied and then
      pasted after closing Keychain still pastes. This counts as work in
      flight under bar 3, and it ends at the withdrawal, at most five
      minutes later.
    - **The island:** Shell1 gains `Copied(kind)` ("password" or ""),
      which posts the island's line, "Password copied" with `key` or
      "Copied" with `check` (P1 `island.md`). The compositor's own copies
      already post it. Keychain calls it after each copy.
19. **Nothing secret is logged, kept or cached** (platform.md 5, bar 11).
    `isle-secrets` types carry no `Debug` that prints a value, and no
    state, settings or thumbnail holds one. A canary test checks this
    (choice 26).

### The generator

20. **`getrandom`, by rejection sampling**, so every character of a
    chosen set is equally likely, with at least one from each set that is
    on.
    - **Length:** 8 to 64, default 20.
    - **The sets:** upper case, lower case, digits, and symbols from
      ASCII punctuation.
    - **Remembered:** the last length and sets, in Keychain's
      `isle-state` domain. The password itself is never kept.

### The window

21. **`isle-keychain` on `isle_app::run`,** one window as P1 draws it:
    - the sidebar of kinds and sources;
    - the list, its pane width dragged between the tokens' limits;
    - the detail, with the toolbar's title, count, New, More and search;
    - the command table holds P1's Keyboard;
    - the menus are P1's File and Item.
    Rows are `conventions.md`'s two-line rows in a virtual list, not
    P19a's table, since Keychain's list has no columns.
22. **Search** matches title, user name, website or server, the storing
    app and the source's name, never a secret, over every source the
    selection spans. It matches word starts first, as the window's other
    lists do.
23. **Absent and failing sources** follow P1's States:
    - no Secret Service is apps.md bar 4's row, with Wi-Fi and SSH keys
      working on;
    - a frozen service is given a timeout, so the window answers within
      a frame and its rows say so;
    - a keyring prompt left open does not block the window.
24. **What Keychain remembers:**
    - in Keychain's `isle-state` domain: the last sidebar row, the list
      pane's width and the generator's settings;
    - in a new `keychain` section of `settings.kdl`, written through the
      compositor (B27): P1's two settings, "Clear copied passwords after"
      and "Hide shown passwords after".
    - Opening Keychain: the desktop entry, service file and package;
      Spotlight finds it by its entry.

### Proof

25. **A Secret Service fake in `isle-fakes`:**
    - collections and items, with labels, attributes, created and
      modified;
    - lock and unlock through a prompt object the test completes;
    - the signals;
    - both session algorithms, the encrypted one through oo7's own key
      exchange;
    - a count of every `GetSecret`.
    `isle-fakes`' polkit authority gains `CheckAuthorization`, which asks
    the registered agent as polkitd does. The NM fake already serves
    `GetSecrets`.
26. **Tests of restraint:**
    - **Nothing fetched:** with 2,000 items in the fake, Keychain lists,
      searches, selects and scrolls every kind with zero `GetSecret`
      calls. Show then makes exactly one.
    - **The canary:** a session in the fake with a canary secret is
      shown, copied, edited, generated beside and withdrawn. The canary
      must then be absent from the captured logs, Keychain's state, the
      settings, clipboard history's files and the compositor's state.
    - **The clipboard:** a copy is not in history; it is withdrawn after
      the period; a later copy by another client is left alone; and the
      process ends at the withdrawal.
    - **The gate:** a refused check shows nothing; an allowed one covers
      the next clicks; and the grant ends after idle and at lock.
27. **The agent and keys against the real tools.** Tests run a real
    `ssh-agent -a <tmp>` and a gcr-shaped proxy that lists `.pub` files.
    - **They check:** add with a passphrase, list, remove, a wrong
      passphrase, and "In agent" read past the proxy.
    - **Generated keys** must load in `ssh-keygen -y` and `ssh-add`, and
      their fingerprints must match `ssh-keygen -l`.
    - **In the VM,** gcr-ssh-agent itself is the proxy.
28. **Both daemons.** In the VM the window is checked against
    gnome-keyring 51 and against oo7-daemon (apps.md Keychain 1):
    listing, unlock through the compositor's prompter, create, edit and
    delete.
29. **Bars** beside `apps.md`'s:
    - **Opening:** the first complete frame within bar 1's 300 ms,
      including the list of 2,000 fake items.
    - **A reveal:** Show draws the value within 100 ms of the click once
      the grant is held.

## The workspace

```
crates/
  isle-secrets/           Secret Service, kinds, Wi-Fi, SSH and the
                          agent, generator, the gate (new)
  isle-keychain/          the app: sidebar, list, detail, sheets,
                          generator, settings window (new)
  isle-fakes/src/
    secrets.rs            the Secret Service fake (new)
    polkit.rs             CheckAuthorization
  isle-app/src/data.rs    the secret selection: zeroed, withdrawn,
                          replaced
  isle-compositor/src/
    dbus.rs               Shell1.Copied
  isle-settings/          the keychain section
data/
  apps/                   Keychain's desktop entry and service file
  polkit/                 io.github.alexparlett.Isle.Keychain.policy
  icons/phosphor/         vault, dice-five (and their fills)
pkg/arch/PKGBUILD         the app and the policy
```

## Phases

Each phase ends with its tests passing headless and a line in
`progress.md`.

### 19c.0 · The note and the record

- `apps-2026.md` §6–7: the gate's change from `auth_self_keep` and the
  agent-owned Wi-Fi path, which closes the research's open point;
- the roadmap's P19c entry;
- choice 4's three lines into `p1/structure/keychain.md`;
- the glyphs vendored;
- polkit's `_keep` expiry checked in the VM (choice 16 rests on it
  running from the grant, not the last use);
- the crate versions pinned, `oo7` and `ssh-agent-lib` checked against
  the workspace's zbus and `ssh-key`.

### 19c.1 · The fakes

The Secret Service fake (choice 25), `CheckAuthorization` on the polkit
fake. The fake is tested against `oo7::dbus` itself.

### 19c.2 · Secrets

`isle-secrets`' Secret Service side (choices 6–10):
- the session and the listing;
- kinds over the host's shapes;
- the fetch;
- create, edit and delete;
- lock, unlock and change;
- the signals.

### 19c.3 · Wi-Fi

Saved networks and their secrets by flag (choice 11), against the NM fake
with system-owned, agent-owned and enterprise profiles.

### 19c.4 · SSH keys and the agent

Keys from `~/.ssh`, the agent behind the proxy, add, remove, generate and
delete (choices 13–15). Choice 27's tests.

### 19c.5 · The gate, copy and the generator

- The policy and the grant (choice 16);
- `isle-app`'s secret selection, the withdrawal and the process held
  (choice 18);
- Shell1's `Copied` and the island's line;
- the generator (choice 20).
Choice 26's clipboard and gate tests.

### 19c.6 · The window

`isle-keychain`:
- the toolbar, sidebar with counts and locks, the list and the detail;
- search;
- Show, Copy and their hiding;
- the menus and command table;
- the desktop entry, service file and package.

### 19c.7 · Edit, new and delete

Edit in place with leaving asked, New login with Save in, the delete
alerts with choice 4's line, the generator's popover.

### 19c.8 · Wi-Fi and SSH details

Wi-Fi's detail and Wi-Fi Settings…; the SSH detail with the public key's
well, Add to agent, the passphrase sheet; New SSH key with its bar;
delete to the Trash with Undo.

### 19c.9 · The generator sheet and settings

The generator sheet; Keychain's settings window and its section.

### 19c.10 · Locks, absent sources and the cuttables

P1's locked row and Unlock…; no Secret Service; a frozen service; a
failed fetch. "Change password…" and dragging a login out (cuttable).

### 19c.11 · Motion, states and accessibility

- P1's Motion under springs, Reduce Motion and Game Mode;
- every row of States;
- Increase Contrast, the largest text size with labels above values,
  focus, the narrow stacked form.

### 19c.12 · Measures and the walk

The bars on a release build (choice 29). Choice 28's checks against both
daemons. Walked in the VM by real input, as `verify-by-interaction` asks:
- the gate asked, refused and allowed, then idle and lock ending it;
- a password shown, hidden by time and by minimising;
- a password copied, pasted into another app, missing from history, and
  withdrawn;
- a copy, then ⌘Q, then a paste;
- a login made, edited, searched and deleted;
- the keyring locked and unlocked through the prompter;
- a Wi-Fi password shown;
- a key generated with a passphrase, added, removed and deleted, then
  undone.
The journal is read after, and the canary test runs over the walk's
session.

### 19c.13 · The host and close

The host run on the installed package:
- the twelve items read through with the user: their kinds, names and
  the storing app's icon;
- a Wi-Fi password shown;
- `id_ed25519` added through gcr and listed truly;
- a login made and deleted;
- bars 1 and 2.
Nothing on the host is edited or deleted except what the run made.
Then the record:
- decisions recorded from P19b's last;
- ARCHITECTURE gains How Keychain reads secrets;
- `apps.md`'s open points stay open (passkeys, certificates);
- the roadmap and `state.md` follow;
- this file ends with its outcome.

## Not in P19c

- Password-manager providers, Spotlight's `*` scope and verification codes
  (P20, with the vault).
- Importing logins from browsers, and Firefox's own store.
- Passkeys, which wait for a credential portal (`passkeys.md`), and
  certificates (apps.md Keychain's open points).
- Changing an SSH key's passphrase, which P1 did not design.
- Editing or forgetting a Wi-Fi network (Settings › Wi-Fi's).
- Isle as a vault of its own (D42: the stores are the Secret Service and
  the providers).

## Depends on, and feeds

- Needs:
  - P8 (`isle-network` and the secret agent);
  - P9 (the polkit agent, the keyring prompter, the lock's signal);
  - P10 (clipboard history's skip);
  - P13 (`isle-app`, `isle-settings` sections);
  - P14a (the package);
  - P15 (`isle-app`'s data device, `isle-fs`'s Trash);
  - P19a (`use_shown()`, the policy pattern).
- Feeds P20's vault provider and Spotlight's `*` scope, both on
  `isle-secrets`' fetch and gate.

## Done when

- `cargo test` passes headless with the fakes, the kinds, the fetch, the
  Wi-Fi, SSH and agent tests, and choice 26's tests of restraint.
- In the VM, Keychain is walked by real input through every row of the
  sheet's Keyboard, Pointer and States, against gnome-keyring and
  oo7-daemon, and P1's board's scenes can be reproduced.
- On the host, the keyring's twelve items are read through with the user,
  a Wi-Fi password shows, and the SSH key's agent state is true.

## Risks

- **A drawn value outlives its hiding.** Freya and Skia copy a shown
  string into their layout and glyph caches, which Keychain cannot zero.
  The value is drawn only on Show and dropped at hide, and the gate is a
  gate against a walked-up session, not against the user's own processes.
  The decision says so.
- **Kinds on the user's own items.** Twelve items are few to tune on, and
  a wrong kind puts a storage key among logins. Every schema in the table
  has a test shape, the fallback is Apps, and the host run reads the
  twelve through.
- **gcr's inner socket is gcr's detail.** If gcr changes where its agent
  lives, "In agent" falls back to the proxy's list, which over-reports.
  The socket is found by `SSH_AUTH_SOCK`'s directory, not a fixed path,
  and the VM test runs the real gcr.
- **The process lingers after ⌘Q.** A copy waiting to be withdrawn keeps
  Keychain alive for up to five minutes with no window. It is invisible,
  short and ends by itself; withdrawing at quit instead would lose the
  paste Mac users expect.
- **oo7 0.7 moves to zbus 6.** 0.6 is pinned with the workspace's zbus 5,
  and moving both is its own change.
