--------------------------------------------------------------------------------
--  Keybindings
--  https://wiki.hypr.land/configuring/core/binds/
--
--  Descriptions are filled in because `hyprctl binds` exposes them — that is
--  what the SUPER+/ cheatsheet reads.
--------------------------------------------------------------------------------

local mod     = "SUPER"
local term    = "alacritty"
local files   = "dolphin"
local scripts = "~/.config/hypr/scripts"

local function d(text) return { description = text } end

--------------------------------------------------------------------------------
--  Applications
--------------------------------------------------------------------------------

hl.bind(mod .. " + Return", hl.dsp.exec_cmd(term),                     d("Terminal"))
hl.bind(mod .. " + E",      hl.dsp.exec_cmd(files),                    d("File manager"))
hl.bind(mod .. " + D",      hl.dsp.exec_cmd("rofi -show drun"),        d("App launcher"))
hl.bind(mod .. " + R",      hl.dsp.exec_cmd("rofi -show run"),         d("Run a command"))
hl.bind(mod .. " + Tab",    hl.dsp.exec_cmd("rofi -show window"),      d("Switch window"))
hl.bind(mod .. " + V",      hl.dsp.exec_cmd(scripts .. "/clipboard.sh"), d("Clipboard history"))
hl.bind(mod .. " + U",      hl.dsp.exec_cmd(term .. " -e cachy-update"), d("System update"))
hl.bind(mod .. " + slash",  hl.dsp.exec_cmd(scripts .. "/cheatsheet.sh"), d("This cheatsheet"))

--------------------------------------------------------------------------------
--  Session
--------------------------------------------------------------------------------

hl.bind(mod .. " + L",         hl.dsp.exec_cmd("hyprlock"),               d("Lock screen"))
hl.bind(mod .. " + Escape",    hl.dsp.exec_cmd("wlogout -p layer-shell"), d("Power menu"))
hl.bind(mod .. " + Q",         hl.dsp.window.close(),                     d("Close window"))
hl.bind(mod .. " + SHIFT + Q", hl.dsp.window.kill(),                      d("Force kill window"))
hl.bind(mod .. " + SHIFT + R", hl.dsp.exec_cmd("hyprctl reload"),         d("Reload config"))

--------------------------------------------------------------------------------
--  Notifications and bar
--------------------------------------------------------------------------------

hl.bind(mod .. " + N",         hl.dsp.exec_cmd("swaync-client -t -sw"),  d("Notification centre"))
hl.bind(mod .. " + SHIFT + N", hl.dsp.exec_cmd("swaync-client -d -sw"),  d("Dismiss notifications"))
hl.bind(mod .. " + B",         hl.dsp.exec_cmd("pkill -SIGUSR1 waybar"), d("Toggle the bar"))

--------------------------------------------------------------------------------
--  Window management
--------------------------------------------------------------------------------

hl.bind(mod .. " + F",         hl.dsp.window.fullscreen({ mode = "fullscreen" }), d("Fullscreen"))
hl.bind(mod .. " + SHIFT + F", hl.dsp.window.fullscreen({ mode = "maximized" }),  d("Maximise within gaps"))
hl.bind(mod .. " + Space",     hl.dsp.window.float(),                             d("Toggle floating"))
hl.bind(mod .. " + P",         hl.dsp.window.pin(),                               d("Pin to all workspaces"))
hl.bind(mod .. " + C",         hl.dsp.window.center(),                            d("Centre window"))
hl.bind(mod .. " + J",         hl.dsp.layout("togglesplit"),                      d("Toggle split direction"))
hl.bind(mod .. " + G",         hl.dsp.group.toggle(),                             d("Toggle tab group"))
hl.bind(mod .. " + SHIFT + Tab", hl.dsp.group.next(),                             d("Next tab in group"))

-- Focus
for key, dir in pairs({ left = "left", right = "right", up = "up", down = "down" }) do
    hl.bind(mod .. " + " .. key, hl.dsp.focus({ direction = dir }), d("Focus " .. dir))
end

-- Move
for key, dir in pairs({ left = "left", right = "right", up = "up", down = "down" }) do
    hl.bind(mod .. " + SHIFT + " .. key, hl.dsp.window.move({ direction = dir }), d("Move window " .. dir))
end

-- Resize, repeatable while held
local resize_step = 40
for key, delta in pairs({
    left  = { -resize_step, 0 },
    right = { resize_step, 0 },
    up    = { 0, -resize_step },
    down  = { 0, resize_step },
}) do
    hl.bind(
        mod .. " + CTRL + " .. key,
        hl.dsp.window.resize({ x = delta[1], y = delta[2], relative = true }),
        { repeating = true, description = "Resize window" }
    )
end

-- Mouse
hl.bind(mod .. " + mouse:272", hl.dsp.window.drag(),   { mouse = true, description = "Drag window" })
hl.bind(mod .. " + mouse:273", hl.dsp.window.resize(), { mouse = true, description = "Resize window" })

--------------------------------------------------------------------------------
--  Ultrawide thirds
--
--  3440 wide, 10px gaps, ~40px bar: three columns of 1133px starting at
--  x = 10 / 1153 / 2296. ALT+0 hands the window back to the tiler.
--------------------------------------------------------------------------------

local COL_W, COL_H, TOP = 1133, 1380, 50

local function snap_to(x)
    return function()
        hl.dispatch(hl.dsp.window.float({ action = "enable" }))
        hl.dispatch(hl.dsp.window.resize({ x = COL_W, y = COL_H, relative = false }))
        hl.dispatch(hl.dsp.window.move({ x = x, y = TOP, relative = false }))
    end
end

hl.bind(mod .. " + ALT + 1", snap_to(10),   d("Snap to left third"))
hl.bind(mod .. " + ALT + 2", snap_to(1153), d("Snap to centre third"))
hl.bind(mod .. " + ALT + 3", snap_to(2296), d("Snap to right third"))
hl.bind(mod .. " + ALT + 0", hl.dsp.window.float({ action = "disable" }), d("Back to tiling"))

--------------------------------------------------------------------------------
--  Workspaces
--------------------------------------------------------------------------------

for i = 1, 10 do
    local key = i % 10 -- workspace 10 lives on the 0 key
    hl.bind(mod .. " + " .. key, hl.dsp.focus({ workspace = i }), d("Workspace " .. i))
    hl.bind(
        mod .. " + SHIFT + " .. key,
        hl.dsp.window.move({ workspace = i, follow = false }),
        d("Send window to workspace " .. i)
    )
end

hl.bind(mod .. " + bracketright", hl.dsp.focus({ workspace = "e+1" }), d("Next workspace"))
hl.bind(mod .. " + bracketleft",  hl.dsp.focus({ workspace = "e-1" }), d("Previous workspace"))
hl.bind(mod .. " + mouse_down",   hl.dsp.focus({ workspace = "e+1" }), d("Next workspace"))
hl.bind(mod .. " + mouse_up",     hl.dsp.focus({ workspace = "e-1" }), d("Previous workspace"))
hl.bind(mod .. " + grave",        hl.dsp.focus({ workspace = "previous" }), d("Last workspace"))

-- Scratchpad
hl.bind(mod .. " + S",         hl.dsp.workspace.toggle_special("scratchpad"),          d("Toggle scratchpad"))
hl.bind(mod .. " + SHIFT + S", hl.dsp.window.move({ workspace = "special:scratchpad" }), d("Send to scratchpad"))

--------------------------------------------------------------------------------
--  Screenshots and colour
--------------------------------------------------------------------------------

hl.bind("Print",               hl.dsp.exec_cmd(scripts .. "/screenshot.sh region"), d("Screenshot a region"))
hl.bind("SHIFT + Print",       hl.dsp.exec_cmd(scripts .. "/screenshot.sh screen"), d("Screenshot the screen"))
hl.bind("CTRL + Print",        hl.dsp.exec_cmd(scripts .. "/screenshot.sh window"), d("Screenshot a window"))
hl.bind(mod .. " + Print",     hl.dsp.exec_cmd(scripts .. "/screenshot.sh edit"),   d("Screenshot and annotate"))
hl.bind(mod .. " + SHIFT + C", hl.dsp.exec_cmd("hyprpicker -a -n"),                 d("Pick a colour"))

--------------------------------------------------------------------------------
--  Media, volume, night light
--  `locked` keeps these working while hyprlock is up.
--------------------------------------------------------------------------------

local media = { locked = true }
local held  = { locked = true, repeating = true }

hl.bind("XF86AudioPlay",  hl.dsp.exec_cmd("playerctl play-pause"), media)
hl.bind("XF86AudioPause", hl.dsp.exec_cmd("playerctl play-pause"), media)
hl.bind("XF86AudioNext",  hl.dsp.exec_cmd("playerctl next"),       media)
hl.bind("XF86AudioPrev",  hl.dsp.exec_cmd("playerctl previous"),   media)
hl.bind("XF86AudioStop",  hl.dsp.exec_cmd("playerctl stop"),       media)

hl.bind("XF86AudioRaiseVolume", hl.dsp.exec_cmd("swayosd-client --output-volume raise"), held)
hl.bind("XF86AudioLowerVolume", hl.dsp.exec_cmd("swayosd-client --output-volume lower"), held)
hl.bind("XF86AudioMute",        hl.dsp.exec_cmd("swayosd-client --output-volume mute-toggle"), media)
hl.bind("XF86AudioMicMute",     hl.dsp.exec_cmd("swayosd-client --input-volume mute-toggle"),  media)
hl.bind("Caps_Lock",            hl.dsp.exec_cmd("swayosd-client --caps-lock"),                 media)

hl.bind(mod .. " + F9",  hl.dsp.exec_cmd("hyprctl hyprsunset temperature 4000"), d("Warm the display"))
hl.bind(mod .. " + F10", hl.dsp.exec_cmd("hyprctl hyprsunset identity"),         d("Normal colour temperature"))
