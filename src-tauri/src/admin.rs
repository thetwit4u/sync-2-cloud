//! Admin functionality for key management and activity tracking
//! Uses a special admin folder in S3 that users cannot access

use rusoto_core::{Region, HttpClient};
use rusoto_credential::StaticProvider;
use rusoto_s3::{
    S3Client as RusotoS3Client, S3,
    GetObjectRequest, PutObjectRequest,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use futures::TryStreamExt;
use chrono::{DateTime, Utc};
use crate::secrets;

// Scaleway S3 Configuration
const S3_ENDPOINT: &str = "https://s3.nl-ams.scw.cloud";
const S3_REGION: &str = "nl-ams";
const S3_BUCKET: &str = "cloud-storage-exad";

// Admin folder path (not accessible by user keys)
const ADMIN_PREFIX: &str = "_admin/";
const WHITELIST_FILE: &str = "_admin/whitelist.json";
const BLACKLIST_FILE: &str = "_admin/blacklist.json";
const ACTIVITY_LOG_FILE: &str = "_admin/activity_log.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelistEntry {
    pub key_hash: String,  // SHA256 hash of the key (not the full key for security)
    pub user_name: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlacklistEntry {
    pub key_hash: String,
    pub user_name: String,
    pub user_id: String,
    pub blacklisted_at: DateTime<Utc>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLogEntry {
    pub key_hash: String,
    pub user_name: String,
    pub user_id: String,
    pub action: String,  // "login", "logout", "upload", "download", etc.
    pub timestamp: DateTime<Utc>,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Whitelist {
    pub entries: HashMap<String, WhitelistEntry>,  // key_hash -> entry
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Blacklist {
    pub entries: HashMap<String, BlacklistEntry>,  // key_hash -> entry
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActivityLog {
    pub entries: Vec<ActivityLogEntry>,
}

/// Hash a key for storage (we don't store raw keys)
pub fn hash_key(key: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

pub struct AdminClient {
    client: RusotoS3Client,
}

impl AdminClient {
    pub fn new() -> Result<Self, String> {
        let credentials = StaticProvider::new_minimal(
            secrets::S3_ACCESS_KEY.to_string(),
            secrets::S3_SECRET_KEY.to_string(),
        );

        let region = Region::Custom {
            name: S3_REGION.to_string(),
            endpoint: S3_ENDPOINT.to_string(),
        };

        let http_client = HttpClient::new()
            .map_err(|e| e.to_string())?;

        let client = RusotoS3Client::new_with(http_client, credentials, region);

        Ok(Self { client })
    }

    /// Read a JSON file from S3
    async fn read_json<T: for<'de> Deserialize<'de> + Default>(&self, key: &str) -> Result<T, String> {
        let request = GetObjectRequest {
            bucket: S3_BUCKET.to_string(),
            key: key.to_string(),
            ..Default::default()
        };

        match self.client.get_object(request).await {
            Ok(response) => {
                let body = response.body.ok_or("No body")?;
                let bytes: Vec<u8> = body
                    .map_ok(|b| b.to_vec())
                    .try_concat()
                    .await
                    .map_err(|e| e.to_string())?;
                
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())
            }
            Err(e) => {
                // If file doesn't exist, return default
                if e.to_string().contains("NoSuchKey") || e.to_string().contains("404") {
                    Ok(T::default())
                } else {
                    Err(e.to_string())
                }
            }
        }
    }

    /// Write a JSON file to S3
    async fn write_json<T: Serialize>(&self, key: &str, data: &T) -> Result<(), String> {
        let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
        
        let request = PutObjectRequest {
            bucket: S3_BUCKET.to_string(),
            key: key.to_string(),
            body: Some(json.into_bytes().into()),
            content_type: Some("application/json".to_string()),
            ..Default::default()
        };

        self.client.put_object(request).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get the whitelist
    pub async fn get_whitelist(&self) -> Result<Whitelist, String> {
        self.read_json(WHITELIST_FILE).await
    }

    /// Get the blacklist
    pub async fn get_blacklist(&self) -> Result<Blacklist, String> {
        self.read_json(BLACKLIST_FILE).await
    }

    /// Check if a key is whitelisted
    pub async fn is_whitelisted(&self, key: &str) -> Result<bool, String> {
        let key_hash = hash_key(key);
        let whitelist = self.get_whitelist().await?;
        Ok(whitelist.entries.contains_key(&key_hash))
    }

    /// Check if a key is blacklisted
    pub async fn is_blacklisted(&self, key: &str) -> Result<(bool, Option<String>), String> {
        let key_hash = hash_key(key);
        let blacklist = self.get_blacklist().await?;
        
        if let Some(entry) = blacklist.entries.get(&key_hash) {
            Ok((true, Some(entry.reason.clone())))
        } else {
            Ok((false, None))
        }
    }

    /// Add a key to the whitelist
    pub async fn add_to_whitelist(
        &self,
        key: &str,
        user_name: &str,
        user_id: &str,
        notes: Option<String>,
    ) -> Result<(), String> {
        let key_hash = hash_key(key);
        let mut whitelist = self.get_whitelist().await?;
        
        whitelist.entries.insert(key_hash.clone(), WhitelistEntry {
            key_hash,
            user_name: user_name.to_string(),
            user_id: user_id.to_string(),
            created_at: Utc::now(),
            notes,
        });
        
        self.write_json(WHITELIST_FILE, &whitelist).await
    }

    /// Remove a key from the whitelist
    pub async fn remove_from_whitelist(&self, key: &str) -> Result<(), String> {
        let key_hash = hash_key(key);
        let mut whitelist = self.get_whitelist().await?;
        whitelist.entries.remove(&key_hash);
        self.write_json(WHITELIST_FILE, &whitelist).await
    }

    /// Add a key to the blacklist
    pub async fn add_to_blacklist(
        &self,
        key: &str,
        user_name: &str,
        user_id: &str,
        reason: &str,
    ) -> Result<(), String> {
        let key_hash = hash_key(key);
        let mut blacklist = self.get_blacklist().await?;
        
        blacklist.entries.insert(key_hash.clone(), BlacklistEntry {
            key_hash,
            user_name: user_name.to_string(),
            user_id: user_id.to_string(),
            blacklisted_at: Utc::now(),
            reason: reason.to_string(),
        });
        
        self.write_json(BLACKLIST_FILE, &blacklist).await
    }

    /// Remove a key from the blacklist
    pub async fn remove_from_blacklist(&self, key: &str) -> Result<(), String> {
        let key_hash = hash_key(key);
        let mut blacklist = self.get_blacklist().await?;
        blacklist.entries.remove(&key_hash);
        self.write_json(BLACKLIST_FILE, &blacklist).await
    }

    /// Log an activity
    pub async fn log_activity(
        &self,
        key: &str,
        user_name: &str,
        user_id: &str,
        action: &str,
        details: Option<String>,
    ) -> Result<(), String> {
        let key_hash = hash_key(key);
        let mut log = self.get_activity_log().await.unwrap_or_default();
        
        log.entries.push(ActivityLogEntry {
            key_hash,
            user_name: user_name.to_string(),
            user_id: user_id.to_string(),
            action: action.to_string(),
            timestamp: Utc::now(),
            details,
        });
        
        // Keep only last 10000 entries to prevent file from growing too large
        if log.entries.len() > 10000 {
            log.entries = log.entries.split_off(log.entries.len() - 10000);
        }
        
        self.write_json(ACTIVITY_LOG_FILE, &log).await
    }

    /// Get the activity log
    pub async fn get_activity_log(&self) -> Result<ActivityLog, String> {
        self.read_json(ACTIVITY_LOG_FILE).await
    }

    /// Validate a key (check whitelist and blacklist)
    pub async fn validate_key_access(&self, key: &str) -> Result<KeyValidationResult, String> {
        // First check blacklist
        let (is_blacklisted, reason) = self.is_blacklisted(key).await?;
        if is_blacklisted {
            return Ok(KeyValidationResult {
                allowed: false,
                reason: Some(format!("Key has been disabled: {}", reason.unwrap_or_default())),
            });
        }

        // Then check whitelist (if whitelist is empty, allow all keys)
        let whitelist = self.get_whitelist().await?;
        if !whitelist.entries.is_empty() {
            let key_hash = hash_key(key);
            if !whitelist.entries.contains_key(&key_hash) {
                return Ok(KeyValidationResult {
                    allowed: false,
                    reason: Some("Key is not authorized. Please contact your administrator.".to_string()),
                });
            }
        }

        Ok(KeyValidationResult {
            allowed: true,
            reason: None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValidationResult {
    pub allowed: bool,
    pub reason: Option<String>,
}

