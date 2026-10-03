# Copy

Batch 1, with batches 2 and 3's recurring strings collected. Answers to:
shell.md "Shell runtime" (UI copy rules), and the copy items every other
sheet points here for. The rules every string in the shell and Isle's apps
follows, and the strings that recur.

## Rules

1. **Sentence case** everywhere: titles, buttons, menu items, settings,
   notifications. Proper nouns and app names keep their own case. No
   uppercase labels, no letter-spaced captions. (v1 §14.) The exception is
   macOS's standard menu items and system commands, which keep Apple's
   title case so they read as the Mac's own: Hide Others, Show All, Bring
   All to Front, Force Quit, Shut Down, Log Out, Lock Screen (gate 2).
   A link to a Settings pane is such an item: "{Pane} Settings…".
   Isle's own phrases stay in sentence case: Move and resize, Keep on top.
2. **macOS words where macOS has one.** Desktop, not workspace; Trash, not
   bin or wastebasket; Quit, not exit; Settings, not preferences or
   options; Hide, Minimise, Zoom, Full Screen; Force Quit; Log Out, Sleep,
   Restart, Shut Down, Lock Screen. A Linux word is used only where macOS
   has none (Flatpak, PipeWire are not shown at all unless the user must
   act on them). (brief P3.)
3. **British spelling**: colour, centre, minimise, licence (noun).
   Apple's spellings yield to it, so "Control Centre", "Minimise".
4. **Name buttons for what they do.** "Delete 3 items", "Share",
   "Install update", not "OK" or "Yes". Cancel is "Cancel". A destructive
   button names the loss. Two buttons never both mean yes.
5. **Say it once, short.** A settings page's subtitle is one sentence,
   under seventy characters. A row has a description only when its label
   leaves something to say. No instructions under switches. (v1
   `39c886d`.)
6. **No ellipsis** except on a command that asks for more before acting
   ("Settings…", "Rename…"), as on macOS, and on work under way that ends
   by itself ("Quitting apps…").
7. **Errors say what happened and what to do**, in that order, without
   blame or codes up front: "Couldn't connect to Home Wi-Fi. Check the
   password and try again." A code or log path goes after, for the user
   who wants it.
8. **No exclamation marks, no jokes, no "oops"**, no first person for the
   system ("I couldn't…"), no "please". The user is "you" only where a
   sentence needs a subject.
9. **Numbers as numerals**, units with a space and percent without (4 GB, 3 min, 72%), times
   in the user's locale (24-hour by default on this machine). Counts
   agree: "1 update", "2 updates".
10. **Status reads as a state, not a log.** "Connected", "Charging",
    "Playing", not "Connection established".
11. **Keyboard shortcuts** are shown with macOS's symbols (⌘ ⌥ ⌃ ⇧) on a
    Mac-layout keyboard and as words (Super, Alt, Ctrl, Shift) on a PC
    one, from the focused device's profile (B30).

## Recurring strings

| Where | String |
|---|---|
| Summary after silence | "While you were away" / "While you played", then "{n} notifications from {app}"; its sender "Notification Centre" |
| Notification reply | "Reply" (the field's placeholder and the action), "Send", "Cancel" |
| Banner overflow | "{n} more in Notification Centre" |
| Refused permission (callers) | "Permission was not given." |
| Password prompts | "{App} needs your password", "“{network}” needs a password", "“{VPN}” needs a password"; status "Wrong password. Try again.", "Caps Lock is on", "Too many attempts. Try again in {n} min." (prompts and the lock alike) |
| Sharing picker | "Choose what to share with {app}" |
| Capture files | "Screenshot {date} at {time}.png", "Screen Recording {date} at {time}.{ext}", in macOS's case |
| Restart owed | "Restart to finish updating" |
| Updates | "Updates" with the count as a badge; "Install updates…"; the sources "Repositories", "AUR", "Isle" |
| Settings links | "{Pane} Settings…", naming the pane it opens: "Wi-Fi Settings…", "Focus Settings…", "Keyboard Settings…", "Software Update Settings…", "Control Centre Settings…" (rule 1) |
| Control Centre | "Known Networks"; "Not playing"; a failed action's first clause, "Couldn't connect"; Dark Mode "On", "Off"; Accessibility "{n} on" |
| Rejected hand edit | "settings.kdl has an error on line 12, column 4. Changes won't be saved until it's fixed." |
| Rejected token edit | "tokens.kdl has an error on line 12, column 4. The last good look is in use." |
| Safe start | "Isle started without your settings because they stopped it starting."; "Use my settings", "Report…" |
| Crash notice | "{App} quit unexpectedly." with "Reopen" and "Report…"; for Isle, "Isle quit unexpectedly." and "Your apps were reopened." |
| Log out | "Are you sure you want to log out?" with "Reopen windows when logging back in" and "Log Out"; restart and shut down likewise, with "If you do nothing, the computer will shut down in {n} seconds." |
| Session chooser | "Lock Screen", "Log Out", "Sleep", "Restart", "Shut Down", "Cancel" |
| Quitting for a restart or shut down | "Quitting apps…" (the lock's confirm in place) |
| Session interrupted | "Couldn't restart because {App} didn't quit." ("shut down", "log out" likewise), "Show {App}" |
| Force Quit | "Force Quit" (window title and button); "Do you want to force {App} to quit? You will lose any unsaved changes." |
| Quit with work in flight | Title naming the work ("Copying 3 items to Backup"), "Quitting stops it.", "Cancel", "Quit" |
| Display change | "Keep these display settings?", "Reverting in {n} seconds.", "Revert", "Keep" |
| A changed setting | "Reset to default" |
| Search with no match | "No results for “{query}”" |
| Feature names | "Do Not Disturb", "Game Mode", "Night Light", "Dark Mode", "Energy Mode", "Now Playing", "Notification Centre", "Mission Control", "App Exposé", "Show Desktop", "Split View", "Stage Manager", "Quick Look" keep their capitals as names (rule 1). The user reads "desktop", never "Space" or "workspace": "Desktop 1" |
| Hot corner actions | "Mission Control", "Application Windows", "Desktop", "Notification Centre", "Apps", "Widgets", "Lock Screen", "Put Display to Sleep", "—" |
| Clearing | "Clear" (one), "Clear {app} notifications" (a stack), "Clear all", a close mark that widens to it before acting |
| Trash | "Move to Trash", "Put Back", "Empty Trash…"; alert "Permanently erase {n} items in the Trash?", "You can't undo this.", "Empty Trash" (Files and Cleaner alike) |
| Undo | "Undo {act}" naming it ("Undo Move to Trash", "Undo Rename"); "Can't Undo" when there is none; "Undo" on a done notice |
| Name taken | "An item named “{name}” already exists in “{folder}”." with "Keep Both", "Replace" ("Merge" for folders; "Apply to all", "Stop" for many) |
| Deleting and quitting one thing | Title asks with the name: "Delete “{title}”?", "Quit “{process}”?"; the button names the act: "Delete", "Quit", "Force Quit". Monitor's menu item is "Quit Process" |
| Locked source | "{Source} is locked" with "Unlock…" |
| Copied secret | "Password copied" (the island's event, from Keychain and Spotlight) |
| A file in another app's menu | "Show in Files", "Quick Look", "Copy path", "Open With" |
| More of a list | "{n} more", "Show less"; "+{n}" where a count stands for the rest in a row (timers, schedules, open apps) |
| Spotlight Apps categories | "All apps", "Favourites"; "Developer tools", "Games", "Graphics & design", "Internet", "Music", "Video", "Productivity", "Education", "Utilities", "Other" |
| Empty states | One line saying what will appear: "No notifications". Never a card of filler. |

Strings a later sheet owns are added there and collected here at the
gate.
