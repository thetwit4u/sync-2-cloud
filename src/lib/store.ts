import { create } from 'zustand';
import type { AppState, KeyPayload, SyncProgress, CloudFolder, AppScreen } from './types';

interface AppStore extends AppState {
  // Actions
  setScreen: (screen: AppScreen) => void;
  setUser: (user: KeyPayload | null) => void;
  setProgress: (progress: SyncProgress | null) => void;
  setCloudFolders: (folders: CloudFolder[]) => void;
  addSourceFolder: (path: string) => void;
  removeSourceFolder: (path: string) => void;
  clearSourceFolders: () => void;
  setTargetFolder: (path: string | null) => void;
  setSelectedCloudFolder: (path: string | null) => void;
  setError: (error: string | null) => void;
  reset: () => void;
}

const initialState: AppState = {
  screen: 'loading',
  user: null,
  progress: null,
  cloudFolders: [],
  selectedSourceFolders: [],
  selectedTargetFolder: null,
  selectedCloudFolder: null,
  error: null,
};

export const useAppStore = create<AppStore>((set) => ({
  ...initialState,
  
  setScreen: (screen) => set({ screen }),
  
  setUser: (user) => set({ user }),
  
  setProgress: (progress) => set({ progress }),
  
  setCloudFolders: (folders) => set({ cloudFolders: folders }),
  
  addSourceFolder: (path) => set((state) => ({
    selectedSourceFolders: state.selectedSourceFolders.includes(path)
      ? state.selectedSourceFolders
      : [...state.selectedSourceFolders, path],
  })),
  
  removeSourceFolder: (path) => set((state) => ({
    selectedSourceFolders: state.selectedSourceFolders.filter((p) => p !== path),
  })),
  
  clearSourceFolders: () => set({ selectedSourceFolders: [] }),
  
  setTargetFolder: (path) => set({ selectedTargetFolder: path }),
  
  setSelectedCloudFolder: (path) => set({ selectedCloudFolder: path }),
  
  setError: (error) => set({ error }),
  
  reset: () => set(initialState),
}));

