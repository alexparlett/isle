//! Fake system daemons for tests, served on a private bus. Each answers the calls Isle's
//! proxies make, can change its state as the daemon would, and can leave the bus and return.

pub mod bluez;
pub mod logind;
pub mod mpris;
pub mod network;
pub mod upower;

use zbus::Connection;

/// Connects to the bus at `address`.
pub async fn connect(address: &str) -> zbus::Result<Connection> {
    zbus::connection::Builder::address(address)?.build().await
}
