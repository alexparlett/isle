# Shell UI spike

Throwaway. The question and the measurements are in
`docs/work/rust-desktop/spike-shell-ui.md`.

```
cargo run --release --bin island-empty   # the base with a blank island
cargo run --release --bin island-freya
cargo run --release --bin island-slint
```

`--bench` runs the scripted toggle and prints the numbers; `--relayout`
makes the content lay out at the animated size every frame.
Click the island or press Space to toggle it.
