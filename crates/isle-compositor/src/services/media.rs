//! Media: the island's media activity, the Now Playing module and page, and the transport
//! keys, on the most recently playing player.

use std::time::{Duration, Instant};

use isle_bus::Snapshot;
use isle_media::{Media, Player, Status};
use isle_shell::island::{
    grid::TileSize,
    model::{Activity, ActivityKind},
    panel::{Module, PanelAction},
    rows::{Choice, Lead, MediaCard, PageRow},
};
use tracing::debug;

use crate::state::{Isle, State};

fn clock(micros: u64) -> String {
    let seconds = micros / 1_000_000;
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

impl Isle {
    /// The player shown: one the user chose on the page while it lasts, else the most
    /// recently playing.
    pub fn chosen_player(&self) -> Option<&Player> {
        let players = &self.services.media.state.players;
        self.services
            .chosen_player
            .as_ref()
            .and_then(|bus| players.iter().find(|p| &p.bus == bus))
            .or_else(|| self.services.media.state.chosen())
            .filter(|_| self.services.media.available)
    }

    /// Where a player is now, running on from when it was last read.
    fn position(&self, player: &Player) -> u64 {
        let ran = if player.status == Status::Playing {
            (self.services.media_read.elapsed().as_secs_f64() * player.rate * 1e6) as u64
        } else {
            0
        };
        let at = player.position + ran;
        player.length.map_or(at, |l| at.min(l))
    }

    fn card(&self, player: &Player) -> MediaCard {
        let position = self.position(player);
        MediaCard {
            player: self.app_or_identity(player),
            title: if player.title.is_empty() {
                player.identity.clone()
            } else {
                player.title.clone()
            },
            artist: player.artist.clone(),
            art: player
                .art_url
                .as_ref()
                .and_then(|u| self.services.art.get(u).cloned().flatten()),
            playing: player.status == Status::Playing,
            can_previous: player.can_go_previous,
            can_next: player.can_go_next,
            position: player
                .length
                .filter(|_| player.can_seek && player.track.is_some())
                .map(|l| position as f32 / l as f32),
            elapsed: clock(position),
            remaining: player
                .length
                .map(|l| format!("-{}", clock(l.saturating_sub(position))))
                .unwrap_or_default(),
            shuffle: player.shuffle,
            repeat: player.loop_status.clone(),
        }
    }

    /// A player's app name from its desktop entry, else what it calls itself.
    fn app_or_identity(&self, player: &Player) -> String {
        player
            .desktop_entry
            .as_ref()
            .map(|d| self.app_name(d))
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| player.identity.clone())
    }

    pub fn now_playing_module(&self, size: TileSize) -> Option<Module> {
        let chosen = self.chosen_player();
        let card = chosen.map(|p| self.card(p));
        let playing = card.as_ref().is_some_and(|c| c.playing);
        let mut rows = Vec::new();
        if let Some(card) = card.clone() {
            rows.push(PageRow::Media(card));
        } else {
            rows.push(PageRow::Status("Not playing".into()));
        }
        let others: Vec<&Player> = self
            .services
            .media
            .state
            .players
            .iter()
            .filter(|p| Some(&p.bus) != chosen.map(|c| &c.bus))
            .collect();
        if !others.is_empty() {
            rows.push(PageRow::Heading("Players".into()));
            rows.extend(others.iter().map(|p| {
                PageRow::Choice(Choice {
                    id: format!("player:{}", p.bus),
                    label: self.app_or_identity(p),
                    detail: (!p.title.is_empty()).then(|| p.title.clone()),
                    lead: Lead::Glyph(
                        if p.status == Status::Playing {
                            "play"
                        } else {
                            "pause"
                        }
                        .into(),
                    ),
                    enabled: true,
                    ..Default::default()
                })
            }));
        }
        Some(Module {
            id: "now-playing".into(),
            size,
            name: "Now Playing".into(),
            state: card
                .as_ref()
                .map_or("Not playing".into(), |c| c.title.clone()),
            glyph: if playing { "pause" } else { "play" }.into(),
            on: false,
            toggles: card.is_some(),
            page: Some(rows),
            pane: "Sound".into(),
            plain_head: true,
            media: card.or(Some(MediaCard {
                title: "Not playing".into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}

impl State {
    pub fn media_changed(&mut self, snapshot: Snapshot<Media>) {
        self.isle.services.media = snapshot;
        self.isle.services.media_read = Instant::now();
        let chosen = self.isle.chosen_player().cloned();
        if let Some(url) = chosen.as_ref().and_then(|p| p.art_url.clone())
            && !self.isle.services.art.contains_key(&url)
        {
            self.isle.services.art.insert(url.clone(), None);
            let fetch = url.clone();
            self.isle.run_service(
                async move { Ok(isle_media::art(&fetch).await) },
                move |state, path| {
                    state.isle.services.art.insert(url, path.ok().flatten());
                    state.media_activity();
                    state.isle.refresh_island();
                },
            );
        }
        self.media_activity();
        self.isle.services_changed();
    }

    /// The island's media activity: the chosen player while it plays.
    fn media_activity(&mut self) {
        let isle = &mut self.isle;
        let Some(player) = isle
            .chosen_player()
            .filter(|p| p.status == Status::Playing)
            .cloned()
        else {
            isle.island.model.end_activity(ActivityKind::Media);
            return;
        };
        let card = isle.card(&player);
        let started = isle
            .island
            .model
            .activities()
            .find(|a| a.kind == ActivityKind::Media)
            .map_or_else(Instant::now, |a| a.started);
        isle.island.model.set_activity(
            Activity {
                kind: ActivityKind::Media,
                face: card.title.clone(),
                title: card.title,
                detail: card.artist,
                actions: Vec::new(),
                app: player.desktop_entry.clone(),
                art: card.art,
                started,
            },
            Instant::now(),
        );
    }

    /// The play, next and previous keys, on the chosen player.
    pub fn media_key(&mut self, key: crate::binds::Media) {
        use crate::binds::Media as Key;
        let Some(bus) = self.isle.chosen_player().map(|p| p.bus.clone()) else {
            return;
        };
        self.isle.run_service(
            async move {
                match key {
                    Key::Next => isle_media::next(&bus).await,
                    Key::Previous => isle_media::previous(&bus).await,
                    _ => isle_media::play_pause(&bus).await,
                }
            },
            |_, result| {
                if let Err(err) = result {
                    debug!("a media key: {err}");
                }
            },
        );
    }

    pub fn media_action(&mut self, action: PanelAction) {
        let Some(player) = self.isle.chosen_player().cloned() else {
            return;
        };
        let bus = player.bus.clone();
        let command: std::pin::Pin<Box<dyn Future<Output = Result<(), isle_bus::Error>> + Send>> =
            match action {
                PanelAction::Toggle(_) => {
                    Box::pin(async move { isle_media::play_pause(&bus).await })
                }
                PanelAction::Choose { row, .. } => match row.as_str() {
                    "play-pause" => Box::pin(async move { isle_media::play_pause(&bus).await }),
                    "next" => Box::pin(async move { isle_media::next(&bus).await }),
                    "previous" => Box::pin(async move { isle_media::previous(&bus).await }),
                    "raise" => Box::pin(async move { isle_media::raise(&bus).await }),
                    "shuffle" => {
                        let on = !player.shuffle.unwrap_or(false);
                        Box::pin(async move { isle_media::set_shuffle(&bus, on).await })
                    }
                    "repeat" => {
                        let next = match player.loop_status.as_deref() {
                            Some("None") => "Playlist",
                            Some("Playlist") => "Track",
                            _ => "None",
                        };
                        Box::pin(async move { isle_media::set_loop(&bus, next).await })
                    }
                    other => {
                        if let Some(chosen) = other.strip_prefix("player:") {
                            self.isle.services.chosen_player = Some(chosen.to_owned());
                            self.media_activity();
                        }
                        return;
                    }
                },
                PanelAction::Slide { value, .. } => {
                    let (Some(length), Some(track)) = (player.length, player.track.clone()) else {
                        return;
                    };
                    let to = (value.clamp(0.0, 1.0) as f64 * length as f64) as u64;
                    Box::pin(async move { isle_media::set_position(&bus, &track, to).await })
                }
                _ => return,
            };
        self.isle.run_service(command, |state, result| {
            state
                .isle
                .service_error("now-playing", result, "The player didn't answer");
        });
    }

    /// Redraws the page's scrubber once a second while it shows a player that plays.
    pub fn follow_media_clock(&mut self) {
        let isle = &mut self.isle;
        let wanted = isle
            .island
            .panel
            .as_ref()
            .and_then(|p| p.page.as_deref())
            .is_some_and(|p| p == "now-playing")
            && isle
                .chosen_player()
                .is_some_and(|p| p.status == Status::Playing);
        match (wanted, isle.services.media_clock.is_some()) {
            (true, false) => {
                let (Some(runtime), Some(to_loop)) = (&isle.worker, &isle.to_loop) else {
                    return;
                };
                let to_loop = to_loop.clone();
                isle.services.media_clock = Some(runtime.spawn(async move {
                    loop {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        to_loop.post(|state| state.isle.refresh_island());
                    }
                }));
            }
            (false, true) => {
                if let Some(task) = isle.services.media_clock.take() {
                    task.abort();
                }
            }
            _ => {}
        }
    }
}
