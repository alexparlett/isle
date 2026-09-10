# hyprbars, as Isle builds it

Upstream: https://github.com/hyprwm/hyprland-plugins/tree/main/hyprbars (BSD-3-Clause),
at the commit upstream's hyprpm.toml pins for the Hyprland in packages/shell.txt.

One change: `CHyprBar::updateRules` hides the bar when the window's client asked
for client-side decorations through xdg-decoration, which is how a browser with
its own tab strip, an Electron app with a custom title bar or a libadwaita app
say they draw their own controls. Hyprland records the request but always answers
"server side", so the compositor's own rules cannot see it; the plugin reads it.
`hyprbars:no_bar` still overrides either way.

Refresh from upstream by copying the pinned directory over this one and
re-applying the block marked `isle:` in barDeco.cpp.
