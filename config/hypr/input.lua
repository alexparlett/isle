--------------------------------------------------------------------------------
--  Input
--  https://wiki.hypr.land/configuring/core/config-options/#input
--------------------------------------------------------------------------------

hl.config({
    input = {
        kb_layout  = "gb", -- matches KEYMAP=uk in /etc/vconsole.conf
        kb_variant = "",
        kb_model   = "",
        kb_options = "",
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
