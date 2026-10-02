# CLAUDE.md

Isle: a purpose-built, macOS-style desktop for the user's own Arch Linux
(CachyOS) machine, used mostly for gaming and coding.

On `feature/rust-desktop` it is being rebuilt as its own Smithay compositor
with the shell drawn in Freya (the rust-desktop program). Isle v1, a shell
for Hyprland in Quickshell (`shell/`, `hypr/`, `plugins/`, `native/`,
`theme/`), is still in the tree and on `main`, and is the daily desktop
until the new one replaces it.

## Working here

- Read `docs/ARCHITECTURE.md` for how the new Isle is put together,
  `docs/design/` for what each area does, `docs/DESIGN.md` for how things
  look and `docs/DECISIONS.md` for why. Keep them current in the change
  that makes them stale. v1's own docs are on `main`.
- The program's working state is `docs/work/rust-desktop/`: `state.md`
  first, `roadmap.md` for the build packets, `brainstorm.md` for how
  decisions were reached. Evidence is in `docs/research/rust-desktop/`.
- `spikes/` is throwaway code behind a decision; do not build on it.

The notes below are about v1.

- The desktop runs from `~/.local/share/isle`, a copy that `tools/install.sh`
  makes from this checkout. Editing here changes nothing on the host until
  the user runs it; the VM mounts this checkout directly.
- Verify in the VM, not by reasoning: `dev/test-vm.sh` boots the guest
  with the repo at `/repo`, `dev/guest.sh` runs a command in the
  session, `dev/test-vm.sh shot` takes a screenshot. `dev/` is the
  maintainer's local tooling and is git-ignored; never stage it. Restart the shell with
  `pkill -x qs`, which matches the process name: a `-f` pattern is matched against
  every command line including the one running the kill, so `pkill -f quickshell/isle`
  kills the shell that is doing the killing. Check it actually restarted
  (`ps -eo etime,args | grep '[q]s -p'`) before reading anything: a stale process
  serves an old compiled module, and every screenshot, every log line and every
  "not a function" from it is a lie. `isle-session` restarts `qs` on its own, so
  killing only `qs` is enough; killing the runner as well leaves nothing running.
  `dev/vm-qmp.py click` puts the pointer somewhere and presses in one go, which
  a menu or dropdown that opened under the old pointer never sees: `move` there
  first, then `click`. A headless check of the engine beats a screenshot: `QT_QPA_PLATFORM=offscreen
  /usr/lib/qt6/bin/qml -I ~/.local/share/isle/qml file.qml` in the guest, with the
  result carried out through `Qt.exit`. Plain `qml` there is Qt 5 and loads nothing.
- Stage explicit paths. Never `git add -A`: other work may be in the tree.
- `hypr/generated/` and `theme/__pycache__/` are written at run time and
  are not committed.

## Comments

Comments say what the code does and how it works, at the point where that is
not obvious from the code. Nothing else.

- **Tight scope.** A comment covers the lines directly under it: a
  non-obvious binding, a workaround and the bug it works around, a unit, a
  contract a caller must honour. One or two lines.
- **No exposition.** No history, no "this used to be", no measurements, no
  round numbers, no critic notes, no argument for why this is right. That
  record lives in `docs/DECISIONS.md`; cite an entry by number if the
  reader needs it: `// alpha on the effect, not the body: D10`.
- **No task notes.** No TODO, FIXME, "Wave 2 removes this", "verify on the
  real machine". Those go in the slice's sheet or the commit message.
- **No file-header essays.** A file starts with at most a one-line
  statement of what it is. The design documents say why it exists.
- **No restating the code.** `// set volume` above `setVolume()` is noise.
- **Uppercase shouting is not emphasis.** Plain sentences.

Existing files under `shell/` carry long narrative comments from Wave 0.
They are the record before it had a home and are cut down to this rule by
the slice that next touches the file, not by a drive-by edit.
