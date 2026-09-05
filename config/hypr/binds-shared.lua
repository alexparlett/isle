--------------------------------------------------------------------------------
--  Bindings common to every keymap profile
--
--  Anything here is identical whichever profile is active: media keys, the
--  agent panels, the workspace loop. The profiles supply the modifiers they
--  want and add their own idioms on top.
--------------------------------------------------------------------------------

local snap = require("snap")

local M = {
    term    = "kitty",
    scripts = "~/.config/hypr/scripts",
}

--------------------------------------------------------------------------------
--  User rebinds
--
--  ~/.config/hypr/keybinds.conf holds `description = keys` lines, written by
--  the Settings window or by hand. A bind's description is its identity: it is
--  already unique, already human-readable, and already what the UI and the ⌘?
--  cheatsheet show, so nothing needs a synthetic id.
--
--      Terminal = SUPER + T
--      Snap left half = SUPER + KP_Left
--
--  Overrides are applied as binds register, because that is the only moment
--  they exist — there is no rebinding a live keybind in Hyprland.
--------------------------------------------------------------------------------

M.overrides_path = (os.getenv("HOME") or "") .. "/.config/hypr/keybinds.conf"

local function load_overrides()
    local out = {}
    local file = io.open(M.overrides_path, "r")
    if not file then return out end

    for line in file:lines() do
        if not line:match("^%s*#") then
            local desc, keys = line:match("^%s*(.-)%s*=%s*(.-)%s*$")
            if desc and keys and desc ~= "" and keys ~= "" then
                out[desc] = keys
            end
        end
    end
    file:close()
    return out
end

M.overrides = load_overrides()

--------------------------------------------------------------------------------
--  Yielding to HyprMod
--
--  HyprMod is a GTK settings app for Hyprland with its own keybind editor. In
--  Lua mode it writes hyprland-gui.lua, which hyprland.lua requires after this
--  file — so a bind it sets on a key we already use does not replace ours,
--  Hyprland registers both and fires both. That is the one place the two
--  configs genuinely collide.
--
--  Rather than declaring its keybind page off-limits, read what it claims and
--  step aside. Any key HyprMod binds is a key we do not, so its editor simply
--  works and the more recent, hand-made choice wins.
--
--  If its output format ever changes, the pattern stops matching, nothing is
--  claimed, and behaviour falls back to what it is today. Failure is a no-op.
--------------------------------------------------------------------------------

--- Comparable form of a key combo: modifiers sorted, case-folded, key last.
--- "ALT + SUPER + T" and "super + alt + T" both become "ALT+SUPER|t".
local function normalise(keys)
    local mods, key = {}, nil
    for raw in tostring(keys):gmatch("[^+]+") do
        local part = raw:match("^%s*(.-)%s*$")
        if part ~= "" then
            local upper = part:upper()
            if upper == "CONTROL" then upper = "CTRL" end
            if upper == "SUPER" or upper == "ALT" or upper == "CTRL" or upper == "SHIFT" then
                table.insert(mods, upper)
            else
                key = part
            end
        end
    end
    table.sort(mods)
    return table.concat(mods, "+") .. "|" .. string.lower(key or "")
end

local function load_claimed()
    local claimed = {}
    local path = (os.getenv("HOME") or "") .. "/.config/hypr/hyprland-gui.lua"
    local file = io.open(path, "r")
    if not file then return claimed end

    local body = file:read("*a") or ""
    file:close()

    -- HyprMod renders `-- Keybinds` above matching `hl.bind(...)` lines.
    for _, pattern in ipairs({ 'hl%.bind%s*%(%s*"([^"]+)"', "hl%.bind%s*%(%s*'([^']+)'" }) do
        for keys in body:gmatch(pattern) do
            claimed[normalise(keys)] = keys
        end
    end
    return claimed
end

M.claimed = load_claimed()
M.normalise = normalise

--- Register a bind, unless the user has rebound it or HyprMod has claimed the
--- key. Binds without a description cannot be rebound from the settings window,
--- which is deliberate: those are the mouse and media bindings.
function M.bind(keys, action, flags)
    local desc = type(flags) == "table" and flags.description or nil
    if desc and M.overrides[desc] then
        keys = M.overrides[desc]
    end
    if M.claimed[normalise(keys)] then
        return nil -- HyprMod binds this key; let its version be the only one
    end
    return hl.bind(keys, action, flags)
end

-- Pick whichever file manager is installed, so this survives swapping Dolphin
-- for a GTK one later.
local function first_installed(candidates, fallback)
    for _, name in ipairs(candidates) do
        local handle = io.open("/usr/bin/" .. name, "r")
        if handle then handle:close() return name end
    end
    return fallback
end

M.files = first_installed({ "dolphin", "thunar", "nautilus", "nemo" }, "dolphin")

function M.d(text) return { description = text } end

--- Media, volume and night light. `locked` keeps them working over hyprlock.
function M.media(mod)
    local once = { locked = true }
    local held = { locked = true, repeating = true }

    M.bind("XF86AudioPlay",  hl.dsp.exec_cmd("playerctl play-pause"), once)
    M.bind("XF86AudioPause", hl.dsp.exec_cmd("playerctl play-pause"), once)
    M.bind("XF86AudioNext",  hl.dsp.exec_cmd("playerctl next"),       once)
    M.bind("XF86AudioPrev",  hl.dsp.exec_cmd("playerctl previous"),   once)
    M.bind("XF86AudioStop",  hl.dsp.exec_cmd("playerctl stop"),       once)

    M.bind("XF86AudioRaiseVolume", hl.dsp.exec_cmd("swayosd-client --output-volume raise"), held)
    M.bind("XF86AudioLowerVolume", hl.dsp.exec_cmd("swayosd-client --output-volume lower"), held)
    M.bind("XF86AudioMute",        hl.dsp.exec_cmd("swayosd-client --output-volume mute-toggle"), once)
    M.bind("XF86AudioMicMute",     hl.dsp.exec_cmd("swayosd-client --input-volume mute-toggle"),  once)
    M.bind("Caps_Lock",            hl.dsp.exec_cmd("swayosd-client --caps-lock"),                 once)

    M.bind(mod .. " + F9",  hl.dsp.exec_cmd("hyprctl hyprsunset temperature 4000"), M.d("Warm the display"))
    M.bind(mod .. " + F10", hl.dsp.exec_cmd("hyprctl hyprsunset identity"),         M.d("Normal colour temperature"))
end

--- Workspace switching and moving windows between them.
function M.workspaces(switch_mod, move_mod)
    for i = 1, 10 do
        local key = i % 10 -- workspace 10 lives on the 0 key
        M.bind(switch_mod .. " + " .. key, hl.dsp.focus({ workspace = i }), M.d("Workspace " .. i))
        M.bind(move_mod .. " + " .. key,
                hl.dsp.window.move({ workspace = i, follow = false }),
                M.d("Send window to workspace " .. i))
    end
    M.bind(switch_mod .. " + mouse_down", hl.dsp.focus({ workspace = "e+1" }), M.d("Next workspace"))
    M.bind(switch_mod .. " + mouse_up",   hl.dsp.focus({ workspace = "e-1" }), M.d("Previous workspace"))
end

--- The AI assistant panels and the jump-to-blocked-session bind.
function M.agents(mod)
    M.bind(mod .. " + A", hl.dsp.exec_cmd(M.scripts .. "/ai.sh claude"),  M.d("Claude panel"))
    M.bind(mod .. " + G", hl.dsp.exec_cmd(M.scripts .. "/ai.sh chatgpt"), M.d("ChatGPT panel"))
    M.bind(mod .. " + SHIFT + A", hl.dsp.exec_cmd(M.scripts .. "/agent-attention.sh"),
            M.d("Go to the agent waiting on you"))
end

--- Mouse move and resize.
function M.mouse(mod)
    M.bind(mod .. " + mouse:272", hl.dsp.window.drag(),   { mouse = true, description = "Drag window" })
    M.bind(mod .. " + mouse:273", hl.dsp.window.resize(), { mouse = true, description = "Resize window" })
end

M.snap_to = snap.snap_to

return M
