-- Hyprland, configured for the shell. The shell writes hypr/generated/*.lua; this file owns the rest.
--
-- ISLE_SHELL_PATH points at the shell's Quickshell config; unset, the systemd unit's default applies.

package.path = (os.getenv("XDG_CONFIG_HOME") or (os.getenv("HOME") .. "/.config")) .. "/hypr/?.lua;" .. package.path

-- Monitors: the fragment Settings renders, else everything preferred and auto.
local cfgdir = (os.getenv("XDG_CONFIG_HOME") or (os.getenv("HOME") .. "/.config")) .. "/hypr"
local okm, mons = pcall(loadfile, cfgdir .. "/generated/monitors.lua")
if okm and mons then mons() else hl.monitor({ output = "", mode = "preferred", position = "auto", scale = "auto" }) end

hl.env("XCURSOR_SIZE", "24")
hl.env("HYPRCURSOR_SIZE", "24")
-- Toolkits on Wayland with X11 as the fallback; Qt leaves decorations to the compositor.
hl.env("QT_QPA_PLATFORM", "wayland;xcb")
hl.env("QT_WAYLAND_DISABLE_WINDOWDECORATION", "1")
hl.env("QT_AUTO_SCREEN_SCALE_FACTOR", "1")
hl.env("GDK_BACKEND", "wayland,x11,*")
hl.env("CLUTTER_BACKEND", "wayland")
hl.env("XDG_SESSION_TYPE", "wayland")
hl.env("XDG_SESSION_DESKTOP", "Hyprland")
-- The GTK bridge exists for Qt 5 and Qt 6 alike and yields Isle's GTK palette, font, icons and dialogs; one name serves both.
hl.env("QT_QPA_PLATFORMTHEME", "gtk3")
hl.env("XDG_CURRENT_DESKTOP", "Hyprland")
-- Electron apps follow the session rather than defaulting to X11.
hl.env("ELECTRON_OZONE_PLATFORM_HINT", "auto")
-- With the NVIDIA driver loaded, the two variables its wiki page asks for: VA-API and GLX through NVIDIA.
if io.open("/proc/driver/nvidia/version") then
    hl.env("LIBVA_DRIVER_NAME", "nvidia")
    hl.env("__GLX_VENDOR_LIBRARY_NAME", "nvidia")
end
-- The session's SSH agent: gcr's wrapper around ssh-agent, socket-activated by systemd (tools/install.sh enables it).
hl.env("SSH_AUTH_SOCK", (os.getenv("XDG_RUNTIME_DIR") or ("/run/user/" .. (os.getenv("UID") or "1000"))) .. "/gcr/ssh")

local shell = os.getenv("ISLE_SHELL_PATH") or (os.getenv("HOME") .. "/.config/quickshell/isle")

local terminal = "kitty"
local function ipc(target, fn, arg)
    return hl.dsp.exec_cmd("qs -p " .. shell .. " ipc call " .. target .. " " .. fn .. (arg and (" " .. arg) or ""))
end

hl.on("hyprland.start", function()
    -- Plugins built by hyprpm (hyprbars for title bars), then a reload so the theme fragment sees them.
    hl.exec_cmd("sh -c 'command -v hyprpm >/dev/null && hyprpm reload -n && hyprctl reload'")
    -- Units that need the display: xremap. The shell itself runs in the session scope (D17).
    hl.exec_cmd("systemctl --user import-environment WAYLAND_DISPLAY HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP ISLE_SHELL_PATH"
        .. " && systemctl --user start xremap.service 2>/dev/null")
    hl.exec_cmd("ISLE_SHELL_PATH=" .. shell .. " " .. shell .. "/scripts/isle-session")
end)

-- Rendered by theme/render.py at install; a checkout that has not been installed still starts.
local okt, theme = pcall(loadfile, cfgdir .. "/generated/theme.lua")
if okt and theme then theme() end

hl.config({
    general = {
        gaps_in = 4,
        gaps_out = { top = 48, right = 8, bottom = 8, left = 8 },
        border_size = 1,
        resize_on_border = true,
        -- Floating windows snap to screen edges and to each other while dragged.
        snap = { enabled = true, window_gap = 8, monitor_gap = 8 },
        allow_tearing = false,
        layout = "dwindle",
    },
    decoration = {
        active_opacity = 1.0,
        inactive_opacity = 1.0,
        shadow = { enabled = true, range = 24, render_power = 3, color = 0x66000000 },
    },
    animations = { enabled = true },
    dwindle = { preserve_split = true },
    misc = {
        disable_hyprland_logo = true,
        disable_splash_rendering = true,
        force_default_wallpaper = 0,
        vrr = 1,
        focus_on_activate = true,
        middle_click_paste = false,
    },
    -- Focus does not move the pointer, as on macOS and Windows.
    -- Hardware cursors: the compositor's "auto" turns them off on NVIDIA and draws the cursor into the
    -- frame, where every screenshot then finds it. The open driver handles them.
    cursor = { no_warps = true, no_hardware_cursors = false },
    input = {
        -- The generated input fragment sets the real layout from localectl; an unset one is an error.
        kb_layout = "gb",
        -- Click to focus: the pointer moving over a window does not take the keyboard from the one in use.
        follow_mouse = 2,
        sensitivity = 0,
        accel_profile = "flat",
    },
    render = { direct_scanout = 1 },
})

-- Input: the fragment Settings renders overrides the defaults above.
local oki, inp = pcall(loadfile, cfgdir .. "/generated/input.lua")
if oki and inp then inp() end

hl.curve("easeOutQuint", { type = "bezier", points = { { 0.23, 1 }, { 0.32, 1 } } })
hl.curve("linear", { type = "bezier", points = { { 0, 0 }, { 1, 1 } } })
hl.curve("island", { type = "spring", mass = 1, stiffness = 180, dampening = 22 })

hl.animation({ leaf = "global", enabled = true, speed = 10, bezier = "default" })
hl.animation({ leaf = "windows", enabled = true, speed = 4.5, spring = "island" })
hl.animation({ leaf = "windowsIn", enabled = true, speed = 4, spring = "island", style = "popin 96%" })
hl.animation({ leaf = "windowsOut", enabled = true, speed = 1.5, bezier = "linear", style = "popin 96%" })
hl.animation({ leaf = "fade", enabled = true, speed = 3, bezier = "easeOutQuint" })
hl.animation({ leaf = "layers", enabled = true, speed = 4, bezier = "easeOutQuint" })
hl.animation({ leaf = "layersIn", enabled = true, speed = 4, bezier = "easeOutQuint", style = "fade" })
hl.animation({ leaf = "layersOut", enabled = true, speed = 1.5, bezier = "linear", style = "fade" })
hl.animation({ leaf = "workspaces", enabled = true, speed = 2, bezier = "easeOutQuint", style = "fade" })
hl.animation({ leaf = "border", enabled = true, speed = 5, bezier = "easeOutQuint" })

-- X11 windows carry their own shadows and menus as translucent surfaces; blur under them reads as a smear.
hl.window_rule({ name = "x11-no-blur", match = { xwayland = true }, no_blur = true })
-- Steam's Big Picture takes a workspace of its own, so it is never left behind the desktop's windows.
hl.window_rule({ name = "steam-bigpicture", match = { class = "^(steam|gamescope)$", title = "^Steam Big Picture Mode" }, workspace = "name:steam" })
-- Menus, tooltips and other override-redirect X11 windows, tagged by the isle-windows plugin: drawn as the
-- client placed them, at once, with nothing of the compositor's around them.
hl.window_rule({ name = "x11-popup", match = { tag = "x11popup" }, no_anim = true, no_shadow = true, rounding = 0, no_focus = true, min_size = "1 1" })
-- The shell's surfaces get blur; the wallpaper is under everything and gets none.
hl.layer_rule({ name = "isle-blur", match = { namespace = "^isle-(island|panel|dashboard|launcher|switcher|capture|power|overview)$" }, blur = true, ignore_alpha = 0.2 })
hl.layer_rule({ name = "isle-wallpaper", match = { namespace = "^isle-wallpaper$" }, no_anim = true })

-- Tearing, when game mode allows it, applies to fullscreen windows only.
hl.window_rule({ name = "game-tearing", match = { fullscreen = true }, immediate = true })
-- The dropdown terminal lives on a special workspace, floating across the top.
hl.window_rule({ name = "isle-dropdown", match = { class = "^isle-dropdown$" }, workspace = "special:terminal silent", float = true, size = "70% 50%", move = "15% 60", opacity = 0.96 })
-- The shell's own windows float, centred.
hl.window_rule({ name = "isle-windows", match = { class = "^(quickshell|org\\.quickshell)$" }, float = true, center = true })
-- Browser picture-in-picture: a small pinned float without a border; the shell's Pip service parks it bottom-right.
hl.window_rule({ name = "pip", match = { title = "^(Picture-in-Picture|Picture in picture)$" }, float = true, pin = true, keep_aspect_ratio = true, size = "480 270", border_size = 0 })
hl.window_rule({
    name = "fix-xwayland-drags",
    match = { class = "^$", title = "^$", xwayland = true, float = true, fullscreen = false, pin = false },
    no_focus = true,
})

local fileManager = "kitty -e yazi"

-- Every shortcut: rendered from shell/keymap.json by the shell's Keyboard service.
local ok, binds = pcall(loadfile, (os.getenv("XDG_CONFIG_HOME") or (os.getenv("HOME") .. "/.config")) .. "/hypr/generated/binds.lua")
if ok and binds then binds(ipc, terminal, fileManager) end

-- While Settings records a shortcut every bind steps aside; Escape is the way back if the shell is gone.
hl.define_submap("isle-record", "reset", function()
    hl.bind("Escape", hl.dsp.submap("reset"))
end)

-- Mouse
-- Three fingers up is Mission Control, down closes it.
hl.gesture({ fingers = 3, direction = "up", action = function() hl.dispatch(ipc("surfaces", "overview", "open")) end })
hl.gesture({ fingers = 3, direction = "down", action = function() hl.dispatch(ipc("surfaces", "overview", "close")) end })

hl.bind("SUPER + mouse:272", hl.dsp.window.drag(), { mouse = true })
hl.bind("SUPER + mouse:273", hl.dsp.window.resize(), { mouse = true })
