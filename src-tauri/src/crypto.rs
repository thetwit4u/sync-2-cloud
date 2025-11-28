use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::secrets;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Invalid key format")]
    InvalidFormat,
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Invalid JSON payload")]
    InvalidPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPayload {
    pub uid: String,
    pub name: String,
    pub created: i64,
}

impl KeyPayload {
    pub fn new(name: &str) -> Self {
        let uid = generate_uid(name);
        Self {
            uid,
            name: name.to_string(),
            created: chrono::Utc::now().timestamp(),
        }
    }

    /// Get the S3 folder prefix for this user
    pub fn folder_prefix(&self) -> String {
        format!("users/{}/", self.uid)
    }
}

/// Generate a unique ID from a name
fn generate_uid(name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.update(chrono::Utc::now().timestamp().to_le_bytes());
    let result = hasher.finalize();
    format!("u_{}", hex::encode(&result[..8]))
}

/// Encrypt a KeyPayload into an EXAD-prefixed license key
pub fn encrypt_key(payload: &KeyPayload) -> Result<String, CryptoError> {
    let json = serde_json::to_string(payload).map_err(|_| CryptoError::InvalidPayload)?;
    
    // Generate a random nonce (12 bytes for AES-GCM)
    let nonce_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let cipher = Aes256Gcm::new_from_slice(secrets::MASTER_ENCRYPTION_KEY)
        .map_err(|_| CryptoError::EncryptionFailed)?;
    
    let ciphertext = cipher
        .encrypt(nonce, json.as_bytes())
        .map_err(|_| CryptoError::EncryptionFailed)?;
    
    // Combine nonce + ciphertext and encode
    let mut combined = nonce_bytes.to_vec();
    combined.extend(ciphertext);
    
    let encoded = URL_SAFE_NO_PAD.encode(&combined);
    Ok(format!("EXAD-{}", encoded))
}

/// Decrypt an EXAD-prefixed license key into a KeyPayload
pub fn decrypt_key(key: &str) -> Result<KeyPayload, CryptoError> {
    // Remove EXAD- prefix
    let encoded = key
        .strip_prefix("EXAD-")
        .ok_or(CryptoError::InvalidFormat)?;
    
    let combined = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| CryptoError::InvalidFormat)?;
    
    if combined.len() < 13 {
        return Err(CryptoError::InvalidFormat);
    }
    
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    
    let cipher = Aes256Gcm::new_from_slice(secrets::MASTER_ENCRYPTION_KEY)
        .map_err(|_| CryptoError::DecryptionFailed)?;
    
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::DecryptionFailed)?;
    
    let json = String::from_utf8(plaintext).map_err(|_| CryptoError::DecryptionFailed)?;
    
    serde_json::from_str(&json).map_err(|_| CryptoError::InvalidPayload)
}

/// Validate a key without fully decrypting (just check format)
pub fn validate_key_format(key: &str) -> bool {
    if !key.starts_with("EXAD-") {
        return false;
    }
    
    let encoded = match key.strip_prefix("EXAD-") {
        Some(e) => e,
        None => return false,
    };
    
    // Check if it's valid base64
    URL_SAFE_NO_PAD.decode(encoded).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let payload = KeyPayload::new("Test User");
        let encrypted = encrypt_key(&payload).unwrap();
        
        assert!(encrypted.starts_with("EXAD-"));
        
        let decrypted = decrypt_key(&encrypted).unwrap();
        assert_eq!(decrypted.name, "Test User");
        assert!(decrypted.uid.starts_with("u_"));
    }

    #[test]
    fn test_invalid_key() {
        assert!(decrypt_key("invalid").is_err());
        assert!(decrypt_key("EXAD-invalid").is_err());
    }
}

