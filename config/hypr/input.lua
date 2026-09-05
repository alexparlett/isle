--------------------------------------------------------------------------------
--  Input
--  https://wiki.hypr.land/configuring/core/config-options/#input
--------------------------------------------------------------------------------

-- Mac keycaps: the ⌘ cap sits on the physical Alt key, so swap Alt and Super
-- to make it send SUPER — which is what binds.lua treats as ⌘.
--
-- Unless the keyd profile is installed, in which case keyd already does the
-- swap at the evdev level and doing it again here would undo it.
local function keyd_owns_the_swap()
    local f = io.open("/etc/keyd/default.conf", "r")
    if not f then return false end
    local body = f:read("*a")
    f:close()
    return body:find("hypr%-mac%-profile") ~= nil
end

hl.config({
    input = {
        kb_layout  = "gb", -- matches KEYMAP=uk in /etc/vconsole.conf
        kb_variant = "",
        kb_model   = "",
        kb_options = keyd_owns_the_swap() and "" or "altwin:swap_alt_win",
        kb_rules   = "",

        follow_mouse       = 1,
        numlock_by_default = true,

        sensitivity   = 0,      -- 0 = libinput default, no modification
        accel_profile = "flat", -- raw mouse input; what you want for games

        touchpad = {
            natural_scroll       = true,
            disable_while_typing = true,
        },
    },

    misc = {
        focus_on_activate          = false, -- apps don't get to steal focus
        mouse_move_focuses_monitor = true,
        middle_click_paste         = false, -- accidental paste is never wanted
    },
})

-- Three-finger horizontal swipe changes workspace (touchpads only; harmless
-- on a desktop).
hl.gesture({ fingers = 3, direction = "horizontal", action = "workspace" })
