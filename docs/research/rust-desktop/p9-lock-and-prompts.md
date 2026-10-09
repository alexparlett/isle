# P9: the lock's and prompts' measures, and what the VM found

## Measures

`cargo test --release -p isle-compositor --features test-hooks --test bench
lock_and_prompts_cost -- --ignored --nocapture`, headless on the 5070 Ti: a
3440 by 1440 output over the frost wallpaper, a window open, a fake logind.
Draws are timed with `glFinish` after each; typing is 36 keys 120 ms apart.
Six runs:

| | Runs 1–3 | Runs 4–6 |
|---|---|---|
| Lock request to every screen locked | 44–46 ms the first time, then 19.2–21.9 ms | the same |
| The lock's face, typing (42 draws) | p50 0.60–0.67 ms, p95 1.63–2.10 ms, max 2.5–3.2 ms | p50 0.45–0.56 ms, p95 0.62–0.77 ms, max 0.96–1.02 ms |
| A prompt, typing (38 draws) | p50 0.42–0.43 ms, p95 1.55–2.81 ms, max 3.3–3.5 ms | p50 0.35–0.36 ms, p95 0.41–0.43 ms, max 2.83–2.86 ms |
| At rest for 10 s, locked and unlocked | nothing drew | nothing drew |
| RSS at rest, locked, after | 150, 161, 164 MB | the same within 3 MB |
| VRAM at rest, locked, with a prompt | 92, 94, 58 MiB | the same |

- **The face and a prompt draw well under 2 ms in a quiet run.** A prompt's
  worst frame, about 2.9 ms, is the one that opens it.
- **The first three runs had single slow frames**, 1.7 to 3.5 ms, four of 38
  in one run, on no particular key. Timed one by one they did not follow the
  typing gap (two at 30 ms between keys, none at 120 ms in the next run) and
  did not come back in runs 4 to 6. The host was otherwise idle; the GPU was
  also drawing the user's desktop. With so few draws a p95 is the second or
  third worst frame, so two such frames move it over 2 ms.
- **The lock reaches every screen in one or two frames**: about 20 ms from
  the request to the presented locked frame; the first lock after start
  takes 45 ms, which is the softened wallpaper being made.
- **Locked, the softened backdrop costs 2 MiB of VRAM**; RSS and VRAM stay
  inside 250 MB and 200 MiB.
- **At rest nothing draws**, the face dozed or the desktop idle.

## The VM

In the guest (llvmpipe, 2560 by 1080), with Isle's PAM service installed as
`/etc/pam.d/isle`, gnome-keyring and NetworkManager's OpenVPN plugin, kitty:

- **The lock through logind.** `loginctl lock-session` locks Isle's session
  and sets the locked hint; `unlock-session` unlocks and clears it. One wrong
  password through the real `system-local-login` stack gives "Wrong
  password. Try again." and the field's error edge. The right password was
  not typed: the guest account has no test password.
- **A session restarted while locked starts locked**, from the marker; with
  tty1's autologin anything else would open the desktop.
- **pkexec in kitty**: polkit's question over the scrim, Details, Cancel
  reaching pkexec as "Request dismissed", exit 126.
- **Found and fixed: pkexec in a terminal named no app.** Only a window's own
  process was known, and pkexec is the shell's child; the asker is now the
  nearest ancestor with a window, or the scope Isle launched it in. The
  prompt reads "kitty needs your password" with kitty's icon and the lock
  badge, and counts as the user's own request.
- **The keyring**: `secret-tool store` with no login keyring brought
  gnome-keyring's "Choose password for new keyring" through Isle's system
  prompter, Password and Confirm; Cancel failed the store.
- **A VPN**: an OpenVPN connection with an agent-owned password, brought up
  with `nmcli` in kitty, asked Isle's secret agent; the plugin's auth dialog
  in external-UI mode described its field, and the prompt read "“IsleTest”
  needs a password" with Show password and Connect. Cancel ended the
  activation. No Wi-Fi in the guest.
- **The power key** opened the chooser with kitty's icon and the 60-second
  countdown on Shut Down; R quit kitty, the session logged "every app quit",
  and logind rebooted the guest.
- **Idle**, with the steps at one, two and three minutes: dimmed at 60 s,
  locked at 120 s, screens off at 180 s; the pointer turned them on with the
  face shown.
- **Found and fixed: the CLI panicked writing into a closed pipe**
  (`isle idle | head -1`); it now ends quietly, as other tools do.

The host run is optional, as for P6 to P8, and is where DPMS on the NVIDIA
driver, the KVM's return and a real login keyring are seen.
