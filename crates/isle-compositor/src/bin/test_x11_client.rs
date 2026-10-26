//! An X11 client for the integration tests: windows of one colour with the properties the
//! window manager's rules read, printing what they receive, one event per line, and taking
//! commands on standard input. Lines about any window but the first start with `wN `.
//!
//! Arguments: `--class C`, `--title T` (none with `--untitled`), `--size WxH`, `--pos X,Y`,
//! `--colour RRGGBB`, `--override-redirect`, `--no-decorations` (Motif hints), `--motif N`
//! (Motif decorations as a number), `--type T` (`_NET_WM_WINDOW_TYPE_T`, such as `SPLASH`),
//! `--gtk-frame-extents N` (a frame of N on every side), `--shaped` (a bounding shape: the
//! window's left half), `--argb` (a 32-bit visual, transparent but for its left half, which
//! is painted in the colour), `--steam-game ID` (Steam's `STEAM_GAME` label).
//! Commands: `fullscreen`, `unfullscreen`, `maximize`, `unmaximize`, `minimize`,
//! `dialog` (a window transient for the first), `move-on-press`, `urgent` and `calm` (the
//! `WM_HINTS` urgency bit), `demand` (`_NET_WM_STATE_DEMANDS_ATTENTION`), `activate`
//! (`_NET_ACTIVE_WINDOW` as an application), `quit`.

use std::{
    io::{BufRead, Write},
    os::fd::AsRawFd,
};

use x11rb::{
    connection::Connection,
    protocol::{
        Event,
        xproto::{
            AtomEnum, ClientMessageEvent, ConnectionExt, CreateWindowAux, EventMask, PropMode,
            WindowClass,
        },
    },
    wrapper::ConnectionExt as _,
};

fn say(index: usize, line: String) {
    let mut out = std::io::stdout().lock();
    let _ = if index == 0 {
        writeln!(out, "{line}")
    } else {
        writeln!(out, "w{index} {line}")
    };
    let _ = out.flush();
}

struct Atoms {
    wm_protocols: u32,
    wm_delete: u32,
    net_wm_name: u32,
    utf8: u32,
    motif: u32,
    net_wm_state: u32,
    fullscreen: u32,
    max_h: u32,
    max_v: u32,
    wm_change_state: u32,
    moveresize: u32,
    demands: u32,
    active: u32,
    clipboard: u32,
    paste: u32,
}

fn atom(conn: &impl Connection, name: &str) -> u32 {
    conn.intern_atom(false, name.as_bytes())
        .unwrap()
        .reply()
        .unwrap()
        .atom
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str, default: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1).cloned())
            .unwrap_or_else(|| default.to_owned())
    };
    let flag = |name: &str| args.iter().any(|a| a == name);
    let class = arg("--class", "isle.test.X11");
    let title = (!flag("--untitled")).then(|| arg("--title", "x11 client"));
    let (w, h): (u16, u16) = arg("--size", "400x300")
        .split_once('x')
        .map(|(w, h)| (w.parse().unwrap(), h.parse().unwrap()))
        .unwrap();
    let (x, y): (i16, i16) = arg("--pos", "0,0")
        .split_once(',')
        .map(|(x, y)| (x.parse().unwrap(), y.parse().unwrap()))
        .unwrap();
    let colour = u32::from_str_radix(&arg("--colour", "ff3366"), 16).unwrap();

    let (conn, screen_num) = x11rb::connect(None).expect("an X display");
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let atoms = Atoms {
        wm_protocols: atom(&conn, "WM_PROTOCOLS"),
        wm_delete: atom(&conn, "WM_DELETE_WINDOW"),
        net_wm_name: atom(&conn, "_NET_WM_NAME"),
        utf8: atom(&conn, "UTF8_STRING"),
        motif: atom(&conn, "_MOTIF_WM_HINTS"),
        net_wm_state: atom(&conn, "_NET_WM_STATE"),
        fullscreen: atom(&conn, "_NET_WM_STATE_FULLSCREEN"),
        max_h: atom(&conn, "_NET_WM_STATE_MAXIMIZED_HORZ"),
        max_v: atom(&conn, "_NET_WM_STATE_MAXIMIZED_VERT"),
        wm_change_state: atom(&conn, "WM_CHANGE_STATE"),
        moveresize: atom(&conn, "_NET_WM_MOVERESIZE"),
        demands: atom(&conn, "_NET_WM_STATE_DEMANDS_ATTENTION"),
        active: atom(&conn, "_NET_ACTIVE_WINDOW"),
        clipboard: atom(&conn, "CLIPBOARD"),
        paste: atom(&conn, "ISLE_PASTE"),
    };

    // With `--argb`, a 32-bit visual with its own colormap, as xeyes and many launchers use.
    let (depth, visual, colormap, pixel) = if flag("--argb") {
        let (depth, visual) = screen
            .allowed_depths
            .iter()
            .filter(|d| d.depth == 32)
            .find_map(|d| d.visuals.first().map(|v| (d.depth, v.visual_id)))
            .expect("a 32-bit visual");
        let colormap = conn.generate_id().unwrap();
        conn.create_colormap(
            x11rb::protocol::xproto::ColormapAlloc::NONE,
            colormap,
            root,
            visual,
        )
        .unwrap();
        (depth, visual, Some(colormap), 0)
    } else {
        (screen.root_depth, screen.root_visual, None, colour)
    };
    let make = |transient_for: Option<u32>, override_redirect: bool, title: Option<&str>| {
        let win = conn.generate_id().unwrap();
        let aux = CreateWindowAux::new()
            .background_pixel(pixel)
            .border_pixel(0)
            .colormap(colormap)
            .override_redirect(override_redirect as u32)
            .event_mask(
                EventMask::STRUCTURE_NOTIFY
                    | EventMask::FOCUS_CHANGE
                    | EventMask::KEY_PRESS
                    | EventMask::BUTTON_PRESS
                    | EventMask::EXPOSURE,
            );
        conn.create_window(
            depth,
            win,
            root,
            x,
            y,
            w,
            h,
            0,
            WindowClass::INPUT_OUTPUT,
            visual,
            &aux,
        )
        .unwrap();
        let mut wm_class = class.clone().into_bytes();
        wm_class.push(0);
        wm_class.extend(class.as_bytes());
        wm_class.push(0);
        conn.change_property8(
            PropMode::REPLACE,
            win,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            &wm_class,
        )
        .unwrap();
        if let Some(title) = title {
            conn.change_property8(
                PropMode::REPLACE,
                win,
                atoms.net_wm_name,
                atoms.utf8,
                title.as_bytes(),
            )
            .unwrap();
            conn.change_property8(
                PropMode::REPLACE,
                win,
                AtomEnum::WM_NAME,
                AtomEnum::STRING,
                title.as_bytes(),
            )
            .unwrap();
        }
        conn.change_property32(
            PropMode::REPLACE,
            win,
            atoms.wm_protocols,
            AtomEnum::ATOM,
            &[atoms.wm_delete],
        )
        .unwrap();
        conn.change_property32(
            PropMode::REPLACE,
            win,
            AtomEnum::WM_CLIENT_MACHINE,
            AtomEnum::STRING,
            &[],
        )
        .ok();
        let motif = if flag("--no-decorations") {
            Some(0)
        } else {
            args.iter()
                .position(|a| a == "--motif")
                .and_then(|i| args.get(i + 1))
                .map(|v| v.parse::<u32>().unwrap())
        };
        if let Some(decorations) = motif {
            // flags: decorations.
            conn.change_property32(
                PropMode::REPLACE,
                win,
                atoms.motif,
                atoms.motif,
                &[2, 0, decorations, 0, 0],
            )
            .unwrap();
        }
        if let Some(kind) = args
            .iter()
            .position(|a| a == "--type")
            .and_then(|i| args.get(i + 1))
        {
            let type_atom = atom(&conn, "_NET_WM_WINDOW_TYPE");
            let value = atom(&conn, &format!("_NET_WM_WINDOW_TYPE_{kind}"));
            conn.change_property32(PropMode::REPLACE, win, type_atom, AtomEnum::ATOM, &[value])
                .unwrap();
        }
        if let Some(n) = args
            .iter()
            .position(|a| a == "--gtk-frame-extents")
            .and_then(|i| args.get(i + 1))
            .map(|v| v.parse::<u32>().unwrap())
        {
            let extents = atom(&conn, "_GTK_FRAME_EXTENTS");
            conn.change_property32(
                PropMode::REPLACE,
                win,
                extents,
                AtomEnum::CARDINAL,
                &[n, n, n, n],
            )
            .unwrap();
        }
        if let Some(id) = args
            .iter()
            .position(|a| a == "--steam-game")
            .and_then(|i| args.get(i + 1))
            .map(|v| v.parse::<u32>().unwrap())
        {
            let steam_game = atom(&conn, "STEAM_GAME");
            conn.change_property32(PropMode::REPLACE, win, steam_game, AtomEnum::CARDINAL, &[id])
                .unwrap();
        }
        if flag("--shaped") {
            use x11rb::protocol::shape::{ConnectionExt as _, SK, SO};
            let half = x11rb::protocol::xproto::Rectangle {
                x: 0,
                y: 0,
                width: w / 2,
                height: h,
            };
            conn.shape_rectangles(
                SO::SET,
                SK::BOUNDING,
                x11rb::protocol::xproto::ClipOrdering::UNSORTED,
                win,
                0,
                0,
                &[half],
            )
            .unwrap();
        }
        if let Some(parent) = transient_for {
            conn.change_property32(
                PropMode::REPLACE,
                win,
                AtomEnum::WM_TRANSIENT_FOR,
                AtomEnum::WINDOW,
                &[parent],
            )
            .unwrap();
        }
        conn.map_window(win).unwrap();
        conn.flush().unwrap();
        win
    };

    let mut wins = vec![make(None, flag("--override-redirect"), title.as_deref())];
    let mut move_on_press = false;
    let state_message = |win: u32, action: u32, a: u32, b: u32| {
        let event = ClientMessageEvent::new(32, win, atoms.net_wm_state, [action, a, b, 1, 0]);
        conn.send_event(
            false,
            root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        )
        .unwrap();
        conn.flush().unwrap();
    };

    let stdin = std::io::stdin();
    let mut lines = stdin.lock();
    let mut stdin_open = true;
    loop {
        loop {
            let event = match conn.poll_for_event() {
                Ok(Some(event)) => event,
                Ok(None) => break,
                Err(_) => {
                    say(0, "disconnected".into());
                    return;
                }
            };
            let index = |win: u32| wins.iter().position(|w| *w == win).unwrap_or(0);
            match event {
                Event::MapNotify(e) => say(index(e.window), "mapped".into()),
                Event::UnmapNotify(e) => say(index(e.window), "unmapped".into()),
                Event::ConfigureNotify(e) => say(
                    index(e.window),
                    format!("configured {} {} at {} {}", e.width, e.height, e.x, e.y),
                ),
                Event::Expose(e) if flag("--argb") => {
                    let gc = conn.generate_id().unwrap();
                    conn.create_gc(
                        gc,
                        e.window,
                        &x11rb::protocol::xproto::CreateGCAux::new()
                            .foreground(colour | 0xff00_0000),
                    )
                    .unwrap();
                    let half = x11rb::protocol::xproto::Rectangle {
                        x: 0,
                        y: 0,
                        width: w / 2,
                        height: h,
                    };
                    conn.poly_fill_rectangle(e.window, gc, &[half]).unwrap();
                    conn.free_gc(gc).unwrap();
                    conn.flush().unwrap();
                }
                Event::FocusIn(e) => say(index(e.event), "focus-in".into()),
                Event::FocusOut(e) => {
                    say(index(e.event), "focus-out".into());
                    // As Wine lets ClipCursor go when the window loses focus.
                    if flag("--ungrab-on-focus-out") {
                        conn.ungrab_pointer(x11rb::CURRENT_TIME).unwrap();
                        conn.flush().unwrap();
                    }
                }
                Event::KeyPress(e) => {
                    say(index(e.event), format!("key {} pressed", e.detail));
                    // Ctrl+V asks for the clipboard's text, printed when it comes.
                    let ctrl = u16::from(e.state) & u16::from(x11rb::protocol::xproto::KeyButMask::CONTROL) != 0;
                    if ctrl && e.detail == 47 + 8 {
                        conn.convert_selection(e.event, atoms.clipboard, atoms.utf8, atoms.paste, e.time)
                            .unwrap();
                        conn.flush().unwrap();
                    }
                }
                Event::SelectionNotify(e) if e.property == atoms.paste => {
                    let reply = conn
                        .get_property(true, e.requestor, atoms.paste, x11rb::protocol::xproto::AtomEnum::ANY, 0, 1 << 20)
                        .unwrap()
                        .reply()
                        .unwrap();
                    say(index(e.requestor), format!("pasted {}", String::from_utf8_lossy(&reply.value)));
                }
                Event::SelectionNotify(e) => say(index(e.requestor), "pasted nothing".into()),
                Event::ButtonPress(e) => {
                    say(index(e.event), format!("button {} pressed", e.detail));
                    if move_on_press {
                        move_on_press = false;
                        conn.ungrab_pointer(x11rb::CURRENT_TIME).unwrap();
                        let msg = ClientMessageEvent::new(
                            32,
                            e.event,
                            atoms.moveresize,
                            [e.root_x as u32, e.root_y as u32, 8, e.detail as u32, 1],
                        );
                        conn.send_event(
                            false,
                            root,
                            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                            msg,
                        )
                        .unwrap();
                        conn.flush().unwrap();
                    }
                }
                Event::ClientMessage(e) if e.data.as_data32()[0] == atoms.wm_delete => {
                    say(index(e.window), "closed".into());
                    if index(e.window) == 0 {
                        return;
                    }
                    conn.destroy_window(e.window).unwrap();
                    conn.flush().unwrap();
                }
                _ => {}
            }
        }
        if conn.flush().is_err() {
            return;
        }
        let mut fds = [
            libc::pollfd {
                fd: conn.stream().as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: if stdin_open { 0 } else { -1 },
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        if unsafe { libc::poll(fds.as_mut_ptr(), 2, -1) } < 0 {
            continue;
        }
        if fds[1].revents & (libc::POLLIN | libc::POLLHUP) != 0 {
            let mut line = String::new();
            match lines.read_line(&mut line) {
                Ok(0) | Err(_) => stdin_open = false,
                Ok(_) => {
                    let win = wins[0];
                    match line.trim() {
                        "fullscreen" => state_message(win, 1, atoms.fullscreen, 0),
                        "unfullscreen" => state_message(win, 0, atoms.fullscreen, 0),
                        "maximize" => state_message(win, 1, atoms.max_v, atoms.max_h),
                        "unmaximize" => state_message(win, 0, atoms.max_v, atoms.max_h),
                        "minimize" => {
                            // IconicState.
                            let msg = ClientMessageEvent::new(
                                32,
                                win,
                                atoms.wm_change_state,
                                [3, 0, 0, 0, 0],
                            );
                            conn.send_event(
                                false,
                                root,
                                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                                msg,
                            )
                            .unwrap();
                            conn.flush().unwrap();
                        }
                        "dialog" => {
                            let title = format!("dialog {}", wins.len());
                            let new = make(Some(win), false, Some(&title));
                            wins.push(new);
                            say(wins.len() - 1, "created".into());
                        }
                        "move-on-press" => move_on_press = true,
                        // Confines the pointer to the window, as Wine's ClipCursor does.
                        "grab" => {
                            use x11rb::protocol::xproto::{GrabMode, GrabStatus};
                            let status = conn
                                .grab_pointer(
                                    false,
                                    win,
                                    EventMask::BUTTON_PRESS
                                        | EventMask::BUTTON_RELEASE
                                        | EventMask::POINTER_MOTION,
                                    GrabMode::ASYNC,
                                    GrabMode::ASYNC,
                                    win,
                                    x11rb::NONE,
                                    x11rb::CURRENT_TIME,
                                )
                                .unwrap()
                                .reply()
                                .map(|r| r.status);
                            say(0, format!("grab {}", if matches!(status, Ok(GrabStatus::SUCCESS)) { "held" } else { "refused" }));
                        }
                        "urgent" | "calm" => {
                            let mut hints = x11rb::properties::WmHints::new();
                            hints.urgent = line.trim() == "urgent";
                            hints.set(&conn, win).unwrap();
                            conn.flush().unwrap();
                        }
                        "demand" => state_message(win, 1, atoms.demands, 0),
                        "activate" => {
                            let msg = ClientMessageEvent::new(32, win, atoms.active, [1, 0, 0, 0, 0]);
                            conn.send_event(
                                false,
                                root,
                                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                                msg,
                            )
                            .unwrap();
                            conn.flush().unwrap();
                        }
                        "quit" => return,
                        other => say(0, format!("unknown command {other}")),
                    }
                }
            }
        }
    }
}
