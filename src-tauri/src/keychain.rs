use keyring::Entry;
use thiserror::Error;

const SERVICE_NAME: &str = "com.exad.sync2bucket";
const KEY_NAME: &str = "license_key";

#[derive(Debug, Error)]
pub enum KeychainError {
    #[error("Keychain access error: {0}")]
    AccessError(String),
    #[error("Key not found in keychain")]
    NotFound,
}

/// Store a license key in the macOS Keychain
pub fn store_key(key: &str) -> Result<(), KeychainError> {
    let entry = Entry::new(SERVICE_NAME, KEY_NAME)
        .map_err(|e| KeychainError::AccessError(e.to_string()))?;
    
    entry
        .set_password(key)
        .map_err(|e| KeychainError::AccessError(e.to_string()))?;
    
    Ok(())
}

/// Retrieve a license key from the macOS Keychain
pub fn get_key() -> Result<String, KeychainError> {
    let entry = Entry::new(SERVICE_NAME, KEY_NAME)
        .map_err(|e| KeychainError::AccessError(e.to_string()))?;
    
    entry.get_password().map_err(|e| match e {
        keyring::Error::NoEntry => KeychainError::NotFound,
        _ => KeychainError::AccessError(e.to_string()),
    })
}

/// Delete a license key from the macOS Keychain
pub fn delete_key() -> Result<(), KeychainError> {
    let entry = Entry::new(SERVICE_NAME, KEY_NAME)
        .map_err(|e| KeychainError::AccessError(e.to_string()))?;
    
    entry
        .delete_password()
        .map_err(|e| KeychainError::AccessError(e.to_string()))?;
    
    Ok(())
}

/// Check if a key exists in the keychain
pub fn has_key() -> bool {
    get_key().is_ok()
}

