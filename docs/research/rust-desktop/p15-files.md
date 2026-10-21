# P15: Files' measures, the VM, and the host run

## Headless

`cargo test -p isle-compositor --features test-hooks --test files_bars`
against `target/debug/isle-files`, on a headless Isle with a private system
bus (no UDisks2):

| `apps.md` bar | Check | Result |
|---|---|---|
| 3 Nothing when closed | ⌘W on the last window, then the process | gone, well within 5 s |
| 4 Responsive under failure | Go › Home with no UDisks2 | the command taken at once; Locations lists the computer and the Trash |
| 7 Containment | `kill -9` while a panel holds a FileChooser request | the request answers cancel (the portal sees Files leave the bus); the shell answers; Files opens again |
| 9 Large folder | a folder of 50,000 empty files | first batch drawn at 0 ms, the listing done at 1,480 ms (debug build) |

Bars 1 and 2 are the host's. `tests/files.rs` (17) drives the window, the
views, thumbnails, jobs, rename, clipboard and drag, Open With, volumes,
search, Get Info, the settings window, the panels through a fake frontend
(Open, Save, SaveFiles' folder, cancel, a sheet on a foreign window), Quick
Look from Files and from Spotlight (its float and keyboard), Markup and
Trim.

## Memory, headless

One sample, debug build, Files idle 30 s with one window at Home on a
headless Isle, `smaps_rollup` (`files_bars::idle_memory`, ignored, run by
name):

| Pss | Pss_Anon | Pss_File | Private_Clean | Private_Dirty | Rss |
|---|---|---|---|---|---|
| 87.2 MB | 21.0 MB | 61.5 MB | 55.1 MB | 21.0 MB | 131.4 MB |

Over bar 2's 80 MB in a debug build, whose code pages are several times a
release build's (Settings' debug build measured 59.5 MB with 45 MB of file
pages; Files links far more: syntect, resvg, image, freya-video and its
audio). The anonymous 21 MB is twice Settings'; the release build on the host
settles it.

## The VM

In the guest (llvmpipe, 2560 by 1080), the workspace's debug build from
`/repo`, Isle's session on tty1, Files' desktop entry and D-Bus activation
files installed for the user pointing at `/repo`, `isle-portals.conf`
giving FileChooser to Isle:

- **A FileChooser request with Files not running.** Through the real
  `xdg-desktop-portal` frontend: Isle's backend called `Files1.Choose`,
  D-Bus activation started `isle-files --gapplication-service`, which
  opened the panel alone (no browser window). Escape answered cancel; with
  its last window gone Files exited (bar 3). A folder request with
  `current_folder` `/repo/data` opened there and Return answered
  `file:///repo/data`.
- **Found and fixed: a panel orphaned by a quick Close.** A caller that
  leaves at once (`gdbus call` exits after its reply) makes the frontend
  close the Request; the portal sent `Cancel` before the panel's window
  existed, `AppHandle::close` ignored a window with no id yet, and the
  panel stayed up answering nothing. A window closed before it is created
  now closes as it opens.
- **Found: the session bus reads a user's service files only at its
  start.** Installed under `~/.local/share/dbus-1/services` after login,
  the activation failed with `ServiceUnknown` until `ReloadConfig`; the
  package installs them under `/usr/share`, read before the session.
- **Firefox** (`widget.use-xdg-desktop-portal.file-picker` on): ⌘O raised
  Isle's panel as a sheet on Firefox's window through its xdg-foreign
  handle, the window dimmed under it, with Firefox's own All Files filter
  in Show:. Escape answered cancel. The frontend passes an empty app id
  for an unsandboxed app, so every host app shares one remembered folder
  and view.
- **Show in folder** as Chromium sends it (FileManager1 `ShowItems` from
  another process): a Files window at `/repo/data/apps` with the item
  selected.
- **A loop image** (ext4, `udisksctl loop-setup`): the drive notice posted
  as Files ("VMSTICK connected"); VMSTICK listed dimmed in Locations; a
  click mounted it through UDisks2 in the session and showed it with its
  Eject button; Eject unmounted and detached it, and the tab on it moved
  to Home.
- **Quick Look** from Files on a shell script: highlighted, floating over
  the window, which kept the keyboard and its accent selection; Space
  closed it. "Open with kitty URL Launcher" named the guest's default for
  `text/x-shellscript`, which `kitty-open.desktop` claims.

Not run in the guest: Zed, a GTK 4 app and an Electron app through the
panel (none installed), an encrypted image (no `cryptsetup` image prepared),
Spotlight's ⌘Y and the capture thumbnail's Markup (both checked headless).

## The host run, the user's

Not done. What it settles:

1. Bar 1: Files' first complete frame within 300 ms of launch, warm, and
   the panel on screen within 300 ms of a portal request with Files not
   running, on the release build.
2. Bar 2: PSS, private memory and VRAM after 30 s idle for the release
   build, alone and with the usual apps open.
3. Bar 9's scrolling: no dropped frame scrolling a 50,000-entry folder
   while it fills, at the screens' scale, in software.
4. A day of use: copies across the drives, the Trash, a USB stick and an
   encrypted one, the panels from Firefox, Zed, Steam and an Electron app,
   Quick Look and Markup on screenshots.
