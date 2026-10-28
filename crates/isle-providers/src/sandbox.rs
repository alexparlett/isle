//! bubblewrap's arguments for one provider run (`p17-widgets.md` choice 18).

use std::path::{Path, PathBuf};

use crate::grant::{self, Grant};

/// Where the relay listens inside the sandbox.
pub const RELAY_PORT: u16 = 3128;

/// Everything a run's sandbox is made from.
pub struct Plan {
    pub folder: PathBuf,
    /// The provider's own writable folder, its HOME inside.
    pub state: PathBuf,
    pub home: PathBuf,
    /// Declared and granted.
    pub grants: Vec<Grant>,
    pub init: PathBuf,
    pub proxy: Option<PathBuf>,
    pub bus: Option<PathBuf>,
    pub system_bus: Option<PathBuf>,
    pub argv: Vec<String>,
    /// Its script runs under python3.
    pub python: bool,
}

/// A command on the host's path.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_else(|| "/usr/bin".into());
    std::env::split_paths(&path)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

fn bind(out: &mut Vec<String>, how: &str, from: &Path, to: &Path) {
    out.push(how.into());
    out.push(from.to_string_lossy().into_owned());
    out.push(to.to_string_lossy().into_owned());
}

/// The arguments after `bwrap`.
pub fn args(plan: &Plan) -> Vec<String> {
    let mut a: Vec<String> = [
        "--unshare-all",
        "--die-with-parent",
        "--new-session",
        "--clearenv",
        "--ro-bind",
        "/usr",
        "/usr",
        // Only the commands granted are on the path.
        "--tmpfs",
        "/usr/bin",
        "--symlink",
        "usr/lib",
        "/lib",
        "--symlink",
        "usr/lib",
        "/lib64",
        "--symlink",
        "usr/bin",
        "/bin",
        "--symlink",
        "usr/bin",
        "/sbin",
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--tmpfs",
        "/run",
        "--dir",
        "/run/isle",
    ]
    .map(String::from)
    .to_vec();
    let mut programs = vec!["sh".to_owned(), "env".to_owned()];
    if plan.python {
        programs.push("python3".into());
    }
    for g in &plan.grants {
        if let Grant::Run(name) = g {
            programs.push(name.clone());
        }
    }
    for name in programs {
        if let Some(found) = which(&name) {
            bind(&mut a, "--ro-bind", &found, &Path::new("/usr/bin").join(&name));
        }
    }
    for etc in [
        "/etc/ssl",
        "/etc/ca-certificates",
        "/etc/passwd",
        "/etc/group",
        "/etc/localtime",
        "/etc/nsswitch.conf",
        "/etc/hosts",
        "/etc/os-release",
        "/etc/gitconfig",
    ] {
        bind(&mut a, "--ro-bind-try", Path::new(etc), Path::new(etc));
    }
    bind(&mut a, "--ro-bind", &plan.folder, &plan.folder);
    bind(&mut a, "--bind", &plan.state, &plan.state);
    let mut granted_paths = Vec::new();
    for g in &plan.grants {
        let (how, path) = match g {
            Grant::Read(p) => ("--ro-bind-try", p),
            Grant::Change(p) => ("--bind-try", p),
            _ => continue,
        };
        let path = grant::expand(path, &plan.home);
        if grant::inside_guarded(&path, &plan.home) {
            continue;
        }
        bind(&mut a, how, &path, &path);
        granted_paths.push(path);
    }
    // Isle's own places stay out of sight inside any folder granted around them.
    for guarded in grant::guarded(&plan.home) {
        if granted_paths.iter().any(|p| guarded.starts_with(p)) && guarded.exists() {
            a.push("--tmpfs".into());
            a.push(guarded.to_string_lossy().into_owned());
        }
    }
    bind(&mut a, "--ro-bind", &plan.init, Path::new("/run/isle/init"));
    if let Some(proxy) = &plan.proxy {
        bind(&mut a, "--bind", proxy, Path::new("/run/isle/proxy.sock"));
        let url = format!("http://127.0.0.1:{RELAY_PORT}");
        for var in ["HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY", "https_proxy", "http_proxy", "all_proxy"] {
            a.extend(["--setenv".into(), var.into(), url.clone()]);
        }
        a.extend(["--setenv".into(), "NODE_USE_ENV_PROXY".into(), "1".into()]);
    }
    if let Some(bus) = &plan.bus {
        bind(&mut a, "--bind", bus, Path::new("/run/isle/bus"));
        a.extend(["--setenv", "DBUS_SESSION_BUS_ADDRESS", "unix:path=/run/isle/bus"].map(String::from));
    }
    if let Some(bus) = &plan.system_bus {
        bind(&mut a, "--bind", bus, Path::new("/run/isle/system-bus"));
        a.extend(["--setenv", "DBUS_SYSTEM_BUS_ADDRESS", "unix:path=/run/isle/system-bus"].map(String::from));
    }
    let lang = std::env::var("LANG").unwrap_or_else(|_| "C.UTF-8".into());
    for (k, v) in [
        ("PATH", "/usr/bin".to_owned()),
        ("HOME", plan.state.to_string_lossy().into_owned()),
        // Where `~` in its grants points, since HOME is its own folder.
        ("ISLE_USER_HOME", plan.home.to_string_lossy().into_owned()),
        ("LANG", lang),
        ("TMPDIR", "/tmp".to_owned()),
    ] {
        a.extend(["--setenv".into(), k.into(), v]);
    }
    a.push("--chdir".into());
    a.push(plan.folder.to_string_lossy().into_owned());
    a.push("--".into());
    a.push("/run/isle/init".into());
    if plan.proxy.is_some() {
        a.extend(["--relay".into(), "/run/isle/proxy.sock".into(), RELAY_PORT.to_string()]);
    }
    a.push("--".into());
    a.extend(plan.argv.iter().cloned());
    a
}
