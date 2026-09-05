--------------------------------------------------------------------------------
--  Bindings common to every keymap profile
--
--  Anything here is identical whichever profile is active: media keys, the
--  agent panels, the workspace loop. The profiles supply the modifiers they
--  want and add their own idioms on top.
--------------------------------------------------------------------------------

local snap = require("snap")

local M = {
    term    = "alacritty",
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

--- Register a bind, honouring any rebind the user has set for its description.
--- Binds without a description cannot be rebound, which is deliberate: those
--- are the mouse and media bindings that have no business being remapped from
--- a settings window.
function M.bind(keys, action, flags)
    local desc = type(flags) == "table" and flags.description or nil
    if desc and M.overrides[desc] then
        keys = M.overrides[desc]
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
