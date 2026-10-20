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
