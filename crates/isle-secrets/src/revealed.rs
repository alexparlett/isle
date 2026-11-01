//! A fetched secret: zeroed when dropped, never printed.

use zeroize::Zeroizing;

pub struct Revealed {
    bytes: Zeroizing<Vec<u8>>,
    text: bool,
}

impl std::fmt::Debug for Revealed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Revealed(..)")
    }
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64(bytes: &[u8]) -> Zeroizing<String> {
    let mut out = Zeroizing::new(String::with_capacity(bytes.len().div_ceil(3) * 4));
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(BASE64[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

impl Revealed {
    /// `content_type` as the service gave it: a `text/` type that holds UTF-8 reads as text.
    pub fn new(bytes: Zeroizing<Vec<u8>>, content_type: &str) -> Self {
        let text = content_type.starts_with("text/") && std::str::from_utf8(&bytes).is_ok();
        Self { bytes, text }
    }

    pub fn from_text(text: Zeroizing<String>) -> Self {
        Self {
            bytes: Zeroizing::new(text.as_bytes().to_vec()),
            text: true,
        }
    }

    /// Whether the secret is text; one that is not shows in base64.
    pub fn is_text(&self) -> bool {
        self.text
    }

    /// The secret as text to show or copy.
    pub fn text(&self) -> Zeroizing<String> {
        if self.text {
            Zeroizing::new(String::from_utf8_lossy(&self.bytes).into_owned())
        } else {
            base64(&self.bytes)
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_shows_in_base64() {
        let r = Revealed::new(
            Zeroizing::new(vec![0xff, 0, 1, 2]),
            "application/octet-stream",
        );
        assert!(!r.is_text());
        assert_eq!(&*r.text(), "/wABAg==");
        let r = Revealed::new(
            Zeroizing::new(b"hunter2".to_vec()),
            "text/plain; charset=utf8",
        );
        assert_eq!(&*r.text(), "hunter2");
        assert_eq!(format!("{r:?}"), "Revealed(..)");
    }
}
