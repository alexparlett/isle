--------------------------------------------------------------------------------
--  Keybindings — profile selector
--
--  Two profiles ship: `mac` (default) and `windows`. The choice lives in
--  ~/.config/hypr/keymap, read by keymap.lua, because binds are registered
--  while the config loads and local.lua comes far too late to influence them.
--
--  Switch with:
--      scripts/keymap.sh windows     (or mac, or toggle)
--      the Settings window
--      ⌥⌘K / Win+Alt+K
--
--  What is shared between the two lives in binds-shared.lua; what differs is
--  the whole point, so the profiles are written out separately rather than
--  generated from a table of modifiers. They disagree about more than which
--  key is the modifier: ⌘W versus Alt+F4, ⌘⇧4 versus Win+Shift+S, and Aero
--  Snap on Win+arrows has no Mac equivalent at all.
--------------------------------------------------------------------------------

local keymap = require("keymap")

if keymap.is("windows") then
    require("binds-windows")
else
    require("binds-mac")
end

return keymap
