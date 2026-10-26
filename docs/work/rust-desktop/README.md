# Rust desktop

A from-scratch desktop in Rust: an own compositor on Smithay with the shell
drawn by it, replacing Isle v1 (Hyprland + Quickshell). This is a program:
the design is brainstormed here, then built as a series of packets planned
one at a time.

- `brainstorm.md`: how the decisions were reached; their text is in
  `docs/DECISIONS.md`.
- `state.md`: where the program stands.
- `progress.md`: running log, newest first.
- `roadmap.md`: the build packets in order.
- `spike-shell-ui.md`: the spike that decided how the shell UI is drawn.
- `p1-design.md`: P1, the design process; its working files in `p1/`.
- `p2-compositor.md`: P2, the compositor core.
- `p3-shell-runtime.md`: P3, the shell runtime.
- `p4-windows.md`: P4, windows.
- `p5-keyboard.md`: P5, keyboard.
- `p6-island-and-menus.md`: P6, the island, menus and wallpaper.
- `p7-switcher.md`: P7, the app switcher.
- `p8-services.md`: P8, services, notifications and Control Centre.
- `p9-lock-and-prompts.md`: P9, the lock, idle, the system prompts and the
  session's end.
- `p10-spotlight.md`: P10, Spotlight.
- `p11-capture.md`: P11, capture and portals.
- `p12-gaming.md`: P12, gaming and night light.
- `p13-settings.md`: P13, Settings, core; its gates in `p13/`.
- `p14a-session.md`: P14a, installed beside Hyprland.
- `p15-files.md`: P15, Files.
- `p16-mission-control.md`: P16, Mission Control, gestures, Split View and
  Stage Manager.
- Evidence: `docs/research/rust-desktop/`.
