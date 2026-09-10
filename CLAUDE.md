# CLAUDE.md

Isle: a desktop shell for Hyprland in Quickshell.
Targets CachyOS; built for gaming and for running coding agents.

## Working here

- Read `docs/ARCHITECTURE.md` for what drives each capability and
  `docs/DESIGN.md` for how things look; `docs/DECISIONS.md` is the record
  of why. Keep all three current in the change that makes them stale.
- The desktop runs from `~/.local/share/isle`, a copy that `tools/install.sh`
  makes from this checkout. Editing here changes nothing on the host until
  the user runs it; the VM mounts this checkout directly.
- Verify in the VM, not by reasoning: `dev/test-vm.sh` boots the guest
  with the repo at `/repo`, `dev/guest.sh` runs a command in the
  session, `dev/test-vm.sh shot` takes a screenshot. `dev/` is the
  maintainer's local tooling and is git-ignored; never stage it. Restart the shell
  with `pkill -x qs` in the guest; the compositor's start hook relaunches it.
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
