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
   title case so they read as the Mac's own: About This Computer, Hide
   Others, Show All, Bring All to Front, Force Quit, Shut Down, Log Out,
   Lock Screen (gate 2); and macOS's standard alert buttons: Keep Both,
   Replace, Don't Save, Don't Allow, Try Again, Put Back, Force Eject,
   Empty Trash. A link to a Settings pane is such an item: "{pane}
   Settings…". Isle's own phrases and buttons stay in sentence case: Move
   and resize, Keep on top. The accessibility switches are settings, in
   sentence case as macOS's Settings writes them: "Increase contrast",
   "Reduce motion", "Reduce transparency", "On-screen keyboard".
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
   ("Settings…", "Rename…"), as on macOS, and on a line naming work under
   way that ends by itself ("Quitting apps…", "Scanning… 61%",
   "Calculating…"). A one-word status is a state and takes none
   ("Connecting", "Checking", rule 10). The lock's power row keeps the
   login window's bare labels, "Restart" and "Shut Down", though they
   confirm in place.
7. **Errors say what happened and what to do**, in that order, without
   blame or codes up front: "Couldn't connect to “Home Wi-Fi”. Check the
   password and try again." A code or log path goes after, for the user
   who wants it. A {reason} is one sentence saying why and, where the
   user can act, what to do ("The screen is in use by another
   recording.").
8. **No exclamation marks, no jokes, no "oops"**, no first person for the
   system ("I couldn't…"), no "please". The user is "you" only where a
   sentence needs a subject.
9. **Numbers as numerals**, units with a space and percent without (4 GB,
   3 min, 72%), times in the user's locale (24-hour by default on this
   machine). Counts agree: "1 update", "2 updates". A pop-up of durations
   abbreviates its units ("5 s", "30 s", "1 min"); units are spelt out
   only in a sentence ("Reverting in {n} seconds.").
10. **Status reads as a state, not a log.** "Connected", "Charging",
    "Playing", not "Connection established".
11. **Keyboard shortcuts** are shown with macOS's symbols (⌘ ⌥ ⌃ ⇧) on a
    Mac-layout keyboard and as words (Super, Alt, Ctrl, Shift) on a PC
    one, from the focused device's profile (B30). The words name the keys
    pressed: with the PC swap on, Cmd is shown as Alt and Option as Super.
12. **Typographic punctuation.** Quotes and apostrophes are curly in the
    UI (“ ” ’), as is the ellipsis (…), and × in sizes, spaced ("1920 ×
    1080"). The sheets write the apostrophe straight; the build renders
    it as ’.
13. **Placeholders** are {lower case} in braces, since the value carries
    its own case: {app}, {pane}, {network}. One quantity has one name:
    "{n} of {total}" for every progress or position count, {free} for
    free space, {volume} for a drive, {display} for a screen.
14. **Quoted names.** Curly quotes go round a name the user or the file
    system gave (a file, folder, volume, network, VPN, widget, key, item
    title, process, query) inside a sentence: "Couldn't connect to
    “{network}”.", "“{widget}” isn't installed.". None round an app,
    device or person's name, or round a name standing alone as a label or
    title.
15. **Accessible names.** Every symbol-only control, glyph and mark has a
    name: the thing, then its state as a second clause ("Wi-Fi: Home,
    signal low"; "Shuffle, on"). A control with a tooltip uses the
    tooltip's words without the chord. A state shown only by a mark
    (changed, waiting, nearly full) is the item's accessible description.

## Recurring strings

| Where | String |
|---|---|
| Summary after silence | "While you were away" / "While you played", then "{n} notifications from {app}"; its sender "Notification Centre" |
| Notification reply | "Reply" (the field's placeholder and the action), "Send", "Cancel" |
| Banner overflow | "{n} more in Notification Centre" |
| Refused permission (callers) | "Permission was not given." |
| Password prompts | "{app} needs your password", "“{network}” needs a password", "“{vpn}” needs a password"; status "Wrong password. Try again.", "Caps Lock is on", "Too many attempts. Try again in {n} min." (prompts and the lock alike); the lock only: from the second, "Wrong password, {n} attempts. Try again.", and with several layouts set, "Wrong password. Check the keyboard layout ({name})." |
| Sharing picker | "Choose what to share with {app}" |
| Capture files | "Screenshot {date} at {time}.png", "Screen Recording {date} at {time}.{ext}", in macOS's case |
| Restart owed | "Restart to finish updating" |
| Updates | "Updates" with the count as a badge; "Install updates…"; "Check now"; the sources "Repositories", "AUR", "Isle" |
| Settings links | "{pane} Settings…", naming the pane it opens: "Wi-Fi Settings…", "Focus Settings…", "Keyboard Settings…", "Software Update Settings…", "Control Centre Settings…", "Notifications Settings…", "Calendars Settings…", "Wallpaper Settings…" (rule 1) |
| Control Centre | "Known networks", "Other networks", "Join other network…"; "Not playing"; a failed action's first clause, "Couldn't connect"; Dark Mode "On", "Off"; Accessibility "{n} on"; "Edit controls" (Control Centre's and the island's footers) |
| Rejected hand edit | "settings.kdl has an error on line 12, column 4. Changes won't be saved until it's fixed." with "Open in editor", "Show in Files" |
| Rejected token edit | "tokens.kdl has an error on line 12, column 4. The last good look is in use." |
| Safe start | "Isle started without your settings because they stopped it starting."; "Use my settings", "Report…" |
| Crash notice | "{app} quit unexpectedly." with "Reopen" and "Report…"; for Isle, "Isle quit unexpectedly." and "Your apps were reopened." |
| Log out | "Are you sure you want to log out?" with "Reopen windows when logging back in" and "Log Out"; restart and shut down likewise, with "If you do nothing, the computer will shut down in {n} seconds." |
| Session chooser | "Lock Screen", "Log Out", "Sleep", "Restart", "Shut Down", "Cancel" |
| Quitting for a restart or shut down | "Quitting apps…" (the lock's confirm in place) |
| Session interrupted | "Couldn't restart because {app} didn't quit." ("shut down", "log out" likewise), "Show {app}" |
| Force Quit | "Force Quit" (window title and button); "Do you want to force {app} to quit? You will lose any unsaved changes." |
| Quit with work in flight | Title naming the work ("Copying 3 items to “Backup”"), "Quitting stops it.", "Cancel", "Quit" |
| Display change | "Keep these display settings?", "Reverting in {n} seconds.", "Revert", "Keep" |
| A changed setting | "Reset to default" |
| Search with no match | "No results for “{query}”" |
| Feature names | "Do Not Disturb", "Game Mode", "Night Light", "Dark Mode", "Energy Mode" with its modes "Low Power" and "High Power" ("Automatic" is a plain word), "Now Playing", "Notification Centre", "Mission Control", "App Exposé", "Show Desktop", "Split View", "Stage Manager", "Quick Look" keep their capitals as names (rule 1). The user reads "desktop", never "Space" or "workspace": "Desktop 1" |
| Hot corner actions | "Mission Control", "Application Windows", "Desktop", "Notification Centre", "Apps", "Widgets", "Lock Screen", "Put Display to Sleep", "—" |
| Clearing | "Clear" (one), "Clear {app} notifications" (a stack), "Clear all", a close mark that widens to it before acting |
| Trash | "Move to Trash", "Put Back", "Empty Trash…"; alert "Permanently erase {n} items in the Trash?", "You can't undo this.", "Empty Trash" (Files and Cleaner alike) |
| Undo | "Undo {act}" naming it ("Undo Move to Trash", "Undo Rename"); "Can't Undo" when there is none; "Undo" on a done notice |
| Name taken | "An item named “{name}” already exists in “{folder}”." with "Keep Both", "Replace" ("Merge" for folders; "Apply to all", "Stop" for many) |
| Deleting and quitting one thing | Title asks with the name: "Delete “{title}”?", "Quit “{process}”?"; the button names the act: "Delete", "Quit", "Force Quit". Monitor's menu item is "Quit Process" |
| Locked source | "{source} is locked" with "Unlock…"; in Spotlight, "Unlock in Keychain" |
| Copied | "Copied", "Colour copied", "Screenshot copied"; a secret "Password copied" (the island's events, from Spotlight, Keychain and capture) |
| A file in another app's menu | "Show in Files", "Quick Look", "Copy path", "Open With" |
| More of a list | "{n} more", "Show less"; "+{n}" where a count stands for the rest in a row (timers, schedules, open apps) |
| Spotlight Apps categories | "All apps", "Favourites"; "Developer tools", "Games", "Graphics and design", "Internet", "Music", "Video", "Productivity", "Education", "Utilities", "Other" |
| Empty states | One line saying what will appear: "No notifications", "No windows", "No devices", "No calendars". Never a card of filler. |
| Window states | "Minimised", "Hidden", "Not responding" (the switcher, Mission Control, Force Quit) |
| Retry | "Try Again" (rule 1) |
| Search field | "Search" (the placeholder) |
| Provider row | "Options", "Permissions…"; "{provider} didn't answer. Try again later." |
| Progress and position | "{n} of {total}" for every count of a whole ("Installing {n} of {total}") |

Strings a later sheet owns are added there and collected here at the
gate.

## Collected from the surfaces

Strings the wave C surface sheets draw that their structure sheets do not
list.

| Where | String |
|---|---|
| Weather | One word per glyph: "Clear", "Partly cloudy", "Cloudy", "Rain", "Snow", "Thunderstorms", "Fog", "Windy"; "{n}°"; high and low "H:{high}° L:{low}°" |
| Storage widget, Cleaner's disks | "{size} free" |
| Grants, read only | "Allowed", "Not allowed", then "Change…" |
| Monitor's GPU block | "Memory" as "{used} of {total}", "Temperature", "Power", "Fan", "Clocks"; graph titles "CPU load", "Memory pressure", "Power" and the GPU's name |
| Keychain sheets | "Save" (new login, new SSH key), "Add" (a key's passphrase) |
| Cleaner's Overview | "{size} used", "{size} free", "{size} scanned"; legend "Not scanned", "Free"; "Disks", a disk "{used} of {total}" |
| Cleaner's tables | Space's list "Name", "Size", "Share", "Kind"; "{n} smaller items"; Duplicates' groups "{size} each", "{n} copies", their rows "Folder", "Modified"; Large files and Untouched "Name", "Folder", "Kind", "Last opened", "Size" |
| Files' status bar | "{n} items, {free} available"; with a selection "{n} of {total} selected, {size}" |
| Quick Look's trim | The kept length as a duration: "0:42", "1:02:05" |
| Split View chooser | "No other windows fit" |
| Game controllers | "Steam Input", "Virtual pad from {program}" (virtual pads' names) |
