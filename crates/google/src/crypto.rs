use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::error::GoogleError;

const NONCE_LEN: usize = 12;

/// AES-256-GCM encryption for tokens at rest.
///
/// The 32-byte key is derived from an arbitrary-length secret via SHA-256, so
/// `TOKEN_ENCRYPTION_KEY` can be any sufficiently random string. Each ciphertext
/// is `nonce || ciphertext`, with a fresh random nonce per encryption.
#[derive(Clone)]
pub struct TokenCipher {
    cipher: Aes256Gcm,
}

impl TokenCipher {
    pub fn new(secret: &str) -> Self {
        let hash = Sha256::digest(secret.as_bytes());
        let key = Key::<Aes256Gcm>::from_slice(&hash);

        Self {
            cipher: Aes256Gcm::new(key),
        }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<Vec<u8>, GoogleError> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| GoogleError::Crypto(e.to_string()))?;

        let mut out = nonce_bytes.to_vec();
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<String, GoogleError> {
        if data.len() <= NONCE_LEN {
            return Err(GoogleError::Crypto("ciphertext too short".to_string()));
        }

        let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| GoogleError::Crypto(e.to_string()))?;

        String::from_utf8(plaintext).map_err(|e| GoogleError::Crypto(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let cipher = TokenCipher::new("a-test-secret");
        let encrypted = cipher.encrypt("ya29.super-secret-token").unwrap();
        assert_ne!(encrypted, b"ya29.super-secret-token");
        assert_eq!(cipher.decrypt(&encrypted).unwrap(), "ya29.super-secret-token");
    }

    #[test]
    fn different_nonce_each_time() {
        let cipher = TokenCipher::new("a-test-secret");
        let a = cipher.encrypt("same").unwrap();
        let b = cipher.encrypt("same").unwrap();
        assert_ne!(a, b, "nonce reuse would make ciphertexts identical");
    }

    #[test]
    fn wrong_key_fails() {
        let a = TokenCipher::new("key-a");
        let b = TokenCipher::new("key-b");
        let encrypted = a.encrypt("secret").unwrap();
        assert!(b.decrypt(&encrypted).is_err());
    }
}
