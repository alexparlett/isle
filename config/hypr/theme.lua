--------------------------------------------------------------------------------
-- Catppuccin Mocha palette, shared by every other config module.
-- https://github.com/catppuccin/catppuccin
--------------------------------------------------------------------------------

local M = {}

M.hex = {
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

--- "rgba(cba6f7ee)" string, the form Hyprland colour options want.
--- @param name string key from M.hex
--- @param alpha string|nil two hex digits, defaults to "ff"
function M.rgba(name, alpha)
    return "rgba(" .. M.hex[name] .. (alpha or "ff") .. ")"
end

--- 0xAARRGGBB integer, which shadow and background colours want instead.
function M.argb(name, alpha)
    return tonumber((alpha or "ff") .. M.hex[name], 16)
end

M.font   = "JetBrainsMono Nerd Font"
M.cursor = { theme = "catppuccin-mocha-dark-cursors", size = 24 }

return M
