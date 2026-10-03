# Capture

Batch 2. Answers to: `shell.md` Capture (the toolbar's layout, what follows
a shot, the picker's layout and button names); `platform.md` Portal backend
(the picker's layout; where a running cast is shown, confirmed from
`island.md`); `gaming.md` and `compositor.md` (what "screenshot" covers in
the game escape set). B13, B17, B26, B30.

## Purpose

Taking pictures and recordings of the screen, picking a colour off it, and
choosing what an app may share. Four parts: the **pick** (a frozen screen
to choose a region or window from, Shift+Cmd+4), the **toolbar**
(Shift+Cmd+5: every mode and option), the **thumbnail** that follows a
shot, and the **picker** an app's screen-share or screenshot request opens
(B13). The compositor makes every frame, still or stream; the shell draws
the parts above. It is macOS's screenshot tool (`macos-model.md` §3,
Screenshots row) with v1's recording, countdown, audio, colour pick and
annotation (`isle-v1-inventory.md` §4, Capture bar). It is not a gameplay
recorder with a replay buffer (Steam's own does that, `design-references.md`
§11), not an image editor (Markup is Quick Look's, below), and not a
screen-share controller after the share starts (the island's Now rows are).

## Content

**The pick.** Shift+Cmd+4 freezes every screen at that moment: the
compositor keeps each output's last composed frame and the pick draws over
it, so the shot is cut from that frame and the pick's own drawing never
reaches it (`shell.md` Capture 3; inventory §16 Capture). Over the frozen
frame:

- **Region mode** (the default). A light veil over every screen, a
  crosshair pointer, and the selection cut clear of the veil with its size
  beside it in pixels. A region lies on one screen: the screen the drag
  began on.
- **Window mode** (Space toggles). The pointer becomes a camera; the
  window under it is lifted clear of the veil. Pickable: windows, an open
  menu or popover, and a shell panel open at the freeze (Control Centre,
  Notification Centre, a banner, Spotlight). Not the wallpaper or the
  island at rest. A window shot is the window alone, drawn by the
  compositor from the buffers it held at the freeze, so a covered window
  comes out whole; its shadow is kept on a transparent ground, as macOS,
  and Option leaves it out.

**The toolbar.** A bar at the bottom centre of the screen with the
pointer, in the overlay layer; nothing else lives at the bottom edge with
no Dock (`competition.md`). It can be dragged, and opens where it was left.
Leading to trailing:

| Group | Controls |
|---|---|
| Close | Close |
| Shots | Capture entire screen; Capture selected window; Capture selected portion |
| Recordings | Record entire screen; Record selected window; Record selected portion |
| Colour | Pick colour |
| Options | A menu (below) |
| Action | One button named for the mode: Capture or Record; in window mode none, since a click on the window acts |

The modes are one segmented choice across the Shots and Recordings groups.
Record selected window is Isle's addition to macOS's five: the compositor
streams the window's own surface, so a window recorded is the window, not
whatever crosses its rectangle, and it follows the window when moved.

Unlike the pick, the toolbar's screen is live: a countdown and a recording
need the moment to come later. Its shot is composed by the compositor
without the capture tool's parts (the toolbar, veil, selection, countdown
and thumbnail), so the rule of `shell.md` Capture 3 holds without a freeze.
In portion mode the selection is a rectangle with handles over the live
screen, the last one restored when Remember last selection is on.

**Options menu**, the toolbar's state, kept in `settings.kdl`:

| Section | Items |
|---|---|
| Save to | Screenshots and Recordings (the default folders, below), Documents, Clipboard, Other location… |
| Timer | None, 5 s, 10 s |
| Microphone | None, then each input device by name |
| Audio | Record system audio |
| Options | Show floating thumbnail, Remember last selection, Show mouse pointer, Show mouse clicks |

Show mouse pointer is off by default (`shell.md` Capture 3; v1 D27); the
compositor leaves the pointer out because it draws it on the cursor plane
(`compositor.md` Pointer 4). Show mouse clicks applies to recordings.

**Where files go and their names.** Isle shows nothing on the desktop
(`wallpaper.md`), so macOS's Desktop default would hide every shot. The
defaults are v1's folders, so existing files stay together:

| Kind | Folder | Name |
|---|---|---|
| Shot | `~/Pictures/Screenshots` | `Screenshot 2026-10-03 at 14.02.11.png` |
| Recording | `~/Videos/Recordings` | `Screen Recording 2026-10-03 at 14.02.11.mp4` |

macOS's names, with dots in the time (no colon, so the names copy to any
disk) and the user's 24-hour clock (`copy.md` 9). Shift+Cmd+3 with two
screens writes one file per screen, the second ending " (2)", as macOS.
A name already taken in the same second gets the same suffix. The file is
written the moment the shot is taken; the thumbnail is a handle on a file
that already exists, so a crash never loses a shot. The recording's
container and codec are its packet's (v1 wrote MP4); the extension follows
them.

**The thumbnail.** After a shot or a finished recording, one thumbnail in
the bottom-trailing corner of the screen the shot was taken on (macOS
§3.13; `shell.md` Capture 4):

- **Click** opens it in Markup (below).
- **Drag** carries the file out. Dropped on a folder in Files, the shot
  moves there, as macOS. Dropped into an app, the app gets the file and the
  shot stays in its folder too, so nothing is lost to an app that ignored
  the drop.
- **Flick toward the screen's edge**, or leave it, and it goes; the file
  is already saved. Hover holds it.
- **Secondary click**: Save to (the Save to list, moving the file), Copy,
  Show in Files, Markup, Move to Trash, Close.
- **A new shot while one shows** replaces it; the earlier one is already
  saved (For the user 1).
- **A recording's thumbnail** shows its first frame; a click opens it in
  Quick Look, where trimming is `files.md`'s.

**Markup.** Quick Look's markup mode, in the Files process (B26; `apps.md`
Quick Look 21): pen, highlighter, line, arrow, rectangle, ellipse, text,
redact (a box that blurs what it covers), crop, undo, Done and Revert, with
the tools in one row over the image. One markup for shots and for any image
Quick Look shows, as macOS (For the user 2). It replaces v1's satty, which
looked like another desktop (brief P2).

**Colour pick.** From the toolbar or its chord, over the frozen frame
(`shell.md` Capture 6): a loupe at the pointer magnifying the pixels round
it, with the colour's value under it. A click copies the value and the
island shows "Colour copied". The value's format is a setting, hex by
default. A portal PickColor request (`platform.md` Portal backend 6) uses
the same loupe and returns the colour to the app instead.

**Recording.** Starts at once, or after the countdown. While it runs:

- The island's recording activity: the elapsed time as its compact face,
  and in Control Centre's Now row what is recorded, with or without audio,
  and Stop (`island.md`, adopted there). The screen privacy mark shows on
  every screen; the microphone's too when the microphone is recorded.
- Stop is the Now row's button, Cmd+Ctrl+Escape (macOS's), or the toolbar
  opened again: it opens on the running recording's segment with Stop
  recording; a shot segment makes the action Capture, so a shot can still
  be taken; the other record segments are disabled.
- One recording at a time. Stop saves the file and shows the thumbnail
  (`shell.md` Capture 5).
- A recorded window that closes, or a screen that goes (the KVM switching
  away), ends the recording cleanly and keeps what was recorded, with
  `island.md`'s ended event (`shell.md` Capture 8; v1 D55).
- Banners are held while recording, as while sharing (`platform.md` Focus
  3), so a recording does not catch a stranger's message; for
  `notifications.md` to adopt.

**The countdown.** With a timer set, the toolbar goes and the island shows
the seconds left as an OSD-like event: it pre-empts other events, shows
over a full-screen window in the OSD's place, and is never in the shot.
Cmd+Ctrl+Escape, or a click on it, cancels. The pick's frame is not frozen
during a countdown: the point of the timer is to set the screen up.

**The screen-share picker.** Opened by a ScreenCast request through
`xdg-desktop-portal-isle` (B13), drawn by the shell, attached to the
requesting app's window as a sheet as the Open and Save panels are
(`apps.md` File chooser 22; `window-chrome.md` Sheets), or centred on the
screen with the focused window when the request names no window. It closes
no other surface (`platform.md` Portal backend 2). Top to bottom:

| Part | What | Data |
|---|---|---|
| Title | "Choose what to share with {app}", the app's icon beside it | the request's app id and desktop entry |
| Source tabs | Screens, Windows, Portion of screen; only the kinds the request accepts. Portion is a screen stream cropped by the compositor, offered when screens are | the request's source types |
| Grid | One tile per source: a live preview, and under it the screen's name and size, or the window's app icon and title. Windows grouped by app, front to back. Hidden and minimised windows are not listed, since nothing shown can be streamed (`platform.md` Portal backend 5) | the compositor's outputs and window list; previews only while the picker is open (D55) |
| Portion | "Select portion…", which steps the picker aside for the toolbar's portion selection over the live screen, then comes back with it shown | the toolbar's selection |
| Audio | "Share audio", shown only when the request asks for audio | the request |
| Remember | "Let {app} share this again without asking", shown only when the app asks to persist; on by default, since the app asked; revoked in Settings › Privacy & Security › Screen & System Audio Recording (`settings.md`) | restore tokens in state (`platform.md` Portal backend 3) |
| Buttons | Cancel, Share. Share is the default and is off until a source is chosen; with one screen and screens only, that screen is chosen already | |

When the request allows several sources, tiles are checked rather than
selected, and Share names the count: "Share 2 sources". The pointer's
appearance follows the request's cursor mode; the picker offers no switch.

**Where a running share shows.** `island.md`, adopted: the sharing activity
and the screen privacy mark, a Now row per stream naming the app and the
screen or window, each with Stop, which ends that stream from Isle's side
(`platform.md` Portal backend 5).

**A screenshot request.** A portal Screenshot request that asks for
interaction opens the toolbar with only the shot modes and a line naming
the app: "{app} wants a screenshot". Its action button reads Share; the
shot goes to the app, not to a folder, and no thumbnail follows. A request
that does not ask for interaction is the portal packet's policy
(`platform.md` Portal backend, Open); if it asks the user, the question is
`prompts.md`'s form.

## States

| State | What shows |
|---|---|
| Rest | Nothing. |
| Pick, region | Frozen screens, veil, crosshair; the selection and its size while dragging. |
| Pick, window | Frozen screens, veil, the window under the pointer lifted clear, camera pointer. |
| Toolbar open | The bar; in portion mode, the selection; in window mode, the window under the pointer lifted clear on hover; in screen mode with two screens, the pointer becomes a camera and the screen under it is marked (click a screen to take it). |
| Countdown | The island's countdown; nothing else drawn. |
| Recording | The island's activity and privacy marks; nothing else drawn. |
| Thumbnail | The thumbnail in its corner for its duration (stage 4; v1 held capture results 5 to 6 s). |
| Colour pick | Frozen screens without the veil, the loupe at the pointer. |
| Picker | The sheet over the requesting window; previews live. |
| Picker, empty | A tab with no sources reads "No windows"; the Screens tab is never empty. |
| Loading | Picker tiles show the icon or the screen's name until their preview arrives; Share never waits on a preview. The pick and the toolbar wait on nothing. |
| Error | A preview that cannot be had (a protected surface) shows the icon. A file that cannot be written: the shot goes to the clipboard instead and a banner says so, "Couldn't save the screenshot. “{folder}” is full. It was copied instead." A recording that cannot start: a banner, "Couldn't start recording. {reason}". The picker is cancelled if the app goes, and a source that goes while the picker is open drops out of the grid. |
| Many sources | The picker's grid scrolls; typing filters the windows by app or title. |
| Second screen | Both screens freeze and are pickable; a region stays on one. The toolbar opens on the screen with the pointer; the thumbnail on the screen shot. Shift+Cmd+3 writes a file per screen. The picker lists every screen; the island's activity shows on the main screen, the privacy marks on both (`island.md`). |
| Windowed game in front | As normal, in solid forms and without transitions (`competition.md` For the user 1). |
| Full-screen game in front | The capture chords are in the escape set (B30; Keyboard). Shift+Cmd+3 shoots with nothing drawn; no thumbnail; the island acknowledges with "Screenshot saved" in the OSD's place, with the screenshot sound if sounds are on. Shift+Cmd+4 and the toolbar draw over the game without motion or blur, take the keys and release the game's pointer lock or confinement until they close, then give both back (as the switcher, `switcher.md`). A recording of a game is allowed; its cost beside the game is the recording packet's to measure. |
| Full-screen app, not a game | As normal; the thumbnail shows (macOS). |
| Under shortcuts-inhibit | The capture chords go to the inhibiting client, as every chord but the escape chord does (`competition.md` Keys). The toolbar is still reachable from Spotlight and Control Centre with the pointer. |
| Locked | Capture chords do nothing; the lock owns the keys. A running recording or share keeps going: one of a whole screen carries the lock screen as drawn, never the session behind it; one of a window holds that window's last frame until unlock (`lock.md`, as macOS). A picker request waits until unlock. |
| Focus on | Unchanged. The screenshot sound follows only its own switch: it answers the user's key, as the volume tick does. |
| Reduce Motion | The veil, toolbar and thumbnail appear and go by a fade; a flicked thumbnail fades instead of travelling. |
| Reduce Transparency | The toolbar, thumbnail and picker in solid forms; the veil is unaffected (it is a dim, not a material). |
| Increase Contrast | An edge on the toolbar, thumbnail and picker; the selection's and the lifted window's outline take their stronger form. |
| Largest text size | The picker's titles and the size readout grow; the toolbar is icons with tooltips and keeps its size; the Options menu grows as menus do. |

## Opening and closing

- **The pick** appears at once on the chord: the freeze is instant, the
  veil fades in quickly. It closes at once on the shot or Escape, faster
  than it came (brief P5).
- **The toolbar** fades in place at its spot; no slide up from the edge.
  It closes on Close, Escape, a shot in window mode, Capture or Record, or
  another overlay surface opening.
- **The thumbnail** fades in at its corner, in place, scaling from
  `motion-panel-scale` about that corner; a flick sends it
  out under the pointer; otherwise it fades out. Exits are faster.
- **The picker** opens as a sheet does (`window-chrome.md`, grows down
  from its anchor), and closes on Share, Cancel, Escape, or its window
  closing (answering cancel, so no app waits; D67).
- **Over a game** nothing moves or blurs (`gaming.md` Game Mode 9).

## Keyboard

| Action | Key |
|---|---|
| Shot of the screen | Shift+Cmd+3 |
| Pick a region or window | Shift+Cmd+4 |
| Open the toolbar | Shift+Cmd+5 |
| The same, to the clipboard instead of a file | add Ctrl; the island shows "Screenshot copied" |
| Pick a colour | Shift+Cmd+6 |
| Stop a recording, or cancel a countdown | Cmd+Ctrl+Escape |
| On a PC-layout keyboard, also | Print for Shift+Cmd+4, Shift+Print for Shift+Cmd+3, Ctrl added to copy |

In the pick:

| Action | Key |
|---|---|
| Switch region and window mode | Space (before a drag) |
| Move the crosshair; with Shift, ten pixels | arrows |
| Set the selection's first corner, then size it with the arrows; take it | Return, arrows, Return |
| While dragging: lock an axis; grow from the centre; move the selection | hold Shift; hold Option; hold Space |
| Step through pickable windows, front to back | Tab, Shift+Tab |
| Take the window without its shadow | Option with the click or Return |
| Cancel | Escape |

In the toolbar: Tab and Shift+Tab move between groups, the arrows between
controls in a group; Return or Space acts; the Options menu opens on Return
or Space and works as any menu. In portion mode the arrows move the
selection and Option with the arrows resizes it from its trailing edges
(GNOME's, `design-references.md` §11); Return captures or records; Escape
closes. In the colour pick, arrows move the loupe a pixel (Shift, ten),
Return copies, Escape cancels.

In the picker: Tab moves between the tabs, the grid, the options and the
buttons; Left and Right switch tabs, arrows move in the grid, Space checks
a tile when several are allowed; typing filters; Return shares; Escape
cancels. Focus is the sheet's: it holds its window's focus and nothing
else's.

The thumbnail never takes focus, so typing continues where it was. Its
actions are all reached without it: the file is in its folder, Files and
Spotlight open it, Quick Look's Markup edits it, and Show floating thumbnail
off saves without one.

The four capture chords and Cmd+Ctrl+Escape are taken by the compositor
before Cmd translation, so apps never see them, and they are what B30's
"screenshot" means in the escape set. Shift+Cmd+6 is not in the escape set.

## Pointer

- **Pick:** drag a region; click a window in window mode; nothing on hover
  beyond the crosshair or the lifted window. Hot corners do not fire and
  the top edge reveals nothing while it is open.
- **Toolbar:** click a mode, then Capture or Record (a click on a window
  in window mode acts at once); drag the bar's handle to move it; drag the
  selection's body or handles. Hover shows each control's tooltip with its
  name and chord. No secondary click.
- **Thumbnail:** hover holds; click opens Markup; drag carries the file;
  flick toward the edge dismisses; secondary click the menu above.
- **Colour pick:** the loupe follows; click copies.
- **Picker:** click a tile to choose it (Cmd-click or its check when
  several are allowed); double-click shares that one source; scroll
  scrolls the grid. No hover opens anything, so no guards are needed.

## Shared places

| Contender | Proposal |
|---|---|
| Overlay, one at a time | The toolbar and the colour pick are overlay surfaces with Spotlight, Mission Control, the switcher and the session dialog: opening one closes the open one (`competition.md`). **Adds:** the freeze pick (Shift+Cmd+4) closes nothing. It draws over a still frame and gives focus back, so a shot can show Spotlight, Mission Control or an open Control Centre. |
| Bottom centre | The toolbar. Nothing else claims it with no Dock; it sits above windows. |
| Bottom-trailing corner | The thumbnail sits in the top layer above a picture-in-picture window there and does not move it, since it is gone in seconds; whether PiP steps aside is the PiP sheet's (batch 3). Settles `competition.md`'s open row. |
| The island | Carries the recording and sharing activities, the countdown and the clipboard's "Screenshot copied" (`island.md`). **Adds** two OSD-class events that pre-empt and show over a full-screen window: the countdown, and "Screenshot saved" when a shot is taken over a full-screen game. Like an OSD they answer a key pressed now (`island.md` The queue 7). |
| Banners | Held while recording, as while sharing; for `notifications.md`. |
| The picker against other surfaces | A sheet on the app's window: it closes nothing and holds only its window's focus (`competition.md` focus 6). A system prompt goes over it; the lock over both. |
| Keys | The four capture chords and Cmd+Ctrl+Escape are the desktop's, before Cmd translation; in the game escape set; given up under shortcuts-inhibit. Shift+Cmd+6 is the desktop's and goes to a game in front. v1's Super+Shift+R and Super+Shift+P would now be Shift+Cmd+R and Shift+Cmd+P, a browser's hard reload and an editor's command palette, so neither is kept. |

## Answers

- **Capture, the toolbar's layout** (`shell.md`; proposal: unstated).
  macOS's bar at bottom centre, the six modes as one choice, Options as a
  menu, an action button named for the mode, with Pick colour added and
  Record selected window as Isle's sixth mode. The bottom is free with no
  Dock.
- **Capture, what follows a shot** (proposal: a thumbnail in a corner,
  click to annotate, drag to drop, left to save). **Adopted**, macOS's one
  thumbnail at the bottom-trailing corner, with the file written at once
  rather than on timeout, a drop into an app keeping the file, and no
  thumbnail over a full-screen game. CleanShot's stack and OCR are not
  taken, its redaction is, as a Markup tool (For the user 1).
- **Capture, annotation** (`shell.md` Capture, v1's satty). **Replaced**
  by Markup in Quick Look (For the user 2).
- **Capture, the picker's layout and button names** (proposal: Share,
  Cancel). **Adopted**: a sheet on the requesting window with Screens,
  Windows and Portion of screen tabs, live tiles, audio and remember only
  when asked; Share names the count when there are several.
- **Capture, the recording timer and Stop** (`island.md`, adopted there).
  Confirmed, with Cmd+Ctrl+Escape as the stop chord, as macOS. It has no
  other owner: Force Quit is Cmd+Option+Escape, and the inhibit escape
  chord is still open in `compositor.md`, which must not take it.
- **Capture, where shots go** (`shell.md` Capture 2, "where to save"). v1's
  folders by default, macOS's names, the Options menu's Save to.
- **Capture, region and window pick** (`shell.md` Capture 1, 3). macOS's
  interaction on a frozen frame (Space for window mode, Shift and Option
  while dragging), with GNOME's keyboard selection so nothing is pointer
  only (`design-references.md` §11).
- **Capture, chords from v1** (inventory §11; `compositor.md` Shortcuts:
  every v1 action keeps a chord). Capture: the Mac chords, with Print on
  PC keyboards. Record: Shift+Cmd+5 opens on the last mode, so Return
  starts a recording; no chord of its own, since Shift+Cmd+R is an app's.
  Colour pick: Shift+Cmd+6 (macOS's Touch Bar shot, unused here) instead of
  Shift+Cmd+P.
- **Portal backend, the picker's layout** (`platform.md`). As above.
- **Portal backend, where a running cast is shown** (proposal: the island
  with Stop). **Adopted** in `island.md`, Stop per stream.
- **The escape set's "screenshot"** (B30; `gaming.md` Game Mode 2,
  `compositor.md` Shortcuts 7). The four capture chords, with Ctrl, Print
  on a PC keyboard, and Cmd+Ctrl+Escape.

## Copy

| Where | String |
|---|---|
| Toolbar (accessible names and tooltips) | "Close", "Capture entire screen", "Capture selected window", "Capture selected portion", "Record entire screen", "Record selected window", "Record selected portion", "Pick colour", "Options" |
| Toolbar action | "Capture", "Record", "Stop recording"; for a portal request, "Share" |
| Options menu | "Save to", "Screenshots", "Recordings", "Documents", "Clipboard", "Other location…"; "Timer", "None", "5 s", "10 s"; "Microphone", "None", device names; "Record system audio"; "Show floating thumbnail", "Remember last selection", "Show mouse pointer", "Show mouse clicks" |
| Pick | the size, "1280 × 720" |
| Thumbnail menu | "Save to", "Copy", "Show in Files", "Markup", "Move to Trash", "Close" |
| Markup (accessible names and tooltips) | "Pen", "Highlighter", "Line", "Arrow", "Rectangle", "Ellipse", "Text", "Redact", "Crop", "Undo", "Colour"; swatches "Red", "Orange", "Yellow", "Green", "Blue", "Purple", "Black", "White" |
| Island events (added to `island.md`'s) | "Screenshot copied", "Screenshot saved", the countdown's seconds |
| Picker | "Choose what to share with {app}"; "Screens", "Windows", "Portion of screen"; "Select portion…"; "Share audio"; "Let {app} share this again without asking"; "No windows"; "Cancel", "Share", "Share 2 sources" |
| Screenshot request | "{app} wants a screenshot" |
| Errors (banners) | "Couldn't save the screenshot. “{folder}” is full. It was copied instead.", "Couldn't start recording. {reason}" |
| Files | "Screenshot {date} at {time}.png", "Screen Recording {date} at {time}.{ext}" |

"Markup" and "Screen Recording" keep macOS's words (`copy.md` 2); the
file names keep macOS's case. Words follow `copy.md`.

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **What follows a shot.** (a) macOS's one thumbnail: a new shot replaces
   it, the earlier one already saved; redaction as a Markup tool; no OCR.
   (b) CleanShot's stack: recent shots pile up in the corner, each with
   copy, save, Markup and drag, plus OCR ("Copy text") and automatic
   redaction (`design-references.md` §11). **Recommended: (a).** A stack is
   a pile in the corner (CleanShot's own cost) against calm at rest (brief
   P1), and every shot is already a file in its folder. OCR needs an engine
   Isle would ship and tune; it is better added later as macOS's Live Text,
   a "Copy text" in Quick Look for any image, than built into capture.
2. **Annotation.** (a) Markup built into Quick Look in the Files process:
   one set of tools for shots and any image, in Isle's look, as macOS. (b)
   An external tool as v1 (satty), launched from the thumbnail. **Recommended:
   (a).** satty looks like another desktop (brief P2, the interview's
   dislike), and Quick Look already exists to host it (B26). Any external
   editor stays reachable through Open With.
