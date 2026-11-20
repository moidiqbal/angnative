/**
 * crypto.rs — Crypto + secure storage (behind `crypto` feature).
 *
 * Provides secure random generation and zeroizing secret wrappers. The
 * `encrypt`/`decrypt` pair is XOR-based obfuscation for API-shape demonstration
 * only — it is NOT authenticated encryption and must not be used for secrets.
 * Real production use requires AEAD (e.g. AES-GCM or ChaCha20-Poly1305).
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

#[cfg(feature = "crypto")]
use rand::RngCore;
#[cfg(feature = "crypto")]
use zeroize::Zeroize;

/** Error returned by crypto operations. */
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Random generation failed: {0}")]
    Random(String),
    #[error("Encryption failed: {0}")]
    Encrypt(String),
    #[error("Decryption failed: {0}")]
    Decrypt(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),
}

/** A zeroizing container for sensitive key material. On drop, memory is
 * zeroed to prevent key leakage in memory dumps. */
#[derive(Debug)]
pub struct SecureKey {
    inner: zeroize::Zeroizing<Vec<u8>>,
}

impl SecureKey {
    /// Generate a new random secure key of the given length (bytes).
    #[cfg(feature = "crypto")]
    pub fn generate(len: usize) -> Result<Self, CryptoError> {
        let mut buf = vec![0u8; len];
        rand::thread_rng().fill_bytes(&mut buf);
        Ok(Self {
            inner: zeroize::Zeroizing::new(buf),
        })
    }

    /// Create a SecureKey from raw bytes (consumes them).
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            inner: zeroize::Zeroizing::new(bytes),
        }
    }

    /// Access the key bytes immutably.
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }

    /// Length of the key in bytes.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Clone for SecureKey {
    fn clone(&self) -> Self {
        Self::from_bytes(self.inner.to_vec())
    }
}

impl Zeroize for SecureKey {
    fn zeroize(&mut self) {
        self.inner.zeroize();
    }
}

/** A simple authenticated encryption container (placeholder structure).
 *
 * Production implementations should use AES-GCM or ChaCha20-Poly1305 via
 * the `ring` or `rust-crypto` crates. This structure defines the API shape
 * so platform shells can integrate real crypto without changing call sites.
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedBox {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub key_id: String,
}

/** A crypto service that manages secure keys and provides encryption/decryption. */
#[derive(Debug, Default)]
pub struct CryptoService {
    keys: RwLock<HashMap<String, SecureKey>>,
}

impl CryptoService {
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate and store a new secure key under the given ID.
    #[cfg(feature = "crypto")]
    pub fn generate_key(&self, key_id: &str, len: usize) -> Result<(), CryptoError> {
        let key = SecureKey::generate(len)?;
        self.keys.write().unwrap().insert(key_id.to_string(), key);
        Ok(())
    }

    /// Import an existing key under the given ID.
    pub fn import_key(&self, key_id: &str, key: SecureKey) {
        self.keys.write().unwrap().insert(key_id.to_string(), key);
    }

    /// Get a reference to a stored key (read-only, zeroized on drop).
    pub fn get_key(&self, key_id: &str) -> Result<SecureKey, CryptoError> {
        self.keys
            .read()
            .unwrap()
            .get(key_id)
            .cloned()
            .ok_or_else(|| CryptoError::KeyNotFound(key_id.to_string()))
    }

    /// Remove a key from the store (zeroized on drop).
    pub fn remove_key(&self, key_id: &str) -> Result<(), CryptoError> {
        self.keys
            .write()
            .unwrap()
            .remove(key_id)
            .ok_or_else(|| CryptoError::KeyNotFound(key_id.to_string()))?;
        Ok(())
    }

    /// Encrypt a plaintext using the key with the given ID.
    ///
    /// Placeholder: XOR-based obfuscation for API-shape demonstration only.
    /// Replace with AES-GCM/ChaCha20-Poly1305 in production.
    #[cfg(feature = "crypto")]
    pub fn encrypt(&self, key_id: &str, plaintext: &[u8]) -> Result<SealedBox, CryptoError> {
        let key = self.get_key(key_id)?;
        let key_bytes = key.as_bytes();
        if key_bytes.is_empty() {
            return Err(CryptoError::Encrypt("empty key".to_string()));
        }

        let mut nonce = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce);
        let ciphertext: Vec<u8> = plaintext
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ key_bytes[i % key_bytes.len()] ^ nonce[i % nonce.len()])
            .collect();

        Ok(SealedBox {
            nonce: nonce.to_vec(),
            ciphertext,
            key_id: key_id.to_string(),
        })
    }

    /// Decrypt a SealedBox using the key with the given ID.
    #[cfg(feature = "crypto")]
    pub fn decrypt(&self, sealed: &SealedBox) -> Result<Vec<u8>, CryptoError> {
        let key = self.get_key(&sealed.key_id)?;
        let key_bytes = key.as_bytes();
        if key_bytes.is_empty() {
            return Err(CryptoError::Decrypt("empty key".to_string()));
        }
        if sealed.nonce.is_empty() {
            return Err(CryptoError::Decrypt("empty nonce".to_string()));
        }

        let plaintext: Vec<u8> = sealed
            .ciphertext
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ key_bytes[i % key_bytes.len()] ^ sealed.nonce[i % sealed.nonce.len()])
            .collect();

        Ok(plaintext)
    }

    /// Number of keys currently stored.
    pub fn key_count(&self) -> usize {
        self.keys.read().unwrap().len()
    }

    /// List all key IDs.
    pub fn key_ids(&self) -> Vec<String> {
        self.keys.read().unwrap().keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_key_generate_and_zeroize() {
        #[cfg(feature = "crypto")]
        {
            let key = SecureKey::generate(32).unwrap();
            assert_eq!(key.len(), 32);
            assert!(!key.is_empty());
            let bytes = key.as_bytes().to_vec();
            drop(key);
            // After drop, memory should be zeroized (we can't directly verify,
            // but the type guarantees it).
            let _ = bytes;
        }
        #[cfg(not(feature = "crypto"))]
        {
            // Without the crypto feature, SecureKey::generate is unavailable.
            // Just test from_bytes.
            let key = SecureKey::from_bytes(vec![1, 2, 3, 4]);
            assert_eq!(key.len(), 4);
        }
    }

    #[test]
    fn crypto_service_key_management() {
        let service = CryptoService::new();
        assert_eq!(service.key_count(), 0);

        #[cfg(feature = "crypto")]
        {
            service.generate_key("key-1", 32).unwrap();
            assert_eq!(service.key_count(), 1);

            let retrieved = service.get_key("key-1").unwrap();
            assert_eq!(retrieved.len(), 32);

            service.remove_key("key-1").unwrap();
            assert_eq!(service.key_count(), 0);

            let result = service.get_key("key-1");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                CryptoError::KeyNotFound(_)
            ));
        }
        #[cfg(not(feature = "crypto"))]
        {
            // Import a key manually without the crypto feature.
            let key = SecureKey::from_bytes(vec![10, 20, 30]);
            service.import_key("key-1", key);
            assert_eq!(service.key_count(), 1);
        }
    }

    #[test]
    #[cfg(feature = "crypto")]
    fn encrypt_decrypt_round_trip() {
        let service = CryptoService::new();
        service.generate_key("key-1", 32).unwrap();

        let plaintext = b"hello world";
        let sealed = service.encrypt("key-1", plaintext).unwrap();
        assert_ne!(sealed.ciphertext, plaintext);
        assert_eq!(sealed.key_id, "key-1");

        let decrypted = service.decrypt(&sealed).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn encrypt_wrong_key_fails() {
        #[cfg(feature = "crypto")]
        {
            let service = CryptoService::new();
            service.generate_key("key-1", 32).unwrap();

            let plaintext = b"test";
            let sealed = service.encrypt("key-1", plaintext).unwrap();

            // Try to decrypt with a different key ID.
            let mut sealed_wrong = sealed.clone();
            sealed_wrong.key_id = "key-2".to_string();
            let result = service.decrypt(&sealed_wrong);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                CryptoError::KeyNotFound(_)
            ));
        }
        #[cfg(not(feature = "crypto"))]
        {
            // Without crypto feature, encrypt/decrypt are unavailable.
        }
    }
}
