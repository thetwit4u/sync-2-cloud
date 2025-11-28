import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { ValidationResult, KeyPayload, SyncProgress, CloudFolder, CredentialsStatus } from './types';

// Check if running in Tauri environment
export const isTauri = () => {
  return typeof window !== 'undefined' && '__TAURI__' in window;
};

// Auth commands
export async function checkStoredKey(): Promise<boolean> {
  if (!isTauri()) return false;
  return invoke<boolean>('check_stored_key');
}

export async function validateKey(key: string): Promise<ValidationResult> {
  return invoke<ValidationResult>('validate_key', { key });
}

export async function getUserInfo(): Promise<KeyPayload | null> {
  return invoke<KeyPayload | null>('get_user_info');
}

export async function logout(): Promise<void> {
  return invoke<void>('logout');
}

// Sync commands
export async function startUpload(sourcePaths: string[]): Promise<void> {
  return invoke<void>('start_upload', { sourcePaths });
}

export async function startDownload(cloudFolder: string, targetPath: string): Promise<void> {
  return invoke<void>('start_download', { cloudFolder, targetPath });
}

export async function pauseSync(): Promise<void> {
  return invoke<void>('pause_sync');
}

export async function resumeSync(): Promise<void> {
  return invoke<void>('resume_sync');
}

export async function cancelSync(): Promise<void> {
  return invoke<void>('cancel_sync');
}

export async function getSyncProgress(): Promise<SyncProgress> {
  return invoke<SyncProgress>('get_sync_progress');
}

export async function listCloudFolders(): Promise<CloudFolder[]> {
  return invoke<CloudFolder[]>('list_cloud_folders');
}

export async function deleteAllFiles(): Promise<number> {
  return invoke<number>('delete_all_files');
}

export async function checkCredentialsStatus(): Promise<CredentialsStatus> {
  return invoke<CredentialsStatus>('check_credentials_status');
}

// Dialog helpers
export async function selectFolder(): Promise<string | null> {
  const result = await open({
    directory: true,
    multiple: false,
    title: 'Select Folder',
  });
  return result as string | null;
}

export async function selectFolders(): Promise<string[]> {
  const result = await open({
    directory: true,
    multiple: true,
    title: 'Select Folders to Sync',
  });
  if (Array.isArray(result)) {
    return result;
  }
  return result ? [result] : [];
}

// Utility functions
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

export function formatSpeed(bytesPerSecond: number): string {
  return `${formatBytes(bytesPerSecond)}/s`;
}

export function formatEta(seconds: number | null): string {
  if (seconds === null || seconds <= 0) return '--';
  
  if (seconds < 60) {
    return `${Math.round(seconds)}s`;
  } else if (seconds < 3600) {
    const mins = Math.floor(seconds / 60);
    const secs = Math.round(seconds % 60);
    return `${mins}m ${secs}s`;
  } else {
    const hours = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    return `${hours}h ${mins}m`;
  }
}

export function getStatusText(status: SyncProgress['status']): string {
  if (typeof status === 'string') {
    switch (status) {
      case 'Idle': return 'Ready';
      case 'Scanning': return 'Scanning files...';
      case 'Syncing': return 'Syncing...';
      case 'Paused': return 'Paused';
      case 'Completed': return 'Completed';
      default: return status;
    }
  }
  if ('Error' in status) {
    return `Error: ${status.Error}`;
  }
  return 'Unknown';
}

