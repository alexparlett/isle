--------------------------------------------------------------------------------
--  Automatic do-not-disturb
--  https://wiki.hypr.land/configuring/core/advanced-configuration/events/
--
--  Notifications are useful right up until they appear over a fullscreen game
--  or, worse, in the middle of a screen share. Two sources turn DND on here:
--
--    screenshare.state   anything capturing the screen — calls, OBS, a portal
--                        share. This is the one that saves you from a private
--                        message appearing in front of an audience.
--    window.fullscreen   a game going fullscreen.
--
--  They are reference-counted rather than toggled, so a game going fullscreen
--  during a call does not switch notifications back on when it exits.
--------------------------------------------------------------------------------

local sources = {}   -- name -> true, for whatever currently wants silence
local silenced = false

local function refresh()
    local wanted = next(sources) ~= nil
    if wanted == silenced then
        return
    end
    silenced = wanted
    hl.exec_cmd(wanted and "swaync-client --dnd-on" or "swaync-client --dnd-off")
end

local function set_source(name, active)
    sources[name] = active or nil
    refresh()
end

--------------------------------------------------------------------------------
--  Screen sharing
--------------------------------------------------------------------------------

hl.on("screenshare.state", function(active)
    set_source("screenshare", active == true)
end)

--------------------------------------------------------------------------------
--  Fullscreen games
--
--  The Window object's field names are not part of the documented API, so this
--  is defensive: if `class` or `fullscreen` are ever renamed, the pcall turns
--  the integration into a no-op instead of throwing on every window event.
--  `hyprctl clients -j` shows the field names if this ever needs revisiting.
--------------------------------------------------------------------------------

-- Lua patterns are not regex; match the classes the plain way.
local function is_game(class)
    if type(class) ~= "string" then return false end
    return class:match("^steam_app_%d+$") ~= nil
        or class == "gamescope"
        or class == "lutris"
        or class == "net.lutris.Lutris"
        or class == "heroic"
end

hl.on("window.fullscreen", function(w)
    local ok, game = pcall(function()
        return type(w) == "table" and is_game(w.class) and w.fullscreen and true or false
    end)
    if not ok then
        return
    end
    set_source("game", game)
end)

-- A game closing while still fullscreen never fires window.fullscreen again,
-- which would strand DND on. Clear it when the window goes away.
hl.on("window.close", function(w)
    local ok, game = pcall(function()
        return type(w) == "table" and is_game(w.class)
    end)
    if ok and game then
        set_source("game", false)
    end
end)

-- Belt and braces: if Hyprland exits while silenced, leave notifications on.
hl.on("hyprland.shutdown", function()
    if silenced then
        hl.exec_cmd("swaync-client --dnd-off")
    end
end)
