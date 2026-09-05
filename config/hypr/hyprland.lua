--------------------------------------------------------------------------------
--  Hyprland configuration — entrypoint
--
--  Hyprland 0.55+ uses Lua; hyprlang (hyprland.conf) is deprecated.
--  https://wiki.hypr.land/configuring/
--
--  Modules are required in dependency order: env before anything that reads
--  the environment, monitors before workspace rules that pin to them.
--------------------------------------------------------------------------------

require("env")
require("monitors")
require("input")
require("look")
require("snap")   -- defines the zone helpers the keymap profiles bind to
require("binds")  -- selects mac or windows, see keymap.lua
require("rules")
require("gaming")
require("dnd")
require("autostart")

-- Machine-local overrides, not tracked by git. Create ~/.config/hypr/local.lua
-- to tweak anything above without dirtying the repo.
pcall(require, "local")
