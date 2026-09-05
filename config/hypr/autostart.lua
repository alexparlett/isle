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

    -- Secret Service. pam_gnome_keyring (auto_start in /etc/pam.d/sddm) brings
    -- the daemon up and unlocks it with your login password, but the secrets
    -- and ssh components are normally started by autostart entries marked
    -- OnlyShowIn=GNOME, which dex correctly skips. Without this, anything
    -- asking for org.freedesktop.secrets finds nobody home — including the
    -- Proton packages. Idempotent: it attaches to the running daemon.
    hl.exec_cmd("gnome-keyring-daemon --start --components=secrets,pkcs11,ssh")

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
    -- It honours OnlyShowIn, so the KDE-only entries (plasmashell, powerdevil,
    -- kglobalacceld, baloo) are correctly skipped while KDE Connect still runs.
    hl.exec_cmd("dex --autostart --environment Hyprland")

    -- Baloo is one of the entries dex skips, so Dolphin's search falls back to
    -- matching filenames rather than content. Uncomment to get indexed search
    -- back, at the cost of a background indexer churning your disk. `fd` and
    -- `rg` from a terminal are the other answer.
    -- hl.exec_cmd("/usr/lib/kf6/baloo_file")

    -- Night light. hyprsunset idles until something asks it for a temperature.
    hl.exec_cmd("hyprsunset")

    -- Cursor theme for XWayland clients, which ignore the env vars.
    hl.exec_cmd("hyprctl setcursor " .. require("theme").cursor.theme .. " " .. require("theme").cursor.size)
end)
