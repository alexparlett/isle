//! How long the shell keeps a decoded image, and how large it decodes a preview.

use freya::prelude::{AssetAge, DecodeMode};

/// Long enough to come back to a result or a cover without decoding it again.
pub const AGE: AssetAge = AssetAge::Duration(std::time::Duration::from_secs(300));

/// Spotlight's preview fills its pane, which no screen makes larger than this in pixels.
pub const PREVIEW: DecodeMode = DecodeMode::Custom(freya::torin::prelude::Size2D::new(1600.0, 1600.0));
