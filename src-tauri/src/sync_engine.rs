use crate::s3_client::S3Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("S3 error: {0}")]
    S3Error(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Sync cancelled")]
    Cancelled,
    #[error("No active sync")]
    NoActiveSync,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncDirection {
    LocalToCloud,
    CloudToLocal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    Idle,
    Scanning,
    Syncing,
    Paused,
    Completed,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgress {
    pub status: SyncStatus,
    pub direction: Option<SyncDirection>,
    pub total_files: u64,
    pub completed_files: u64,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub current_file: Option<String>,
    pub bytes_per_second: f64,
    pub eta_seconds: Option<u64>,
}

impl Default for SyncProgress {
    fn default() -> Self {
        Self {
            status: SyncStatus::Idle,
            direction: None,
            total_files: 0,
            completed_files: 0,
            total_bytes: 0,
            transferred_bytes: 0,
            current_file: None,
            bytes_per_second: 0.0,
            eta_seconds: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
}

pub struct SyncEngine {
    s3_client: Arc<S3Client>,
    progress: Arc<RwLock<SyncProgress>>,
    is_paused: Arc<AtomicBool>,
    is_cancelled: Arc<AtomicBool>,
    transferred_bytes: Arc<AtomicU64>,
    start_time: Arc<RwLock<Option<std::time::Instant>>>,
}

impl SyncEngine {
    pub fn new(s3_client: S3Client) -> Self {
        Self {
            s3_client: Arc::new(s3_client),
            progress: Arc::new(RwLock::new(SyncProgress::default())),
            is_paused: Arc::new(AtomicBool::new(false)),
            is_cancelled: Arc::new(AtomicBool::new(false)),
            transferred_bytes: Arc::new(AtomicU64::new(0)),
            start_time: Arc::new(RwLock::new(None)),
        }
    }

    /// Get current sync progress
    pub async fn get_progress(&self) -> SyncProgress {
        let mut progress = self.progress.read().await.clone();
        
        // Calculate transfer speed and ETA
        if let Some(start) = *self.start_time.read().await {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                let transferred = self.transferred_bytes.load(Ordering::Relaxed);
                progress.transferred_bytes = transferred;
                progress.bytes_per_second = transferred as f64 / elapsed;
                
                if progress.bytes_per_second > 0.0 && progress.total_bytes > transferred {
                    let remaining = progress.total_bytes - transferred;
                    progress.eta_seconds = Some((remaining as f64 / progress.bytes_per_second) as u64);
                }
            }
        }
        
        progress
    }

    /// Pause the sync
    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::Relaxed);
    }

    /// Resume the sync
    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::Relaxed);
    }

    /// Cancel the sync
    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::Relaxed);
        self.is_paused.store(false, Ordering::Relaxed);
    }

    /// Check if sync is paused
    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::Relaxed)
    }

    /// Wait while paused, return error if cancelled
    async fn wait_if_paused(&self) -> Result<(), SyncError> {
        while self.is_paused.load(Ordering::Relaxed) {
            if self.is_cancelled.load(Ordering::Relaxed) {
                return Err(SyncError::Cancelled);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        if self.is_cancelled.load(Ordering::Relaxed) {
            return Err(SyncError::Cancelled);
        }
        
        Ok(())
    }

    /// Scan local folders to get list of files
    pub async fn scan_local_folders(&self, paths: &[PathBuf]) -> Result<Vec<FileEntry>, SyncError> {
        let mut entries = Vec::new();
        
        for base_path in paths {
            let folder_name = base_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "folder".to_string());
            
            for entry in WalkDir::new(base_path).follow_links(true) {
                let entry = entry.map_err(|e| SyncError::IoError(e.to_string()))?;
                let path = entry.path();
                
                if path.is_file() {
                    let relative = path
                        .strip_prefix(base_path)
                        .map_err(|e| SyncError::IoError(e.to_string()))?;
                    
                    let remote_path = format!("{}/{}", folder_name, relative.display());
                    let metadata = std::fs::metadata(path)
                        .map_err(|e| SyncError::IoError(e.to_string()))?;
                    
                    entries.push(FileEntry {
                        path: remote_path,
                        size: metadata.len(),
                        is_dir: false,
                    });
                }
            }
        }
        
        Ok(entries)
    }

    /// Sync local folders to cloud
    pub async fn sync_to_cloud(&self, source_paths: &[PathBuf]) -> Result<(), SyncError> {
        // Reset state
        self.is_cancelled.store(false, Ordering::Relaxed);
        self.is_paused.store(false, Ordering::Relaxed);
        self.transferred_bytes.store(0, Ordering::Relaxed);
        *self.start_time.write().await = Some(std::time::Instant::now());
        
        // Update status to scanning
        {
            let mut progress = self.progress.write().await;
            progress.status = SyncStatus::Scanning;
            progress.direction = Some(SyncDirection::LocalToCloud);
        }
        
        // Scan files
        let files = self.scan_local_folders(source_paths).await?;
        let total_bytes: u64 = files.iter().map(|f| f.size).sum();
        let total_files = files.len() as u64;
        
        // Update progress with totals
        {
            let mut progress = self.progress.write().await;
            progress.status = SyncStatus::Syncing;
            progress.total_files = total_files;
            progress.total_bytes = total_bytes;
            progress.completed_files = 0;
        }
        
        // Upload each file
        for (idx, file) in files.iter().enumerate() {
            self.wait_if_paused().await?;
            
            // Update current file
            {
                let mut progress = self.progress.write().await;
                progress.current_file = Some(file.path.clone());
            }
            
            // Find the source path for this file
            let source_file = self.find_source_file(source_paths, &file.path)?;
            
            // Upload
            self.s3_client
                .upload_file(&source_file, &file.path)
                .await
                .map_err(|e| SyncError::S3Error(e.to_string()))?;
            
            // Update progress
            self.transferred_bytes.fetch_add(file.size, Ordering::Relaxed);
            {
                let mut progress = self.progress.write().await;
                progress.completed_files = (idx + 1) as u64;
            }
        }
        
        // Mark as completed
        {
            let mut progress = self.progress.write().await;
            progress.status = SyncStatus::Completed;
            progress.current_file = None;
        }
        
        Ok(())
    }

    /// Find the actual source file path given the remote path
    fn find_source_file(&self, source_paths: &[PathBuf], remote_path: &str) -> Result<PathBuf, SyncError> {
        for base_path in source_paths {
            let folder_name = base_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "folder".to_string());
            
            if let Some(relative) = remote_path.strip_prefix(&format!("{}/", folder_name)) {
                let full_path = base_path.join(relative);
                if full_path.exists() {
                    return Ok(full_path);
                }
            }
        }
        
        Err(SyncError::IoError(format!("Source file not found: {}", remote_path)))
    }

    /// Sync cloud folder to local
    pub async fn sync_to_local(
        &self,
        cloud_folder: &str,
        target_path: &Path,
    ) -> Result<(), SyncError> {
        // Reset state
        self.is_cancelled.store(false, Ordering::Relaxed);
        self.is_paused.store(false, Ordering::Relaxed);
        self.transferred_bytes.store(0, Ordering::Relaxed);
        *self.start_time.write().await = Some(std::time::Instant::now());
        
        // Update status to scanning
        {
            let mut progress = self.progress.write().await;
            progress.status = SyncStatus::Scanning;
            progress.direction = Some(SyncDirection::CloudToLocal);
        }
        
        // List cloud files
        let objects = self.s3_client
            .list_objects(cloud_folder)
            .await
            .map_err(|e| SyncError::S3Error(e.to_string()))?;
        
        let total_bytes: u64 = objects.iter().map(|o| o.size).sum();
        let total_files = objects.len() as u64;
        
        // Update progress with totals
        {
            let mut progress = self.progress.write().await;
            progress.status = SyncStatus::Syncing;
            progress.total_files = total_files;
            progress.total_bytes = total_bytes;
            progress.completed_files = 0;
        }
        
        // Download each file
        for (idx, obj) in objects.iter().enumerate() {
            self.wait_if_paused().await?;
            
            // Skip directories (keys ending with /)
            if obj.key.ends_with('/') {
                continue;
            }
            
            // Update current file
            {
                let mut progress = self.progress.write().await;
                progress.current_file = Some(obj.key.clone());
            }
            
            // Calculate local path
            let relative = obj.key.strip_prefix(cloud_folder).unwrap_or(&obj.key);
            let relative = relative.trim_start_matches('/');
            let local_path = target_path.join(relative);
            
            // Download
            self.s3_client
                .download_file(&obj.key, &local_path)
                .await
                .map_err(|e| SyncError::S3Error(e.to_string()))?;
            
            // Update progress
            self.transferred_bytes.fetch_add(obj.size, Ordering::Relaxed);
            {
                let mut progress = self.progress.write().await;
                progress.completed_files = (idx + 1) as u64;
            }
        }
        
        // Mark as completed
        {
            let mut progress = self.progress.write().await;
            progress.status = SyncStatus::Completed;
            progress.current_file = None;
        }
        
        Ok(())
    }

    /// Get cloud folder structure for browsing
    pub async fn list_cloud_folders(&self) -> Result<Vec<CloudFolder>, SyncError> {
        let folders = self.s3_client
            .list_folders("")
            .await
            .map_err(|e| SyncError::S3Error(e.to_string()))?;
        
        let mut result = Vec::new();
        for folder in folders {
            // Get total size of folder
            let objects = self.s3_client
                .list_objects(&folder)
                .await
                .map_err(|e| SyncError::S3Error(e.to_string()))?;
            
            let total_size: u64 = objects.iter().map(|o| o.size).sum();
            let file_count = objects.len();
            
            result.push(CloudFolder {
                name: folder.trim_end_matches('/').to_string(),
                path: folder,
                total_size,
                file_count,
            });
        }
        
        Ok(result)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudFolder {
    pub name: String,
    pub path: String,
    pub total_size: u64,
    pub file_count: usize,
}

