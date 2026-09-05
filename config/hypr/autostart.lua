--------------------------------------------------------------------------------
--  Autostart
--  https://wiki.hypr.land/configuring/core/autostart/
--------------------------------------------------------------------------------

hl.on("hyprland.start", function()
    -- Hand the session environment to systemd --user and D-Bus. Without this,
    -- xdg-desktop-portal and friends come up with an empty WAYLAND_DISPLAY and
    -- misbehave in ways that are miserable to debug.
    hl.exec_cmd("dbus-update-activation-environment --systemd --all")
    hl.exec_cmd("systemctl --user import-environment DISPLAY WAYLAND_DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_TYPE HYPRLAND_INSTANCE_SIGNATURE")

    -- Authentication dialogs (GUI apps asking for root, udisks mounts, ...)
    hl.exec_cmd("systemctl --user start hyprpolkitagent")

    -- Bar, notifications, on-screen volume popups
    hl.exec_cmd("waybar")
    hl.exec_cmd("swaync")
    hl.exec_cmd("swayosd-server")

    -- Wallpaper and idle management
    hl.exec_cmd("hyprpaper")
    hl.exec_cmd("hypridle")

    -- Clipboard history — text and images, queried with SUPER+V
    hl.exec_cmd("wl-paste --type text --watch cliphist store")
    hl.exec_cmd("wl-paste --type image --watch cliphist store")

    -- Tray applets
    hl.exec_cmd("nm-applet --indicator")
    hl.exec_cmd("blueman-applet")
    hl.exec_cmd("udiskie --tray")

    -- Plasma runs ~/.config/autostart/*.desktop for you; Hyprland does not.
    -- dex replays them so nothing you already rely on silently stops starting.
    hl.exec_cmd("dex --autostart --environment Hyprland")

    -- Night light. hyprsunset idles until something asks it for a temperature.
    hl.exec_cmd("hyprsunset")

    -- Cursor theme for XWayland clients, which ignore the env vars.
    hl.exec_cmd("hyprctl setcursor " .. require("theme").cursor.theme .. " " .. require("theme").cursor.size)
end)
