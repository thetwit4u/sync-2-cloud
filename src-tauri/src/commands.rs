use crate::admin::AdminClient;
use crate::crypto::{decrypt_key, KeyPayload};
use crate::s3_client::S3Client;
use crate::sync_engine::{CloudFolder, SyncEngine, SyncProgress};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

/// App state shared across commands
pub struct AppState {
    pub key_payload: RwLock<Option<KeyPayload>>,
    pub sync_engine: RwLock<Option<Arc<SyncEngine>>>,
    pub current_key: RwLock<Option<String>>,  // Store the key for activity logging
}

impl AppState {
    pub fn new() -> Self {
        Self {
            key_payload: RwLock::new(None),
            sync_engine: RwLock::new(None),
            current_key: RwLock::new(None),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub user_name: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncRequest {
    pub source_paths: Vec<String>,
    pub target_path: Option<String>,
    pub cloud_folder: Option<String>,
}

/// Check if user has a stored key (disabled - always returns false)
#[tauri::command]
pub async fn check_stored_key(_state: State<'_, AppState>) -> Result<bool, String> {
    // Key storage disabled - user must enter key each time
    Ok(false)
}

/// Validate and store a license key
#[tauri::command]
pub async fn validate_key(key: String, state: State<'_, AppState>) -> Result<ValidationResult, String> {
    // Validate key format and decrypt
    let payload = match decrypt_key(&key) {
        Ok(p) => p,
        Err(e) => {
            return Ok(ValidationResult {
                valid: false,
                user_name: None,
                error: Some(format!("Invalid key: {}", e)),
            });
        }
    };

    // Check whitelist/blacklist
    if let Ok(admin) = AdminClient::new() {
        match admin.validate_key_access(&key).await {
            Ok(validation) => {
                if !validation.allowed {
                    // Log the failed attempt
                    let _ = admin.log_activity(
                        &key,
                        &payload.name,
                        &payload.uid,
                        "login_blocked",
                        validation.reason.clone(),
                    ).await;

                    return Ok(ValidationResult {
                        valid: false,
                        user_name: None,
                        error: validation.reason,
                    });
                }
            }
            Err(e) => {
                log::warn!("Failed to check key access: {}", e);
                // Continue anyway if admin check fails (network issue, etc.)
            }
        }
    }

    // Try to create S3 client to verify connectivity
    let s3_client = match S3Client::new(payload.folder_prefix()).await {
        Ok(c) => c,
        Err(e) => {
            return Ok(ValidationResult {
                valid: false,
                user_name: None,
                error: Some(format!("Connection failed: {}", e)),
            });
        }
    };

    let user_name = payload.name.clone();
    let user_id = payload.uid.clone();
    
    // Log successful login
    if let Ok(admin) = AdminClient::new() {
        let _ = admin.log_activity(
            &key,
            &user_name,
            &user_id,
            "login",
            Some("Key entry".to_string()),
        ).await;
    }
    
    // Initialize sync engine (key stored in memory only, not persisted)
    *state.sync_engine.write().await = Some(Arc::new(SyncEngine::new(s3_client)));
    *state.key_payload.write().await = Some(payload);
    *state.current_key.write().await = Some(key);

    Ok(ValidationResult {
        valid: true,
        user_name: Some(user_name),
        error: None,
    })
}

/// Get current user info
#[tauri::command]
pub async fn get_user_info(state: State<'_, AppState>) -> Result<Option<KeyPayload>, String> {
    Ok(state.key_payload.read().await.clone())
}

/// Logout - clear session
#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    // Log logout activity
    if let (Some(key), Some(payload)) = (
        state.current_key.read().await.clone(),
        state.key_payload.read().await.clone(),
    ) {
        if let Ok(admin) = AdminClient::new() {
            let _ = admin.log_activity(
                &key,
                &payload.name,
                &payload.uid,
                "logout",
                None,
            ).await;
        }
    }

    // Clear session (no keychain to delete)
    *state.key_payload.write().await = None;
    *state.sync_engine.write().await = None;
    *state.current_key.write().await = None;
    Ok(())
}

/// Start sync from local to cloud
#[tauri::command]
pub async fn start_upload(
    source_paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let engine = state.sync_engine.read().await;
    let engine = engine.as_ref().ok_or("Not authenticated")?;
    
    // Log upload activity
    if let (Some(key), Some(payload)) = (
        state.current_key.read().await.clone(),
        state.key_payload.read().await.clone(),
    ) {
        if let Ok(admin) = AdminClient::new() {
            let _ = admin.log_activity(
                &key,
                &payload.name,
                &payload.uid,
                "upload_started",
                Some(format!("Folders: {}", source_paths.join(", "))),
            ).await;
        }
    }
    
    let paths: Vec<PathBuf> = source_paths.iter().map(PathBuf::from).collect();
    
    // Clone the Arc to move into the async block
    let engine = Arc::clone(engine);
    
    // Spawn the sync task
    tokio::spawn(async move {
        if let Err(e) = engine.sync_to_cloud(&paths).await {
            log::error!("Upload failed: {}", e);
        }
    });
    
    Ok(())
}

/// Start sync from cloud to local
#[tauri::command]
pub async fn start_download(
    cloud_folder: String,
    target_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let engine = state.sync_engine.read().await;
    let engine = engine.as_ref().ok_or("Not authenticated")?;
    
    // Log download activity
    if let (Some(key), Some(payload)) = (
        state.current_key.read().await.clone(),
        state.key_payload.read().await.clone(),
    ) {
        if let Ok(admin) = AdminClient::new() {
            let _ = admin.log_activity(
                &key,
                &payload.name,
                &payload.uid,
                "download_started",
                Some(format!("Folder: {} -> {}", cloud_folder, target_path)),
            ).await;
        }
    }
    
    let target = PathBuf::from(target_path);
    let engine = Arc::clone(engine);
    let folder = cloud_folder.clone();
    
    // Spawn the sync task
    tokio::spawn(async move {
        if let Err(e) = engine.sync_to_local(&folder, &target).await {
            log::error!("Download failed: {}", e);
        }
    });
    
    Ok(())
}

/// Pause the current sync
#[tauri::command]
pub async fn pause_sync(state: State<'_, AppState>) -> Result<(), String> {
    let engine = state.sync_engine.read().await;
    let engine = engine.as_ref().ok_or("No active sync")?;
    engine.pause();
    Ok(())
}

/// Resume the current sync
#[tauri::command]
pub async fn resume_sync(state: State<'_, AppState>) -> Result<(), String> {
    let engine = state.sync_engine.read().await;
    let engine = engine.as_ref().ok_or("No active sync")?;
    engine.resume();
    Ok(())
}

/// Cancel the current sync
#[tauri::command]
pub async fn cancel_sync(state: State<'_, AppState>) -> Result<(), String> {
    let engine = state.sync_engine.read().await;
    let engine = engine.as_ref().ok_or("No active sync")?;
    engine.cancel();
    Ok(())
}

/// Get current sync progress
#[tauri::command]
pub async fn get_sync_progress(state: State<'_, AppState>) -> Result<SyncProgress, String> {
    let engine = state.sync_engine.read().await;
    let engine = engine.as_ref().ok_or("Not authenticated")?;
    Ok(engine.get_progress().await)
}

/// List cloud folders
#[tauri::command]
pub async fn list_cloud_folders(state: State<'_, AppState>) -> Result<Vec<CloudFolder>, String> {
    let engine = state.sync_engine.read().await;
    let engine = engine.as_ref().ok_or("Not authenticated")?;
    engine.list_cloud_folders().await.map_err(|e| e.to_string())
}

/// Delete all files in the user's cloud storage
#[tauri::command]
pub async fn delete_all_files(state: State<'_, AppState>) -> Result<usize, String> {
    let payload = state.key_payload.read().await;
    let payload = payload.as_ref().ok_or("Not authenticated")?.clone();
    
    // Log delete activity
    if let Some(key) = state.current_key.read().await.clone() {
        if let Ok(admin) = AdminClient::new() {
            let _ = admin.log_activity(
                &key,
                &payload.name,
                &payload.uid,
                "delete_all_files",
                Some("User requested deletion of all cloud files".to_string()),
            ).await;
        }
    }
    
    let s3_client = crate::s3_client::S3Client::new(payload.folder_prefix())
        .await
        .map_err(|e| e.to_string())?;
    
    s3_client.delete_all_objects().await.map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialsStatus {
    pub valid: bool,
    pub days_remaining: i64,
    pub expiry_date: String,
    pub warning: Option<String>,
}

/// Check credentials expiration status
#[tauri::command]
pub async fn check_credentials_status() -> Result<CredentialsStatus, String> {
    let days_remaining = crate::s3_client::S3Client::days_until_expiry();
    let expiry_date = "2026-11-28".to_string();
    
    let warning = if days_remaining <= 0 {
        Some("API credentials have expired. Please contact your administrator to renew access.".to_string())
    } else if days_remaining <= 30 {
        Some(format!("Warning: API credentials will expire in {} days. Please contact your administrator.", days_remaining))
    } else if days_remaining <= 90 {
        Some(format!("Notice: API credentials will expire in {} days.", days_remaining))
    } else {
        None
    };
    
    Ok(CredentialsStatus {
        valid: days_remaining > 0,
        days_remaining,
        expiry_date,
        warning,
    })
}

