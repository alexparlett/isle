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
