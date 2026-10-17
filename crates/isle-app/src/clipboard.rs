//! The clipboard through the window's own `wl_data_device`, which every client may use for
//! the selection while it has focus; arboard, Freya's default, wants data-control, which Isle
//! keeps from ordinary clients.

use freya::clipboard::{ClipboardError, ClipboardImage, ClipboardProvider};

pub struct WaylandClipboard(smithay_clipboard::Clipboard);

impl WaylandClipboard {
    /// # Safety
    /// `display` is a live `wl_display` that outlives the clipboard.
    pub unsafe fn new(display: *mut std::ffi::c_void) -> Self {
        Self(unsafe { smithay_clipboard::Clipboard::new(display) })
    }
}

impl ClipboardProvider for WaylandClipboard {
    fn get_text(&mut self) -> Result<String, ClipboardError> {
        self.0.load().map_err(|_| ClipboardError::FailedToRead)
    }

    fn set_text(&mut self, contents: String) -> Result<(), ClipboardError> {
        self.0.store(contents);
        Ok(())
    }

    fn get_image(&mut self) -> Result<ClipboardImage, ClipboardError> {
        Err(ClipboardError::FailedToRead)
    }

    fn set_image(&mut self, _image: ClipboardImage) -> Result<(), ClipboardError> {
        Err(ClipboardError::FailedToSet)
    }
}
