# Keychain

Batch 3. Answers to: `apps.md` For the design process (Keychain: how kinds
and providers are laid out) and the presentation side of `apps.md`
Keychain 1–11, its Open items where they are presentation (what gates a
reveal, browsers' own entries), and the Parity map's Keychain rows;
`spotlight.md` (the `*` scope, which this sheet supplies); `prompts.md`
(the keyring prompt, which this sheet only calls); `settings.md` (General ›
Passwords, where providers are added); B12, B17, D20, D42.

## Purpose

The Passwords app with Keychain Access's reach: one window to find, read,
copy, add and remove every secret the user keeps, whether it lives in the
login keyring, NetworkManager, `~/.ssh` or a password manager (`apps.md`
Keychain). It is where a Wi-Fi password is read out for a guest, an SSH key
is added to the agent, and a login saved by a tool is checked. It is not
a password manager of its own (the stores are the Secret Service and the
providers), not the keyring's unlock dialog (the compositor's prompter,
B12), and not where providers are installed or chosen (Settings › General
› Passwords).

## Content

### The window

```
┌──────────────────┬──────────────────────────┬────────────────────────────┐
│ ● ● ●        ◧   │ Logins  42 items          │                 +  …  🔍  │  toolbar
│                  │ ┌──────────────────────┐ │                            │
│ All              │ │ 🔑 GitHub             │ │  GitHub                    │
│ Logins           │ │    alex@…  · Login    │ │                            │
│ Wi-Fi            │ │ 🔑 Grafana            │ │  User name  alex   Copy    │
│ SSH keys         │ │    admin · Proton Pass│ │  Password   ••••••  Show   │
│ Apps             │ │ …                     │ │                    Copy    │
│                  │ └──────────────────────┘ │  Website    github.com     │
│ Login       🔒   │                          │  Notes                     │
│ Proton Pass      │                          │                     Edit   │
└──────────────────┴──────────────────────────┴────────────────────────────┘
   sidebar              list                        detail
```

Sidebar and a content split into the list and the selected item's detail,
as Passwords; no inspector (`conventions.md`, Window anatomy).

- **Toolbar.** Leading: the lights; the sidebar toggle; the title, the
  selected sidebar row's name, with the item count as subtitle. Centre:
  nothing. Trailing: New (+), the one primary action, which makes the
  selected kind's item (a login, or an SSH key in SSH keys) and is
  disabled where nothing can be made (Wi-Fi, Apps, a provider that offers
  no create action); More (…) with Generate password…, Lock, Unlock and
  Change password… for the selected keyring; the search field, which
  searches every source the selection spans.
- **The sidebar**, two groups separated by space, no headings
  (`conventions.md`):
  1. **Kinds**: All, Logins, Wi-Fi, SSH keys, Apps. A kind spans every
     source: Logins shows the keyring's logins and every provider's.
  2. **Sources**: each Secret Service collection by its label (on most
     machines one, "Login"), then each password manager enabled in
     Settings › General › Passwords, by the provider's name. A source
     shows only its own items. A locked source carries a lock.
- **The list**: one row per item, its kind's symbol, its title, and a
  second line with the user name or the item's main attribute and, where
  the list spans sources, the source's name. Sorted by title. A provider
  with more than one vault groups its rows under vault names. No row ever
  holds a secret (`apps.md` Keychain 3).
- **The detail**: the item's title, then its fields as rows: user name,
  password, verification code (when the provider gives one), website or
  server, notes; then a group of what Keychain knows about it: kind,
  stored by (the application attribute), source, created, modified.
  Secret fields show dots, with Show and Copy.

### Kinds

| Kind | Holds | From |
|---|---|---|
| Logins | Items with a user and a website or server, whoever stored them (a browser's site logins included), and providers' login items | Secret Service; providers |
| Wi-Fi | Saved networks: name and security; the password on Show | NetworkManager's own secrets call, polkit gated (`apps.md` Keychain 10) |
| SSH keys | Each key in `~/.ssh`: name, type, fingerprint, comment, whether the agent holds it | `~/.ssh`; the agent (`apps.md` Keychain 8) |
| Apps | Every other Secret Service item: tokens and keys apps keep for themselves, a browser's storage key included, named by the app that stored it | Secret Service |

Items a provider reports of other kinds (notes, cards) show under All and
their provider only. Browser is not a kind of its own (v1 had one): a
browser's site logins are logins and its storage key is an app secret, so
the kind follows what the item is, not who wrote it. Deleting an Apps
item says what will stop working ("{App} will ask you to sign in again.").

### Reveal and copy

- **Nothing in memory until asked** (D42; `apps.md` Keychain 3). Show
  fetches one field of one item at that moment; Copy fetches it, copies
  it and drops it. A value is never fetched for hover, search or a list.
  Show is a click, never a hover, since a pointer passing over a row is
  not asking.
- **What gates a reveal** beyond an unlocked source: the user's password
  or fingerprint, asked in the desktop's polkit dialog once when Keychain
  opens and again after it has sat idle (For the user 1). It gates Show,
  Copy and Spotlight's `*` scope alike.
- **Shown values hide again** after Keychain's idle period, when the
  window closes or is minimised, and when the session locks (`apps.md`
  Keychain 11). Only one field shows at a time; showing another hides the
  first.
- **Copy** marks the clipboard as a secret so clipboard history skips it,
  and withdraws it after the period set in Keychain's settings unless
  something else has been copied since (`apps.md` Keychain 5). The island
  shows "Password copied" (`island.md`).
- **Wi-Fi** passwords ask in the desktop's polkit dialog when
  NetworkManager requires it; refused, the field stays dots.

### Editing and deleting

Edit turns the detail's fields into text fields in place, with Cancel and
Done, as Passwords; the password field has a generator button
(Generator, below). Delete is in Edit mode, the Item menu (⌘Delete) and
the context menu, and asks: "Delete “{title}”?", "You can't undo this.",
Cancel (default) and Delete (`apps.md` Keychain 6; `conventions.md`).
Wi-Fi items are read-only here: forgetting a network and its settings are
Settings › Wi-Fi's, reached by "Wi-Fi Settings…" in the detail.

**New login** is a sheet: title, user name, password, website or server,
notes, and "Save in", a pop-up of the sources that accept new items.

### SSH keys

The detail shows type, fingerprint (selectable), comment, the public key
with Copy public key (it is not a secret), and Add to agent or Remove from
Agent. A key with a passphrase asks for it in a sheet, passed to `ssh-add`
on a pipe (`apps.md` Keychain 8). **New SSH key** is a sheet: name (the
file name), type (Ed25519 default, RSA), comment, passphrase and its
confirmation; generating is work in flight (`apps.md` Conventions 4).
Delete moves both files of the pair to the Trash, with Undo.

### The generator

Item › Generate password… opens a sheet: the password, large and in the
monospaced face; length (a slider with a number field); switches for
upper case, lower case, digits and symbols; Generate again; Copy and Done
(`apps.md` Keychain 7). In New login and Edit the same controls open as a
popover from the password field, with Use instead of Copy, since a sheet
cannot stack on a sheet (`conventions.md`). The last length and sets are
remembered.

### Locked and absent sources

- **A locked keyring** (as after a fingerprint login, `apps.md`
  Keychain 4): its sidebar row shows a lock. Selecting it, or any kind
  that spans it, puts one row at the top of the list: "{Source} is
  locked" with Unlock, which calls the Secret Service's Unlock and brings
  up the compositor's keyring prompt (`prompts.md`). Keychain draws no
  password field for it. Titles a locked collection still lists stay
  listed; their secrets wait.
- **A locked or signed-out provider** shows its state sentence and its own
  actions (Unlock, Sign In) in the content, in the provider's words
  (D20, D42); a sign-in form the provider declares is a sheet. In All and
  the kinds, it is the same one row at the top as a locked keyring.
- **No provider enabled**: the Sources group holds the keyrings only;
  nothing is drawn for absent providers (brief P1).
- **No Secret Service**: the content says "No keyring service is
  running." and Wi-Fi and SSH keys keep working (`apps.md` Conventions
  23).

### Spotlight's `*` scope

`spotlight.md` draws it; Keychain supplies it. The scope searches Logins
and Wi-Fi across every unlocked source, never Apps items or SSH keys,
which are plumbing rather than something typed. Rows carry the title and
the source's name; Return copies the password, Shift+Return the user name,
Ctrl+Return the verification code, through the same fetch and gate as
Keychain's Copy. A locked source is one row, "{Source} is locked", whose
action opens Keychain on that source. The row's menu (Cmd+K) adds "Show in
Keychain".

### Keychain's settings

A settings window of one pane (`conventions.md`): "Clear copied passwords
after" (30 s, 1 min, 2 min, 5 min; default 1 min), "Hide shown passwords
after" (30 s, 1 min, 5 min; default 1 min). The gate has no row of its
own (For the user 1).

## States

| State | What shows |
|---|---|
| Rest | The last sidebar row, the list, the detail of the selected item |
| Loading | Sidebar at once; list rows as each source answers; a slow provider adds its rows below without moving the selection |
| Empty kind | One line: "No saved logins", "No saved Wi-Fi networks", "No SSH keys" with "New SSH key…", "No app secrets" |
| Nothing selected | The detail says "No item selected" |
| No results | "No results for “{query}”" |
| Locked source | Its lock in the sidebar; one "{Source} is locked" row at the top of each list that spans it |
| Provider failed | Its state sentence and error in the content (`copy.md` rule 7); other sources unaffected |
| Shown value | The field's value in place of the dots, until hidden as above |
| Editing | The detail's fields as text fields; the list and sidebar still work, and leaving asks to keep or discard changes |
| Generating a key | Progress in the sheet and on the switcher card; quitting asks (`conventions.md`) |
| Narrow window | The sidebar collapses first, then the list and detail stack, the detail opening over the list with Back |
| Second screen | No difference |
| Game in front | An ordinary window |
| Locked session | Nothing shows; shown values are hidden at lock |
| Focus on | No change |
| Reduce Motion | Edit mode and sheets fade |
| Reduce Transparency | Sidebar solid |
| Increase Contrast | Borders on fields, the list's rows and the detail's groups |
| Largest text size | Rows grow; the detail's labels go above their values |

## Opening and closing

From Spotlight, the Apps grid, a `*` row's "Show in Keychain" or a locked
row's unlock (opening on that source), and the Spotlight empty state's
"Open Keychain". It opens as any Isle app, focus on the list, at the last
sidebar row. Closing hides every shown value and drops every fetched one;
the process ends with the window unless a key is being generated.

## Keyboard

| Key | Action |
|---|---|
| ↑ ↓, type to select | Move in the sidebar or the list |
| Return | Edit the selected item; in Edit, Done |
| ⌘N, ⇧⌘N | New login, New SSH key |
| ⌘C | Copy the password, when no text is selected |
| ⇧⌘C, ⌥⌘C | Copy the user name, the verification code |
| ⌘R | Show or hide the password |
| ⌥⌘G | Generate password… |
| ⌘Delete | Delete (asks) |
| ⌘F | Search |
| Escape | Cancel Edit; clear the search |

The standard map is `conventions.md`'s.

## Pointer

- Click selects; secondary click opens the item's menu: Copy Password,
  Copy User Name, Copy Verification Code, Edit, Delete…; on an SSH key,
  Add to agent or Remove from agent, Copy public key; on a source, Lock,
  Unlock, Change password….
- Show and Copy are buttons on the field's row; nothing reveals on hover.
- A login dragged out drags its title and website, never a secret.

## Shared places

| Place or input | Competes with | Proposed winner |
|---|---|---|
| The clipboard | Clipboard history | A copied secret is marked so history never records it, and withdrawn after the set time unless replaced |
| The keyring's unlock | A dialog of Keychain's own | The compositor's keyring prompter (B12): Keychain asks the Secret Service and draws nothing |
| A Wi-Fi password | Settings › Wi-Fi | Keychain shows and copies; Settings forgets and edits. One place each |
| Providers | Settings › General › Passwords | Settings adds, enables and removes them; Keychain signs in, unlocks and uses them, in the provider's words |
| `*` in Spotlight | Keychain's own search | Both, one gate: Spotlight's copies pass through the same fetch and the same gate as Keychain's |
| The island's "Copied" event | Spotlight's copies | Shared: "Password copied" from either (`island.md`) |

## Answers

- **How kinds and providers are laid out** (proposal: categories Logins,
  Wi-Fi, Browser, Apps, then one section per provider). **Changed**: a
  sidebar of kinds (All, Logins, Wi-Fi, SSH keys, Apps) then sources (each
  keyring, then each provider); a kind spans every source, a source shows
  its own. Browser folds into Logins and Apps by what each item is.
- **Reveal and copy** (`apps.md` Keychain 3, 5, 11). **Adopted**, with
  Show as a click, one field at a time, hidden at lock as well.
- **Whether browsers' own Secret Service entries are shown** (`apps.md`
  Keychain Open). **Answered**: yes, under Apps, with a warning on delete.
- **The generator** (`apps.md` Keychain 7). **New**: a sheet from the Item
  menu, and the same controls as a popover in a password field.
- **Locked providers** (`apps.md` Keychain 4, 9). **New**: a lock in the
  sidebar and one unlock row at the head of any list that spans the
  source, matching Spotlight's.
- **Spotlight's `*` scope** (`spotlight.md`). **New** on Keychain's side:
  Logins and Wi-Fi only, through the same gate.
- **What gates a reveal** (`apps.md` Keychain Open). **Answered** as the
  Passwords app: the password or fingerprint once on opening and again
  after idle, covering Spotlight's `*` copies (For the user 1).
- Certificates and passkeys stay out until a credential portal exists
  (`apps.md` Keychain Open).

## Copy

| Where | String |
|---|---|
| Sidebar | "All", "Logins", "Wi-Fi", "SSH keys", "Apps"; sources by their names |
| Detail labels | "User name", "Password", "Verification code", "Website", "Server", "Notes", "Kind", "Stored by", "Source", "Created", "Modified", "Type", "Fingerprint", "Comment", "Public key", "In agent" |
| Buttons | "Show", "Hide", "Copy", "Edit", "Done", "Cancel", "Unlock", "Wi-Fi Settings…", "Add to agent", "Remove from agent", "Copy public key", "Generate again", "Use" |
| File | "New login…", "New SSH key…" |
| Item menu | "Copy Password", "Copy User Name", "Copy Verification Code", "Show Password", "Hide Password", "Edit", "Delete…", "Generate password…", "Lock", "Unlock…", "Change password…" |
| Generator | "Length", "Upper case", "Lower case", "Digits", "Symbols" |
| New Login | "Save in" |
| Delete alert | "Delete “{title}”?", "You can't undo this.", "Cancel", "Delete"; for an Apps item, "{App} will ask you to sign in again." |
| Locked | "{Source} is locked" |
| Empty | "No saved logins", "No saved Wi-Fi networks", "No SSH keys", "No app secrets", "No item selected" |
| Missing | "No keyring service is running." |
| Passphrase sheet | "Enter the passphrase for “{key}”" |
| Settings | "Clear copied passwords after", "Hide shown passwords after" |
| Island | "Password copied" (`island.md`) |

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **What a reveal asks for.** With the keyring unlocked: (a) nothing
   more, Show and Copy act at once; (b) as the Passwords app, the user's
   password or fingerprint once when Keychain opens and again after it
   has sat idle, in the desktop's polkit dialog, covering Spotlight's `*`
   copies too; (c) as Keychain Access, the password at every reveal.
   **Recommended: (b).** It is what a Mac user expects (B17), it guards a
   secret on a walked-up, unlocked session, and with the fingerprint
   reader (D33) it costs a touch; (c) asks so often that it trains the
   user to type the password without reading the dialog.
