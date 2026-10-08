//! The prompt queue: which system prompt shows, which wait and why (`structure/prompts.md`,
//! One at a time and Waiting). Pure, as the banners' model is.

use std::collections::HashSet;

/// Why a prompt is not on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wait {
    /// Another prompt is ahead of it.
    Behind,
    Locked,
    /// A full-screen game is in front.
    Game,
    /// A full-screen window of another app is in front.
    FullScreen,
    /// Cmd+Tab set it aside.
    SetAside,
}

/// A request as the queue sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: u64,
    /// The asking app, by desktop id.
    pub app: Option<String>,
    /// The user started it: a network or device chosen in Control Centre, `pkexec` in the
    /// front terminal.
    pub user_started: bool,
}

/// What holds every prompt back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Holds {
    pub locked: bool,
    pub game_in_front: bool,
    /// The app of the full-screen window in front, when one is.
    pub full_screen_app: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Queue {
    /// In the order they will show: the user's own first, then by arrival.
    entries: Vec<Entry>,
    set_aside: HashSet<u64>,
}

impl Queue {
    /// Takes a request in; one the user started goes ahead of the others but never ahead of
    /// `showing`, the prompt on screen.
    pub fn push(&mut self, entry: Entry, showing: Option<u64>) {
        let after = showing
            .and_then(|id| self.entries.iter().position(|e| e.id == id))
            .map_or(0, |i| i + 1);
        let at = if entry.user_started {
            self.entries
                .iter()
                .enumerate()
                .skip(after)
                .find(|(_, e)| !e.user_started)
                .map_or(self.entries.len(), |(i, _)| i)
        } else {
            self.entries.len()
        };
        self.entries.insert(at, entry);
    }

    /// Answered or cancelled, on screen or waiting.
    pub fn remove(&mut self, id: u64) -> bool {
        self.set_aside.remove(&id);
        let before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.entries.len() != before
    }

    pub fn set_aside(&mut self, id: u64) {
        if self.entries.iter().any(|e| e.id == id) {
            self.set_aside.insert(id);
        }
    }

    /// Show from the island: back as a new prompt.
    pub fn bring_back(&mut self, id: u64) {
        self.set_aside.remove(&id);
        if let Some(at) = self.entries.iter().position(|e| e.id == id) {
            let entry = self.entries.remove(at);
            self.entries.insert(0, entry);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    fn held(&self, entry: &Entry, holds: &Holds) -> Option<Wait> {
        if holds.locked {
            return Some(Wait::Locked);
        }
        if self.set_aside.contains(&entry.id) {
            return Some(Wait::SetAside);
        }
        if holds.game_in_front {
            return Some(Wait::Game);
        }
        match &holds.full_screen_app {
            Some(app) if entry.app.as_ref() != Some(app) => Some(Wait::FullScreen),
            _ => None,
        }
    }

    /// The prompt on screen: the first nothing holds back.
    pub fn showing(&self, holds: &Holds) -> Option<u64> {
        self.entries
            .iter()
            .find(|e| self.held(e, holds).is_none())
            .map(|e| e.id)
    }

    /// Every prompt not on screen, in order, with why.
    pub fn waiting(&self, holds: &Holds) -> Vec<(u64, Wait)> {
        let showing = self.showing(holds);
        self.entries
            .iter()
            .filter(|e| Some(e.id) != showing)
            .map(|e| {
                let why = self.held(e, holds).unwrap_or(Wait::Behind);
                (e.id, why)
            })
            .collect()
    }

    /// Prompts waiting while one shows: "1 more request".
    pub fn more(&self, holds: &Holds) -> usize {
        match self.showing(holds) {
            Some(_) => self.entries.len() - 1,
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u64, app: &str, user: bool) -> Entry {
        Entry {
            id,
            app: Some(app.into()),
            user_started: user,
        }
    }

    #[test]
    fn one_shows_and_the_rest_wait_in_arrival_order() {
        let mut q = Queue::default();
        q.push(entry(1, "a", false), None);
        q.push(entry(2, "b", false), None);
        q.push(entry(3, "c", false), None);
        let holds = Holds::default();
        assert_eq!(q.showing(&holds), Some(1));
        assert_eq!(
            q.waiting(&holds),
            vec![(2, Wait::Behind), (3, Wait::Behind)]
        );
        assert_eq!(q.more(&holds), 2);
        q.remove(1);
        assert_eq!(q.showing(&holds), Some(2));
    }

    #[test]
    fn what_the_user_started_goes_first_in_its_own_order() {
        let mut q = Queue::default();
        q.push(entry(1, "keyring", false), None);
        q.push(entry(2, "wifi", true), None);
        q.push(entry(3, "bluetooth", true), None);
        assert_eq!(q.showing(&Holds::default()), Some(2));
        q.remove(2);
        assert_eq!(q.showing(&Holds::default()), Some(3));
    }

    #[test]
    fn the_users_own_never_goes_ahead_of_the_prompt_on_screen() {
        let mut q = Queue::default();
        q.push(entry(1, "keyring", false), None);
        q.push(entry(2, "polkit", false), Some(1));
        q.push(entry(3, "wifi", true), Some(1));
        assert_eq!(q.showing(&Holds::default()), Some(1));
        q.remove(1);
        assert_eq!(q.showing(&Holds::default()), Some(3));
    }

    #[test]
    fn a_cancel_removes_one_on_screen_or_waiting() {
        let mut q = Queue::default();
        q.push(entry(1, "a", false), None);
        q.push(entry(2, "b", false), None);
        assert!(q.remove(2));
        assert_eq!(q.waiting(&Holds::default()), vec![]);
        assert!(!q.remove(9));
    }

    #[test]
    fn the_lock_and_a_game_hold_every_prompt() {
        let mut q = Queue::default();
        q.push(entry(1, "a", false), None);
        let locked = Holds {
            locked: true,
            ..Default::default()
        };
        assert_eq!(q.showing(&locked), None);
        assert_eq!(q.waiting(&locked), vec![(1, Wait::Locked)]);
        let game = Holds {
            game_in_front: true,
            ..Default::default()
        };
        assert_eq!(q.waiting(&game), vec![(1, Wait::Game)]);
    }

    #[test]
    fn a_full_screen_window_holds_all_but_its_own_apps() {
        let mut q = Queue::default();
        q.push(entry(1, "keyring", false), None);
        q.push(entry(2, "kitty", false), None);
        let holds = Holds {
            full_screen_app: Some("kitty".into()),
            ..Default::default()
        };
        assert_eq!(q.showing(&holds), Some(2));
        assert_eq!(q.waiting(&holds), vec![(1, Wait::FullScreen)]);
    }

    #[test]
    fn set_aside_waits_until_brought_back_first() {
        let mut q = Queue::default();
        q.push(entry(1, "a", false), None);
        q.push(entry(2, "b", false), None);
        q.set_aside(1);
        assert_eq!(q.showing(&Holds::default()), Some(2));
        q.bring_back(1);
        assert_eq!(q.showing(&Holds::default()), Some(1));
    }
}
