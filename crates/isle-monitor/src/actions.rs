//! The processes an action is aimed at, and the menus that act on them: the Process menu,
//! More and the row's context menu (`structure/monitor.md` § Quit, Force Quit and the rest).

use std::rc::Rc;

use isle_sysinfo::procs::Key;
use isle_ui::{MenuEntry, MenuItem, menu::Mark};

use crate::{
    act::{self, Priority},
    sampler::Sample,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub key: Key,
    pub name: String,
    pub ppid: i32,
    pub cgroup: String,
    pub stopped: bool,
    pub cmdline: String,
}

/// The selected processes as the last sample has them.
pub fn targets(sample: Option<&Sample>, selected: &[u64]) -> Vec<Target> {
    let Some(sample) = sample else { return Vec::new() };
    let named = crate::model::processes(&sample.full, &sample.extras);
    sample
        .full
        .processes
        .iter()
        .zip(named)
        .filter(|(_, p)| selected.contains(&p.key))
        .filter_map(|(raw, p)| {
            Some(Target {
                key: raw.key?,
                name: p.name,
                ppid: p.ppid,
                cgroup: p.cgroup,
                stopped: p.stopped,
                cmdline: raw.cmdline.join(" "),
            })
        })
        .collect()
}

/// Something is selected and none of it is the session's own (choice 21).
pub fn quittable(targets: &[Target]) -> bool {
    !targets.is_empty() && !targets.iter().any(|t| act::protected(&t.name, t.ppid))
}

pub const QUIT: u64 = 1;
pub const FORCE_QUIT: u64 = 2;
pub const STOP: u64 = 3;
pub const CONTINUE: u64 = 4;
pub const SEND_SIGNAL: u64 = 5;
pub const INSPECT: u64 = 6;
pub const COPY: u64 = 7;
/// Priority's levels from here, in `Priority::ALL`'s order.
pub const PRIORITY: u64 = 10;

/// The menu over `targets`: the context menu with its Quit rows, More without.
pub fn entries(targets: &[Target], with_quit: bool) -> Rc<Vec<MenuEntry>> {
    let any = !targets.is_empty();
    let quittable = quittable(targets);
    let stopped = any && targets.iter().all(|t| t.stopped);
    let now = targets.first().and_then(|t| isle_sysinfo::act::nice_of(t.key.pid)).map(Priority::of);
    let mut out = Vec::new();
    if with_quit {
        out.push(MenuEntry::Item(MenuItem::new(QUIT, "Quit Process").enabled(quittable)));
        out.push(MenuEntry::Item(MenuItem::new(FORCE_QUIT, "Force Quit").enabled(quittable)));
        out.push(MenuEntry::Separator);
    }
    out.push(MenuEntry::Item(MenuItem::new(INSPECT, "Inspect Process").enabled(targets.len() == 1)));
    out.push(MenuEntry::Item(match stopped {
        true => MenuItem::new(CONTINUE, "Continue").enabled(quittable),
        false => MenuItem::new(STOP, "Stop").enabled(quittable),
    }));
    let levels = Priority::ALL
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let item = MenuItem::new(PRIORITY + i as u64, p.label()).enabled(quittable);
            MenuEntry::Item(if now == Some(*p) { item.mark(Mark::Check) } else { item })
        })
        .collect();
    out.push(MenuEntry::Item(MenuItem::new(PRIORITY - 1, "Priority").enabled(quittable).submenu(levels)));
    out.push(MenuEntry::Item(MenuItem::new(SEND_SIGNAL, "Send Signal to Process…").enabled(quittable)));
    if with_quit {
        out.push(MenuEntry::Separator);
        out.push(MenuEntry::Item(MenuItem::new(COPY, "Copy").enabled(any)));
    }
    Rc::new(out)
}

/// What Copy puts on the clipboard: each process's name, pid and command line.
pub fn copied(targets: &[Target]) -> String {
    targets.iter().map(|t| format!("{}\t{}\t{}", t.name, t.key.pid, t.cmdline)).collect::<Vec<_>>().join("\n")
}

/// The Quit alert's title (`structure/monitor.md` Copy).
pub fn quit_title(targets: &[Target]) -> String {
    match targets {
        [one] => format!("Quit “{}”?", one.name),
        many => format!("Quit {} processes?", many.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(name: &str, ppid: i32) -> Target {
        Target { key: Key { pid: 5, start: 1 }, name: name.into(), ppid, cgroup: String::new(), stopped: false, cmdline: format!("/usr/bin/{name} -x") }
    }

    #[test]
    fn the_session_cannot_be_quit_and_titles_count() {
        assert!(quittable(&[target("cargo", 2)]));
        assert!(!quittable(&[target("cargo", 2), target("isle-compositor", 2)]));
        assert!(!quittable(&[]));
        assert_eq!(quit_title(&[target("cargo", 2)]), "Quit “cargo”?");
        assert_eq!(quit_title(&[target("a", 2), target("b", 2)]), "Quit 2 processes?");
        assert_eq!(copied(&[target("cargo", 2)]), "cargo\t5\t/usr/bin/cargo -x");
    }
}
