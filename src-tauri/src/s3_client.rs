use rusoto_core::{Region, HttpClient};
use rusoto_credential::StaticProvider;
use rusoto_s3::{
    S3Client as RusotoS3Client, S3,
    GetObjectRequest, PutObjectRequest, ListObjectsV2Request,
    HeadObjectRequest, DeleteObjectRequest,
};
use std::path::Path;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use futures::TryStreamExt;

use crate::secrets;

// Scaleway S3 Configuration
const S3_ENDPOINT: &str = "https://s3.nl-ams.scw.cloud";
const S3_REGION: &str = "nl-ams";
const S3_BUCKET: &str = "cloud-storage-exad";

// Credentials expiration date (November 28, 2025 + 1 year = November 28, 2026)
// Update this when renewing credentials
const CREDENTIALS_EXPIRY_YEAR: i32 = 2026;
const CREDENTIALS_EXPIRY_MONTH: u32 = 11;
const CREDENTIALS_EXPIRY_DAY: u32 = 28;

#[derive(Debug, Error)]
pub enum S3Error {
    #[error("S3 operation failed: {0}")]
    OperationFailed(String),
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("API credentials have expired. Please contact your administrator to renew access. Expiry date: {0}")]
    CredentialsExpired(String),
}

pub struct S3Client {
    client: RusotoS3Client,
    user_prefix: String,
}

impl S3Client {
    /// Check if credentials have expired
    fn check_credentials_expiry() -> Result<(), S3Error> {
        use chrono::{Utc, TimeZone};
        
        let expiry_date = Utc.with_ymd_and_hms(
            CREDENTIALS_EXPIRY_YEAR,
            CREDENTIALS_EXPIRY_MONTH,
            CREDENTIALS_EXPIRY_DAY,
            23, 59, 59
        ).unwrap();
        
        let now = Utc::now();
        
        if now > expiry_date {
            let expiry_str = format!("{}-{:02}-{:02}", 
                CREDENTIALS_EXPIRY_YEAR, 
                CREDENTIALS_EXPIRY_MONTH, 
                CREDENTIALS_EXPIRY_DAY
            );
            return Err(S3Error::CredentialsExpired(expiry_str));
        }
        
        Ok(())
    }

    /// Get days until credentials expire (for warning)
    pub fn days_until_expiry() -> i64 {
        use chrono::{Utc, TimeZone};
        
        let expiry_date = Utc.with_ymd_and_hms(
            CREDENTIALS_EXPIRY_YEAR,
            CREDENTIALS_EXPIRY_MONTH,
            CREDENTIALS_EXPIRY_DAY,
            23, 59, 59
        ).unwrap();
        
        let now = Utc::now();
        (expiry_date - now).num_days()
    }

    /// Create a new S3 client with the user's folder prefix
    pub async fn new(user_prefix: String) -> Result<Self, S3Error> {
        // Check if credentials have expired
        Self::check_credentials_expiry()?;

        let credentials = StaticProvider::new_minimal(
            secrets::S3_ACCESS_KEY.to_string(),
            secrets::S3_SECRET_KEY.to_string(),
        );

        let region = Region::Custom {
            name: S3_REGION.to_string(),
            endpoint: S3_ENDPOINT.to_string(),
        };

        let http_client = HttpClient::new()
            .map_err(|e| S3Error::OperationFailed(e.to_string()))?;

        let client = RusotoS3Client::new_with(http_client, credentials, region);

        Ok(Self {
            client,
            user_prefix,
        })
    }

    /// Get the full S3 key for a relative path
    fn full_key(&self, relative_path: &str) -> String {
        format!("{}{}", self.user_prefix, relative_path)
    }

    /// Upload a file to S3
    pub async fn upload_file(
        &self,
        local_path: &Path,
        remote_path: &str,
    ) -> Result<(), S3Error> {
        let mut file = File::open(local_path)
            .await
            .map_err(|e| S3Error::IoError(e.to_string()))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .await
            .map_err(|e| S3Error::IoError(e.to_string()))?;

        let key = self.full_key(remote_path);

        let request = PutObjectRequest {
            bucket: S3_BUCKET.to_string(),
            key,
            body: Some(contents.into()),
            ..Default::default()
        };

        self.client
            .put_object(request)
            .await
            .map_err(|e| S3Error::OperationFailed(e.to_string()))?;

        Ok(())
    }

    /// Download a file from S3
    pub async fn download_file(
        &self,
        remote_path: &str,
        local_path: &Path,
    ) -> Result<(), S3Error> {
        let key = self.full_key(remote_path);

        let request = GetObjectRequest {
            bucket: S3_BUCKET.to_string(),
            key,
            ..Default::default()
        };

        let response = self
            .client
            .get_object(request)
            .await
            .map_err(|e| S3Error::OperationFailed(e.to_string()))?;

        let body = response.body.ok_or_else(|| S3Error::FileNotFound("No body".into()))?;
        
        let bytes: Vec<u8> = body
            .map_ok(|b| b.to_vec())
            .try_concat()
            .await
            .map_err(|e| S3Error::IoError(e.to_string()))?;

        // Ensure parent directory exists
        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| S3Error::IoError(e.to_string()))?;
        }

        let mut file = File::create(local_path)
            .await
            .map_err(|e| S3Error::IoError(e.to_string()))?;
        
        file.write_all(&bytes)
            .await
            .map_err(|e| S3Error::IoError(e.to_string()))?;

        Ok(())
    }

    /// List all objects in the user's folder
    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<S3Object>, S3Error> {
        let full_prefix = self.full_key(prefix);
        let mut objects = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let request = ListObjectsV2Request {
                bucket: S3_BUCKET.to_string(),
                prefix: Some(full_prefix.clone()),
                continuation_token: continuation_token.clone(),
                ..Default::default()
            };

            let response = self
                .client
                .list_objects_v2(request)
                .await
                .map_err(|e| S3Error::OperationFailed(e.to_string()))?;

            if let Some(contents) = response.contents {
                for obj in contents {
                    if let Some(key) = obj.key {
                        // Remove user prefix to get relative path
                        let relative_key = key
                            .strip_prefix(&self.user_prefix)
                            .unwrap_or(&key)
                            .to_string();

                        objects.push(S3Object {
                            key: relative_key,
                            size: obj.size.unwrap_or(0) as u64,
                            last_modified: obj
                                .last_modified
                                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                                .map(|dt| dt.timestamp())
                                .unwrap_or(0),
                        });
                    }
                }
            }

            if response.is_truncated.unwrap_or(false) {
                continuation_token = response.next_continuation_token;
            } else {
                break;
            }
        }

        Ok(objects)
    }

    /// List folders (common prefixes) at a given path
    pub async fn list_folders(&self, prefix: &str) -> Result<Vec<String>, S3Error> {
        let full_prefix = self.full_key(prefix);
        let mut folders = Vec::new();

        let request = ListObjectsV2Request {
            bucket: S3_BUCKET.to_string(),
            prefix: Some(full_prefix.clone()),
            delimiter: Some("/".to_string()),
            ..Default::default()
        };

        let response = self
            .client
            .list_objects_v2(request)
            .await
            .map_err(|e| S3Error::OperationFailed(e.to_string()))?;

        if let Some(common_prefixes) = response.common_prefixes {
            for prefix in common_prefixes {
                if let Some(p) = prefix.prefix {
                    // Remove user prefix to get relative path
                    let relative = p
                        .strip_prefix(&self.user_prefix)
                        .unwrap_or(&p)
                        .to_string();
                    folders.push(relative);
                }
            }
        }

        Ok(folders)
    }

    /// Delete an object from S3
    pub async fn delete_object(&self, remote_path: &str) -> Result<(), S3Error> {
        let key = self.full_key(remote_path);

        let request = DeleteObjectRequest {
            bucket: S3_BUCKET.to_string(),
            key,
            ..Default::default()
        };

        self.client
            .delete_object(request)
            .await
            .map_err(|e| S3Error::OperationFailed(e.to_string()))?;

        Ok(())
    }

    /// Delete all objects in the user's folder
    pub async fn delete_all_objects(&self) -> Result<usize, S3Error> {
        // First list all objects
        let objects = self.list_objects("").await?;
        let count = objects.len();

        // Delete each object
        for obj in objects {
            let key = self.full_key(&obj.key);
            let request = DeleteObjectRequest {
                bucket: S3_BUCKET.to_string(),
                key,
                ..Default::default()
            };

            self.client
                .delete_object(request)
                .await
                .map_err(|e| S3Error::OperationFailed(e.to_string()))?;
        }

        Ok(count)
    }

    /// Get object metadata (size, last modified)
    pub async fn get_object_info(&self, remote_path: &str) -> Result<S3Object, S3Error> {
        let key = self.full_key(remote_path);

        let request = HeadObjectRequest {
            bucket: S3_BUCKET.to_string(),
            key: key.clone(),
            ..Default::default()
        };

        let response = self
            .client
            .head_object(request)
            .await
            .map_err(|e| S3Error::OperationFailed(e.to_string()))?;

        Ok(S3Object {
            key: remote_path.to_string(),
            size: response.content_length.unwrap_or(0) as u64,
            last_modified: response
                .last_modified
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.timestamp())
                .unwrap_or(0),
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct S3Object {
    pub key: String,
    pub size: u64,
    pub last_modified: i64,
}
