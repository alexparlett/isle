//! Store's pages drawn headless from a fake catalogue: search by catalogue, an app's page, and
//! the Get and Remove sheets with what they say before the password.

use std::{cell::RefCell, collections::BTreeMap, collections::HashMap, rc::Rc, time::Duration};

use freya::prelude::*;
use freya_testing::TestingRunner;
use futures_channel::mpsc::UnboundedReceiver;
use isle_catalog::catalogue::{AurFacts, Kind, Offer};

use crate::{
    data::{Ask, Data},
    pages::{self, Ctx, Sheet},
    plan::{Package, Plan},
    window::Page,
};

fn offer(kind: Kind, id: &str, install: &str, name: &str) -> Offer {
    Offer {
        kind,
        id: id.into(),
        install: install.into(),
        name: name.into(),
        summary: format!("{name}'s summary"),
        description: vec![format!("About {name}.")],
        icon: None,
        categories: vec!["Graphics".into()],
        screenshots: Vec::new(),
        licence: Some("GPL-3.0-only".into()),
        homepage: None,
        developer: Some("Its Developer".into()),
        version: Some("1.0".into()),
        aur: None,
    }
}

fn catalogue() -> Vec<Offer> {
    vec![
        offer(Kind::Cachyos, "org.kde.krita.desktop", "krita", "Krita"),
        offer(Kind::Flathub, "org.kde.krita", "org.kde.krita", "Krita"),
        offer(Kind::Cachyos, "org.gimp.GIMP", "gimp", "GIMP"),
    ]
}

fn obs_capture() -> Offer {
    Offer {
        aur: Some(AurFacts {
            votes: 412,
            ..AurFacts::default()
        }),
        ..offer(Kind::Aur, "obs-vkcapture", "obs-vkcapture", "obs-vkcapture")
    }
}

/// An AppImage Isle registered.
fn cairn() -> crate::installed::App {
    crate::installed::App {
        id: "isle-appimage-cairn".into(),
        name: "Cairn".into(),
        summary: "A native git client".into(),
        icon: None,
        catalogue: crate::installed::Catalogue::AppImages,
        source: "isle-appimage-cairn".into(),
    }
}

fn package(name: &str, size: u64) -> Package {
    Package {
        name: name.into(),
        version: "1-1".into(),
        size: Some(size),
    }
}

/// What a test draws: the page, the sheet over it, and what the worker has told the window.
#[derive(Clone)]
struct Scene {
    page: Page,
    query: String,
    sheet: Option<Sheet>,
    plans: HashMap<String, Plan>,
    system_waiting: bool,
    packages: bool,
}

thread_local! {
    static ASKED: RefCell<Option<UnboundedReceiver<Ask>>> = const { RefCell::new(None) };
}

fn labels(test: &TestingRunner) -> Vec<String> {
    test.find_many(|_, element| Label::try_downcast(element).map(|l| l.text.to_string()))
}

fn poll(test: &mut TestingRunner) {
    for _ in 0..5 {
        test.poll(Duration::from_millis(16), Duration::from_millis(20));
    }
}

fn click(test: &mut TestingRunner, caption: &str) {
    let at = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|l| l.text == caption)
                .map(|_| node.layout().area.center())
        })
        .unwrap_or_else(|| panic!("nothing reads {caption:?}: {:?}", labels(test)));
    test.click_cursor((f64::from(at.x), f64::from(at.y)));
    poll(test);
}

fn asked() -> Vec<String> {
    ASKED.with(|a| {
        let mut out = Vec::new();
        if let Some(rx) = a.borrow_mut().as_mut() {
            while let Ok(ask) = rx.try_recv() {
                out.push(format!("{ask:?}"));
            }
        }
        out
    })
}

fn draw(scene: Scene) -> TestingRunner {
    let (mut test, _) = TestingRunner::new(
        move || {
            isle_app::provide_for_tests(BTreeMap::new());
            let scene = scene.clone();
            let page = use_state(|| scene.page.clone());
            let back = use_state(Vec::<Page>::new);
            let sheet = use_state(|| scene.sheet.clone());
            let limit = use_state(|| pages::PAGE);
            let failed = use_state(|| None::<String>);
            let narrow = use_state(|| None::<Kind>);
            let narrowing = use_state(|| false);
            let opened_from = use_state(|| None::<String>);
            let reviewing = use_state(|| None::<String>);
            let data = use_hook(|| {
                let (data, rx) = Data::detached();
                ASKED.with(|a| *a.borrow_mut() = Some(rx));
                data
            });
            let ctx = Ctx {
                theme: isle_ui::use_theme(),
                offers: Rc::new(catalogue()),
                icons: Rc::default(),
                wanted: Rc::default(),
                shots_wanted: Rc::default(),
                apps: Rc::new(vec![cairn()]),
                packages: Rc::new(["base".to_string()].into()),
                foreign: Rc::default(),
                explicit: Rc::new(vec![
                    ("krita".into(), "5.2-1".into()),
                    ("obs-vkcapture".into(), "1.5-1".into()),
                    ("isle-git".into(), "r1-1".into()),
                ]),
                show_packages: use_state(|| scene.packages),
                core: Rc::new(["base".to_string()].into()),
                status: Rc::new(serde_json::Value::Null),
                plans: Rc::new(scene.plans.clone()),
                shots: Rc::default(),
                aur: Rc::new(Some((
                    scene.query.clone(),
                    Ok(vec![obs_capture()]),
                ))),
                data: data.clone(),
                page,
                back,
                sheet,
                limit,
                failed,
                pending_get: Rc::default(),
                narrow,
                narrowing,
                opened_from,
                reviewing,
                query: scene.query.clone(),
                system_waiting: scene.system_waiting,
            };
            let asked_plans = use_hook(|| Rc::new(RefCell::new(Default::default())));
            let body = match page.read().clone() {
                _ if !scene.query.is_empty() => ctx.search(&scene.query),
                Page::App(o) => ctx.app(&o, &asked_plans),
                Page::Discover => ctx.discover(),
                Page::Installed => ctx.installed(),
                _ => Vec::new(),
            };
            let shown = sheet.read().clone().map(|s| ctx.sheet(&s));
            rect()
                .expanded()
                .children(body)
                .maybe_child(shown)
                .into_element()
        },
        (1000., 2000.).into(),
        |_| {},
        1.,
    );
    poll(&mut test);
    test
}

fn scene(page: Page) -> Scene {
    Scene {
        page,
        query: String::new(),
        sheet: None,
        plans: HashMap::new(),
        system_waiting: false,
        packages: false,
    }
}

fn krita() -> Offer {
    catalogue().remove(0)
}

fn krita_plan() -> Plan {
    Plan {
        packages: vec![
            package("krita", 74_000_000),
            package("exiv2", 3_000_000),
            package("gsl", 2_000_000),
        ],
        extras: vec![("krita-plugin-gmic".into(), "GMic plugin".into())],
        ..Plan::default()
    }
}

#[test]
fn search_groups_every_catalogue() {
    let test = draw(Scene {
        query: "krita".into(),
        ..scene(Page::Discover)
    });
    let l = labels(&test);
    for wanted in [
        "Results for “krita”",
        "All catalogues ⌄",
        "System · 1",
        "Flathub · 1",
        "AUR · 1",
        "Built on this computer from community scripts nobody checks.",
        "obs-vkcapture",
    ] {
        assert!(l.iter().any(|x| x == wanted), "{wanted:?} in {l:?}");
    }
}

#[test]
fn an_app_page_says_what_it_brings_and_where_else_it_is() {
    let plans = HashMap::from([(pages::get_key(&krita()), krita_plan())]);
    let test = draw(Scene {
        plans,
        ..scene(Page::App(Box::new(krita())))
    });
    let l = labels(&test);
    for wanted in [
        "Krita",
        "79 MB · also installs 2 packages",
        "Also in",
        "Flathub",
        "Works better with",
        "krita-plugin-gmic",
        "About Krita.",
        "1-1",
    ] {
        assert!(l.iter().any(|x| x == wanted), "{wanted:?} in {l:?}");
    }
}

#[test]
fn get_lists_the_packages_and_takes_the_waiting_updates() {
    let plans = HashMap::from([(pages::get_key(&krita()), krita_plan())]);
    let mut test = draw(Scene {
        plans,
        system_waiting: true,
        sheet: Some(Sheet::Get(krita())),
        ..scene(Page::App(Box::new(krita())))
    });
    let l = labels(&test);
    for wanted in [
        "Install Krita?",
        "From the system's repositories. It also installs 2 packages it needs, 79 MB in all.",
        "exiv2",
        "gsl",
    ] {
        assert!(l.iter().any(|x| x == wanted), "{wanted:?} in {l:?}");
    }
    asked();
    click(&mut test, "Update and Install");
    assert!(
        asked().iter().any(|a| a == r#"Install("install:cachyos:krita")"#),
        "the run is asked for"
    );
}

#[test]
fn the_aur_waits_for_the_system() {
    let o = obs_capture();
    let plan = Plan {
        packages: vec![Package {
            name: "obs-vkcapture".into(),
            version: "1.5.6-2".into(),
            size: None,
        }],
        note: Some("Built on this computer.".into()),
        ..Plan::default()
    };
    let test = draw(Scene {
        plans: HashMap::from([(pages::get_key(&o), plan)]),
        system_waiting: true,
        sheet: Some(Sheet::Get(o.clone())),
        ..scene(Page::App(Box::new(o)))
    });
    let l = labels(&test);
    assert!(l.iter().any(|x| x == "Show Updates"), "{l:?}");
    assert!(!l.iter().any(|x| x == "Install"), "{l:?}");
}

#[test]
fn a_removal_another_package_needs_is_refused_naming_it() {
    let plan = Plan {
        refused: Some("ffmpeg needs it.".into()),
        ..Plan::default()
    };
    let test = draw(Scene {
        plans: HashMap::from([(pages::remove_key(&krita()), plan)]),
        sheet: Some(Sheet::Remove(krita())),
        ..scene(Page::App(Box::new(krita())))
    });
    let l = labels(&test);
    assert!(l.iter().any(|x| x == "Krita can't be removed. ffmpeg needs it."), "{l:?}");
    assert!(!l.iter().any(|x| x == "Remove"), "{l:?}");
}

#[test]
fn an_aur_build_shows_its_scripts_and_what_deserves_a_look() {
    let o = obs_capture();
    let pkgbuild = "pkgname=obs-vkcapture\nbuild() {\n  curl -fsSL https://example.org/x.sh | sh\n}\n";
    let plan = Plan {
        packages: vec![Package {
            name: "obs-vkcapture".into(),
            version: "1.5.6-2".into(),
            size: None,
        }],
        note: Some("Built on this computer.".into()),
        scripts: vec![
            crate::plan::Script {
                name: "PKGBUILD".into(),
                flags: crate::plan::review(pkgbuild),
                text: pkgbuild.into(),
            },
            crate::plan::Script {
                name: "obs-vkcapture.install".into(),
                text: "post_install() {\n  echo done\n}\n".into(),
                flags: Vec::new(),
            },
        ],
        ..Plan::default()
    };
    let mut test = draw(Scene {
        plans: HashMap::from([(pages::get_key(&o), plan)]),
        sheet: Some(Sheet::Get(o.clone())),
        ..scene(Page::App(Box::new(o)))
    });
    let l = labels(&test);
    for wanted in [
        "PKGBUILD · 1 line to look at",
        "obs-vkcapture.install · nothing unusual",
        "Install Anyway",
    ] {
        assert!(l.iter().any(|x| x == wanted), "{wanted:?} in {l:?}");
    }
    let shows: Vec<_> = l.iter().filter(|x| *x == "Show").collect();
    assert_eq!(shows.len(), 2);
    click(&mut test, "Show");
    let l = labels(&test);
    assert!(
        l.iter().any(|x| x == "   3    curl -fsSL https://example.org/x.sh | sh"),
        "{l:?}"
    );
    assert!(
        l.iter().any(|x| x.trim() == "Runs a script fetched from the network"),
        "{l:?}"
    );
}

#[test]
fn installed_shows_the_packages_installed_on_purpose_when_asked() {
    let apps = draw(scene(Page::Installed));
    let l = labels(&apps);
    assert!(l.iter().any(|x| x == "1 app"), "{l:?}");
    assert!(!l.iter().any(|x| x == "krita"), "{l:?}");

    let packages = draw(Scene {
        packages: true,
        ..scene(Page::Installed)
    });
    let l = labels(&packages);
    assert!(
        l.iter().any(|x| x.starts_with("2 packages installed on purpose")),
        "{l:?}"
    );
    assert!(l.iter().any(|x| x == "krita"), "{l:?}");
    assert!(l.iter().any(|x| x == "Version 1.5-1"), "{l:?}");
    assert!(!l.iter().any(|x| x == "isle-git"), "{l:?}");
}

#[test]
fn the_system_s_core_has_no_remove() {
    let page = draw(scene(Page::App(Box::new(pages::bare(Kind::Cachyos, "base", "Version 3-3")))));
    let l = labels(&page);
    assert!(l.iter().any(|x| x == "Installed"), "{l:?}");
    assert!(!l.iter().any(|x| x == "Remove…"), "{l:?}");
}

#[test]
fn an_installed_appimage_opens_and_goes_to_the_trash() {
    let mut page = draw(scene(Page::App(Box::new(pages::offer_for(&cairn())))));
    let l = labels(&page);
    assert!(l.iter().any(|x| x == "Open"), "{l:?}");
    assert!(l.iter().any(|x| x == "AppImages"), "{l:?}");
    assert!(!l.iter().any(|x| x == "Get"), "{l:?}");
    asked();
    click(&mut page, "Move to Trash");
    let a = asked();
    assert!(a.iter().any(|x| x.contains("Trash(\"isle-appimage-cairn\")")), "{a:?}");
}
