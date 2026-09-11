-- Rendered from shell/theme/tokens.json by theme/render.py. Edit the tokens, not this.
hl.config({
    general = {
        col = {
            active_border = "rgba({{border_active}})",
            inactive_border = "rgba({{border_inactive}})",
        },
    },
    decoration = {
        rounding = {{radius_control}},
        rounding_power = 2,
        blur = {
            enabled = true,
            size = {{blur_size}},
            passes = {{blur_passes}},
            new_optimizations = true,
            xray = false,
            noise = 0.01,
            contrast = 1.0,
            brightness = 0.9,
            vibrancy = 0.15,
            popups = true,
            popups_ignorealpha = 0.2,
        },
    },
})

-- Title bars, when the hyprbars plugin is loaded (tools/bootstrap.sh builds it with hyprpm).
-- Buttons add right to left: close, fullscreen, then minimise to the shell's hidden stack.
if hl.plugin and hl.plugin.hyprbars and not {{bars_off}} then
    local function rgb(h) return tonumber("ff" .. h:sub(2), 16) end
    hl.config({ plugin = { hyprbars = {
        bar_height = 28, bar_color = rgb("{{window}}"), ["col.text"] = rgb("{{text2}}"),
        bar_text_font = "{{font_ui}}", bar_text_size = 11, bar_text_align = "left",
        bar_buttons_alignment = "right", bar_part_of_window = true, bar_precedence_over_border = true,
        bar_padding = 12, bar_button_padding = 8, icon_on_hover = true, inactive_button_color = rgb("{{pressed}}"),
        -- A double click on the bar fills the work area, and restores when already filled.
        on_double_click = "hyprctl isle zoom",
    } } })
    {{bars_except_line}}
    hl.plugin.hyprbars.add_button({ bg_color = rgb("{{danger}}"), fg_color = rgb("{{window}}"), size = 12, icon = "×", action = "hyprctl dispatch 'hl.dsp.window.close()'" })
    hl.plugin.hyprbars.add_button({ bg_color = rgb("{{ok}}"), fg_color = rgb("{{window}}"), size = 12, icon = "+", action = "hyprctl dispatch 'hl.dsp.window.fullscreen({ mode = \"maximized\" })'" })
    hl.plugin.hyprbars.add_button({ bg_color = rgb("{{warn}}"), fg_color = rgb("{{window}}"), size = 12, icon = "–", action = "qs -p $HOME/.config/quickshell/isle ipc call switcher hide" })
end
