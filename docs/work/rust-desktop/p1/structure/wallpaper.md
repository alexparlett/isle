# Wallpaper

Batch 1, revised by revision 1. Answers to: `shell.md` Wallpaper and idle dimmer 1–2 and its
"For the design process" item (the change transition); the wallpaper's
part in `shell.md` Lock screen (its backdrop), Widgets (where widgets live)
and `compositor.md` Stacking (what Show Desktop reveals). Decisions B3,
B10, B14, B24.

## Purpose

The picture behind everything on each screen: most of what the user sees
at rest (brief P1), and what the frosted materials take their colour from
(brief P4). It is still and quiet, and it says nothing. It is not a place
for files, widgets or status at rest, and its colours never feed the
palette (v1 D16; `design-v1-lessons.md` §8). The idle dimmer that shares
its `shell.md` section is `lock.md`'s (batch 2).

## Content

| Part | What it is | Data from |
|---|---|---|
| Image | A picture file, scaled to its screen by the fit mode | The user's file or folder, or Isle's bundled set; `settings.kdl` |
| Colour | A solid colour, also the fill around a fitted image | `settings.kdl` |
| Drawn backdrop | Isle's default: drawn from neutral tokens, with a light and a dark form; never from the accent (brief P6) | `tokens.kdl` |
| Softened form | The same picture blurred and darkened, made once per change, for the lock and the widgets surface | Drawn by the compositor from the above |

- **Choice per screen or for all** (`shell.md` Wallpaper 1). A screen is
  keyed by its monitor's identity, not its connector, so a KVM switch or a
  replug brings back its own choice (v1 D56, D76).
- **Fit modes**: Fill screen (crop to cover, the default), Fit to screen
  (whole image, the rest filled with the colour), Centre (actual size on the
  colour). The same image on the 21:9 and 16:9 screens crops differently;
  Fill is cropped about the centre.
- **A folder as the source**, changed on a schedule (every hour, every day,
  at log in), in order or shuffled; the place in the cycle is kept in state.
- **Light and dark variants** of one choice follow the appearance, Auto
  included.
- **Isle's bundled set**: dark and quiet pictures (brief, interview), each
  with a light variant, free to ship. Which ones is stage 4's.
- **Still.** No video, moving or parallax wallpapers: the screen at rest
  draws nothing (`shell.md` acceptance bar 2) and stays calm (brief P1).
  Because it is still, the blurred backdrop that the on-demand menu bar,
  Control Centre and other panels sample over it is computed once per
  change, not per frame.
- **Per screen, not per desktop.** Every desktop on a screen has the same
  wallpaper, so a desktop switch moves windows over a still picture
  (`compositor.md` Spaces 3).
- **Nothing on it at rest**: no file icons, no widgets (For the user 1).

Images are decoded and scaled to each screen's size on the service worker
(B9), never on the main loop, and only the scaled copy and its softened
form are kept.

## States

| State | What shows |
|---|---|
| Rest | The chosen image, colour or backdrop, still |
| Empty: nothing chosen | Isle's drawn backdrop in the current appearance |
| Loading | At session start, the image's mean colour, kept in state from the last decode, fills the screen in the first frame; the image crossfades in once decoded. A change in Settings keeps the old picture until the new one is decoded |
| Error | A file that is missing, unreadable or fails to decode: the last good picture stays if there is one, else the drawn backdrop. Settings › Wallpaper marks the choice as missing; nothing in the shell announces it |
| Folder cycling | Changes on schedule while the screen shows the desktop; a change that falls due while the wallpaper is covered or locked waits until it shows |
| Appearance change | The light or dark variant crossfades in with the appearance |
| Second screen | Its own picture and fit, or the shared choice cropped to its shape |
| Full-screen window or game | Covered. Nothing is drawn and the wallpaper contributes no render element, so direct scanout stays possible (`shell.md` acceptance bar 13). Changes apply without a transition (`shell.md` Wallpaper 2) |
| Windowed game, Game Mode | Shown, still; any change applies at once, no crossfade (`shell.md` Wallpaper 2; `gaming.md` Game Mode 9) |
| Locked | The lock shows the softened form (`lock.md`); the wallpaper itself is not seen |
| Show Desktop | Windows move aside and the wallpaper shows plain (`compositor.md` Stacking 10) |
| Mission Control | Behind the spread (`mission-control.md`, batch 3) |
| Idle dim | Dimmed with everything else (`lock.md`) |
| Focus on | No change |
| Reduce Motion | The crossfade stays, since it moves nothing; a shorter one is stage 4's call |
| Reduce Transparency | No change to the wallpaper; the materials over it turn solid |
| Increase Contrast | No change |
| Largest text size | Not applicable |

## Opening and closing

The wallpaper is there from the session's first frame and is never opened
or closed. A change (Settings, the folder schedule, the appearance) is a
crossfade drawn by the compositor from the old picture to the new, on that
screen only (`shell.md` Wallpaper 2). It is instant in Game Mode and
whenever the wallpaper is covered. Values are stage 4's.

The handover from the greeter or the lock into the session goes from the
softened form to the plain one; its motion is `lock.md`'s.

## Keyboard

The wallpaper never takes keyboard focus and has no keys of its own.
Everything its menu offers is reached elsewhere: Change wallpaper from
Settings › Wallpaper and Spotlight's settings results; the widgets from
their chord (`widgets.md`); Show Desktop from its chord.

## Pointer

| Input | Does |
|---|---|
| Click | Takes focus from the windows; the front app becomes whatever the on-demand menu bar shows with no window focused (`menu-bar.md`). It does not reveal the desktop: macOS's "Click wallpaper to reveal desktop" is a setting, off by default |
| Secondary click | A menu: "Change wallpaper…" (Settings › Wallpaper), "Edit widgets…" (the widgets surface in edit mode) |
| Drag a file onto it | Nothing: the desktop holds no files |
| Scroll | Nothing |

Hot corners sit over the wallpaper and the windows alike; they are the
compositor's (`shell.md` Mission Control 8).

## Shared places

- **The layer above the wallpaper, under windows.** Empty at rest. Show
  Desktop reveals the plain wallpaper there and nothing else (For the
  user 1).
- **The widgets surface** (`widgets.md`, batch 3). Proposed here as the
  dashboard: a full-screen surface on v1's chord, drawn over this sheet's
  softened form rather than over the windows, so windows neither show
  through nor move (For the user 1). Its grid, editor and gallery are
  `widgets.md`'s.
- **The lock and the greeter** (`lock.md`, batch 2) use the softened form
  of the current wallpaper (`shell.md` proposal). The greeter runs before
  the user's session, so the session keeps a copy of the current wallpaper
  where the greeter can read it.
- **Backdrop of the materials.** The on-demand menu bar, Control Centre,
  Notification Centre and other panels blur what is behind them; over the
  desktop that is this surface, sampled once per change. At rest only the
  island sits over it.
- **Files** may offer "Set as wallpaper" on an image (`files.md`, batch 3),
  through the same setting.

## Answers

1. **The change transition** (`shell.md`: a crossfade). **Adopted**: a
   crossfade per screen, drawn by the compositor; instant in Game Mode and
   when covered; kept under Reduce Motion, as a fade moves nothing.
2. **The lock's backdrop** (`shell.md`: a softened form of the current
   wallpaper). **Adopted** as far as this surface goes: it makes the
   softened form once per change; the lock's layout is `lock.md`'s.
3. **Where widgets live** (`shell.md`, B24: Notification Centre's column
   and the desktop under windows with a "Show widgets" chord and "Dim
   widgets on desktop"). **Replaced**, as far as the desktop goes, by the
   dashboard the user kept (brief, interview): see For the user 1.
   Notification Centre's column is `notification-centre.md`'s.
4. **What Show Desktop reveals** (`compositor.md` Stacking, B24: the
   desktop's widgets). **Replaced**: the plain wallpaper only, since no
   widgets live on the desktop (For the user 1).

## Copy

| Where | String |
|---|---|
| Wallpaper menu | "Change wallpaper…", "Edit widgets…" |
| Settings › Wallpaper | "Show on all screens", "Fill screen", "Fit to screen", "Centre", "Change picture", "Every hour", "Every day", "When logging in", "Random order" |
| Missing file | "<file> couldn't be found. Showing the default wallpaper." |

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **The dashboard and desktop widgets: one surface or two?** The brief
   keeps v1's full-screen dashboard on a chord, "alongside or instead of"
   B24's desktop widgets. Recommend one surface: the dashboard, on v1's
   chord, drawn over the softened wallpaper, with no widgets on the
   desktop at rest. Desktop widgets that show whenever the desktop does
   break brief P1 (the screen at rest is the wallpaper and the island);
   "Show widgets" would move every window aside each time, against brief
   P5; on a 3440-wide screen kept full of windows they would
   seldom be seen; and v1 found the dashboard "a place you go to, not
   something seen in passing" (`design-v1-lessons.md` §5). Two homes would
   also mean two layouts to edit. Show Desktop then reveals the plain
   wallpaper. Whether widgets also sit in Notification Centre's column is
   `notification-centre.md`'s question.
