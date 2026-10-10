//! Games known by the process that started them (`gaming.md` Games 1, Heroic and Lutris 1):
//! a window whose process descends from a launcher is a game, unless it is the launcher's
//! own.

/// The process names of the launchers games descend from.
const LAUNCHERS: &[&str] = &["steam", "heroic", "lutris"];

/// The process names of the launchers' own windows; `comm` keeps 15 bytes of the name.
const LAUNCHER_UI: &[&str] = &["steam", "steamwebhelper", "heroic", "lutris"];

/// Whether a window of process `own` with these ancestors, nearest first, was started as a
/// game by a launcher.
pub fn launched_game(own: &str, ancestors: &[String]) -> bool {
    !LAUNCHER_UI.contains(&own)
        && ancestors.iter().any(|name| LAUNCHERS.contains(&name.as_str()))
        && !ancestors.iter().any(|name| name == "steamwebhelper")
}

/// A process's name and parent, from `/proc/<pid>/stat`.
pub fn name_and_parent(pid: i32) -> Option<(String, i32)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (head, rest) = stat.rsplit_once(") ")?;
    let (_, name) = head.split_once(" (")?;
    let parent = rest.split_whitespace().nth(1)?.parse().ok()?;
    Some((name.to_owned(), parent))
}

/// `pid`'s own name and its ancestors' names, nearest first, up to init.
pub fn lineage(pid: i32) -> Option<(String, Vec<i32>, Vec<String>)> {
    let (own, mut parent) = name_and_parent(pid)?;
    let (mut pids, mut names) = (Vec::new(), Vec::new());
    while parent > 1 && pids.len() < 64 {
        let Some((name, next)) = name_and_parent(parent) else {
            break;
        };
        pids.push(parent);
        names.push(name);
        parent = next;
    }
    Some((own, pids, names))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_launchers_child_is_a_game_and_the_launcher_is_not() {
        // Steam's reaper, Proton's wine and the game.
        assert!(launched_game("Game.exe", &names(&["wineserver", "reaper", "steam", "systemd"])));
        assert!(launched_game("game", &names(&["legendary", "heroic", "systemd"])));
        assert!(launched_game("wine64-preload", &names(&["lutris", "bash"])));
        assert!(!launched_game("steam", &names(&["bash", "systemd"])));
        assert!(!launched_game("steamwebhelper", &names(&["steam", "systemd"])));
        assert!(!launched_game("heroic", &names(&["heroic", "systemd"])));
        assert!(!launched_game("kitty", &names(&["fish", "systemd"])));
    }

    #[test]
    fn a_browser_steam_opened_is_not_a_game() {
        // Steam's web helper opens a link in the browser through xdg-open.
        assert!(!launched_game("firefox", &names(&["xdg-open", "steamwebhelper", "steam"])));
    }

    #[test]
    fn this_process_has_a_lineage() {
        let (own, pids, names) = lineage(std::process::id() as i32).unwrap();
        assert!(!own.is_empty());
        assert_eq!(pids.len(), names.len());
    }
}
