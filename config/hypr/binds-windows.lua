--------------------------------------------------------------------------------
--  Keybindings — Windows profile  (~/.config/hypr/keymap contains "windows")
--
--  Windows conventions wherever they exist, which is more of them than people
--  remember: Win+E, Win+R, Win+I, Win+L, Win+V for clipboard history, Win+.
--  for emoji, Win+Shift+S for a region snip, Alt+F4, Alt+Tab,
--  Ctrl+Shift+Esc for the task manager — and Win+arrows for Aero Snap, which
--  maps straight onto the zones in snap.lua.
--
--  `win` is SUPER. With Mac keycaps and the Alt/Super swap in input.lua, that
--  is the ⌘ cap — the key next to the space bar either way, which is roughly
--  where a Windows keyboard puts it too.
--------------------------------------------------------------------------------

local s = require("binds-shared")
local d = s.d

local win, alt, ctrl, shift = "SUPER", "ALT", "CTRL", "SHIFT"

--------------------------------------------------------------------------------
--  Launching
--------------------------------------------------------------------------------

-- Tapping Win alone opens Start on Windows. Binding a bare modifier works, but
-- press order matters, so Win+S is kept as the reliable one.
hl.bind("Super_L",         hl.dsp.exec_cmd(s.scripts .. "/launcher.sh"), d("Start / search"))
hl.bind(win .. " + S",     hl.dsp.exec_cmd(s.scripts .. "/launcher.sh"), d("Search"))
hl.bind(win .. " + R",     hl.dsp.exec_cmd("rofi -show run"),            d("Run"))
hl.bind(win .. " + period", hl.dsp.exec_cmd("rofi -show emoji -modes emoji"), d("Emoji picker"))

hl.bind(win .. " + Return", hl.dsp.exec_cmd(s.term),  d("Terminal"))
hl.bind(win .. " + E",      hl.dsp.exec_cmd(s.files), d("File manager"))
hl.bind(win .. " + I",      hl.dsp.exec_cmd("python3 ~/.config/hypr/settings/hypr-settings.py"), d("Settings"))
hl.bind(win .. " + V",      hl.dsp.exec_cmd(s.scripts .. "/clipboard.sh"), d("Clipboard history"))
hl.bind(win .. " + slash",  hl.dsp.exec_cmd(s.scripts .. "/cheatsheet.sh"), d("Keybind cheatsheet"))
hl.bind(win .. " + U",      hl.dsp.exec_cmd(s.term .. " -e cachy-update"),  d("System update"))

-- Task manager.
hl.bind(ctrl .. " + " .. shift .. " + Escape", hl.dsp.exec_cmd(s.term .. " -e btop"), d("Task manager"))

s.agents(win .. " + " .. alt)

--------------------------------------------------------------------------------
--  Session
--------------------------------------------------------------------------------

hl.bind(win .. " + L", hl.dsp.exec_cmd("hyprlock"),               d("Lock screen"))
hl.bind(win .. " + X", hl.dsp.exec_cmd("wlogout -p layer-shell"), d("Power menu"))
hl.bind(alt .. " + F4", hl.dsp.window.close(),                    d("Close window"))
hl.bind(ctrl .. " + " .. alt .. " + Delete", hl.dsp.exec_cmd("wlogout -p layer-shell"), d("Power menu"))
hl.bind(win .. " + " .. shift .. " + Escape", hl.dsp.window.kill(), d("Force close window"))
hl.bind(win .. " + " .. shift .. " + R", hl.dsp.exec_cmd("hyprctl reload"), d("Reload config"))

--------------------------------------------------------------------------------
--  Notifications and bar
--------------------------------------------------------------------------------

hl.bind(win .. " + N",                   hl.dsp.exec_cmd("swaync-client -t -sw"),  d("Notification centre"))
hl.bind(win .. " + " .. shift .. " + N", hl.dsp.exec_cmd("swaync-client -d -sw"),  d("Do not disturb"))
hl.bind(win .. " + B",                   hl.dsp.exec_cmd("pkill -SIGUSR1 waybar"), d("Toggle the bar"))

--------------------------------------------------------------------------------
--  Windows
--------------------------------------------------------------------------------

hl.bind(alt .. " + Tab",   hl.dsp.exec_cmd("rofi -show window"), d("Switch window"))
hl.bind(win .. " + Tab",   hl.dsp.exec_cmd("rofi -show window"), d("Task view"))
hl.bind(win .. " + grave", hl.dsp.window.cycle_next(),           d("Cycle windows"))

hl.bind("F11",                          hl.dsp.window.fullscreen({ mode = "fullscreen" }), d("Fullscreen"))
hl.bind(win .. " + " .. shift .. " + F", hl.dsp.window.float(),  d("Toggle floating"))
hl.bind(win .. " + " .. shift .. " + P", hl.dsp.window.pin(),    d("Always on top"))
hl.bind(win .. " + " .. alt .. " + C",   hl.dsp.window.center(), d("Centre window"))

-- Win+M minimises on Windows; the scratchpad is the closest a tiler gets.
hl.bind(win .. " + M",                   hl.dsp.window.move({ workspace = "special:scratchpad" }), d("Minimise window"))
hl.bind(win .. " + " .. shift .. " + M", hl.dsp.workspace.toggle_special("scratchpad"),            d("Restore minimised"))

hl.bind(win .. " + " .. alt .. " + J", hl.dsp.layout("togglesplit"), d("Toggle split direction"))
hl.bind(win .. " + " .. alt .. " + T", hl.dsp.group.toggle(),        d("Toggle tab group"))
hl.bind(ctrl .. " + Tab",              hl.dsp.group.next(),          d("Next tab in group"))

--------------------------------------------------------------------------------
--  Aero Snap — Win + arrows, the shortcut this profile exists for
--------------------------------------------------------------------------------

hl.bind(win .. " + left",  s.snap_to("left"),     d("Snap left half"))
hl.bind(win .. " + right", s.snap_to("right"),    d("Snap right half"))
hl.bind(win .. " + up",    s.snap_to("maximise"), d("Maximise"))
hl.bind(win .. " + down",  hl.dsp.window.float({ action = "disable" }), d("Restore to tiling"))

-- Ultrawide extras: thirds have no Windows equivalent, so they go on the digits.
local zones = {
    { "1", "left-third",  "Snap to left third" },
    { "2", "centre",      "Snap to centre third" },
    { "3", "right-third", "Snap to right third" },
    { "4", "top-left",    "Snap to top-left quarter" },
    { "5", "top-right",   "Snap to top-right quarter" },
    { "6", "bottom-left", "Snap to bottom-left quarter" },
}
for _, z in ipairs(zones) do
    hl.bind(win .. " + " .. alt .. " + " .. z[1], s.snap_to(z[2]), d(z[3]))
end

-- Focus, move and resize.
for _, dir in ipairs({ "left", "right", "up", "down" }) do
    hl.bind(win .. " + " .. alt .. " + " .. dir,
            hl.dsp.focus({ direction = dir }), d("Focus " .. dir))
    hl.bind(win .. " + " .. alt .. " + " .. shift .. " + " .. dir,
            hl.dsp.window.move({ direction = dir }), d("Move window " .. dir))
end

local step = 40
for dir, delta in pairs({
    left  = { -step, 0 }, right = { step, 0 },
    up    = { 0, -step },  down = { 0, step },
}) do
    hl.bind(ctrl .. " + " .. alt .. " + " .. dir,
            hl.dsp.window.resize({ x = delta[1], y = delta[2], relative = true }),
            { repeating = true, description = "Resize window" })
end

s.mouse(win)

--------------------------------------------------------------------------------
--  Virtual desktops
--------------------------------------------------------------------------------

s.workspaces(win, win .. " + " .. shift)

hl.bind(ctrl .. " + " .. win .. " + right", hl.dsp.focus({ workspace = "e+1" }), d("Next desktop"))
hl.bind(ctrl .. " + " .. win .. " + left",  hl.dsp.focus({ workspace = "e-1" }), d("Previous desktop"))

--------------------------------------------------------------------------------
--  Screenshots — Win+Shift+S is the snipping tool
--------------------------------------------------------------------------------

local shot = s.scripts .. "/screenshot.sh"

hl.bind(win .. " + " .. shift .. " + S", hl.dsp.exec_cmd(shot .. " region"), d("Snip a region"))
hl.bind("Print",                         hl.dsp.exec_cmd(shot .. " screen"), d("Screenshot the screen"))
hl.bind(alt .. " + Print",               hl.dsp.exec_cmd(shot .. " window"), d("Screenshot a window"))
hl.bind(win .. " + " .. shift .. " + C", hl.dsp.exec_cmd("hyprpicker -a -n"), d("Pick a colour"))

s.media(win .. " + " .. alt)

hl.bind(win .. " + " .. alt .. " + K", hl.dsp.exec_cmd(s.scripts .. "/keymap.sh toggle"), d("Switch keymap profile"))
