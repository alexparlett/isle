//! Wi-Fi, Bluetooth, Network and VPN, through NetworkManager and BlueZ.

use freya::prelude::*;
use isle_network::Link;
use isle_ui::{Glyph, Switch, colour, form, text, use_theme};

use super::Nav;
use crate::{
    nav::Location,
    services::{Act, Follow, use_follow},
    ui::{self, MoreMenu, Rows},
};

fn link_word(link: Link) -> &'static str {
    match link {
        Link::Connected => "Connected",
        Link::Connecting => "Connecting",
        Link::Unavailable => "Unavailable",
        Link::Failed => "Failed",
        Link::Disconnected => "Not connected",
    }
}

/// A status dot: green while connected, amber while connecting, grey otherwise.
fn dot(link: Link) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    let fill = match link {
        Link::Connected => t.colour_ok,
        Link::Connecting => t.colour_warn,
        _ => t.colour_text_3,
    };
    rect()
        .width(Size::px(8.))
        .height(Size::px(8.))
        .corner_radius(4.)
        .background(colour(fill))
        .into_element()
}

/// A network's lock and signal glyphs.
fn signal(strength: u8, secured: bool) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    rect()
        .horizontal()
        .spacing(t.space_xs)
        .maybe(secured, |el| {
            el.child(Glyph::new("lock-simple", 12., colour(t.colour_text_2)))
        })
        .child(Glyph::new(
            isle_network::signal_glyph(strength),
            16.,
            colour(t.colour_text),
        ))
        .into_element()
}

/// Read-only rows for a device's addresses.
fn detail_rows(r: &Rows, details: &isle_network::Details, hardware_label: &str) -> Vec<Element> {
    let or_none = |v: &Option<String>| v.clone().unwrap_or_else(|| "—".into());
    vec![
        r.info("ip-address", "IP address", or_none(&details.address))
            .into_element(),
        r.info("subnet", "Subnet mask", or_none(&details.subnet))
            .into_element(),
        r.info("router", "Router", or_none(&details.router))
            .into_element(),
        r.info(
            "dns",
            "DNS servers",
            if details.dns.is_empty() {
                "—".into()
            } else {
                details.dns.join("\n")
            },
        )
        .into_element(),
        r.info("hardware", hardware_label, or_none(&details.hardware))
            .into_element(),
    ]
}

/// A device's details, read on the worker when shown.
fn use_details(device: Option<String>) -> State<Option<isle_network::Details>> {
    let worker = isle_app::worker();
    let details = use_state(|| None);
    use_hook(move || {
        let Some(device) = device else { return };
        let job = worker.spawn(async move { isle_network::details(&device).await.ok() });
        let mut details = details;
        spawn(async move {
            if let Ok(found) = job.await {
                details.set(found);
            }
        });
    });
    details
}

#[derive(PartialEq)]
pub struct WiFi {
    pub at: Location,
}

impl Component for WiFi {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Network]);
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let snapshot = services.network.read().clone();
        let state = snapshot.state;
        let device = state.wifi.as_ref().map(|w| w.path.clone());
        // Scans once shown and the device is known; NetworkManager keeps what it found fresh.
        let mut scanned = use_state(|| None::<String>);
        if device.is_some() && *scanned.peek() != device {
            scanned.set(device.clone());
            if let Some(device) = device.clone() {
                services.act(Act::Scan(device));
            }
        }
        let Some(wifi) = state.wifi.clone() else {
            return ui::missing("No Wi-Fi device is connected.");
        };
        let at = self.at.clone();
        let mut top = vec![
            r.row("wi-fi", "Wi-Fi", None)
                .control(Switch::new(state.wifi_enabled, "Wi-Fi", move |on: bool| {
                    services.act(Act::SetWifi(on))
                }))
                .into_element(),
        ];
        if let Some(current) = wifi
            .current
            .clone()
            .filter(|_| wifi.link == Link::Connected)
        {
            let at = at.clone();
            top.push(
                r.row("current", current.ssid.clone(), None)
                    .description(link_word(wifi.link))
                    .leading(signal(current.strength, current.secured))
                    .control(
                        isle_ui::Button::new("Details…")
                            .on_press(move |_| super::open(nav, &at, "Details…")),
                    )
                    .into_element(),
            );
        }
        let known: Vec<Element> = state
            .known
            .iter()
            .map(|k| {
                let (path, auto) = (k.path.clone(), k.auto_join);
                r.row(&format!("known:{}", k.ssid), k.ssid.clone(), None)
                    .control(MoreMenu {
                        alt: format!("{} options", k.ssid),
                        items: vec![
                            ("Auto-Join".into(), auto),
                            ("Forget This Network…".into(), false),
                        ],
                        on_choose: EventHandler::new(move |i: usize| match i {
                            0 => services.act(Act::AutoJoin {
                                saved: path.clone(),
                                on: !auto,
                            }),
                            _ => services.act(Act::Forget(path.clone())),
                        }),
                    })
                    .into_element()
            })
            .collect();
        let others: Vec<Element> = wifi
            .networks
            .iter()
            .filter(|n| wifi.current.as_ref().is_none_or(|c| c.ssid != n.ssid))
            .map(|n| {
                let (device, ssid) = (wifi.path.clone(), n.ssid.clone());
                r.row(&format!("network:{}", n.ssid), n.ssid.clone(), None)
                    .leading(signal(n.strength, n.secured))
                    .control(isle_ui::Button::new("Connect").small().on_press(move |_| {
                        services.act(Act::Join {
                            device: device.clone(),
                            ssid: ssid.clone(),
                        })
                    }))
                    .into_element()
            })
            .collect();
        let sheet = self.at.sub.clone();
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(form::group(None, top))
            .maybe(state.wifi_enabled && !known.is_empty(), |el| {
                el.child(form::group(Some("Known networks".into()), known))
            })
            .maybe(state.wifi_enabled, |el| {
                el.child(if others.is_empty() {
                    form::group(
                        Some("Other networks".into()),
                        vec![
                            form::Row::new("Looking for networks…")
                                .enabled(false)
                                .into_element(),
                        ],
                    )
                    .into_element()
                } else {
                    form::group(Some("Other networks".into()), others).into_element()
                })
                .child(ui::foot(vec![(
                    "Join other network…".into(),
                    EventHandler::new({
                        let at = at.clone();
                        move |_| super::open(nav, &at, "Join other network…")
                    }),
                )]))
            })
            .maybe(sheet.as_deref() == Some("Details…"), |el| {
                el.child(WifiDetails { at: at.clone() })
            })
            .maybe(sheet.as_deref() == Some("Join other network…"), |el| {
                el.child(JoinOther {
                    at: at.clone(),
                    device: wifi.path.clone(),
                })
            })
            .into_element()
    }
}

#[derive(PartialEq)]
struct WifiDetails {
    at: Location,
}

impl Component for WifiDetails {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let services = crate::services::Services::get();
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let state = services.network.read().state.clone();
        let wifi = state.wifi.clone();
        let details = use_details(wifi.as_ref().map(|w| w.path.clone()));
        let mut forgetting = use_state(|| false);
        let current = wifi.as_ref().and_then(|w| w.current.clone());
        let ssid = current.as_ref().map(|c| c.ssid.clone()).unwrap_or_default();
        let saved = state.known.iter().find(|k| k.ssid == ssid).cloned();
        let at = self.at.clone();
        let done = EventHandler::new({
            let at = at.clone();
            move |_| super::close(nav, &at)
        });
        if *forgetting.read() {
            let saved = saved.clone();
            return ui::sheet(
                360.,
                vec![ui::alert_text(
                    &format!("Forget “{ssid}”?"),
                    "This computer won't join it again until you enter its password.",
                )],
                vec![
                    (
                        "Cancel".into(),
                        EventHandler::new(move |_| forgetting.set(false)),
                    ),
                    (
                        "Forget".into(),
                        EventHandler::new(move |_| {
                            if let Some(saved) = &saved {
                                services.act(Act::Forget(saved.path.clone()));
                            }
                            super::close(nav, &at);
                        }),
                    ),
                ],
            );
        }
        let mut rows = Vec::new();
        if let Some(saved) = &saved {
            let (path, auto) = (saved.path.clone(), saved.auto_join);
            rows.push(
                r.switch_with(
                    "auto-join",
                    "Automatically join this network",
                    auto,
                    move |on| {
                        services.act(Act::AutoJoin {
                            saved: path.clone(),
                            on,
                        })
                    },
                )
                .into_element(),
            );
        }
        if let Some(details) = details.read().clone() {
            rows.extend(detail_rows(&r, &details, "Wi-Fi address"));
        }
        rows.push(
            r.info(
                "security",
                "Security",
                if current.as_ref().is_some_and(|c| c.secured) {
                    "WPA2/WPA3 Personal"
                } else {
                    "None"
                },
            )
            .into_element(),
        );
        ui::sheet(
            520.,
            vec![
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(t.space_s)
                    .child(text::line(
                        &theme,
                        text::Style::Headline,
                        colour(t.colour_text),
                        ssid.clone(),
                    ))
                    .map(current.as_ref(), |el, c| {
                        el.child(signal(c.strength, c.secured))
                    })
                    .into_element(),
                form::group(None, rows).into_element(),
                rect()
                    .child(
                        isle_ui::Button::new("Forget This Network…")
                            .kind(isle_ui::button::Kind::Plain)
                            .enabled(saved.is_some())
                            .on_press(move |_| forgetting.set(true)),
                    )
                    .into_element(),
            ],
            vec![("Done".into(), done)],
        )
    }
}

#[derive(PartialEq)]
struct JoinOther {
    at: Location,
    device: String,
}

impl Component for JoinOther {
    fn render(&self) -> impl IntoElement {
        let services = crate::services::Services::get();
        let nav = Nav::get();
        let name = use_state(String::new);
        let mut security = use_state(|| 1usize);
        let at = self.at.clone();
        let device = self.device.clone();
        let cancel = EventHandler::new({
            let at = at.clone();
            move |_| super::close(nav, &at)
        });
        ui::sheet(
            400.,
            vec![
                form::group(
                    None,
                    vec![
                        form::Row::new("Network name")
                            .control(
                                isle_ui::TextField::new(name, "Network name").width(Size::px(200.)),
                            )
                            .into_element(),
                        form::Row::new("Security")
                            .control(form::PopUp::new(
                                vec![
                                    "None".into(),
                                    "WPA2/WPA3 Personal".into(),
                                    "WPA3 Personal".into(),
                                    "WPA2 Enterprise".into(),
                                ],
                                *security.read(),
                                "Security",
                                move |i: usize| security.set(i),
                            ))
                            .into_element(),
                    ],
                )
                .into_element(),
            ],
            vec![
                ("Cancel".into(), cancel),
                (
                    "Join".into(),
                    EventHandler::new(move |_| {
                        let ssid = name.peek().trim().to_owned();
                        if ssid.is_empty() {
                            return;
                        }
                        services.act(Act::JoinHidden {
                            device: device.clone(),
                            ssid,
                            secured: *security.peek() > 0,
                        });
                        super::close(nav, &at);
                    }),
                ),
            ],
        )
    }
}

#[derive(PartialEq)]
pub struct Bluetooth {
    pub at: Location,
}

impl Component for Bluetooth {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Bluetooth]);
        let r = Rows::new(&self.at);
        let theme = isle_ui::theme();
        let t = &theme.tokens;
        let state = services.bluetooth.read().state.clone();
        let adapter = state.adapter.clone();
        // Discovers and is discoverable while shown and powered.
        let path = adapter
            .as_ref()
            .filter(|a| a.powered)
            .map(|a| a.path.clone());
        let discovering = use_hook(|| std::rc::Rc::new(std::cell::RefCell::new(None::<String>)));
        if *discovering.borrow() != path {
            if let Some(adapter) = discovering.borrow().clone() {
                services.act(Act::Discover { adapter, on: false });
            }
            if let Some(adapter) = path.clone() {
                services.act(Act::Discover { adapter, on: true });
            }
            *discovering.borrow_mut() = path;
        }
        use_drop({
            let discovering = discovering.clone();
            move || {
                if let Some(adapter) = discovering.borrow().clone() {
                    services.act(Act::Discover { adapter, on: false });
                }
            }
        });
        let mut renaming = use_state(|| None::<(String, String)>);
        let Some(adapter) = adapter else {
            return ui::missing("No Bluetooth adapter is connected.");
        };
        let path = adapter.path.clone();
        let mut power = r.row("bluetooth", "Bluetooth", None).control(Switch::new(
            adapter.powered,
            "Bluetooth",
            move |on: bool| {
                services.act(Act::SetBluetooth {
                    adapter: path.clone(),
                    on,
                })
            },
        ));
        if adapter.powered {
            power = power.description(format!("Now discoverable as “{}”.", adapter.name));
        }
        let mine: Vec<Element> = state
            .devices
            .iter()
            .filter(|d| d.paired)
            .map(|d| {
                let mut status = if d.connected {
                    "Connected".to_owned()
                } else {
                    "Not connected".to_owned()
                };
                if let Some(battery) = d.battery {
                    status.push_str(&format!(" · {battery}%"));
                }
                let (device, adapter, connected, name) = (
                    d.path.clone(),
                    adapter.path.clone(),
                    d.connected,
                    d.name.clone(),
                );
                r.row(&d.path, d.name.clone(), None)
                    .leading(Glyph::new(d.glyph(), 18., colour(t.colour_text_2)))
                    .description(status)
                    .control(MoreMenu {
                        alt: format!("{} options", d.name),
                        items: vec![
                            (
                                if connected { "Disconnect" } else { "Connect" }.into(),
                                false,
                            ),
                            ("Rename…".into(), false),
                            ("Forget This Device…".into(), false),
                        ],
                        on_choose: EventHandler::new(move |i: usize| match i {
                            0 if connected => services.act(Act::BtDisconnect(device.clone())),
                            0 => services.act(Act::BtConnect(device.clone())),
                            1 => renaming.set(Some((device.clone(), name.clone()))),
                            _ => services.act(Act::BtForget {
                                adapter: adapter.clone(),
                                device: device.clone(),
                            }),
                        }),
                    })
                    .into_element()
            })
            .collect();
        let nearby: Vec<Element> = state
            .devices
            .iter()
            .filter(|d| !d.paired)
            .map(|d| {
                let device = d.path.clone();
                r.row(&d.path, d.name.clone(), None)
                    .leading(Glyph::new(d.glyph(), 18., colour(t.colour_text_2)))
                    .control(
                        isle_ui::Button::new("Connect")
                            .small()
                            .on_press(move |_| services.act(Act::BtPair(device.clone()))),
                    )
                    .into_element()
            })
            .collect();
        let rename = renaming.read().clone();
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(form::group(None, vec![power.into_element()]))
            .maybe(adapter.powered && !mine.is_empty(), |el| {
                el.child(form::group(Some("My devices".into()), mine))
            })
            .maybe(adapter.powered, |el| {
                el.child(ui::heading(if adapter.discovering {
                    "Nearby devices · Looking…"
                } else {
                    "Nearby devices"
                }))
                .child(if nearby.is_empty() {
                    form::group(
                        None,
                        vec![
                            form::Row::new("No devices found")
                                .enabled(false)
                                .into_element(),
                        ],
                    )
                    .into_element()
                } else {
                    form::group(None, nearby).into_element()
                })
            })
            .map(rename, |el, (device, name)| {
                el.child(ui::Ask {
                    title: "Rename".into(),
                    placeholder: name.clone(),
                    initial: name,
                    on_done: EventHandler::new(move |text: String| {
                        services.act(Act::BtRename {
                            device: device.clone(),
                            name: text.trim().to_owned(),
                        });
                        renaming.set(None);
                    }),
                    on_cancel: EventHandler::new(move |_| renaming.set(None)),
                })
            })
            .into_element()
    }
}

#[derive(PartialEq)]
pub struct Network {
    pub at: Location,
}

impl Component for Network {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Network]);
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let state = services.network.read().state.clone();
        if self.at.sub.as_deref() == Some("Ethernet") {
            return Ethernet {
                at: self.at.clone(),
            }
            .into_element();
        }
        let mut rows = Vec::new();
        if let Some(wired) = &state.wired {
            let at = self.at.clone();
            rows.push(
                r.row("ethernet", "Ethernet", None)
                    .leading(dot(wired.link))
                    .opens(
                        Some(if wired.link == Link::Unavailable {
                            "Cable unplugged".into()
                        } else {
                            link_word(wired.link).into()
                        }),
                        move |_| super::open(nav, &at, "Ethernet"),
                    )
                    .into_element(),
            );
        }
        if let Some(wifi) = &state.wifi {
            let word = if !state.wifi_enabled {
                "Off"
            } else {
                link_word(wifi.link)
            };
            rows.push(
                r.row("wi-fi", "Wi-Fi", None)
                    .leading(dot(wifi.link))
                    .opens(Some(word.into()), move |_| nav.go(Location::pane("wi-fi")))
                    .into_element(),
            );
        }
        if !state.vpns.is_empty() {
            let link = if state.vpns.iter().any(|v| v.link == Link::Connected) {
                Link::Connected
            } else {
                Link::Disconnected
            };
            rows.push(
                r.row("vpn", "VPN", None)
                    .leading(dot(link))
                    .opens(Some(link_word(link).into()), move |_| {
                        nav.go(Location::pane("vpn"))
                    })
                    .into_element(),
            );
        }
        if rows.is_empty() {
            return ui::missing("No network devices.");
        }
        form::group(None, rows).into_element()
    }
}

#[derive(PartialEq)]
struct Ethernet {
    at: Location,
}

impl Component for Ethernet {
    fn render(&self) -> impl IntoElement {
        let services = crate::services::Services::get();
        let r = Rows::new(&self.at);
        let wired = services.network.read().state.wired.clone();
        let details = use_details(wired.as_ref().map(|w| w.path.clone()));
        let mut rows = vec![
            r.info(
                "status",
                "Status",
                wired.as_ref().map_or("Unavailable", |w| link_word(w.link)),
            )
            .leading(dot(wired.as_ref().map_or(Link::Unavailable, |w| w.link)))
            .into_element(),
        ];
        if let Some(details) = details.read().clone() {
            rows.push(
                r.info(
                    "speed",
                    "Speed",
                    details.speed.map_or("—".to_owned(), |s| {
                        if s >= 1000 {
                            format!("{} Gb/s", s / 1000)
                        } else {
                            format!("{s} Mb/s")
                        }
                    }),
                )
                .into_element(),
            );
            rows.push(
                r.info("configure", "Configure IPv4", "Using DHCP")
                    .into_element(),
            );
            rows.extend(detail_rows(&r, &details, "Hardware address"));
        }
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(ui::sub_header(
                "globe",
                "Ethernet",
                "The wired connection's addresses and speed.",
            ))
            .child(form::group(None, rows))
            .into_element()
    }
}

#[derive(PartialEq)]
pub struct Vpn {
    pub at: Location,
}

impl Component for Vpn {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Network]);
        let r = Rows::new(&self.at);
        let theme = isle_ui::theme();
        let t = &theme.tokens;
        let vpns = services.network.read().state.vpns.clone();
        if vpns.is_empty() {
            return ui::missing("No VPNs");
        }
        let rows: Vec<Element> = vpns
            .iter()
            .map(|v| {
                let on = v.link == Link::Connected || v.link == Link::Connecting;
                let (saved, active, path) = (v.path.clone(), v.active.clone(), v.path.clone());
                r.row(&v.path, v.name.clone(), None)
                    .description(link_word(v.link))
                    .control(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(t.space_s)
                            .child(Switch::new(on, v.name.clone(), move |on: bool| {
                                if on {
                                    services.act(Act::Activate(saved.clone()))
                                } else if let Some(active) = &active {
                                    services.act(Act::Deactivate(active.clone()))
                                }
                            }))
                            .child(MoreMenu {
                                alt: format!("{} options", v.name),
                                items: vec![("Remove…".into(), false)],
                                on_choose: EventHandler::new(move |_| {
                                    services.act(Act::Forget(path.clone()))
                                }),
                            }),
                    )
                    .into_element()
            })
            .collect();
        form::group(None, rows).into_element()
    }
}
