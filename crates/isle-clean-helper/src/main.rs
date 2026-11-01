//! Clears the system's share as root for Cleaner, run by pkexec under Isle's own policy
//! (`p19b-cleaner.md` choice 18), and reads btrfs's bytes on disk (B139). It takes one fixed
//! subcommand and nothing else: no path or package from its caller reaches a tool.

mod extents;

use std::{
    path::Path,
    process::{Command, ExitCode},
};

const COREDUMPS: &str = "/var/lib/systemd/coredump";

fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("{program}: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(format!(
            "{program} {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

fn clear(what: &str) -> Result<(), String> {
    match what {
        "package-cache" => {
            run("/usr/bin/paccache", &["-rk2"])?;
            run("/usr/bin/paccache", &["-ruk0"])?;
        }
        "orphans" => {
            // Read again here: the caller's list is never trusted.
            let listed = run("/usr/bin/pacman", &["-Qdtq"]).unwrap_or_default();
            let orphans: Vec<&str> = listed.split_whitespace().collect();
            if !orphans.is_empty() {
                let mut args = vec!["-Rns", "--noconfirm"];
                args.extend(orphans);
                run("/usr/bin/pacman", &args)?;
            }
        }
        "journal" => {
            run("/usr/bin/journalctl", &["--vacuum-size=100M"])?;
        }
        "coredumps" => {
            for entry in std::fs::read_dir(COREDUMPS)
                .map_err(|e| e.to_string())?
                .flatten()
            {
                let path = entry.path();
                if path.parent() == Some(Path::new(COREDUMPS))
                    && std::fs::symlink_metadata(&path).is_ok_and(|m| m.is_file())
                {
                    std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                }
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [what] = args.as_slice() else {
        eprintln!("usage: isle-clean-helper package-cache|orphans|journal|coredumps|extents");
        return ExitCode::from(2);
    };
    if what == "extents" {
        return match extents::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("isle-clean-helper: {e}");
                ExitCode::from(3)
            }
        };
    }
    if !matches!(
        what.as_str(),
        "package-cache" | "orphans" | "journal" | "coredumps"
    ) {
        eprintln!("isle-clean-helper: no {what}");
        return ExitCode::from(2);
    }
    match clear(what) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("isle-clean-helper: {e}");
            ExitCode::from(3)
        }
    }
}
