# Updates: pkexec, paru and pacman as a run sees them

P14d phase 14d.0, 2026-11-12. Tried in the test guest (Arch, pacman 7.1,
polkit 127, paru 2.1.0 from the host's cache and then 2.2.0 from the AUR,
`isle-git` r722 running) and read on the host (CachyOS, polkit 128, paru
2.1.0). Recordings are kept as test fixtures in
`crates/isle-updates/tests/data/`.

## 1. pkexec from a transient user unit

`systemd-run --user --unit=… -- pkexec /usr/bin/id -u` puts up Isle's own
polkit dialog. The unit is in `user@1000.service/app.slice`, outside any
logind session, and polkit still finds the agent the compositor registered
for the session. Cancel or Escape makes pkexec exit 126 with "Request
dismissed".

## 2. One password per run

Two `pkexec /usr/bin/pacman` calls from one shell prompt twice under
`org.freedesktop.policykit.exec` (`auth_admin`). With
`data/polkit/io.github.alexparlett.Isle.Updates.policy` installed
(`exec.path` `/usr/bin/pacman`, `auth_admin_keep`), three calls from one
shell prompt once, and the dialog shows the action's message ("Software
Update wants to install updates."). A `pkexec pacman` from another process
straight after still prompts: the keep is for the process that asked.

A full `paru -Syu --sudo pkexec` asked once for its four pacman calls (the
repositories' transaction, the make dependencies, and two AUR installs).

## 3. paru's PKGBUILD repositories

- `[probe]` with `Url = file:///…`, `Path = pkg/arch`, `GenerateSrcinfo`
  and `SkipReview` in `/etc/paru.conf`: `paru -Sy --pkgbuilds` clones it
  and generates `.SRCINFO` as the user, with no root. `paru -Sya` does
  nothing.
- Installing from it records `url`, `branch` and `commit` in
  `~/.local/state/paru/devel.toml`.
- After a new commit, `paru -Qua`, `paru -Qu --pkgbuilds` and
  `--mode=ap` list nothing for it. `paru -Su` and `paru -Syu` find it
  under ":: Looking for devel upgrades..." (`Pkgbuilds (1)
  probe-git-latest-commit`) and rebuild it.
- So a check must compare `devel.toml`'s commit for `isle-git` with
  `git ls-remote <url> <branch>`, which is what paru compares in the run.
- A package not built through paru is reported as ":: packages not in the
  AUR: isle-git" and has no `devel.toml` entry until `paru --gendb`.

## 4. What a run prints through a pipe

- `--noprogressbar` and no terminal: no `(n/total)` counters on packages.
  Each transaction lists `Packages (N) …` (repositories, make
  dependencies) or `Pkgbuilds (N) …` / `Aur (N) …` (paru's), and each
  package then prints `upgrading X...`, `installing X...`,
  `removing X...`, `reinstalling X...` or `downgrading X...`. Downloads
  print ` X-ver-rel-arch downloading...`. Hooks keep their `(n/total)`.
- One run is several transactions: the repositories' (304 packages in the
  recording), make dependencies (5: rust and its friends), then one per
  built package. paru's own lines begin `::` and makepkg's `==>`.
- Mirror errors (`error: failed retrieving file … 404`) and `warning:`
  lines appear in a run that exits 0.
- A PGP key import question and pacman's own "Proceed?" take their
  defaults under `--noconfirm`.
- pacman's lines are translated: the guest's en_GB gives
  "Synchronising". The runner sets `LC_ALL=C.UTF-8`.
- makepkg under paru prints "Removing existing $srcdir/ directory..."
  before each build, so a cargo target under `$srcdir` is rebuilt from
  nothing every update.

## 5. Failures

- A held `db.lck`: "error: failed to synchronize all databases (unable to
  lock database)", exit 1.
- The password refused inside paru: pkexec's "Error executing command as
  another user: Request dismissed", and paru exits 1, not 126.

## 6. Restart owed

- After the guest's run, `uname -r` is 7.2.3-arch1-2 and
  `/usr/lib/modules` holds only 7.2.15-arch1-1.
- The host: `/sys/module/nvidia/version` reads 615.78.08; pacman's local
  database names `nvidia-utils-615.78.08-2`, so the version without its
  release is compared.

## For the plan

- Choice 6: Isle's own update is found from `devel.toml` and `git
  ls-remote`, not from `paru -Qua`.
- Choice 12: the total comes from `Packages`/`Pkgbuilds`/`Aur` lines, a
  package counts as done when the next package line starts; the runner
  runs paru under `LC_ALL=C.UTF-8`.
- Choice 13: a refusal is recognised by pkexec's line, whatever paru's
  exit code.
- The PKGBUILD builds into a target directory beside the PKGBUILD
  (`$startdir`), which makepkg leaves alone, so an update rebuilds only
  what changed.
