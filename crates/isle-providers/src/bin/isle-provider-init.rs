//! A provider sandbox's first process: the loopback relay to the runner's proxy, then the provider.

use std::{
    io,
    net::{TcpListener, TcpStream},
    os::unix::net::UnixStream,
    process::{Command, ExitCode},
    thread,
};

fn pipe(mut from: impl io::Read, mut to: impl io::Write) {
    let _ = io::copy(&mut from, &mut to);
}

/// Carries each connection to `127.0.0.1:port` to the socket and back.
fn relay(socket: String, port: u16) -> io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    thread::spawn(move || {
        for tcp in listener.incoming().flatten() {
            let socket = socket.clone();
            thread::spawn(move || {
                let Ok(unix) = UnixStream::connect(&socket) else {
                    return;
                };
                let (Ok(tcp_back), Ok(unix_back)) = (tcp.try_clone(), unix.try_clone()) else {
                    return;
                };
                let up = thread::spawn(move || {
                    pipe(&tcp, &unix);
                    let _ = unix.shutdown(std::net::Shutdown::Write);
                });
                pipe(&unix_back, &tcp_back);
                let _ = TcpStream::shutdown(&tcp_back, std::net::Shutdown::Write);
                let _ = up.join();
            });
        }
    });
    Ok(())
}

/// `isle-provider-init [--relay SOCKET PORT] -- PROGRAM ARGS…`, exiting with the program's status.
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(split) = args.iter().position(|a| a == "--") else {
        eprintln!("isle-provider-init: no program after --");
        return ExitCode::from(64);
    };
    let (own, program) = (&args[..split], &args[split + 1..]);
    if let [flag, socket, port] = own
        && flag == "--relay"
    {
        let Ok(port) = port.parse() else {
            eprintln!("isle-provider-init: {port} is not a port");
            return ExitCode::from(64);
        };
        if let Err(err) = relay(socket.clone(), port) {
            eprintln!("isle-provider-init: the relay: {err}");
            return ExitCode::from(71);
        }
    }
    let Some((name, rest)) = program.split_first() else {
        eprintln!("isle-provider-init: no program after --");
        return ExitCode::from(64);
    };
    match Command::new(name).args(rest).status() {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1).clamp(0, 255) as u8),
        Err(err) => {
            eprintln!("{name}: {err}");
            ExitCode::from(127)
        }
    }
}
