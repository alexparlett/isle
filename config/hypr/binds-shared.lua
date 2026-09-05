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

    hl.bind("XF86AudioPlay",  hl.dsp.exec_cmd("playerctl play-pause"), once)
    hl.bind("XF86AudioPause", hl.dsp.exec_cmd("playerctl play-pause"), once)
    hl.bind("XF86AudioNext",  hl.dsp.exec_cmd("playerctl next"),       once)
    hl.bind("XF86AudioPrev",  hl.dsp.exec_cmd("playerctl previous"),   once)
    hl.bind("XF86AudioStop",  hl.dsp.exec_cmd("playerctl stop"),       once)

    hl.bind("XF86AudioRaiseVolume", hl.dsp.exec_cmd("swayosd-client --output-volume raise"), held)
    hl.bind("XF86AudioLowerVolume", hl.dsp.exec_cmd("swayosd-client --output-volume lower"), held)
    hl.bind("XF86AudioMute",        hl.dsp.exec_cmd("swayosd-client --output-volume mute-toggle"), once)
    hl.bind("XF86AudioMicMute",     hl.dsp.exec_cmd("swayosd-client --input-volume mute-toggle"),  once)
    hl.bind("Caps_Lock",            hl.dsp.exec_cmd("swayosd-client --caps-lock"),                 once)

    hl.bind(mod .. " + F9",  hl.dsp.exec_cmd("hyprctl hyprsunset temperature 4000"), M.d("Warm the display"))
    hl.bind(mod .. " + F10", hl.dsp.exec_cmd("hyprctl hyprsunset identity"),         M.d("Normal colour temperature"))
end

--- Workspace switching and moving windows between them.
function M.workspaces(switch_mod, move_mod)
    for i = 1, 10 do
        local key = i % 10 -- workspace 10 lives on the 0 key
        hl.bind(switch_mod .. " + " .. key, hl.dsp.focus({ workspace = i }), M.d("Workspace " .. i))
        hl.bind(move_mod .. " + " .. key,
                hl.dsp.window.move({ workspace = i, follow = false }),
                M.d("Send window to workspace " .. i))
    end
    hl.bind(switch_mod .. " + mouse_down", hl.dsp.focus({ workspace = "e+1" }), M.d("Next workspace"))
    hl.bind(switch_mod .. " + mouse_up",   hl.dsp.focus({ workspace = "e-1" }), M.d("Previous workspace"))
end

--- The AI assistant panels and the jump-to-blocked-session bind.
function M.agents(mod)
    hl.bind(mod .. " + A", hl.dsp.exec_cmd(M.scripts .. "/ai.sh claude"),  M.d("Claude panel"))
    hl.bind(mod .. " + G", hl.dsp.exec_cmd(M.scripts .. "/ai.sh chatgpt"), M.d("ChatGPT panel"))
    hl.bind(mod .. " + SHIFT + A", hl.dsp.exec_cmd(M.scripts .. "/agent-attention.sh"),
            M.d("Go to the agent waiting on you"))
end

--- Mouse move and resize.
function M.mouse(mod)
    hl.bind(mod .. " + mouse:272", hl.dsp.window.drag(),   { mouse = true, description = "Drag window" })
    hl.bind(mod .. " + mouse:273", hl.dsp.window.resize(), { mouse = true, description = "Resize window" })
end

M.snap_to = snap.snap_to

return M
