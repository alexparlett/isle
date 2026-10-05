//! An X11 client for the integration tests: windows of one colour with the properties the
//! window manager's rules read, printing what they receive, one event per line, and taking
//! commands on standard input. Lines about any window but the first start with `wN `.
//!
//! Arguments: `--class C`, `--title T` (none with `--untitled`), `--size WxH`, `--pos X,Y`,
//! `--colour RRGGBB`, `--override-redirect`, `--no-decorations` (Motif hints).
//! Commands: `fullscreen`, `unfullscreen`, `maximize`, `unmaximize`, `minimize`,
//! `dialog` (a window transient for the first), `move-on-press`, `quit`.

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
    };

    let make = |transient_for: Option<u32>, override_redirect: bool, title: Option<&str>| {
        let win = conn.generate_id().unwrap();
        let aux = CreateWindowAux::new()
            .background_pixel(colour)
            .override_redirect(override_redirect as u32)
            .event_mask(
                EventMask::STRUCTURE_NOTIFY
                    | EventMask::FOCUS_CHANGE
                    | EventMask::KEY_PRESS
                    | EventMask::BUTTON_PRESS
                    | EventMask::EXPOSURE,
            );
        conn.create_window(
            screen.root_depth,
            win,
            root,
            x,
            y,
            w,
            h,
            0,
            WindowClass::INPUT_OUTPUT,
            screen.root_visual,
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
        if flag("--no-decorations") {
            // flags: decorations; decorations: none.
            conn.change_property32(
                PropMode::REPLACE,
                win,
                atoms.motif,
                atoms.motif,
                &[2, 0, 0, 0, 0],
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
                Event::FocusIn(e) => say(index(e.event), "focus-in".into()),
                Event::FocusOut(e) => say(index(e.event), "focus-out".into()),
                Event::KeyPress(e) => say(index(e.event), format!("key {} pressed", e.detail)),
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
                        "quit" => return,
                        other => say(0, format!("unknown command {other}")),
                    }
                }
            }
        }
    }
}
