--------------------------------------------------------------------------------
--  Gaming — RTX 5070 Ti on a 3440x1440@144 ultrawide
--
--  Three things actually matter here:
--    1. `immediate` (tearing) on games that want the latency
--    2. direct scanout, so a fullscreen game bypasses compositing (look.lua)
--    3. no blur/shadow/animations on game surfaces
--------------------------------------------------------------------------------

local GAMES = "^(steam_app_\\d+|gamescope|lutris|net\\.lutris\\.Lutris|heroic)$"

-- Tearing. The master switch is general.allow_tearing in look.lua; only windows
-- with this rule actually tear, and only when fullscreen and alone on screen.
hl.window_rule({
    name  = "games-allow-tearing",
    match = { class = "^(steam_app_\\d+|gamescope)$" },

    immediate = true,
})

-- Tell Hyprland these really are games, which is what direct_scanout = 2 keys
-- off, and keep the compositor out of the way.
hl.window_rule({
    name  = "games-no-effects",
    match = { class = GAMES },

    content      = "game",
    no_blur      = true,
    no_shadow    = true,
    no_anim      = true,
    opacity      = "1.0 override 1.0 override 1.0 override",
    idle_inhibit = "fullscreen",
})

-- Lock the pointer to the game so it can't wander off during a fullscreen fight.
hl.window_rule({
    name  = "games-confine-pointer",
    match = { class = "^(steam_app_\\d+|gamescope)$", fullscreen = true },

    confine_pointer = true,
})

hl.window_rule({
    name  = "gamescope-fullscreen",
    match = { class = "^gamescope$" },

    fullscreen = true,
})

-- Video shouldn't trip the idle timer either.
hl.window_rule({
    name  = "media-idle-inhibit",
    match = { class = "^(firefox|zen|chromium|brave-browser|mpv|vlc)$" },

    idle_inhibit = "fullscreen",
})

--------------------------------------------------------------------------------
--  gamescope is the reliable way to run a game at a non-native resolution or
--  with FSR on an ultrawide. Steam launch options, for reference:
--
--    native 21:9, 144 Hz, with gamemode:
--      gamescope -W 3440 -H 1440 -r 144 -f -- gamemoderun %command%
--
--    a 16:9 title, upscaled and pillarboxed:
--      gamescope -W 2560 -H 1440 -w 1920 -h 1080 -U -f -- gamemoderun %command%
--------------------------------------------------------------------------------
