--------------------------------------------------------------------------------
--  Options — every tunable in one place
--
--  The magic numbers that used to be scattered through modules live here, so
--  changing behaviour means editing one table rather than grepping for a
--  hardcoded 40 and hoping it was the resize step.
--
--  This is the file to read first, and usually the only one to edit. Modules
--  consume it; they do not define policy of their own.
--
--  Machine-local overrides go in ~/.config/hypr/local.lua, which is loaded last
--  and can reach in here before modules read it.
--------------------------------------------------------------------------------

return {
    --- Programs. Every module referring to "the terminal" goes through this.
    apps = {
        terminal   = "kitty",
        files      = { "dolphin", "thunar", "nautilus", "nemo" }, -- first installed wins
        files_tui  = "yazi",
        browser    = "vivaldi-stable",
        editor     = "kitty -e nvim",
        monitor    = "kitty -e btop",
        settings   = "python3 ~/.config/hypr/settings/system-settings.py",
        hypr_tweak = "hyprmod",
    },

    --- The display this config is built around.
    monitor = {
        name    = "DP-4",
        mode    = "3440x1440@144",
        scale   = 1,
        vrr     = 2,          -- 0 off · 1 always · 2 fullscreen only · 3 games
        position = "0x0",
    },

    input = {
        layout          = "gb",
        repeat_delay    = 600,
        repeat_rate     = 25,
        accel_profile   = "flat",  -- raw input; what games want
        sensitivity     = 0,
        natural_scroll  = true,
        numlock         = true,
    },

    --- Which keybind profile: "mac" or "windows". Overridden by
    --- ~/.config/hypr/keymap, which is what the settings window writes.
    keymap = "mac",

    --- Window management.
    window = {
        resize_step   = 40,      -- pixels per resize keypress
        drag_threshold = 8,      -- pixels before a click counts as a drag
        snap_edge     = 40,      -- how close to an edge triggers a snap zone
        gaps_in       = 5,
        gaps_out      = 10,
    },

    --- Idle behaviour, in minutes.
    ---
    --- `defer_while_agent_works` is off: blanking and locking mid-task is fine,
    --- it does not interrupt anything, and the deferral was solving a problem
    --- nobody had. Turn it on to have hypridle consult agent-busy.sh before
    --- blanking. Note Claude Code has its own agentPushNotifEnabled setting for
    --- getting your attention, which may be the better lever anyway.
    idle = {
        blank    = 5,
        lock     = 10,
        mic_mute = 30,
        suspend  = 0,            -- 0 disables; this is a desktop
        defer_while_agent_works = false,
    },

    --- Automatic do-not-disturb.
    dnd = {
        on_screenshare = true,
        on_fullscreen_game = true,
    },

    --- Night light, by time of day.
    night_light = {
        { time = "7:00",  identity = true },
        { time = "21:00", temperature = 4500, gamma = 0.9 },
        { time = "23:30", temperature = 3600, gamma = 0.85 },
    },

    --- Coding agents surfaced in the bar.
    agents = { "claude", "codex" },

    --- Window classes, in one place because three modules match on them.
    classes = {
        games     = "^(steam_app_\\d+|gamescope|lutris|net\\.lutris\\.Lutris|heroic)$",
        tearing   = "^(steam_app_\\d+|gamescope)$",
        browsers  = "^(firefox|zen|chromium|brave-browser|vivaldi-stable)$",
        chat      = "^(discord|vesktop|Slack|element)$",
        media     = "^(mpv|vlc|celluloid)$",
        secrets   = "(?i)^(proton[- ]?(mail|pass)|1password|bitwarden|keepassxc"
                    .. "|org\\.keepassxc\\.KeePassXC|gnome-keyring.*|seahorse|kwalletmanager5?)$",
        auth      = "(?i)^(hyprpolkitagent|org\\.kde\\.polkit-kde-authentication-agent-1"
                    .. "|polkit-gnome-authentication-agent-1)$",
        claude    = "^com\\.anthropic\\.Claude$",
        chatgpt   = "^([Cc]hat[Gg][Pp][Tt])$",
    },

    --- Workspace assignments: class group -> workspace number.
    workspaces = {
        browsers = 2,
        chat     = 4,
        games    = 5,
    },
}
