--------------------------------------------------------------------------------
--  Which keybinding profile is active
--
--  Two are shipped:
--    mac      ⌘Space launcher, ⌃⌘F fullscreen, ⌘⇧4 region shot, ⌘, settings
--    windows  Win+S search, Win+arrows snap, Alt+F4 close, Alt+Tab, Win+I
--
--  Chosen by a one-word file, ~/.config/hypr/keymap, because binds are
--  registered while the config loads — local.lua is required last and would be
--  far too late to influence them.
--
--  Switch with `scripts/keymap.sh windows` or the settings window; both write
--  the file and reload. The physical Alt/Super swap in input.lua is separate
--  and stays put either way: it is about the keycaps on the board, not about
--  which shortcuts you prefer.
--------------------------------------------------------------------------------

local DEFAULT = "mac"
local VALID = { mac = true, windows = true }

local function read_choice()
    -- Env override, for testing both profiles and for launching Hyprland once
    -- with the other one without touching the file.
    local override = os.getenv("HYPR_KEYMAP")
    if override and VALID[override:lower()] then
        return override:lower()
    end

    local path = (os.getenv("HOME") or "") .. "/.config/hypr/keymap"
    local file = io.open(path, "r")
    if not file then return DEFAULT end

    local body = file:read("*l") or ""
    file:close()

    local name = body:match("^%s*([%w_-]+)")
    if name and VALID[name:lower()] then
        return name:lower()
    end
    return DEFAULT
end

local M = { name = read_choice(), default = DEFAULT }

function M.is(profile) return M.name == profile end

return M
