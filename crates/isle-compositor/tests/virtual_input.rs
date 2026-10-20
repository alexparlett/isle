//! Clients' virtual pointers and keyboards drive the seat as devices do. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::Isle;
use wayland_client::{
    Connection, Dispatch, QueueHandle, globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_pointer, wl_registry, wl_seat::WlSeat},
};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
    zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
};

const LEFT: u32 = 0x110;

struct Driver;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Driver {
    fn event(_: &mut Self, _: &wl_registry::WlRegistry, _: wl_registry::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}
wayland_client::delegate_noop!(Driver: ignore WlSeat);
wayland_client::delegate_noop!(Driver: ZwlrVirtualPointerManagerV1);
wayland_client::delegate_noop!(Driver: ZwlrVirtualPointerV1);

#[tokio::test(flavor = "multi_thread")]
async fn a_virtual_pointer_moves_clicks_and_scrolls() {
    let mut isle = Isle::start("virtual-pointer").await;
    let client = isle.client("isle.test.Virtual", "3366ff");
    let window = isle.wait_for_window("isle.test.Virtual").await;
    client.wait_for("keyboard-enter").await;
    client.clear();

    let socket = isle.dir.join("runtime").join(isle.socket());
    let (cx, cy) = (window.x + window.width / 2, window.y + window.height / 2);
    tokio::task::spawn_blocking(move || {
        let stream = std::os::unix::net::UnixStream::connect(socket).unwrap();
        let conn = Connection::from_socket(stream).unwrap();
        let (globals, mut queue) = registry_queue_init::<Driver>(&conn).unwrap();
        let qh = queue.handle();
        let seat: WlSeat = globals.bind(&qh, 1..=7, ()).unwrap();
        let manager: ZwlrVirtualPointerManagerV1 = globals.bind(&qh, 1..=2, ()).unwrap();
        let pointer = manager.create_virtual_pointer(Some(&seat), &qh, ());
        // The headless output is 1920x1080 at the origin.
        pointer.motion_absolute(0, cx as u32, cy as u32, 1920, 1080);
        pointer.frame();
        pointer.button(0, LEFT, wl_pointer::ButtonState::Pressed);
        pointer.frame();
        pointer.button(0, LEFT, wl_pointer::ButtonState::Released);
        pointer.frame();
        pointer.axis_source(wl_pointer::AxisSource::Wheel);
        pointer.axis_discrete(0, wl_pointer::Axis::VerticalScroll, 15.0, 1);
        pointer.frame();
        queue.roundtrip(&mut Driver).unwrap();
    })
    .await
    .unwrap();

    client.wait_for_part("pointer-enter").await;
    client.wait_for(&format!("button {LEFT} pressed")).await;
    client.wait_for(&format!("button {LEFT} released")).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_virtual_keyboard_types_in_its_keymap_and_hands_the_seat_back() {
    let mut isle = Isle::start("virtual-keyboard").await;
    let client = isle.client("isle.test.Typed", "3366ff");
    isle.wait_for_window("isle.test.Typed").await;
    client.wait_for("keyboard-enter").await;
    client.clear();

    let wtype = isle.spawn_logged("wtype", &["hi"]);
    client.wait_for("sym h pressed").await;
    client.wait_for("sym i released").await;
    common::wait_until(|| async { wtype.exited() }).await;

    client.clear();
    let test = isle.test().await;
    test.key(30, true).await.unwrap();
    test.key(30, false).await.unwrap();
    client.wait_for("sym a pressed").await;
}

/// `wtype` with `args`, to its end.
async fn wtype(isle: &mut Isle, args: &[&str]) {
    let wtype = isle.spawn_logged("wtype", args);
    common::wait_until(|| async { wtype.exited() }).await;
}

async fn spotlight(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.spotlight().await.unwrap()).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_virtual_keyboards_chord_opens_spotlight_and_its_keys_type_there() {
    let mut isle = Isle::start("virtual-spotlight").await;
    let client = isle.client("isle.test.Front", "3366ff");
    client.wait_for("keyboard-enter").await;

    wtype(&mut isle, &["-P", "Super_L", "-k", "space", "-p", "Super_L"]).await;
    common::wait_until(|| async { spotlight(&isle).await["open"] == true }).await;
    wtype(&mut isle, &["fire"]).await;
    common::wait_until(|| async { spotlight(&isle).await["text"] == "fire" }).await;
    assert!(!client.printed("sym f pressed"), "no key reached the window");

    wtype(&mut isle, &["-k", "Escape"]).await;
    common::wait_until(|| async { spotlight(&isle).await["text"] == "" }).await;
    wtype(&mut isle, &["-k", "Escape"]).await;
    common::wait_until(|| async { spotlight(&isle).await["open"] == false }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_virtual_keyboards_own_keymap_is_not_translated() {
    let mut isle = Isle::start("virtual-untranslated").await;
    let client = isle.client("isle.test.Front", "3366ff");
    client.wait_for("keyboard-enter").await;

    // A keyboard's Cmd+C reaches a PC app as Ctrl+C.
    isle.chord(&[125, 46]).await;
    client.wait_for("sym Control_L pressed").await;
    client.clear();

    wtype(&mut isle, &["-P", "Super_L", "-k", "c", "-p", "Super_L"]).await;
    client.wait_for("sym c released").await;
    assert!(client.printed("sym Super_L pressed"), "Cmd reached the window as itself");
    assert!(!client.printed("sym Control_L pressed"), "nothing became Ctrl");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_virtual_keyboard_types_nothing_into_the_lock() {
    let mut isle = Isle::start("virtual-lock").await;
    let client = isle.client("isle.test.Behind", "3366ff");
    client.wait_for("keyboard-enter").await;
    isle_ipc::session::SessionProxy::new(&isle.bus).await.unwrap().lock().await.unwrap();
    common::wait_until(|| async { lock_status(&isle).await["locked"] == true }).await;

    wtype(&mut isle, &["abc"]).await;
    let test = isle.test().await;
    test.key(45, true).await.unwrap();
    test.key(45, false).await.unwrap();
    common::wait_until(|| async { lock_status(&isle).await["dots"] == 1 }).await;
}

async fn lock_status(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.lock_status().await.unwrap()).unwrap()
}
