# P14b: the greeter in the VM, and the host run

What `p14b-greeter.md` 14b.8 found in the guest on 2026-11-02, and the host
run that is part of P14b's done.

## The package

`tools/package.sh --no-install -- --nodeps` built `isle-git r669.g163c387415e6`,
then `r670.g56eea6cc0c50` with the console-key fix below. The first build
stopped at makepkg's check: an `optdepends` line held an apostrophe inside
single quotes. The guest's install was refused for five files no package owns
(`/usr/lib/isle/isle-clean-helper`, `isle-process-helper` and the three polkit
policies, copied by hand in P19's walks); `pacman -Qo` confirmed none was
owned and they were overwritten by name. pacman's tmpfiles hook made
`/var/cache/isle-greeter` and `/var/lib/isle-greeter` (0700, `greeter`) and
`/var/lib/isle/greeter` (0755, root).

## The guest

CachyOS guest, llvmpipe, 2560 by 1080, one user (`user`, uid 1000) in
AccountsService, Hyprland and Isle installed, uwsm not. The guest had no
password for its user; a generated test password was set for this run (with
the user's leave) and kept outside the repository.

| Check | Result |
|---|---|
| `isle setup --check` | `greeter` due (the guest held P14a's tuigreet line and record) |
| `isle setup` with a terminal | through sudo: `[default_session] command = "isle-compositor --greeter"` and `service = "isle-greeter"`, `[general] service = "isle-greetd"`, `/var/lib/isle/greeter/user` owned by `user` |
| greetd on tty1 | `getty@tty1` disabled, greetd enabled: `isle-compositor --greeter` as `greeter` in a logind session of class greeter on seat0 tty1, PAM service `isle-greeter`; one user, so the lock's face with "user" and the session capsule (Isle) |
| A wrong password | greetd's `pam_authenticate` under `isle-greetd` refused it; the field's error edge and "Wrong password. Try again." |
| The right password | Isle started: a user session on seat0 tty1, `/usr/bin/isle-compositor` from the package; the greeter's state named `user` and `isle` and recorded the start |
| The keyring | `pam_gnome_keyring` ran under `isle-greetd` with the typed password and said the login keyring's password was invalid: the guest's keyring predates the test password. The wiring is checked; unlocking is the host's to show |
| The look | `look.kdl` in `/var/lib/isle/greeter/user/` within a second of signing in: light mode, the guest's two keyboards, its layout and its display. After `isle quit` the greeter drew it (light) |
| The session menu | Isle and Hyprland; Hyprland (uwsm-managed) left out by its `TryExec=uwsm`. Opened upward from the capsule, Hyprland chosen, the field took the keyboard back |
| Hyprland from the greeter | `Hyprland --watchdog-fd 4` with `XDG_CURRENT_DESKTOP=Hyprland`; the greeter remembered `hyprland` for `user`, and showed it after Hyprland exited |
| Ctrl+Alt+F2 at the greeter | at first nothing: the guest's virtio keyboard takes the PC profile, which reads Alt as Cmd, and the console chord wanted Ctrl+Option (a fault since P5). Fixed to Ctrl with Option or Cmd: tty2's login, then `chvt 1` back to the same greeter, redrawn |
| The fallback | greetd's command made to fail at the seat (`LIBSEAT_BACKEND=nowhere`): systemd restarted greetd three times, and the fourth start ran tuigreet with P14a's arguments and cleared the count. The command put back, the greeter returned |

Not checked in the guest: NVIDIA and the 5070 Ti (llvmpipe here), the login
keyring unlocked, two or more users (headless only), Restart and Shut Down
at the greeter, a picture wallpaper published (headless only), greetd's
console flash between greeter and session (the guest's shots are taken
after it). The guest was left on greetd with Isle's greeter, the package
r670 installed and setup applied.

## The host run, the user's

1. `tools/package.sh`: it packages the checkout's `HEAD`, builds, and asks
   before installing.
2. `isle setup`: the `greeter` step is due; it asks for the password once.
   `/etc/greetd/config.toml` then names `isle-compositor --greeter`.
3. Still in your current Isle session, change any setting (or sign out and
   back in) so the session publishes its look; check
   `/var/lib/isle/greeter/alexparlett/look.kdl` exists.
4. Before signing out, know the way back. Ctrl+Option+F2 (with Fn while
   the F-row sends media keys) gives tty2's login; there, this puts tuigreet
   back and restarts greetd:

   ```bash
   sudo sed -i 's|^command = "isle-compositor --greeter"|command = "tuigreet --time --remember --remember-session --sessions /usr/share/wayland-sessions"|' /etc/greetd/config.toml && sudo systemctl restart greetd
   ```

   A greeter that cannot draw at all runs tuigreet by itself after three
   tries.
5. Sign out. The greeter should show your wallpaper, dark or light as you
   have it, your name and picture, and the session capsule.
6. Check, in order:
   - a wrong password shakes the field and says so;
   - Ctrl+Option+(Fn+)F2 reaches tty2, Ctrl+Option+F1 comes back;
   - the session menu lists Isle, Hyprland and Hyprland (uwsm-managed);
   - your password signs into Isle; the login keyring is unlocked (Keychain
     opens without asking for it, or `secret-tool` reads an item);
   - sign out, choose Hyprland, sign in, and Hyprland starts; exit it and
     the greeter remembers Hyprland;
   - typing your password with symbols reads as on the lock (the Mac
     layout, from the published keyboard section).
7. Tell me what the screen did between the greeter and the session (a text
   flash or not), and anything that looked wrong.
