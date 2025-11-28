// Secret configuration - COPY THIS FILE TO secrets.rs AND FILL IN YOUR VALUES
// 
// DO NOT COMMIT secrets.rs TO GIT!

/// Scaleway S3 Access Key
pub const S3_ACCESS_KEY: &str = "YOUR_SCALEWAY_ACCESS_KEY";

/// Scaleway S3 Secret Key  
pub const S3_SECRET_KEY: &str = "YOUR_SCALEWAY_SECRET_KEY";

/// Master encryption key for user license keys (MUST be exactly 32 bytes)
/// Generate a random 32-character string for production
pub const MASTER_ENCRYPTION_KEY: &[u8; 32] = b"YOUR_32_CHARACTER_SECRET_KEY!!!";

