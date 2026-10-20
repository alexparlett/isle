# Files, 2026

Evidence for P15 (`roadmap.md`, Files): the FileChooser portal, the file
manager interface, thumbnails, listing and watching at scale, mounts without
GIO, prior art, Quick Look and Markup, and v1's browsing engine. Read
2026-10-20 from specifications, sources and this host (CachyOS, `/` and
`/home` on btrfs, shared-mime-info 2.5.1, libarchive 3.8.10,
ffmpegthumbnailer 2.3.1, tumbler 4.20.2, no pdfium). Crate versions from
crates.io on the same day. Sections are cited from the plan as §n.

## 1. The FileChooser backend

`org.freedesktop.impl.portal.FileChooser`, version 4 in the frontend
(`file-chooser.c`; `directory` came in 3). `OpenFile`, `SaveFile` and
`SaveFiles` take `(o handle, s app_id, s parent_window, s title, a{sv}
options)` and return `(u response, a{sv} results)`: 0 success, 1 cancelled,
2 other.

| Option | Type | Methods | Note |
|---|---|---|---|
| `accept_label` | `s` | all | may carry a mnemonic underscore |
| `modal` | `b` | all | default true |
| `multiple` | `b` | OpenFile | dropped from SaveFile in 1.21.1 |
| `directory` | `b` | OpenFile | choose folders |
| `filters` | `a(sa(us))` | Open, Save | name and (0 glob, 1 MIME) rules; `image/*` allowed (1.21.1) |
| `current_filter` | `(sa(us))` | Open, Save | may be absent from `filters` |
| `choices` | `a(ssa(ss)s)` | all | id, label, options, initial; no options is a checkbox of "true"/"false" |
| `current_folder` | `ay` | all | NUL-terminated |
| `current_name` | `s` | SaveFile | |
| `current_file` | `ay` | SaveFile | the file being saved again |
| `files` | `aay` | SaveFiles | the names to save |

Results: `uris` (`as`, `file://` only; the frontend drops others),
`choices` (`a(ss)`), `current_filter`, `writable` (OpenFile). SaveFile
returns one URI; SaveFiles one per name, in order.

- **Sandboxed apps.** The frontend registers each returned URI with the
  document portal and resolves document paths in `current_folder` and
  `current_file` back to host paths before the backend sees them. It
  treats a missing `writable` as writable, against the spec's default, so
  the backend should always send it.
- **`parent_window`.** `x11:<hex>`, `wayland:<xdg-foreign v2 handle>` or
  empty. A backend imports the handle and calls `set_parent_of`.
- **Recent changes.** 1.21.1 (2026-04) fixed the chooser opening the parent
  of `current_folder`; 1.22.0 (2026-06) improved document path resolution.
- **Prior art.** Nautilus serves the chooser itself; COSMIC links
  cosmic-files into its portal as a library. xdg-desktop-portal-cosmic
  logs `parent_window` without importing it and never sends `writable`.

## 2. FileManager1

Bus name and path `org.freedesktop.FileManager1`,
`/org/freedesktop/FileManager1`: `ShowFolders(as uris, s startup_id)`,
`ShowItems` (open each parent, select the item), `ShowItemProperties`. On
Wayland the startup id is an xdg-activation token.

- **Callers.** Chromium and Electron's `showItemInFolder` try the portal's
  `OpenURI.OpenDirectory`, which itself calls `ShowItems` with the token,
  then `ShowItems` directly; Firefox calls it and falls back to opening the
  folder (bug 1772063); KDE apps through KIO. `xdg-open` does not use it:
  folders go to the `inode/directory` handler.
- **On this host** nothing owns or activates `FileManager1`; v1 is the
  `inode/directory` handler only (`isle-files.desktop` in
  `~/.config/mimeapps.list`).
- **cosmic-files** spawns itself and waits on the child inside the handler,
  ignores the token and selects nothing.

## 3. Thumbnails

**Spec 0.9.0.** `$XDG_CACHE_HOME/thumbnails/{normal 128, large 256, x-large
512, xx-large 1024}/md5(uri).png`; `Thumb::URI` and `Thumb::MTime`
required, `Thumb::Size` checked when present; failures under
`fail/<app>-<version>/`; directories 0700, files 0600; written to a
temporary name and renamed; nothing is cached for files inside the cache
or for images under 128. A 2x display reads `large` for 128-point tiles.

**tumbler** (`org.freedesktop.thumbnails.Thumbnailer1`, activatable here):
`Queue(uris, mimes, flavor, scheduler, unqueue) → handle`, `Dequeue`,
`Ready` and `Error` signals. Many formats for nothing, but a GLib C daemon
running unsandboxed plugins, a D-Bus round trip per batch and no priority
by viewport. v1 removed it (D85).

**`.thumbnailer` entries** (`/usr/share/thumbnailers`): `TryExec`, `Exec`
with `%i`/`%u` in, `%o` out, `%s` size, and `MimeType`. This host has
ffmpegthumbnailer (video, audio covers), glycin (HEIF, JXL, SVG), gsf-office
and exe.

**cosmic-files**: built-in `image`, jxl-oxide and SVG, then `.thumbnailer`
entries, behind a semaphore of min(cpus, 4) and a memory budget per image,
with fail markers. Its issues: parallel decoding of large JXLs pinned the
CPU (#501, fixed by the semaphore and budgets, #1377); no cache meant 40%
CPU on every revisit (#707).

## 4. Listing, watching, MIME, copying

- **Listing.** Stream `getdents` in batches; `d_type` before `statx`; count
  a folder's children off the listing path. cosmic counts every
  subfolder's children synchronously and freezes on 10k entries (#1287,
  #1059, #449).
- **Watching.** One inotify instance for the process, non-recursive watches
  on the folders shown (cosmic #429: a watcher per tab ran out of file
  descriptors). Limits are per user (`max_user_watches` 524288,
  `max_user_instances` 1024 here). `IN_Q_OVERFLOW` means a rescan; rename
  cookies do not always pair; network filesystems send nothing. fanotify
  gives a file manager nothing inotify does not, unprivileged.
- **Collation.** `icu_collator` 2.3.1 with numeric ordering gives natural,
  locale-aware order.
- **MIME.** `xdg-mime` (ebassi; crates.io has 0.4.0 of 2024, cosmic pins
  git) reads shared-mime-info's globs and magic. On a remote filesystem,
  by name only. `mime_guess` 2.0.5 as an extension fallback.
- **Copying.** On one filesystem `FICLONE` first (btrfs here; across
  subvolume mounts since 5.18), then `copy_file_range` in chunks for
  progress, then read and write; afterwards mode, times and `user.*`
  xattrs. A move is `rename`; across devices a copy and unlink. `rustix`
  1.1.5 has both calls.
- **Trash.** Spec 1.0: `$XDG_DATA_HOME/Trash`, `$topdir/.Trash/$uid` when
  sticky and not a link, else `$topdir/.Trash-$uid`; the info file first
  with `O_EXCL`; `Path` relative under a topdir. The `trash` crate 5.2.9
  deletes correctly, but its `list()` and `is_empty()` walk every mount,
  which froze cosmic for minutes and probed CIFS shares continually
  (#1741, #1986).
- **Tags and comments.** `user.xdg.tags` and `user.xdg.comment` (KDE's
  kfilemetadata), through `xattr` 1.6.1; lost on FAT, exFAT and many FUSE
  and NFSv3 mounts.
- **Archives.** `compress-tools` 0.16.1 over libarchive, or `bsdtar`
  as a process; `bsdtar` refuses absolute and `..` paths unless `-P`.

## 5. Mounts without GIO

- **UDisks2** 2.11.2 over zbus (Isle is on zbus 5.19; zbus 6.0 is out but
  ashpd and udisks2 still need 5): `GetManagedObjects` and its signals;
  `Filesystem.Mount` (to `/run/media/$USER`), `Unmount`, `Encrypted.Unlock`,
  `Drive.Eject`, `PowerOff`; honour `HintIgnore`, `HintSystem`, `HintName`.
  Internal disks and LUKS ask polkit.
- **A stale network mount** puts the calling thread in uninterruptible
  sleep; it cannot be cancelled. So: no `stat`, `statfs` or `read_dir` on
  the UI thread; probes per mount on a blocking pool with a deadline and
  one outstanding per mount; mounts learned from `/proc/self/mountinfo`
  (POLLPRI; reading it triggers no automount); the filesystem type
  classifies a mount as remote (cifs, nfs, `fuse.sshfs`, `fuse.rclone`,
  `fuse.gvfsd-fuse`); `AT_NO_AUTOMOUNT` and `AT_STATX_DONT_SYNC` on
  `statx`; autofs points untouched until opened (cosmic #1512, #2034,
  open). On remote mounts by default: no sniffing, no thumbnails, no child
  counts.
- **gvfs** is mounted here at `/run/user/1000/gvfs`. Listing through its
  FUSE is slow (cosmic #1003: 25 s against 75 ms through GIO for 1,300
  items) and MTP writes fail (#1782). Mounting needs GIO.
- **v1's own record** (removed D64, D65): every freeze came from listing,
  never reading; 6,897 photos on an rclone mount took 81 s in
  uninterruptible sleep. The fix that worked was never to let a listing or
  stat on the UI path wait on a FUSE or network mount.

## 6. Prior art

- **cosmic-files** (Rust, iced; read at `0f15f44`, 2026-10-12). One crate
  builds the app, the chooser dialog its portal links, and the applet that
  serves FileManager1; operations with a pause and cancel controller and
  Replace, Keep Both, Skip and Cancel with apply to all; a gvfs-only
  mounter in process. Its lessons are §3, §4 and §5's issue numbers;
  remote thumbnails, first turned off outright, are wanted as a setting
  (#1150, #1341).
- **Nautilus**: GIO for everything, tracker for search, bubblewrapped
  thumbnailers, the chooser served by itself.
- **Dolphin**: KIO workers out of process, KDirWatch with polling for
  network filesystems, thumbnail plugins out of process, tags as xattrs
  indexed by Baloo.
- **Finder**: FSEvents, Spotlight, tags in an xattr, Quick Look extensions
  sandboxed out of process.

Shared: thumbnail queues driven by what is on screen, I/O off the UI
thread or process, remote places a slower class.

## 7. Quick Look and Markup

- **GNOME Sushi** 51.2 is its own process at `org.gnome.NautilusPreviewer2`:
  `ShowFile(uri, window_handle, close_if_shown, activation_token)`,
  `Close`, `ParentHandle` and `Visible`, and `SelectionEvent(direction)` so
  the arrows move the file manager's selection. Viewers: image, PDF,
  office through PDF, text, audio, video, font, HTML.
- **Freya** (the fork at `ce0e9fe`): `freya-video` plays through
  `ffmpeg-sidecar` (ffmpeg as a process) and rodio; `freya-code-editor`
  highlights with tree-sitter, one compiled grammar per language.
  `launch_window`, `close_window` and `focus_window` give an app many
  windows; `DragZone` and `DropZone` are inside one window; winit reports
  no Wayland drag and drop.
- **Highlighting.** `syntect` 5.3.0 with `two-face` 0.5.2 (bat's syntaxes)
  covers many languages cheaply; cosmic caps a preview at 256 KiB.
- **PDF.** `hayro` 0.8.0, pure Rust, MIT or Apache, experimental (no
  unembedded CID fonts); `pdfium-render` 0.9.4 needs `libpdfium.so`, not
  packaged here; `mupdf` is AGPL; `poppler-rs` GPL with GLib and cairo.
- **Video frames.** `ffmpeg-next` 9 follows each FFmpeg major; a process is
  safer.
- **Apple's Markup** (macOS 27): selection tools, Sketch, Draw, shapes with
  Loupe and Highlight, Text, Sign, styles, Rotate, Crop, Image Description.
  Edits are saved in place; Revert discards them before Done, and from
  Quick Look they cannot be undone after the window closes. Isle's tool set
  is `capture.md`'s, and Done keeps the original in the Trash.

## 8. v1's browsing engine

`native/files/` (C++, Qt 6, about 4.6k lines) and `shell/windows/files/`
(QML, about 3.4k), identical on this branch and `main`.

- **Listing.** `opendir` and one `statx` per entry on a pool thread, a
  second for links; returned whole, not in batches, despite D66. A
  generation counter drops stale scans and searches (67ee83d). Sort keys
  collated once per row: 50k entries from 365 ms to about 70 ms, 50 of it
  the kernel (f46b634). Every change was a model reset; the view restored
  its scroll by hand (d56632b).
- **Watching.** One inotify descriptor per folder model, the shown folder
  only; `IN_ATTRIB` dropped because reading a file caused rescans
  (f4e5dfe); 150 ms debounce, then a whole rescan. Folders opened in
  place, columns and gatherings were not watched.
- **Blocking calls on the UI thread** that remain: type sniffing for
  extensionless rows, `isAppImage`, `isDir` in bindings, folder size, the
  first scan of a folder opened in place, `QStorageInfo` for every drive.
  A stalled mount would freeze the window.
- **Thumbnails** (D68). The shared cache at 128 only, validity read from
  the PNG's text chunks without decoding (Qt's reader returned nothing
  until decoded, so everything looked stale, bfe83be); a pool of two to
  four; failures remembered in memory by mtime; video through `ffmpeg -ss 3`
  with a 10 s timeout; no PDF; Grid only.
- **Jobs.** A thread per job; conflicts block the thread on a condition
  until answered, and a conflict with no sheet to show it hung forever
  (a470fed). Copies staged and renamed in; one step of undo, partial jobs
  included, a replace never undone (D70, D71). A failed job recorded no
  undo (4266362).
- **The chooser** (D67). Held replies, a queue showing one at a time, a
  cancel from the destructor. Not supported: `choices`, `modal`,
  `parent_window`, the Request object (an app's Close did nothing). MIME
  filter rules were passed as globs and hid every file; saving never
  asked before replacing, nor changed the extension with the format; the
  click that opened it could close it (2ad2b4b); a folder chooser returned
  the parent (0ad07cc).
- **Smaller lessons.** Drags started by hand past the drag distance; the
  selection held by name across rescans; a btrfs device is mounted once
  per subvolume, so the root is the place shown and `/boot`, `/var` and
  swap are hidden (c75cdce); `xdg-open` handed scripts and AppImages to the
  browser, so text falls back to the editor and an unknown type asks
  (1c5df32); a file named `$(...)` reached a shell (a470fed).

## Sources

- https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.impl.portal.FileChooser.html
- https://flatpak.github.io/xdg-desktop-portal/docs/window-identifiers.html
- https://github.com/flatpak/xdg-desktop-portal/blob/main/NEWS.md
- https://github.com/flatpak/xdg-desktop-portal/blob/main/desktop-portal/file-chooser.c
- https://github.com/pop-os/xdg-desktop-portal-cosmic/blob/master/src/file_chooser.rs
- https://www.freedesktop.org/wiki/Specifications/file-manager-interface/
- https://chromium.googlesource.com/chromium/src/+/HEAD/chrome/browser/platform_util_linux.cc
- https://bugzil.la/1772063
- https://specifications.freedesktop.org/thumbnail/latest/
- https://gitlab.xfce.org/xfce/tumbler/-/blob/master/tumblerd/tumbler-service-dbus.xml.in
- https://specifications.freedesktop.org/trash/latest/
- https://github.com/Byron/trash-rs/blob/master/src/freedesktop.rs
- https://man7.org/linux/man-pages/man7/inotify.7.html
- https://invent.kde.org/frameworks/kfilemetadata/-/blob/master/src/usermetadata.cpp
- https://github.com/pop-os/cosmic-files (issues #429, #449, #501, #707, #1003, #1059, #1150, #1287, #1341, #1377, #1512, #1741, #1782, #1986, #2034)
- https://github.com/storaged-project/udisks/releases
- https://gitlab.gnome.org/GNOME/sushi/-/blob/main/data/org.gnome.NautilusPreviewer2.xml
- https://support.apple.com/guide/mac-help/mark-up-files-mchl1fd88863/mac
- https://github.com/LaurenzV/hayro
- `main:docs/DECISIONS.md` D64–D76, D82, D85; `git show 91ee9e8:docs/DECISIONS.md`
  for D64 and D65.
