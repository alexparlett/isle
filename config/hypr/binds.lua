--------------------------------------------------------------------------------
--  Keybindings — Mac layout
--  https://wiki.hypr.land/configuring/core/binds/
--
--  This box has Mac keycaps, so the bottom row reads Ctrl / ⌥ / ⌘. On a PC
--  board the ⌘ cap sits on the physical Alt key, so input.lua applies the xkb
--  swap that makes it send SUPER. Everything below therefore treats:
--
--      SUPER = ⌘        ALT = ⌥        CTRL = ⌃        SHIFT = ⇧
--
--  Shortcuts follow macOS where macOS has a convention (⌘Space launcher,
--  ⌃⌘F fullscreen, ⌘⇧4 region screenshot, ⌃⌘Q lock, ⌘, settings) and stay out
--  of the way where apps already own a combo — nothing here binds bare ⌘C,
--  ⌘S, ⌘F, ⌘A or ⌘←/→, which are text and app shortcuts on every Mac.
--
--  Install the optional keyd profile (./install.sh --mac-keys) to get ⌘C/⌘V/⌘T
--  and friends translating to their Ctrl equivalents inside applications.
--
--  Descriptions are filled in because `hyprctl binds` exposes them — that is
--  what the ⌘? cheatsheet reads.
--------------------------------------------------------------------------------

local cmd     = "SUPER"        -- ⌘
local opt     = "ALT"          -- ⌥
local ctrl    = "CTRL"         -- ⌃
local shift   = "SHIFT"        -- ⇧

local term    = "alacritty"
local scripts = "~/.config/hypr/scripts"

-- Pick whichever file manager is actually installed, so ⌘E keeps working if
-- Dolphin is ever swapped for a GTK one. Checked once at config load, which is
-- cheap; never do this inside a bind callback.
local function first_installed(candidates, fallback)
    for _, name in ipairs(candidates) do
        local handle = io.open("/usr/bin/" .. name, "r")
        if handle then
            handle:close()
            return name
        end
    end
    return fallback
end

local files = first_installed({ "dolphin", "thunar", "nautilus", "nemo" }, "dolphin")

local function d(text) return { description = text } end

--------------------------------------------------------------------------------
--  Launching
--------------------------------------------------------------------------------

-- ⌘Space is Spotlight; keep the muscle memory.
hl.bind(cmd .. " + Space",                hl.dsp.exec_cmd(scripts .. "/launcher.sh"), d("Launch bar"))
hl.bind(cmd .. " + " .. opt .. " + Space", hl.dsp.exec_cmd("rofi -show run"),          d("Run a command"))
-- ⌃⌘Space is the macOS emoji picker.
hl.bind(ctrl .. " + " .. cmd .. " + Space", hl.dsp.exec_cmd("rofi -show emoji -modes emoji"), d("Emoji picker"))

hl.bind(cmd .. " + Return", hl.dsp.exec_cmd(term),  d("Terminal"))
hl.bind(cmd .. " + E",      hl.dsp.exec_cmd(files), d("File manager"))

-- ⌘, is Preferences in every Mac app.
hl.bind(cmd .. " + comma", hl.dsp.exec_cmd("python3 ~/.config/hypr/settings/hypr-settings.py"), d("Settings"))
-- ⌘? is Help.
hl.bind(cmd .. " + " .. shift .. " + slash", hl.dsp.exec_cmd(scripts .. "/cheatsheet.sh"), d("Keybind cheatsheet"))

hl.bind(cmd .. " + " .. shift .. " + V", hl.dsp.exec_cmd(scripts .. "/clipboard.sh"),  d("Clipboard history"))
hl.bind(cmd .. " + " .. opt .. " + U",   hl.dsp.exec_cmd(term .. " -e cachy-update"),  d("System update"))

-- AI assistants, docked as right-hand columns. ⌘A is Select All, so these sit
-- behind ⌥.
hl.bind(cmd .. " + " .. opt .. " + A", hl.dsp.exec_cmd(scripts .. "/ai.sh claude"),  d("Claude panel"))
hl.bind(cmd .. " + " .. opt .. " + G", hl.dsp.exec_cmd(scripts .. "/ai.sh chatgpt"), d("ChatGPT panel"))

--------------------------------------------------------------------------------
--  Session
--------------------------------------------------------------------------------

-- ⌃⌘Q locks the screen on macOS.
hl.bind(ctrl .. " + " .. cmd .. " + Q", hl.dsp.exec_cmd("hyprlock"), d("Lock screen"))
hl.bind(cmd .. " + Escape", hl.dsp.exec_cmd("wlogout -p layer-shell"), d("Power menu"))

-- ⌘Q quits, ⌘W closes the window. With the keyd profile installed, apps see
-- ⌘W as Ctrl+W (close tab) and this becomes the fallback for windows without
-- tabs — which is exactly how it behaves on a Mac.
hl.bind(cmd .. " + Q", hl.dsp.window.close(), d("Close window"))
hl.bind(cmd .. " + W", hl.dsp.window.close(), d("Close window"))
-- ⌥⌘Esc is Force Quit.
hl.bind(opt .. " + " .. cmd .. " + Escape", hl.dsp.window.kill(), d("Force quit window"))

hl.bind(cmd .. " + " .. shift .. " + R", hl.dsp.exec_cmd("hyprctl reload"), d("Reload config"))

--------------------------------------------------------------------------------
--  Notifications and bar
--------------------------------------------------------------------------------

hl.bind(cmd .. " + " .. opt .. " + N", hl.dsp.exec_cmd("swaync-client -t -sw"), d("Notification centre"))
hl.bind(cmd .. " + " .. opt .. " + D", hl.dsp.exec_cmd("swaync-client -d -sw"), d("Do not disturb"))
hl.bind(cmd .. " + " .. opt .. " + B", hl.dsp.exec_cmd("pkill -SIGUSR1 waybar"), d("Toggle the bar"))

--------------------------------------------------------------------------------
--  Windows
--------------------------------------------------------------------------------

-- ⌃⌘F is Enter Full Screen on macOS.
hl.bind(ctrl .. " + " .. cmd .. " + F", hl.dsp.window.fullscreen({ mode = "fullscreen" }), d("Fullscreen"))
hl.bind(opt .. " + " .. cmd .. " + F",  hl.dsp.window.fullscreen({ mode = "maximized" }),  d("Maximise within gaps"))
hl.bind(cmd .. " + " .. shift .. " + F", hl.dsp.window.float(),                            d("Toggle floating"))
hl.bind(cmd .. " + " .. shift .. " + P", hl.dsp.window.pin(),                              d("Pin to all workspaces"))
hl.bind(cmd .. " + " .. opt .. " + C",   hl.dsp.window.center(),                           d("Centre window"))

-- ⌘Tab is the app switcher; ⌘` cycles windows of the same app.
hl.bind(cmd .. " + Tab",   hl.dsp.exec_cmd("rofi -show window"), d("Window switcher"))
hl.bind(cmd .. " + grave", hl.dsp.window.cycle_next(),           d("Cycle windows"))

-- ⌘H hides, ⌘⇧H brings the hidden pile back. Mapped to the scratchpad, which
-- is the closest thing a tiler has to minimising.
hl.bind(cmd .. " + H",                   hl.dsp.window.move({ workspace = "special:scratchpad" }), d("Hide window"))
hl.bind(cmd .. " + " .. shift .. " + H", hl.dsp.workspace.toggle_special("scratchpad"),            d("Show hidden windows"))

hl.bind(cmd .. " + " .. opt .. " + J", hl.dsp.layout("togglesplit"), d("Toggle split direction"))
hl.bind(cmd .. " + " .. opt .. " + T", hl.dsp.group.toggle(),        d("Toggle tab group"))
hl.bind(ctrl .. " + Tab",              hl.dsp.group.next(),          d("Next tab in group"))

-- Focus: ⌥⌘ + arrows. Bare ⌘ + arrows is line-start/end in every Mac text
-- field, so it is deliberately left alone.
for _, dir in ipairs({ "left", "right", "up", "down" }) do
    hl.bind(opt .. " + " .. cmd .. " + " .. dir,
            hl.dsp.focus({ direction = dir }), d("Focus " .. dir))
    hl.bind(opt .. " + " .. cmd .. " + " .. shift .. " + " .. dir,
            hl.dsp.window.move({ direction = dir }), d("Move window " .. dir))
end

-- Resize: ⌃⌘ + arrows, repeatable while held.
local step = 40
for dir, delta in pairs({
    left  = { -step, 0 },
    right = { step, 0 },
    up    = { 0, -step },
    down  = { 0, step },
}) do
    hl.bind(
        ctrl .. " + " .. cmd .. " + " .. dir,
        hl.dsp.window.resize({ x = delta[1], y = delta[2], relative = true }),
        { repeating = true, description = "Resize window" }
    )
end

-- Mouse
hl.bind(cmd .. " + mouse:272", hl.dsp.window.drag(),   { mouse = true, description = "Drag window" })
hl.bind(cmd .. " + mouse:273", hl.dsp.window.resize(), { mouse = true, description = "Resize window" })

--------------------------------------------------------------------------------
--  Ultrawide thirds
--
--  3440 wide, 10px gaps, ~40px bar: three columns of 1133px starting at
--  x = 10 / 1153 / 2296. ⌥⌘0 hands the window back to the tiler.
--------------------------------------------------------------------------------

local COL_W, COL_H, TOP = 1133, 1380, 50

local function snap_to(x)
    return function()
        hl.dispatch(hl.dsp.window.float({ action = "enable" }))
        hl.dispatch(hl.dsp.window.resize({ x = COL_W, y = COL_H, relative = false }))
        hl.dispatch(hl.dsp.window.move({ x = x, y = TOP, relative = false }))
    end
end

hl.bind(opt .. " + " .. cmd .. " + 1", snap_to(10),   d("Snap to left third"))
hl.bind(opt .. " + " .. cmd .. " + 2", snap_to(1153), d("Snap to centre third"))
hl.bind(opt .. " + " .. cmd .. " + 3", snap_to(2296), d("Snap to right third"))
hl.bind(opt .. " + " .. cmd .. " + 0", hl.dsp.window.float({ action = "disable" }), d("Back to tiling"))

--------------------------------------------------------------------------------
--  Workspaces
--
--  ⌘ + digit rather than macOS's ⌃ + digit: on Linux, Ctrl+1..9 is how
--  browsers switch tabs, and losing that hurts more than the exact match helps.
--------------------------------------------------------------------------------

for i = 1, 10 do
    local key = i % 10 -- workspace 10 lives on the 0 key
    hl.bind(cmd .. " + " .. key, hl.dsp.focus({ workspace = i }), d("Workspace " .. i))
    hl.bind(cmd .. " + " .. shift .. " + " .. key,
            hl.dsp.window.move({ workspace = i, follow = false }),
            d("Send window to workspace " .. i))
end

-- ⌃⌥ + arrows: Mission Control's ⌃ + arrows, shifted one modifier out of the
-- way of word-wise text navigation.
hl.bind(ctrl .. " + " .. opt .. " + right", hl.dsp.focus({ workspace = "e+1" }), d("Next workspace"))
hl.bind(ctrl .. " + " .. opt .. " + left",  hl.dsp.focus({ workspace = "e-1" }), d("Previous workspace"))
hl.bind(cmd .. " + mouse_down",             hl.dsp.focus({ workspace = "e+1" }), d("Next workspace"))
hl.bind(cmd .. " + mouse_up",               hl.dsp.focus({ workspace = "e-1" }), d("Previous workspace"))

--------------------------------------------------------------------------------
--  Screenshots — the macOS combos, plus the PrintScreen key for muscle memory
--  that predates the Mac.
--------------------------------------------------------------------------------

local shot = scripts .. "/screenshot.sh"

hl.bind(cmd .. " + " .. shift .. " + 3", hl.dsp.exec_cmd(shot .. " screen"), d("Screenshot the screen"))
hl.bind(cmd .. " + " .. shift .. " + 4", hl.dsp.exec_cmd(shot .. " region"), d("Screenshot a region"))
hl.bind(cmd .. " + " .. shift .. " + 5", hl.dsp.exec_cmd(shot .. " edit"),   d("Screenshot and annotate"))
hl.bind(cmd .. " + " .. shift .. " + 6", hl.dsp.exec_cmd(shot .. " window"), d("Screenshot a window"))

hl.bind("Print",         hl.dsp.exec_cmd(shot .. " region"), d("Screenshot a region"))
hl.bind(shift .. " + Print", hl.dsp.exec_cmd(shot .. " screen"), d("Screenshot the screen"))

hl.bind(cmd .. " + " .. opt .. " + P", hl.dsp.exec_cmd("hyprpicker -a -n"), d("Pick a colour"))

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

-- Mac keyboards put volume on F10-F12 with no XF86 codes when the function-key
-- row is in F-mode; these cover that case.
hl.bind(cmd .. " + F10", hl.dsp.exec_cmd("swayosd-client --output-volume mute-toggle"), media)
hl.bind(cmd .. " + F11", hl.dsp.exec_cmd("swayosd-client --output-volume lower"),       held)
hl.bind(cmd .. " + F12", hl.dsp.exec_cmd("swayosd-client --output-volume raise"),       held)

hl.bind(cmd .. " + " .. opt .. " + F1", hl.dsp.exec_cmd("hyprctl hyprsunset temperature 4000"), d("Warm the display"))
hl.bind(cmd .. " + " .. opt .. " + F2", hl.dsp.exec_cmd("hyprctl hyprsunset identity"),         d("Normal colour temperature"))
