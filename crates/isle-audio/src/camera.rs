//! Cameras in use, from PipeWire's graph: a camera node linked to another node is being
//! read, by the app that node belongs to. Apps that open `/dev/video*` themselves are not
//! seen (`services-2026.md`, Camera in use).

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    time::Duration,
};

use isle_bus::{Error, Publisher};
use pipewire as pw;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct CameraUse {
    pub app: String,
    pub pid: Option<i32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Cameras {
    pub in_use: Vec<CameraUse>,
}

#[derive(Default)]
struct Graph {
    cameras: BTreeSet<u32>,
    /// Other nodes by id: the app each belongs to.
    nodes: BTreeMap<u32, CameraUse>,
    /// Links by id: from node, to node.
    links: BTreeMap<u32, (u32, u32)>,
}

impl Graph {
    fn in_use(&self) -> Cameras {
        let in_use: BTreeSet<CameraUse> = self
            .links
            .values()
            .filter(|(from, _)| self.cameras.contains(from))
            .filter_map(|(_, to)| self.nodes.get(to).cloned())
            .collect();
        Cameras {
            in_use: in_use.into_iter().collect(),
        }
    }
}

fn is_camera(props: &pw::spa::utils::dict::DictRef) -> bool {
    props.get("media.role") == Some("Camera")
        || (props.get("media.class") == Some("Video/Source")
            && matches!(props.get("device.api"), Some("v4l2" | "libcamera")))
}

/// Follows PipeWire's graph on a thread of its own, connecting again when PipeWire returns.
pub async fn watch(publisher: Publisher<Cameras>) -> Result<(), Error> {
    loop {
        let p = publisher.clone();
        let _ = tokio::task::spawn_blocking(move || follow(p)).await;
        publisher.unavailable();
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

fn follow(publisher: Publisher<Cameras>) -> Result<(), Error> {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(pw::init);
    let mainloop = pw::main_loop::MainLoopRc::new(None)?;
    // Without the realtime module and D-Bus support, nothing in the client loads libdbus.
    let context = pw::context::ContextRc::new(
        &mainloop,
        Some(pw::properties::properties! {
            "module.rt" => "false",
            "support.dbus" => "false",
        }),
    )?;
    let core = context.connect_rc(None)?;
    let registry = core.get_registry_rc()?;
    let graph = Rc::new(RefCell::new(Graph::default()));

    let quit = mainloop.clone();
    let _core = core
        .add_listener_local()
        .error(move |id, _, _, _| {
            if id == pw::core::PW_ID_CORE {
                quit.quit();
            }
        })
        .register();

    let publish = {
        let graph = graph.clone();
        let publisher = publisher.clone();
        move || publisher.publish(graph.borrow().in_use())
    };
    let added = {
        let graph = graph.clone();
        let publish = publish.clone();
        move |obj: &pw::registry::GlobalObject<&pw::spa::utils::dict::DictRef>| {
            let Some(props) = obj.props else {
                return;
            };
            let mut g = graph.borrow_mut();
            match obj.type_ {
                pw::types::ObjectType::Node if is_camera(props) => {
                    g.cameras.insert(obj.id);
                }
                pw::types::ObjectType::Node => {
                    let app = props
                        .get("application.name")
                        .or(props.get("node.description"))
                        .or(props.get("node.name"))
                        .unwrap_or("An app")
                        .to_owned();
                    let pid = props
                        .get("application.process.id")
                        .and_then(|p| p.parse().ok());
                    g.nodes.insert(obj.id, CameraUse { app, pid });
                }
                pw::types::ObjectType::Link => {
                    let node = |key| props.get(key).and_then(|n| n.parse::<u32>().ok());
                    if let (Some(from), Some(to)) =
                        (node("link.output.node"), node("link.input.node"))
                    {
                        g.links.insert(obj.id, (from, to));
                    }
                }
                _ => return,
            }
            drop(g);
            publish();
        }
    };
    let removed = {
        let graph = graph.clone();
        move |id| {
            let mut g = graph.borrow_mut();
            let had = g.cameras.remove(&id)
                | g.nodes.remove(&id).is_some()
                | g.links.remove(&id).is_some();
            drop(g);
            if had {
                publish();
            }
        }
    };
    let _registry = registry
        .add_listener_local()
        .global(added)
        .global_remove(removed)
        .register();
    publisher.publish(Cameras::default());
    mainloop.run();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_camera_is_in_use_by_the_app_its_link_leads_to() {
        let mut g = Graph::default();
        g.cameras.insert(40);
        g.nodes.insert(
            70,
            CameraUse {
                app: "Firefox".into(),
                pid: Some(4242),
            },
        );
        g.nodes.insert(
            71,
            CameraUse {
                app: "Speakers".into(),
                pid: None,
            },
        );
        assert!(g.in_use().in_use.is_empty());
        g.links.insert(90, (40, 70));
        g.links.insert(91, (12, 71));
        assert_eq!(g.in_use().in_use.len(), 1);
        assert_eq!(g.in_use().in_use[0].app, "Firefox");
    }
}
