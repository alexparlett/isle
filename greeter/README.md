# Greeter

The login screen, in the shell's material, run by greetd as its own
Quickshell config. `theme/`, `ui/`, `assets/` and `services/` are links
into `shell/` so it draws with the same tokens and kit.

Install, and refresh after every pull:

```
sudo tools/install-greeter.sh you
```

which copies `greeter/` and `shell/` to `/usr/local/share/isle-greeter`,
root-owned and world-readable, and writes greetd's config to run
`qs -p /usr/local/share/isle-greeter/greeter` with `ISLE_GREETER_USER` set
to you and `ISLE_GREETER_COMMAND` to `start-hyprland`. It runs as the
`greeter` user before anyone is logged in, which is why it cannot live in
a home directory: a 0700 home keeps that user out, and greetd then comes
up as a bare Hyprland with nothing drawn on it. `tools/install.sh` runs
it again when greetd is set up this way.
