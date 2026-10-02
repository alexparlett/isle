# Isle

A purpose-built, macOS-style desktop for my own Arch Linux (CachyOS)
machine, used mostly for gaming and coding.

This branch rebuilds Isle as its own Wayland compositor on Smithay, with
the shell (menu bar, island, Dock, Spotlight, Control Centre, notifications,
lock) drawn by the compositor in Freya, and the desktop's apps (Settings,
Files, Monitor, Keychain, Cleaner) as Freya apps. It is being planned and
built as the rust-desktop program; nothing of it runs yet.

- How it fits together: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- What each area does: [docs/design/](docs/design/)
- Why: [docs/DECISIONS.md](docs/DECISIONS.md)
- Where the program stands: [docs/work/rust-desktop/state.md](docs/work/rust-desktop/state.md)
  and [roadmap.md](docs/work/rust-desktop/roadmap.md)

Isle v1, a shell for Hyprland in Quickshell, is the daily desktop until the
new one replaces it; its README and docs are on `main`.
