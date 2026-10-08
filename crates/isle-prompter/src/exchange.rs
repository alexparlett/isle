//! Gcr's `sx-aes-1` secret exchange: Diffie–Hellman over the 1024-bit IKE group, a key by
//! HKDF-SHA256, the secret in AES-128-CBC; written as a key file of base64 values. oo7's
//! `server/src/gnome/secret_exchange.rs` is the reference.

use std::sync::LazyLock;

use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
use base64::prelude::*;
use num_bigint::BigUint;
use zeroize::Zeroizing;

const PROTOCOL: &str = "[sx-aes-1]";

/// RFC 2409's second Oakley group.
static PRIME: LazyLock<BigUint> = LazyLock::new(|| {
    BigUint::from_bytes_be(&[
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xC9, 0x0F, 0xDA, 0xA2, 0x21, 0x68, 0xC2,
        0x34, 0xC4, 0xC6, 0x62, 0x8B, 0x80, 0xDC, 0x1C, 0xD1, 0x29, 0x02, 0x4E, 0x08, 0x8A, 0x67,
        0xCC, 0x74, 0x02, 0x0B, 0xBE, 0xA6, 0x3B, 0x13, 0x9B, 0x22, 0x51, 0x4A, 0x08, 0x79, 0x8E,
        0x34, 0x04, 0xDD, 0xEF, 0x95, 0x19, 0xB3, 0xCD, 0x3A, 0x43, 0x1B, 0x30, 0x2B, 0x0A, 0x6D,
        0xF2, 0x5F, 0x14, 0x37, 0x4F, 0xE1, 0x35, 0x6D, 0x6D, 0x51, 0xC2, 0x45, 0xE4, 0x85, 0xB5,
        0x76, 0x62, 0x5E, 0x7E, 0xC6, 0xF4, 0x4C, 0x42, 0xE9, 0xA6, 0x37, 0xED, 0x6B, 0x0B, 0xFF,
        0x5C, 0xB6, 0xF4, 0x06, 0xB7, 0xED, 0xEE, 0x38, 0x6B, 0xFB, 0x5A, 0x89, 0x9F, 0xA5, 0xAE,
        0x9F, 0x24, 0x11, 0x7C, 0x4B, 0x1F, 0xE6, 0x49, 0x28, 0x66, 0x51, 0xEC, 0xE6, 0x53, 0x81,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    ])
});

/// The prime's length in bytes, to which the shared secret is padded.
const PRIME_BYTES: usize = 128;

/// One side of an exchange: a private key and its public half.
pub struct Exchange {
    private: Zeroizing<Vec<u8>>,
    public: Vec<u8>,
}

impl Exchange {
    pub fn new() -> Result<Self, getrandom::Error> {
        let mut private = Zeroizing::new(vec![0u8; 16]);
        getrandom::fill(&mut private)?;
        let public = BigUint::from(2u32)
            .modpow(&BigUint::from_bytes_be(&private), &PRIME)
            .to_bytes_be();
        Ok(Self { private, public })
    }

    /// The opening: this side's public key.
    pub fn begin(&self) -> String {
        encode(&[("public", &self.public)])
    }

    /// The AES key both sides now hold, from the other side's opening.
    pub fn key(&self, theirs: &str) -> Option<Zeroizing<Vec<u8>>> {
        let fields = decode(theirs)?;
        let public = field(&fields, "public")?;
        let shared = BigUint::from_bytes_be(public)
            .modpow(&BigUint::from_bytes_be(&self.private), &PRIME)
            .to_bytes_be();
        let mut padded = Zeroizing::new(vec![0u8; PRIME_BYTES.saturating_sub(shared.len())]);
        padded.extend_from_slice(&shared);
        let mut key = Zeroizing::new(vec![0u8; 16]);
        hkdf::Hkdf::<sha2::Sha256>::new(None, &padded)
            .expand(&[], &mut key)
            .ok()?;
        Some(key)
    }

    /// The closing: this side's public key and `secret` encrypted with `key`.
    pub fn send(&self, key: &[u8], secret: &[u8]) -> Option<String> {
        let mut iv = [0u8; 16];
        getrandom::fill(&mut iv).ok()?;
        let sealed = cbc::Encryptor::<aes::Aes128>::new_from_slices(key, &iv)
            .ok()?
            .encrypt_padded_vec_mut::<Pkcs7>(secret);
        Some(encode(&[
            ("public", &self.public),
            ("secret", &sealed),
            ("iv", &iv),
        ]))
    }
}

/// The secret in a closing, opened with `key`.
pub fn receive(closing: &str, key: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    let fields = decode(closing)?;
    let sealed = field(&fields, "secret")?;
    let iv = field(&fields, "iv")?;
    cbc::Decryptor::<aes::Aes128>::new_from_slices(key, iv)
        .ok()?
        .decrypt_padded_vec_mut::<Pkcs7>(sealed)
        .ok()
        .map(Zeroizing::new)
}

fn encode(fields: &[(&str, &[u8])]) -> String {
    let mut out = String::from(PROTOCOL);
    out.push('\n');
    for (key, value) in fields {
        out.push_str(&format!("{key}={}\n", BASE64_STANDARD.encode(value)));
    }
    out
}

fn decode(text: &str) -> Option<Vec<(String, Vec<u8>)>> {
    let (_, body) = text.split_once(PROTOCOL)?;
    Some(
        body.lines()
            .filter_map(|line| line.split_once('='))
            .filter_map(|(k, v)| {
                Some((k.trim().to_owned(), BASE64_STANDARD.decode(v.trim()).ok()?))
            })
            .collect(),
    )
}

fn field<'a>(fields: &'a [(String, Vec<u8>)], key: &str) -> Option<&'a [u8]> {
    fields
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oo7s_vector_opens() {
        let closing = "[sx-aes-1]
public=/V6FpknNXlOGJwPqXtN0RaED2bS5JyYbftv7WbD0gWiVTMoNgxkAuOX2g+zUO/4TdfBJ6viPRcNdYV+KcxskGvhYouFXs+IgKqNO0MF0CNnWra1I6G56SM4Bgstkx9M5J+1f83l/BTAxlLsAppeLkqEEVSQoy9jXhPOrl5XlIzF2DvriYh+FInB7SFz4VzE3KVq40p7tA9+iAVQg1o9qkQHLazFb1DfbWRgvhDVhwNkk1fIlepIeM426gdmHIAxP
secret=DBeLBvEgGuGygDm+XnkxyQ==
iv=8e3N+gx553PgQlfTKRK3JA==";
        let key = [
            204, 53, 139, 40, 55, 167, 183, 240, 191, 252, 186, 174, 28, 36, 229, 26,
        ];
        assert_eq!(receive(closing, &key).unwrap().as_slice(), b"password");
    }

    #[test]
    fn two_sides_agree_on_a_key_and_a_secret_crosses() {
        let prompter = Exchange::new().unwrap();
        let keyring = Exchange::new().unwrap();
        let a = prompter.key(&keyring.begin()).unwrap();
        let b = keyring.key(&prompter.begin()).unwrap();
        assert_eq!(*a, *b);
        for secret in [
            "",
            "a",
            "exactly-16-bytes",
            "a much longer password than one block",
        ] {
            let closing = prompter.send(&a, secret.as_bytes()).unwrap();
            assert_eq!(receive(&closing, &b).unwrap().as_slice(), secret.as_bytes());
        }
    }
}
