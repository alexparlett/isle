//! The banner stack as a pure model: one card per app, three at most, newest at the anchor,
//! time paused while hovered, arrivals waiting while the pointer is on the stack or a panel
//! covers it. It takes a clock and says when it next changes (`p8-services.md` choice 7).

use std::{
    collections::VecDeque,
    path::PathBuf,
    time::{Duration, Instant},
};

/// Banners shown at once.
pub const MAX: usize = 3;

/// The banners' timings, from `timing-banner-*` in the tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timing {
    pub default: Duration,
    pub min: Duration,
    pub max: Duration,
    /// How long the stack waits after the pointer leaves before anything goes.
    pub resume: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            default: Duration::from_millis(5000),
            min: Duration::from_millis(3000),
            max: Duration::from_millis(15000),
            resume: Duration::from_millis(1500),
        }
    }
}

impl Timing {
    pub fn of(theme: &isle_tokens::Theme) -> Self {
        let ms = |v: u32| Duration::from_millis(v.into());
        let t = &theme.tokens;
        Self {
            default: ms(t.timing_banner_default_ms),
            min: ms(t.timing_banner_min_ms),
            max: ms(t.timing_banner_max_ms),
            resume: ms(t.timing_banner_resume_ms),
        }
    }

    /// A notification's life as a banner: its own timeout held between the bounds, the
    /// default when it leaves it to the server, the most when it asks for never.
    pub fn life(&self, expire_ms: i32) -> Duration {
        match expire_ms {
            ms if ms < 0 => self.default,
            0 => self.max,
            ms => Duration::from_millis(ms as u64).clamp(self.min, self.max),
        }
    }
}

/// What a banner shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Card {
    pub id: u32,
    /// The app, which groups cards: one per app.
    pub app: String,
    pub app_name: String,
    pub icon: Option<PathBuf>,
    /// An Isle part's glyph in the icon's place, for those with no app icon.
    pub glyph: Option<String>,
    pub summary: String,
    pub body: String,
    pub image: Option<PathBuf>,
    /// The buttons: key and label, `default` and `inline-reply` left out.
    pub actions: Vec<(String, String)>,
    pub reply: bool,
    pub progress: Option<f32>,
    /// Arrivals from the app this card stands for.
    pub count: u32,
    /// Stays until acted on or closed.
    pub persistent: bool,
    /// "Now", or how long ago a held one arrived.
    pub time: String,
}

#[derive(Clone, Debug, PartialEq)]
struct Shown {
    card: Card,
    /// When it leaves by itself; none while persistent or paused.
    until: Option<Instant>,
    life: Option<Duration>,
    /// What was left of its time when hovering paused it.
    left: Option<Duration>,
}

#[derive(Clone, Debug, PartialEq)]
struct Waiting {
    card: Card,
    life: Option<Duration>,
    since: Instant,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Banners {
    /// Newest first.
    shown: Vec<Shown>,
    waiting: VecDeque<Waiting>,
    /// Arrivals with no room, three persistent banners filling the stack.
    overflow: u32,
    hovered: bool,
    /// A panel, a menu or an overlay is in the way.
    blocked: bool,
    /// Until when the pointer's leaving holds everything.
    grace: Option<Instant>,
    pub timing: Timing,
}

impl Banners {
    /// A notification's banner, its life none when it is persistent. A second card from an
    /// app showing takes that card's place with its count; a replacement updates in place.
    pub fn arrive(&mut self, card: Card, life: Option<Duration>, now: Instant) {
        if let Some(shown) = self
            .shown
            .iter_mut()
            .find(|s| s.card.id == card.id || (s.card.app == card.app && !card.app.is_empty()))
        {
            let count = if shown.card.id == card.id {
                shown.card.count
            } else {
                shown.card.count + 1
            };
            shown.card = Card { count, ..card };
            shown.life = life;
            if self.hovered {
                shown.left = life;
                shown.until = None;
            } else {
                shown.until = life.map(|l| now + l);
            }
            return;
        }
        if self.hovered || self.blocked || self.grace.is_some_and(|g| now < g) {
            if let Some(w) = self.waiting.iter_mut().find(|w| w.card.app == card.app) {
                w.card = Card {
                    count: w.card.count + 1,
                    ..card
                };
                w.life = life;
            } else {
                self.waiting.push_back(Waiting {
                    card,
                    life,
                    since: now,
                });
            }
            return;
        }
        self.show(card, life, now);
    }

    fn show(&mut self, card: Card, life: Option<Duration>, now: Instant) {
        if self.shown.len() >= MAX {
            // The oldest temporary banner leaves early for the newcomer.
            match self.shown.iter().rposition(|s| s.life.is_some()) {
                Some(oldest) => {
                    self.shown.remove(oldest);
                }
                None => {
                    self.overflow += 1;
                    return;
                }
            }
        }
        self.shown.insert(
            0,
            Shown {
                card,
                until: life.map(|l| now + l),
                life,
                left: None,
            },
        );
    }

    /// Takes a notification's card off the stack, or out of the queue.
    pub fn remove(&mut self, id: u32) -> bool {
        let before = self.shown.len() + self.waiting.len();
        self.shown.retain(|s| s.card.id != id);
        self.waiting.retain(|w| w.card.id != id);
        let removed = before != self.shown.len() + self.waiting.len();
        // A slot opened: what overflowed is in the centre.
        if removed {
            self.overflow = 0;
        }
        removed
    }

    /// The pointer on the stack pauses every banner; leaving resumes them after a grace.
    pub fn set_hovered(&mut self, hovered: bool, now: Instant) {
        if hovered == self.hovered {
            return;
        }
        self.hovered = hovered;
        if hovered {
            for s in &mut self.shown {
                s.left = s.until.map(|u| u.saturating_duration_since(now));
                s.until = None;
            }
        } else {
            let grace = self.timing.resume;
            self.grace = Some(now + grace);
            for s in &mut self.shown {
                s.until = s.left.take().map(|left| now + grace + left);
            }
            // Time spent waiting under the pointer does not count against an arrival.
            for w in &mut self.waiting {
                w.since = now;
            }
        }
    }

    pub fn set_blocked(&mut self, blocked: bool, now: Instant) {
        self.blocked = blocked;
        self.tick(now);
    }

    /// Moves on to `now`: banners whose time is up leave, those waiting come in when they
    /// may, and those that waited longer than their life are dropped. Returns the ids that
    /// left the screen, which stay in the centre.
    pub fn tick(&mut self, now: Instant) -> Vec<u32> {
        let mut gone = Vec::new();
        // The card under the pointer went, by its action or its close, with no leave.
        if self.shown.is_empty() {
            self.hovered = false;
        }
        if !self.hovered {
            self.shown.retain(|s| {
                let expired = s.until.is_some_and(|u| u <= now);
                if expired {
                    gone.push(s.card.id);
                }
                !expired
            });
        }
        // One that waited behind a panel past its own life is not shown; a persistent one
        // waits on, and the pointer on the stack pauses waiting too.
        let hovered = self.hovered;
        self.waiting.retain(|w| match w.life {
            Some(life) if !hovered && w.since + life <= now => {
                gone.push(w.card.id);
                false
            }
            _ => true,
        });
        if self.grace.is_some_and(|g| g <= now) {
            self.grace = None;
        }
        if !self.hovered && !self.blocked && self.grace.is_none() {
            // What waited shows for its whole life once it may.
            while let Some(w) = self.waiting.pop_front() {
                self.show(w.card, w.life, now);
            }
        }
        if self.shown.is_empty() {
            self.overflow = 0;
        }
        gone
    }

    /// When the stack next changes by itself.
    pub fn next_change(&self) -> Option<Instant> {
        let mut next: Option<Instant> = None;
        let mut take = |at: Instant| next = Some(next.map_or(at, |n| n.min(at)));
        for s in &self.shown {
            if let Some(until) = s.until {
                take(until);
            }
        }
        if !self.hovered {
            for w in &self.waiting {
                if let Some(life) = w.life {
                    take(w.since + life);
                }
            }
        }
        if let Some(grace) = self.grace {
            take(grace);
        }
        next
    }

    /// The cards shown, newest first.
    pub fn cards(&self) -> Vec<&Card> {
        self.shown.iter().map(|s| &s.card).collect()
    }

    /// Arrivals with no room, shown as "{n} more in Notification Centre".
    pub fn overflow(&self) -> u32 {
        self.overflow
    }

    pub fn is_empty(&self) -> bool {
        self.shown.is_empty()
    }

    /// Nothing shown and nothing waiting.
    pub fn idle(&self) -> bool {
        self.shown.is_empty() && self.waiting.is_empty()
    }

    pub fn hovered(&self) -> bool {
        self.hovered
    }

    /// Everything goes: Notification Centre opened, or the session locked.
    pub fn clear(&mut self) {
        self.shown.clear();
        self.waiting.clear();
        self.overflow = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: u32, app: &str) -> Card {
        Card {
            id,
            app: app.into(),
            count: 1,
            ..Default::default()
        }
    }

    const FIVE: Option<Duration> = Some(Duration::from_secs(5));

    const GRACE: Duration = Duration::from_millis(1500);

    #[test]
    fn lives_are_held_between_three_and_fifteen_seconds() {
        let t = Timing::default();
        assert_eq!(t.life(-1), Duration::from_secs(5));
        assert_eq!(t.life(0), Duration::from_secs(15));
        assert_eq!(t.life(500), Duration::from_secs(3));
        assert_eq!(t.life(60_000), Duration::from_secs(15));
        assert_eq!(t.life(8_000), Duration::from_secs(8));
    }

    #[test]
    fn an_empty_stack_is_not_hovered_so_arrivals_show() {
        let t = Instant::now();
        let mut b = Banners::default();
        b.arrive(card(1, "mail"), FIVE, t);
        b.set_hovered(true, t);
        b.remove(1);
        b.tick(t);
        assert!(!b.hovered());
        b.arrive(card(2, "chat"), FIVE, t);
        b.tick(t);
        assert_eq!(b.cards().len(), 1);
    }

    #[test]
    fn one_banner_per_app_counts_its_arrivals_and_restarts_its_time() {
        let t = Instant::now();
        let mut b = Banners::default();
        b.arrive(card(1, "mail"), FIVE, t);
        b.arrive(card(2, "chat"), FIVE, t + Duration::from_secs(1));
        b.arrive(card(3, "mail"), FIVE, t + Duration::from_secs(4));
        let cards = b.cards();
        assert_eq!(cards.len(), 2);
        // It does not move: chat stays newest.
        assert_eq!(
            (cards[0].app.as_str(), cards[1].id, cards[1].count),
            ("chat", 3, 2)
        );
        assert!(b.tick(t + Duration::from_secs(6)).contains(&2));
        assert_eq!(b.cards().len(), 1);
        assert_eq!(b.tick(t + Duration::from_secs(9)), vec![3]);
    }

    #[test]
    fn a_fourth_app_pushes_the_oldest_temporary_out_and_persistent_ones_overflow() {
        let t = Instant::now();
        let mut b = Banners::default();
        b.arrive(card(1, "a"), None, t);
        b.arrive(card(2, "b"), FIVE, t);
        b.arrive(card(3, "c"), None, t);
        b.arrive(card(4, "d"), FIVE, t);
        let ids: Vec<u32> = b.cards().iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![4, 3, 1]);
        let mut all = Banners::default();
        for i in 1..=3 {
            all.arrive(card(i, &i.to_string()), None, t);
        }
        all.arrive(card(9, "nine"), FIVE, t);
        assert_eq!((all.cards().len(), all.overflow()), (3, 1));
    }

    #[test]
    fn hovering_pauses_time_and_holds_arrivals_until_after_the_grace() {
        let t = Instant::now();
        let mut b = Banners::default();
        b.arrive(card(1, "mail"), FIVE, t);
        b.set_hovered(true, t + Duration::from_secs(4));
        b.arrive(card(2, "chat"), FIVE, t + Duration::from_secs(4));
        assert!(b.tick(t + Duration::from_secs(30)).is_empty());
        assert_eq!(b.cards().len(), 1);
        b.set_hovered(false, t + Duration::from_secs(31));
        b.tick(t + Duration::from_secs(31));
        assert_eq!(b.cards().len(), 1, "nothing moves in the grace");
        b.tick(t + Duration::from_secs(31) + GRACE);
        assert_eq!(b.cards().len(), 2);
        // The paused one had a second left, which runs after the grace.
        assert!(b.tick(t + Duration::from_secs(32) + GRACE).contains(&1));
    }

    #[test]
    fn a_banner_waiting_past_its_life_is_dropped_but_a_persistent_one_waits() {
        let t = Instant::now();
        let mut b = Banners::default();
        b.set_blocked(true, t);
        b.arrive(card(1, "mail"), FIVE, t);
        b.arrive(card(2, "alarm"), None, t);
        assert_eq!(b.tick(t + Duration::from_secs(6)), vec![1]);
        b.set_blocked(false, t + Duration::from_secs(7));
        let ids: Vec<u32> = b.cards().iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![2]);
    }

    #[test]
    fn the_next_change_is_the_soonest_end() {
        let t = Instant::now();
        let mut b = Banners::default();
        assert_eq!(b.next_change(), None);
        b.arrive(card(1, "mail"), Some(Duration::from_secs(8)), t);
        b.arrive(card(2, "chat"), Some(Duration::from_secs(3)), t);
        assert_eq!(b.next_change(), Some(t + Duration::from_secs(3)));
    }
}
