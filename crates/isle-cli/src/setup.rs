//! `isle setup`: what a package cannot do, as steps that each check, apply and undo, with a
//! record of what each changed (`platform.md` Packaging and setup 3 and 4).

use std::{
    io::IsTerminal,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    time::UNIX_EPOCH,
};

use isle_state::{
    StateDir,
    domains::{Setup, SetupStep},
};

/// What greetd runs until Isle's greeter mode (`p14a-session.md` choice 2).
const TUIGREET: &str = "tuigreet --time --remember --remember-session --sessions /usr/share/wayland-sessions";

const XREMAP_DROP_IN: &str = "\
# v1's remapper rewrites Cmd chords before Isle sees them; Isle's session does without it.
[Unit]
ConditionEnvironment=!XDG_CURRENT_DESKTOP=Isle
";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Who {
    Root,
    User,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Check {
    Due,
    Done,
    /// Nothing here for it to change.
    NotHere,
    /// It cannot be applied, and why.
    Blocked(String),
}

/// Where the steps look: the real system, or a test's directory standing in for it.
pub struct Env {
    pub root: PathBuf,
    pub config_home: PathBuf,
    /// Whether to ask the user manager to reload units after a change.
    pub reload: bool,
}

impl Env {
    fn system() -> Self {
        let config_home = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_else(|| PathBuf::from(".config"));
        Self {
            root: PathBuf::from("/"),
            config_home,
            reload: true,
        }
    }

    fn at(&self, path: &str) -> PathBuf {
        self.root.join(path.trim_start_matches('/'))
    }

    fn reload_user_units(&self) {
        if self.reload {
            let _ = Command::new("systemctl")
                .args(["--user", "daemon-reload"])
                .status();
        }
    }
}

type Undo = std::collections::BTreeMap<String, String>;

pub struct Step {
    pub id: &'static str,
    pub who: Who,
    pub what: &'static str,
    pub check: fn(&Env) -> Check,
    pub apply: fn(&Env) -> Result<Undo, String>,
    pub undo: fn(&Env, &Undo) -> Result<(), String>,
}

pub const STEPS: &[Step] = &[
    Step {
        id: "greeter",
        who: Who::Root,
        what: "greetd's greeter is tuigreet, listing every installed session",
        check: greeter::check,
        apply: greeter::apply,
        undo: greeter::undo,
    },
    Step {
        id: "portals",
        who: Who::User,
        what: "your portals.conf applies under Hyprland only, as hyprland-portals.conf",
        check: portals::check,
        apply: portals::apply,
        undo: portals::undo,
    },
    Step {
        id: "files-folders",
        who: Who::User,
        what: "folders open in Files under Isle, in isle-mimeapps.list",
        check: files_folders::check,
        apply: files_folders::apply,
        undo: files_folders::undo,
    },
    Step {
        id: "xremap",
        who: Who::User,
        what: "xremap runs in every session but Isle's",
        check: xremap::check,
        apply: xremap::apply,
        undo: xremap::undo,
    },
];

mod greeter {
    use super::*;

    fn config(env: &Env) -> PathBuf {
        env.at("/etc/greetd/config.toml")
    }

    /// The byte range of `[default_session]`'s `command = …` line, and its text.
    fn command_line(text: &str) -> Option<(std::ops::Range<usize>, &str)> {
        let mut offset = 0;
        let mut in_section = false;
        for line in text.split_inclusive('\n') {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_section = trimmed == "[default_session]";
            } else if in_section && trimmed.starts_with("command") {
                let end = offset + line.trim_end_matches('\n').len();
                return Some((offset..end, &text[offset..end]));
            }
            offset += line.len();
        }
        None
    }

    fn wanted() -> String {
        format!("command = \"{TUIGREET}\"")
    }

    pub fn check(env: &Env) -> Check {
        let Ok(text) = std::fs::read_to_string(config(env)) else {
            return Check::NotHere;
        };
        match command_line(&text) {
            Some((_, line)) if line == wanted() => Check::Done,
            Some(_) if !env.at("/usr/bin/tuigreet").exists() => {
                Check::Blocked("greetd-tuigreet is not installed".into())
            }
            Some(_) => Check::Due,
            None => Check::Blocked("greetd's config has no [default_session] command".into()),
        }
    }

    pub fn apply(env: &Env) -> Result<Undo, String> {
        let path = config(env);
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let (range, old) = command_line(&text).ok_or("no [default_session] command")?;
        let mut new = text.clone();
        new.replace_range(range.clone(), &wanted());
        write(&path, &new)?;
        Ok(Undo::from([("command".to_owned(), old.to_owned())]))
    }

    pub fn undo(env: &Env, undo: &Undo) -> Result<(), String> {
        let path = config(env);
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let old = undo.get("command").ok_or("the record has no command")?;
        match command_line(&text) {
            Some((range, line)) if line == wanted() => {
                let mut new = text.clone();
                new.replace_range(range, old);
                write(&path, &new)
            }
            _ => Err("greetd's command has been changed since; left as it is".into()),
        }
    }
}

mod portals {
    use super::*;

    fn paths(env: &Env) -> (PathBuf, PathBuf) {
        let dir = env.config_home.join("xdg-desktop-portal");
        (dir.join("portals.conf"), dir.join("hyprland-portals.conf"))
    }

    pub fn check(env: &Env) -> Check {
        let (mine, hyprland) = paths(env);
        match (mine.exists(), hyprland.exists()) {
            (true, true) => Check::Blocked(
                "both portals.conf and hyprland-portals.conf exist; merge them by hand".into(),
            ),
            (true, false) => Check::Due,
            (false, true) => Check::Done,
            (false, false) => Check::NotHere,
        }
    }

    pub fn apply(env: &Env) -> Result<Undo, String> {
        let (mine, hyprland) = paths(env);
        std::fs::rename(&mine, &hyprland).map_err(|e| e.to_string())?;
        Ok(Undo::new())
    }

    pub fn undo(env: &Env, _: &Undo) -> Result<(), String> {
        let (mine, hyprland) = paths(env);
        if mine.exists() {
            return Err("a new portals.conf has been written since; left as it is".into());
        }
        std::fs::rename(&hyprland, &mine).map_err(|e| e.to_string())
    }
}

mod files_folders {
    use super::*;

    const LINE: &str = "inode/directory=io.github.alexparlett.Isle.Files.desktop";
    const GROUP: &str = "[Default Applications]";

    fn list(env: &Env) -> PathBuf {
        env.config_home.join("isle-mimeapps.list")
    }

    pub fn check(env: &Env) -> Check {
        if !env.at("usr/share/applications/io.github.alexparlett.Isle.Files.desktop").exists() {
            return Check::NotHere;
        }
        match std::fs::read_to_string(list(env)) {
            Ok(text) if text.lines().any(|l| l.trim() == LINE) => Check::Done,
            _ => Check::Due,
        }
    }

    /// `text` with folders given to Files under its `[Default Applications]`.
    fn with_line(text: &str) -> String {
        let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
        lines.retain(|l| !l.trim_start().starts_with("inode/directory="));
        match lines.iter().position(|l| l.trim() == GROUP) {
            Some(at) => lines.insert(at + 1, LINE.into()),
            None => {
                if lines.last().is_some_and(|l| !l.trim().is_empty()) {
                    lines.push(String::new());
                }
                lines.push(GROUP.into());
                lines.push(LINE.into());
            }
        }
        lines.join("\n") + "\n"
    }

    pub fn apply(env: &Env) -> Result<Undo, String> {
        let path = list(env);
        let before = std::fs::read_to_string(&path).ok();
        std::fs::create_dir_all(&env.config_home).map_err(|e| e.to_string())?;
        write(&path, &with_line(before.as_deref().unwrap_or("")))?;
        let mut undo = Undo::new();
        if let Some(before) = before {
            undo.insert("before".into(), before);
        }
        Ok(undo)
    }

    pub fn undo(env: &Env, undo: &Undo) -> Result<(), String> {
        let path = list(env);
        match undo.get("before") {
            Some(before) => write(&path, before),
            None => std::fs::remove_file(&path).map_err(|e| e.to_string()),
        }
    }
}

mod xremap {
    use super::*;

    fn unit(env: &Env) -> PathBuf {
        env.config_home.join("systemd/user/xremap.service")
    }

    fn drop_in(env: &Env) -> PathBuf {
        env.config_home.join("systemd/user/xremap.service.d/isle.conf")
    }

    pub fn check(env: &Env) -> Check {
        if std::fs::symlink_metadata(unit(env)).is_err() {
            return Check::NotHere;
        }
        match std::fs::read_to_string(drop_in(env)) {
            Ok(text) if text == XREMAP_DROP_IN => Check::Done,
            Ok(_) => Check::Blocked(format!("{} is not Isle's", drop_in(env).display())),
            Err(_) => Check::Due,
        }
    }

    pub fn apply(env: &Env) -> Result<Undo, String> {
        let path = drop_in(env);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        write(&path, XREMAP_DROP_IN)?;
        env.reload_user_units();
        Ok(Undo::new())
    }

    pub fn undo(env: &Env, _: &Undo) -> Result<(), String> {
        let path = drop_in(env);
        match std::fs::read_to_string(&path) {
            Ok(text) if text == XREMAP_DROP_IN => {
                std::fs::remove_file(&path).map_err(|e| e.to_string())?;
                let _ = std::fs::remove_dir(path.parent().unwrap());
                env.reload_user_units();
                Ok(())
            }
            Ok(_) => Err(format!("{} has been edited since; left as it is", path.display())),
            Err(_) => Ok(()),
        }
    }
}

/// Written beside and renamed over, keeping the old file's mode.
fn write(path: &Path, text: &str) -> Result<(), String> {
    let next = path.with_extension("isle-new");
    std::fs::write(&next, text).map_err(|e| e.to_string())?;
    if let Ok(meta) = std::fs::metadata(path) {
        let _ = std::fs::set_permissions(&next, meta.permissions());
    }
    std::fs::rename(&next, path).map_err(|e| e.to_string())
}

fn records(who: Who) -> StateDir {
    match who {
        Who::Root => StateDir::new("/var/lib/isle"),
        Who::User => StateDir::new(StateDir::default_dir()),
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Applies the named steps of `who`, recording each.
fn apply_steps(env: &Env, who: Who, ids: &[&str], dir: &StateDir) -> bool {
    let (mut record, _) = dir.load::<Setup>();
    let mut ok = true;
    for step in STEPS.iter().filter(|s| s.who == who && ids.contains(&s.id)) {
        match (step.apply)(env) {
            Ok(undo) => {
                println!("done     {:<8} {}", step.id, step.what);
                let applied_at = now();
                record
                    .steps
                    .insert(step.id.to_owned(), SetupStep { applied_at, undo });
            }
            Err(err) => {
                eprintln!("failed   {:<8} {err}", step.id);
                ok = false;
            }
        }
    }
    if let Err(err) = dir.save(&record) {
        eprintln!("isle setup: cannot keep the record of what it did: {err}");
        ok = false;
    }
    ok
}

/// Undoes every recorded step of `who`.
fn undo_steps(env: &Env, who: Who, dir: &StateDir) -> bool {
    let (mut record, _) = dir.load::<Setup>();
    let mut ok = true;
    for step in STEPS.iter().rev().filter(|s| s.who == who) {
        let Some(done) = record.steps.get(step.id) else {
            continue;
        };
        match (step.undo)(env, &done.undo) {
            Ok(()) => {
                println!("undone   {:<8} {}", step.id, step.what);
                record.steps.remove(step.id);
            }
            Err(err) => {
                eprintln!("kept     {:<8} {err}", step.id);
                ok = false;
            }
        }
    }
    if let Err(err) = dir.save(&record) {
        eprintln!("isle setup: cannot keep the record of what it did: {err}");
        ok = false;
    }
    ok
}

/// The root half, run again through sudo or pkexec with the steps to act on.
fn escalate(action: &str, ids: &[&str]) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        eprintln!("isle setup: cannot find its own program to run as root");
        return false;
    };
    let through = if std::io::stdin().is_terminal() {
        "sudo"
    } else {
        "pkexec"
    };
    Command::new(through)
        .arg(exe)
        .args(["setup", "--as-root", action])
        .args(ids)
        .status()
        .is_ok_and(|s| s.success())
}

pub fn run(check_only: bool, undo: bool, as_root: &[String]) -> ExitCode {
    let env = Env::system();
    if let Some((action, ids)) = as_root.split_first() {
        if unsafe { libc::geteuid() } != 0 {
            eprintln!("isle setup: --as-root is for the root half, run through sudo or pkexec");
            return ExitCode::FAILURE;
        }
        let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
        let dir = records(Who::Root);
        let ok = match action.as_str() {
            "apply" => apply_steps(&env, Who::Root, &ids, &dir),
            "undo" => undo_steps(&env, Who::Root, &dir),
            _ => false,
        };
        return if ok {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    if undo {
        let user = undo_steps(&env, Who::User, &records(Who::User));
        let (root_record, _) = records(Who::Root).load::<Setup>();
        let root = root_record.steps.is_empty() || escalate("undo", &[]);
        return if user && root {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    let mut due: Vec<&Step> = Vec::new();
    for step in STEPS {
        let check = (step.check)(&env);
        let label = match &check {
            Check::Due => "due",
            Check::Done => "done",
            Check::NotHere => "not here",
            Check::Blocked(_) => "blocked",
        };
        println!("{label:<8} {:<8} {}", step.id, step.what);
        if let Check::Blocked(why) = &check {
            println!("{:<17} {why}", "");
        }
        if check == Check::Due {
            due.push(step);
        }
    }
    if check_only || due.is_empty() {
        return ExitCode::SUCCESS;
    }
    let ids = |who: Who| -> Vec<&str> {
        due.iter()
            .filter(|s| s.who == who)
            .map(|s| s.id)
            .collect()
    };
    let user = ids(Who::User);
    let root = ids(Who::Root);
    let mut ok = user.is_empty() || apply_steps(&env, Who::User, &user, &records(Who::User));
    if !root.is_empty() {
        ok &= escalate("apply", &root);
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("isle-setup-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn env(&self) -> Env {
            Env {
                root: self.0.join("root"),
                config_home: self.0.join("config"),
                reload: false,
            }
        }

        fn put(&self, path: &str, text: &str) -> PathBuf {
            let path = self.0.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, text).unwrap();
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn step(id: &str) -> &'static Step {
        STEPS.iter().find(|s| s.id == id).unwrap()
    }

    /// Applies and undoes `id`, checking it is due before, done between, and that undo puts
    /// every byte back.
    fn round_trip(scratch: &Scratch, id: &str, files: &[&PathBuf]) {
        let env = scratch.env();
        let step = step(id);
        let before: Vec<Option<String>> = files
            .iter()
            .map(|p| std::fs::read_to_string(p).ok())
            .collect();
        assert_eq!((step.check)(&env), Check::Due);
        let undo = (step.apply)(&env).unwrap();
        assert_eq!((step.check)(&env), Check::Done);
        (step.undo)(&env, &undo).unwrap();
        let after: Vec<Option<String>> = files
            .iter()
            .map(|p| std::fs::read_to_string(p).ok())
            .collect();
        assert_eq!(before, after);
        assert_eq!((step.check)(&env), Check::Due);
    }

    #[test]
    fn the_greeter_becomes_tuigreet_and_comes_back() {
        let scratch = Scratch::new("greeter");
        let config = scratch.put(
            "root/etc/greetd/config.toml",
            "[terminal]\nvt = 1\n\n[default_session]\ncommand = \"start-hyprland -- -c /etc/greetd/hyprland.lua\"\nuser = \"greeter\"\n",
        );
        assert!(matches!(
            (step("greeter").check)(&scratch.env()),
            Check::Blocked(_)
        ));
        scratch.put("root/usr/bin/tuigreet", "");
        let env = scratch.env();
        (step("greeter").apply)(&env).unwrap();
        let text = std::fs::read_to_string(&config).unwrap();
        assert!(text.contains(&format!("command = \"{TUIGREET}\"\nuser = \"greeter\"\n")));
        assert!(text.starts_with("[terminal]\nvt = 1\n"));
        std::fs::write(
            &config,
            "[terminal]\nvt = 1\n\n[default_session]\ncommand = \"start-hyprland -- -c /etc/greetd/hyprland.lua\"\nuser = \"greeter\"\n",
        )
        .unwrap();
        round_trip(&scratch, "greeter", &[&config]);
    }

    #[test]
    fn the_greeter_is_left_alone_once_changed_by_hand() {
        let scratch = Scratch::new("greeter-hand");
        let config = scratch.put("root/etc/greetd/config.toml", "[default_session]\ncommand = \"x\"\n");
        scratch.put("root/usr/bin/tuigreet", "");
        let env = scratch.env();
        let undo = (step("greeter").apply)(&env).unwrap();
        std::fs::write(&config, "[default_session]\ncommand = \"mine\"\n").unwrap();
        assert!((step("greeter").undo)(&env, &undo).is_err());
        assert_eq!(
            std::fs::read_to_string(&config).unwrap(),
            "[default_session]\ncommand = \"mine\"\n"
        );
    }

    #[test]
    fn v1s_portals_file_moves_to_hyprlands_name_and_back() {
        let scratch = Scratch::new("portals");
        let mine = scratch.put(
            "config/xdg-desktop-portal/portals.conf",
            "[preferred]\ndefault=hyprland;gtk\n",
        );
        let hyprland = scratch.0.join("config/xdg-desktop-portal/hyprland-portals.conf");
        round_trip(&scratch, "portals", &[&mine, &hyprland]);
        scratch.put("config/xdg-desktop-portal/hyprland-portals.conf", "");
        assert!(matches!(
            (step("portals").check)(&scratch.env()),
            Check::Blocked(_)
        ));
    }

    #[test]
    fn folders_go_to_files_only_where_it_is_installed_and_come_back() {
        let scratch = Scratch::new("files-folders");
        assert_eq!((step("files-folders").check)(&scratch.env()), Check::NotHere);
        scratch.put("root/usr/share/applications/io.github.alexparlett.Isle.Files.desktop", "");
        let list = scratch.0.join("config/isle-mimeapps.list");
        round_trip(&scratch, "files-folders", &[&list]);
        scratch.put("config/isle-mimeapps.list", "[Default Applications]\ninode/directory=org.gnome.Nautilus.desktop\ntext/plain=zed.desktop\n");
        round_trip(&scratch, "files-folders", &[&list]);
    }

    #[test]
    fn xremap_gets_its_condition_only_where_it_is_installed() {
        let scratch = Scratch::new("xremap");
        assert_eq!((step("xremap").check)(&scratch.env()), Check::NotHere);
        scratch.put("config/systemd/user/xremap.service", "[Service]\n");
        let drop_in = scratch.0.join("config/systemd/user/xremap.service.d/isle.conf");
        round_trip(&scratch, "xremap", &[&drop_in]);
        assert!(!drop_in.parent().unwrap().exists());
    }

    #[test]
    fn a_record_undoes_only_what_it_did() {
        let scratch = Scratch::new("record");
        scratch.put("config/systemd/user/xremap.service", "[Service]\n");
        let mine = scratch.put("config/xdg-desktop-portal/portals.conf", "x\n");
        let env = scratch.env();
        let dir = StateDir::new(scratch.0.join("state"));
        assert!(apply_steps(&env, Who::User, &["xremap"], &dir));
        assert!(undo_steps(&env, Who::User, &dir));
        assert_eq!(std::fs::read_to_string(&mine).unwrap(), "x\n");
        assert!(dir.load::<Setup>().0.steps.is_empty());
    }
}
