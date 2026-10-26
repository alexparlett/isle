//! Split View: two windows sharing a full-screen desktop (`compositor.md` Split View).

use smithay::output::Output;
use tracing::debug;

use crate::state::State;

impl State {
    /// Pairs `window` with the window of `output`'s full-screen desktop `index`, on the given
    /// side.
    pub fn split_with(&mut self, window: u64, output: &Output, index: usize, left: bool) -> bool {
        debug!(
            window,
            output = output.name(),
            index,
            left,
            "split view asked"
        );
        false
    }
}

impl State {
    /// The zoom menu, the Window menu, a chord or `Desktops1.Split`: `id` full screen on one
    /// side, the chooser in the other half.
    pub fn begin_split(&mut self, id: u64, left: bool) -> bool {
        debug!(id, left, "split view asked");
        false
    }
}
