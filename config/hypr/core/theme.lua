--------------------------------------------------------------------------------
--  Theme — the single source of colour, type and metric for the whole desktop
--
--  Every colour in this desktop comes from here. Waybar, rofi, swaync, kitty,
--  wlogout, hyprlock and Hyprland itself are rendered from this table by
--  `tools/render-theme.lua`, so changing an accent means changing one line
--  rather than hunting through six files that had drifted copies of the same
--  hex codes.
--
--  Swapping palettes is swapping `M.palette`. Everything downstream is
--  expressed in semantic names — `accent`, `surface`, `urgent` — so a new
--  palette does not require knowing which of thirty widgets used `mauve`.
--------------------------------------------------------------------------------

local M = {}

--------------------------------------------------------------------------------
--  Palette — Catppuccin Mocha
--  https://github.com/catppuccin/catppuccin
--------------------------------------------------------------------------------

M.palette = {
    rosewater = "f5e0dc", flamingo = "f2cdcd", pink     = "f5c2e7",
    mauve     = "cba6f7", red      = "f38ba8", maroon   = "eba0ac",
    peach     = "fab387", yellow   = "f9e2af", green    = "a6e3a1",
    teal      = "94e2d5", sky      = "89dceb", sapphire = "74c7ec",
    blue      = "89b4fa", lavender = "b4befe",
    text      = "cdd6f4", subtext1 = "bac2de", subtext0 = "a6adc8",
    overlay2  = "9399b2", overlay1 = "7f849c", overlay0 = "6c7086",
    surface2  = "585b70", surface1 = "45475a", surface0 = "313244",
    base      = "1e1e2e", mantle   = "181825", crust    = "11111b",
}

--------------------------------------------------------------------------------
--  Semantic roles
--
--  Widgets reference these, never the palette directly. That indirection is
--  what makes a palette swap a one-line change: a green-accented theme only
--  has to say `accent = "green"`, not re-point every consumer.
--------------------------------------------------------------------------------

M.roles = {
    accent      = "mauve",     -- focus, selection, the active thing
    accent_alt  = "blue",      -- the second half of gradients
    background  = "base",
    surface     = "mantle",    -- cards, bars, popups sitting on the background
    surface_alt = "surface0",  -- borders and dividers
    foreground  = "text",
    muted       = "overlay0",  -- disabled, placeholder, secondary
    dim         = "subtext0",
    urgent      = "red",
    warning     = "peach",
    caution     = "yellow",
    success     = "green",
    info        = "sapphire",
    shadow      = "crust",
}

--------------------------------------------------------------------------------
--  Type and metrics
--------------------------------------------------------------------------------

M.font = {
    family   = "JetBrainsMono Nerd Font",
    ui       = "Noto Sans",
    size     = 11,
    bar_size = 13,
}

M.metrics = {
    radius     = 12,   -- corner rounding, shared by every surface
    gap        = 10,   -- the one spacing unit; everything else is a multiple
    border     = 2,
    bar_height = 38,
}

M.cursor = { theme = "catppuccin-mocha-dark-cursors", size = 24 }

--------------------------------------------------------------------------------
--  Accessors
--
--  Colours are stored bare ("cba6f7") so each consumer can wear its own
--  syntax. Everything that needs one goes through these rather than
--  concatenating hashes by hand.
--------------------------------------------------------------------------------

--- Resolve a role or a raw palette name to its six hex digits.
function M.hex(name)
    local role = M.roles[name]
    return M.palette[role or name]
        or error("theme: unknown colour '" .. tostring(name) .. "'", 2)
end

--- "#cba6f7" — CSS, kitty, rasi.
function M.css(name) return "#" .. M.hex(name) end

--- "rgba(cba6f7ee)" — Hyprland colour options.
function M.rgba(name, alpha) return "rgba(" .. M.hex(name) .. (alpha or "ff") .. ")" end

--- 0xAARRGGBB — Hyprland shadow and background colours.
function M.argb(name, alpha) return tonumber((alpha or "ff") .. M.hex(name), 16) end

--- "cba6f7ff" — slurp, and anything else wanting bare RGBA.
function M.raw(name, alpha) return M.hex(name) .. (alpha or "ff") end

return M
