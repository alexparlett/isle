--------------------------------------------------------------------------------
--  Window, layer and workspace rules
--  https://wiki.hypr.land/configuring/core/rules/
--
--  Rules are processed top to bottom and the LAST match wins.
--------------------------------------------------------------------------------

--------------------------------------------------------------------------------
--  Housekeeping
--------------------------------------------------------------------------------

hl.window_rule({
    name  = "suppress-maximize",
    match = { class = ".*" },

    suppress_event = "maximize",
})

hl.window_rule({
    name  = "fix-xwayland-drags",
    match = { class = "^$", title = "^$", xwayland = true, float = true, fullscreen = false, pin = false },

    no_focus = true,
})

--------------------------------------------------------------------------------
--  Floating dialogs
--------------------------------------------------------------------------------

hl.window_rule({
    name  = "float-settings-apps",
    match = { class = "^(pavucontrol|org\\.pulseaudio\\.pavucontrol|blueman-manager|nm-connection-editor|nwg-look|qt6ct|qt5ct|kvantummanager|org\\.kde\\.kwalletmanager5)$" },

    float  = true,
    size   = { 900, 620 },
    center = true,
})

hl.window_rule({
    name  = "float-portals-and-auth",
    match = { class = "^(xdg-desktop-portal-gtk|hyprpolkitagent|org\\.kde\\.polkit-kde-authentication-agent-1)$" },

    float  = true,
    center = true,
})

hl.window_rule({
    name  = "float-file-dialogs",
    match = { title = "^(Open File|Save File|Save As|Open Folder|Choose Files|Select a File)$" },

    float  = true,
    center = true,
})

hl.window_rule({
    name  = "float-modals",
    match = { modal = true },

    float  = true,
    center = true,
})

--------------------------------------------------------------------------------
--  Picture-in-picture: small, pinned, bottom right, out of the way.
--------------------------------------------------------------------------------

hl.window_rule({
    name  = "picture-in-picture",
    match = { title = "^(Picture-in-Picture)$" },

    float = true,
    pin   = true,
    size  = { 640, 360 },
    move  = { "monitor_w-660", "monitor_h-420" },
})

--------------------------------------------------------------------------------
--  Opacity — terminals and the file manager get a hint of transparency;
--  anything where colour accuracy matters stays fully opaque.
--------------------------------------------------------------------------------

hl.window_rule({
    name    = "translucent-terminals",
    match   = { class = "^(Alacritty|org\\.kde\\.dolphin)$" },
    opacity = "0.95 0.88",
})

hl.window_rule({
    name    = "opaque-media",
    match   = { class = "^(firefox|zen|chromium|brave-browser|mpv|vlc|gimp|org\\.kde\\.krita|obs)$" },
    opacity = "1.0 override 1.0 override 1.0 override",
})

--------------------------------------------------------------------------------
--  Privacy — never leak a password manager or an auth prompt into a share.
--------------------------------------------------------------------------------

-- Hyprland matches with RE2, so (?i) works and saves enumerating capitalisation
-- variants — Electron apps are inconsistent about whether the class is the
-- binary name or the product name.
--
-- Covers what is actually installed (Proton Mail, and Proton Pass if you add
-- it — `pacman -S proton-pass`, it is in the cachyos repo) as well as the usual
-- password managers, so the rule keeps working if you switch.
hl.window_rule({
    name  = "hide-secrets-from-screenshare",
    match = { class = "(?i)^(proton[- ]?(mail|pass)|1password|bitwarden|keepassxc|org\\.keepassxc\\.KeePassXC|gnome-keyring.*|seahorse|kwalletmanager5?)$" },

    no_screen_share = true,
})

-- Auth prompts, separately, because these are transient and should also never
-- be captured mid-share.
hl.window_rule({
    name  = "hide-auth-prompts-from-screenshare",
    match = { class = "(?i)^(hyprpolkitagent|org\\.kde\\.polkit-kde-authentication-agent-1|polkit-gnome-authentication-agent-1)$" },

    no_screen_share = true,
})

--------------------------------------------------------------------------------
--  Workspace assignments
--------------------------------------------------------------------------------

hl.window_rule({
    name      = "browsers-to-2",
    match     = { class = "^(firefox|zen|chromium|brave-browser)$" },
    workspace = "2 silent",
})

hl.window_rule({
    name      = "chat-to-4",
    match     = { class = "^(discord|vesktop|Slack|element)$" },
    workspace = "4 silent",
})

hl.window_rule({
    name      = "games-to-5",
    match     = { class = "^(steam|lutris|net\\.lutris\\.Lutris|heroic)$" },
    workspace = "5 silent",
})

--------------------------------------------------------------------------------
--  Steam: everything except the main window is an XWayland child that should
--  never be tiled.
--------------------------------------------------------------------------------

hl.window_rule({
    name  = "steam-children-float",
    match = { class = "^steam$", title = "^(?!Steam$).*$" },

    float = true,
})

hl.window_rule({
    name  = "steam-invisible-helper",
    match = { class = "^steam$", title = "^$" },

    stay_focused = true,
    min_size     = { 1, 1 },
})

--------------------------------------------------------------------------------
--  Scratchpad
--------------------------------------------------------------------------------

hl.workspace_rule({
    workspace        = "special:scratchpad",
    on_created_empty = "alacritty",
})

hl.window_rule({
    name  = "scratchpad-geometry",
    match = { workspace = "special:scratchpad" },

    float  = true,
    size   = { "monitor_w*0.6", "monitor_h*0.6" },
    center = true,
})

--------------------------------------------------------------------------------
--  AI assistants — Claude and ChatGPT, docked as right-hand columns.
--
--  Each gets its own special workspace so it overlays whatever you are doing
--  and disappears again without touching the layout underneath. scripts/ai.sh
--  toggles them; SUPER+A and SUPER+SHIFT+A are the binds.
--
--  Both are Electron. Claude declares StartupWMClass=com.anthropic.Claude;
--  ChatGPT declares nothing, so the class match is deliberately loose.
--------------------------------------------------------------------------------

local AI_PANELS = {
    { class = "^com\\.anthropic\\.Claude$", workspace = "claude" },
    { class = "^([Cc]hat[Gg][Pp][Tt])$",    workspace = "chatgpt" },
}

for _, panel in ipairs(AI_PANELS) do
    -- Send the app to its panel workspace and show it on launch.
    hl.window_rule({
        name      = "ai-" .. panel.workspace .. "-workspace",
        match     = { class = panel.class },
        workspace = "special:" .. panel.workspace,
    })

    -- A 1000px column down the right-hand edge: wide enough to read code in,
    -- narrow enough to leave 2400px of actual work visible beside it.
    hl.window_rule({
        name  = "ai-" .. panel.workspace .. "-geometry",
        match = { class = panel.class },

        float = true,
        size  = { 1000, "monitor_h-100" },
        move  = { "monitor_w-1020", 50 },
    })
end

-- No dimming behind them — the point is to read both at once.
hl.config({ decoration = { dim_special = 0.0 } })

--------------------------------------------------------------------------------
--  Layers — the bar, launcher and notifications get blur.
--------------------------------------------------------------------------------

for _, ns in ipairs({ "waybar", "rofi", "swaync-control-center", "swaync-notification-window", "swayosd" }) do
    hl.layer_rule({ match = { namespace = "^" .. ns .. "$" }, blur = true, ignore_alpha = 0.3 })
end

hl.layer_rule({ match = { namespace = "^wlogout$" }, blur = true, ignore_alpha = 0.1 })
hl.layer_rule({ match = { namespace = "^hyprpicker$" }, no_anim = true })
hl.layer_rule({ match = { namespace = "^selection$" }, no_anim = true })
