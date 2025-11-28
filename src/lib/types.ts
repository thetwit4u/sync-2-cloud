export interface KeyPayload {
  uid: string;
  name: string;
  created: number;
}

export interface ValidationResult {
  valid: boolean;
  user_name: string | null;
  error: string | null;
}

export type SyncDirection = 'LocalToCloud' | 'CloudToLocal';

export type SyncStatus = 
  | 'Idle'
  | 'Scanning'
  | 'Syncing'
  | 'Paused'
  | 'Completed'
  | { Error: string };

export interface SyncProgress {
  status: SyncStatus;
  direction: SyncDirection | null;
  total_files: number;
  completed_files: number;
  total_bytes: number;
  transferred_bytes: number;
  current_file: string | null;
  bytes_per_second: number;
  eta_seconds: number | null;
}

export interface CloudFolder {
  name: string;
  path: string;
  total_size: number;
  file_count: number;
}

export interface CredentialsStatus {
  valid: boolean;
  days_remaining: number;
  expiry_date: string;
  warning: string | null;
}

export type AppScreen = 'loading' | 'key-entry' | 'main';

export interface AppState {
  screen: AppScreen;
  user: KeyPayload | null;
  progress: SyncProgress | null;
  cloudFolders: CloudFolder[];
  selectedSourceFolders: string[];
  selectedTargetFolder: string | null;
  selectedCloudFolder: string | null;
  error: string | null;
}

