--------------------------------------------------------------------------------
--  Keybindings — macOS profile  (~/.config/hypr/keymap contains "mac")
--
--  The bottom row reads Ctrl / ⌥ / ⌘. input.lua swaps Alt and Super so the ⌘
--  cap sends SUPER, which is what this file calls `cmd`.
--
--  Follows macOS wherever macOS has a convention, and stays out of the way
--  where applications already own a combo: nothing here binds bare ⌘C, ⌘S,
--  ⌘F, ⌘A or ⌘←/→, which are text and app shortcuts on every Mac.
--------------------------------------------------------------------------------

local s = require("binds-shared")
local d = s.d

local cmd, opt, ctrl, shift = "SUPER", "ALT", "CTRL", "SHIFT"

--------------------------------------------------------------------------------
--  Launching
--------------------------------------------------------------------------------

s.bind(cmd .. " + Space",                  hl.dsp.exec_cmd(s.scripts .. "/launcher.sh"), d("Launch bar"))
s.bind(cmd .. " + " .. opt .. " + Space",  hl.dsp.exec_cmd("rofi -show run"),            d("Run a command"))
s.bind(ctrl .. " + " .. cmd .. " + Space", hl.dsp.exec_cmd("rofi -show emoji -modes emoji"), d("Emoji picker"))

s.bind(cmd .. " + Return", hl.dsp.exec_cmd(s.term),  d("Terminal"))
s.bind(cmd .. " + E",      hl.dsp.exec_cmd(s.files), d("File manager"))
s.bind(cmd .. " + " .. shift .. " + E", hl.dsp.exec_cmd(s.term .. " -e yazi"), d("File manager (terminal)"))

s.bind(cmd .. " + comma", hl.dsp.exec_cmd("python3 ~/.config/hypr/settings/system-settings.py"), d("Settings"))
s.bind(cmd .. " + " .. shift .. " + slash", hl.dsp.exec_cmd(s.scripts .. "/cheatsheet.sh"), d("Keybind cheatsheet"))

s.bind(cmd .. " + " .. shift .. " + V", hl.dsp.exec_cmd(s.scripts .. "/clipboard.sh"), d("Clipboard history"))
s.bind(cmd .. " + " .. opt .. " + U",   hl.dsp.exec_cmd(s.term .. " -e cachy-update"), d("System update"))

s.agents(cmd .. " + " .. opt)

--------------------------------------------------------------------------------
--  Session
--------------------------------------------------------------------------------

s.bind(ctrl .. " + " .. cmd .. " + Q", hl.dsp.exec_cmd("hyprlock"),               d("Lock screen"))
s.bind(cmd .. " + Escape",             hl.dsp.exec_cmd("wlogout -p layer-shell"), d("Power menu"))
s.bind(cmd .. " + Q", hl.dsp.window.close(), d("Close window"))
s.bind(cmd .. " + W", hl.dsp.window.close(), d("Close window"))
s.bind(opt .. " + " .. cmd .. " + Escape", hl.dsp.window.kill(), d("Force quit window"))
s.bind(cmd .. " + " .. shift .. " + R", hl.dsp.exec_cmd("hyprctl reload"), d("Reload config"))

--------------------------------------------------------------------------------
--  Notifications and bar
--------------------------------------------------------------------------------

s.bind(cmd .. " + " .. opt .. " + N", hl.dsp.exec_cmd("swaync-client -t -sw"),  d("Notification centre"))
s.bind(cmd .. " + " .. opt .. " + D", hl.dsp.exec_cmd("swaync-client -d -sw"),  d("Do not disturb"))
s.bind(cmd .. " + " .. opt .. " + B", hl.dsp.exec_cmd("pkill -SIGUSR1 waybar"), d("Toggle the bar"))

--------------------------------------------------------------------------------
--  Windows
--------------------------------------------------------------------------------

s.bind(ctrl .. " + " .. cmd .. " + F", hl.dsp.window.fullscreen({ mode = "fullscreen" }), d("Fullscreen"))
s.bind(opt .. " + " .. cmd .. " + F",  hl.dsp.window.fullscreen({ mode = "maximized" }),  d("Maximise within gaps"))
s.bind(cmd .. " + " .. shift .. " + F", hl.dsp.window.float(),  d("Toggle floating"))
s.bind(cmd .. " + " .. shift .. " + P", hl.dsp.window.pin(),    d("Pin to all workspaces"))
s.bind(cmd .. " + " .. opt .. " + C",   hl.dsp.window.center(), d("Centre window"))

s.bind(cmd .. " + Tab",   hl.dsp.exec_cmd("rofi -show window"), d("Window switcher"))
s.bind(cmd .. " + grave", hl.dsp.window.cycle_next(),           d("Cycle windows"))

s.bind(cmd .. " + H",                   hl.dsp.window.move({ workspace = "special:scratchpad" }), d("Hide window"))
s.bind(cmd .. " + " .. shift .. " + H", hl.dsp.workspace.toggle_special("scratchpad"),            d("Show hidden windows"))

s.bind(cmd .. " + " .. opt .. " + J", hl.dsp.layout("togglesplit"), d("Toggle split direction"))
s.bind(cmd .. " + " .. opt .. " + T", hl.dsp.group.toggle(),        d("Toggle tab group"))
s.bind(ctrl .. " + Tab",              hl.dsp.group.next(),          d("Next tab in group"))

-- Focus and move: ⌥⌘ arrows. Bare ⌘ + arrows is line start/end in every Mac
-- text field, so it is deliberately left alone.
for _, dir in ipairs({ "left", "right", "up", "down" }) do
    s.bind(opt .. " + " .. cmd .. " + " .. dir,
            hl.dsp.focus({ direction = dir }), d("Focus " .. dir))
    s.bind(opt .. " + " .. cmd .. " + " .. shift .. " + " .. dir,
            hl.dsp.window.move({ direction = dir }), d("Move window " .. dir))
end

local step = 40
for dir, delta in pairs({
    left  = { -step, 0 }, right = { step, 0 },
    up    = { 0, -step },  down = { 0, step },
}) do
    s.bind(ctrl .. " + " .. cmd .. " + " .. dir,
            hl.dsp.window.resize({ x = delta[1], y = delta[2], relative = true }),
            { repeating = true, description = "Resize window" })
end

s.mouse(cmd)

--------------------------------------------------------------------------------
--  Window zones — ⌥⌘1-6, matching the drag-to-edge zones in snap.lua
--------------------------------------------------------------------------------

local zones = {
    { "1", "left-third",  "Snap to left third" },
    { "2", "centre",      "Snap to centre third" },
    { "3", "right-third", "Snap to right third" },
    { "4", "left",        "Snap to left half" },
    { "5", "right",       "Snap to right half" },
    { "6", "maximise",    "Snap to full width" },
}
for _, z in ipairs(zones) do
    s.bind(opt .. " + " .. cmd .. " + " .. z[1], s.snap_to(z[2]), d(z[3]))
end
s.bind(opt .. " + " .. cmd .. " + 0", hl.dsp.window.float({ action = "disable" }), d("Back to tiling"))

--------------------------------------------------------------------------------
--  Workspaces — ⌘digit. Send-to uses ⌃⌘ because ⌘⇧3/4/5 are the screenshot
--  shortcuts, and those get used far more often.
--------------------------------------------------------------------------------

s.workspaces(cmd, ctrl .. " + " .. cmd)

s.bind(ctrl .. " + " .. opt .. " + right", hl.dsp.focus({ workspace = "e+1" }), d("Next workspace"))
s.bind(ctrl .. " + " .. opt .. " + left",  hl.dsp.focus({ workspace = "e-1" }), d("Previous workspace"))

--------------------------------------------------------------------------------
--  Screenshots — the macOS combos, plus PrintScreen
--------------------------------------------------------------------------------

local shot = s.scripts .. "/screenshot.sh"

s.bind(cmd .. " + " .. shift .. " + 3", hl.dsp.exec_cmd(shot .. " screen"), d("Screenshot the screen"))
s.bind(cmd .. " + " .. shift .. " + 4", hl.dsp.exec_cmd(shot .. " region"), d("Screenshot a region"))
s.bind(cmd .. " + " .. shift .. " + 5", hl.dsp.exec_cmd(shot .. " edit"),   d("Screenshot and annotate"))
s.bind(cmd .. " + " .. shift .. " + 6", hl.dsp.exec_cmd(shot .. " window"), d("Screenshot a window"))
s.bind("Print",             hl.dsp.exec_cmd(shot .. " region"), d("Screenshot a region"))
s.bind(shift .. " + Print", hl.dsp.exec_cmd(shot .. " screen"), d("Screenshot the screen"))

s.bind(cmd .. " + " .. opt .. " + P", hl.dsp.exec_cmd("hyprpicker -a -n"), d("Pick a colour"))

s.media(cmd .. " + " .. opt)

-- Mac keyboards send no XF86 codes for volume when the function row is in
-- F-mode; these cover that.
s.bind(cmd .. " + F10", hl.dsp.exec_cmd("swayosd-client --output-volume mute-toggle"), { locked = true })
s.bind(cmd .. " + F11", hl.dsp.exec_cmd("swayosd-client --output-volume lower"), { locked = true, repeating = true })
s.bind(cmd .. " + F12", hl.dsp.exec_cmd("swayosd-client --output-volume raise"), { locked = true, repeating = true })

s.bind(cmd .. " + " .. opt .. " + K", hl.dsp.exec_cmd(s.scripts .. "/keymap.sh toggle"), d("Switch keymap profile"))
