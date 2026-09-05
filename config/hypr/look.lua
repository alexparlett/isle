--------------------------------------------------------------------------------
--  Look and feel — Catppuccin Mocha
--  https://wiki.hypr.land/configuring/core/config-options/
--------------------------------------------------------------------------------

local t = require("theme")

hl.config({
    general = {
        gaps_in  = 5,
        gaps_out = 10,

        border_size = 2,

        col = {
            active_border   = { colors = { t.rgba("mauve", "ee"), t.rgba("blue", "ee") }, angle = 45 },
            inactive_border = t.rgba("surface0", "aa"),
        },

        resize_on_border = true,

        -- Master switch. Individual games opt in with the `immediate` rule in
        -- gaming.lua; nothing tears without that.
        allow_tearing = true,

        layout = "dwindle",

        snap = {
            enabled = true,
        },
    },

    decoration = {
        rounding       = 10,
        rounding_power = 2,

        active_opacity   = 1.0,
        inactive_opacity = 1.0,

        shadow = {
            enabled      = true,
            range        = 20,
            render_power = 3,
            color        = t.argb("crust", "aa"),
        },

        blur = {
            enabled           = true,
            size              = 6,
            passes            = 3,
            new_optimizations = true,
            ignore_opacity    = true,
            popups            = true,
            vibrancy          = 0.1696,
        },
    },

    dwindle = {
        preserve_split = true,
        -- Drop a dragged window where the cursor actually is, rather than
        -- wherever the tree thinks it belongs. This is the difference between
        -- dragging feeling like rearranging windows and feeling like a fight.
        precise_mouse_move = true,
        -- On a 3440px-wide screen a new window should land beside the old one
        -- rather than under it until things actually get narrow.
        force_split            = 2,
        split_width_multiplier = 1.35,
    },

    master = {
        new_status = "master",
        mfact      = 0.5,
    },

    misc = {
        disable_hyprland_logo    = true,
        disable_splash_rendering = true,
        force_default_wallpaper  = 0,
        background_color         = t.argb("base"),

        font_family = t.font,

        -- Adaptive sync for fullscreen apps only. Full-desktop VRR makes this
        -- AOC panel flicker in brightness when the screen is mostly static.
        vrr = 2,

        on_focus_under_fullscreen = 2, -- focusing a tiled window un-fullscreens
    },

    cursor = {
        -- 0 = use hardware cursors. Fine on nvidia 555+; flip to 1 if the
        -- cursor ever stutters or disappears.
        no_hardware_cursors = 0,
        inactive_timeout    = 5,
        hide_on_key_press   = true,
    },

    render = {
        -- 2 = auto: a fullscreen window tagged as `game` content bypasses
        -- compositing entirely.
        direct_scanout = 2,
    },

    xwayland = {
        force_zero_scaling = true,
    },

    ecosystem = {
        no_update_news  = true,
        no_donation_nag = true,
    },
})

--------------------------------------------------------------------------------
--  Animations — tuned for a 144 Hz panel: quick, but not instant.
--------------------------------------------------------------------------------

hl.curve("easeOutQuint",   { type = "bezier", points = { { 0.23, 1 },   { 0.32, 1 } } })
hl.curve("easeInOutCubic", { type = "bezier", points = { { 0.65, 0.05 }, { 0.36, 1 } } })
hl.curve("linear",         { type = "bezier", points = { { 0, 0 },      { 1, 1 } } })
hl.curve("almostLinear",   { type = "bezier", points = { { 0.5, 0.5 },  { 0.75, 1 } } })
hl.curve("quick",          { type = "bezier", points = { { 0.15, 0 },   { 0.1, 1 } } })
hl.curve("snappy",         { type = "spring", mass = 1, stiffness = 250, damping = 24 })

hl.config({ animations = { enabled = true } })

hl.animation({ leaf = "global",       enabled = true, speed = 8 })
hl.animation({ leaf = "border",       enabled = true, speed = 5,    bezier = "easeOutQuint" })
hl.animation({ leaf = "windows",      enabled = true, speed = 4.5,  spring = "snappy" })
hl.animation({ leaf = "windowsIn",    enabled = true, speed = 4,    spring = "snappy",       style = "popin 90%" })
hl.animation({ leaf = "windowsOut",   enabled = true, speed = 1.5,  bezier = "linear",       style = "popin 90%" })
hl.animation({ leaf = "fadeIn",       enabled = true, speed = 1.7,  bezier = "almostLinear" })
hl.animation({ leaf = "fadeOut",      enabled = true, speed = 1.5,  bezier = "almostLinear" })
hl.animation({ leaf = "fade",         enabled = true, speed = 3,    bezier = "quick" })
hl.animation({ leaf = "layers",       enabled = true, speed = 3.8,  bezier = "easeOutQuint" })
hl.animation({ leaf = "layersIn",     enabled = true, speed = 4,    bezier = "easeOutQuint", style = "fade" })
hl.animation({ leaf = "layersOut",    enabled = true, speed = 1.5,  bezier = "linear",       style = "fade" })
hl.animation({ leaf = "workspaces",   enabled = true, speed = 2,    bezier = "almostLinear", style = "fade" })
hl.animation({ leaf = "workspacesIn", enabled = true, speed = 1.2,  bezier = "almostLinear", style = "fade" })
hl.animation({ leaf = "workspacesOut",enabled = true, speed = 2,    bezier = "almostLinear", style = "fade" })
