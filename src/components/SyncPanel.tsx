'use client';

import { useState, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useAppStore } from '@/lib/store';
import {
  logout,
  selectFolders,
  selectFolder,
  startUpload,
  startDownload,
  pauseSync,
  resumeSync,
  cancelSync,
  getSyncProgress,
  listCloudFolders,
  deleteAllFiles,
  checkCredentialsStatus,
  formatBytes,
} from '@/lib/tauri';
import Progress from './Progress';
import type { SyncProgress, CloudFolder, CredentialsStatus } from '@/lib/types';

type SyncMode = 'upload' | 'download' | null;

export default function SyncPanel() {
  const { 
    user, 
    selectedSourceFolders,
    selectedTargetFolder,
    selectedCloudFolder,
    cloudFolders,
    addSourceFolder,
    removeSourceFolder,
    clearSourceFolders,
    setTargetFolder,
    setSelectedCloudFolder,
    setCloudFolders,
    setScreen,
    setUser,
  } = useAppStore();

  const [mode, setMode] = useState<SyncMode>(null);
  const [progress, setProgress] = useState<SyncProgress | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isSyncing, setIsSyncing] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteResult, setDeleteResult] = useState<{ success: boolean; count?: number; error?: string } | null>(null);
  const [credentialsStatus, setCredentialsStatus] = useState<CredentialsStatus | null>(null);

  // Poll for progress during sync
  useEffect(() => {
    let interval: NodeJS.Timeout;
    
    if (isSyncing) {
      interval = setInterval(async () => {
        try {
          const p = await getSyncProgress();
          setProgress(p);
          
          if (p.status === 'Completed' || (typeof p.status === 'object' && 'Error' in p.status)) {
            setIsSyncing(false);
          }
        } catch (err) {
          console.error('Failed to get progress:', err);
        }
      }, 500);
    }

    return () => clearInterval(interval);
  }, [isSyncing]);

  // Check credentials status on mount
  useEffect(() => {
    const checkStatus = async () => {
      try {
        const status = await checkCredentialsStatus();
        setCredentialsStatus(status);
      } catch (err) {
        console.error('Failed to check credentials status:', err);
      }
    };
    checkStatus();
  }, []);

  // Load cloud folders when download mode is selected
  const loadCloudFolders = useCallback(async () => {
    setIsLoading(true);
    try {
      const folders = await listCloudFolders();
      setCloudFolders(folders);
    } catch (err) {
      console.error('Failed to load cloud folders:', err);
    } finally {
      setIsLoading(false);
    }
  }, [setCloudFolders]);

  useEffect(() => {
    if (mode === 'download') {
      loadCloudFolders();
    }
  }, [mode, loadCloudFolders]);

  const handleSelectSourceFolders = async () => {
    const folders = await selectFolders();
    folders.forEach(addSourceFolder);
  };

  const handleSelectTargetFolder = async () => {
    const folder = await selectFolder();
    if (folder) {
      setTargetFolder(folder);
    }
  };

  const handleStartUpload = async () => {
    if (selectedSourceFolders.length === 0) return;
    
    setIsSyncing(true);
    try {
      await startUpload(selectedSourceFolders);
    } catch (err) {
      console.error('Upload failed:', err);
      setIsSyncing(false);
    }
  };

  const handleStartDownload = async () => {
    if (!selectedCloudFolder || !selectedTargetFolder) return;
    
    setIsSyncing(true);
    try {
      await startDownload(selectedCloudFolder, selectedTargetFolder);
    } catch (err) {
      console.error('Download failed:', err);
      setIsSyncing(false);
    }
  };

  const handlePause = async () => {
    await pauseSync();
  };

  const handleResume = async () => {
    await resumeSync();
  };

  const handleCancel = async () => {
    await cancelSync();
    setIsSyncing(false);
    setProgress(null);
  };

  const handleLogout = async () => {
    await logout();
    setUser(null);
    setScreen('key-entry');
  };

  const handleBack = () => {
    setMode(null);
    clearSourceFolders();
    setTargetFolder(null);
    setSelectedCloudFolder(null);
    setProgress(null);
  };

  const handleDeleteAll = async () => {
    setIsDeleting(true);
    setDeleteResult(null);
    try {
      const count = await deleteAllFiles();
      setDeleteResult({ success: true, count });
      setShowDeleteConfirm(false);
    } catch (err) {
      setDeleteResult({ success: false, error: err instanceof Error ? err.message : 'Delete failed' });
    } finally {
      setIsDeleting(false);
    }
  };

  // Show progress panel when syncing
  if (progress && isSyncing) {
    return (
      <div className="min-h-screen flex items-center justify-center p-8">
        <div className="w-full max-w-lg">
          <Progress
            progress={progress}
            onPause={handlePause}
            onResume={handleResume}
            onCancel={handleCancel}
          />
        </div>
      </div>
    );
  }

  // Mode selection screen
  if (!mode) {
    return (
      <div className="min-h-screen flex flex-col p-6 md:p-8">
        {/* Header */}
        <motion.header
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          className="flex items-center justify-between mb-6"
        >
          <div className="flex items-center gap-4">
            <div className="w-12 h-12 rounded-2xl icon-cyan flex items-center justify-center">
              <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none">
                <circle cx="8" cy="14" r="5" stroke="#3DBDB8" strokeWidth="2.5" fill="none"/>
                <circle cx="16" cy="10" r="5" stroke="#F5B841" strokeWidth="2.5" fill="none"/>
                <circle cx="8" cy="14" r="1.5" fill="#3DBDB8"/>
                <circle cx="16" cy="10" r="1.5" fill="#F5B841"/>
              </svg>
            </div>
            <div>
              <h1 className="text-xl font-bold text-white tracking-tight">Sync2Bucket</h1>
              <p className="text-sm text-slate-400">Welcome back, <span className="text-[#3DBDB8]">{user?.name}</span></p>
            </div>
          </div>
          
          <button onClick={handleLogout} className="btn-secondary text-sm px-4 py-2 rounded-xl">
            Sign Out
          </button>
        </motion.header>

        {/* Credentials Warning Banner */}
        {credentialsStatus?.warning && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            className={`mb-6 p-4 rounded-2xl flex items-center gap-3 ${
              !credentialsStatus.valid 
                ? 'bg-red-500/10 border border-red-500/20' 
                : credentialsStatus.days_remaining <= 30 
                  ? 'bg-[#F5B841]/10 border border-[#F5B841]/20'
                  : 'bg-[#3DBDB8]/10 border border-[#3DBDB8]/20'
            }`}
          >
            <div className={`w-10 h-10 rounded-xl flex items-center justify-center flex-shrink-0 ${
              !credentialsStatus.valid 
                ? 'bg-red-500/20' 
                : credentialsStatus.days_remaining <= 30 
                  ? 'bg-[#F5B841]/20'
                  : 'bg-[#3DBDB8]/20'
            }`}>
              {!credentialsStatus.valid ? (
                <svg className="w-5 h-5 text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                </svg>
              ) : (
                <svg className="w-5 h-5 text-[#F5B841]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              )}
            </div>
            <div className="flex-1">
              <p className={`text-sm font-medium ${
                !credentialsStatus.valid ? 'text-red-300' : 'text-[#F5B841]'
              }`}>
                {credentialsStatus.warning}
              </p>
              <p className="text-xs text-slate-500 mt-1">
                Expiry date: {credentialsStatus.expiry_date}
              </p>
            </div>
          </motion.div>
        )}

        {/* Mode Selection */}
        <div className="flex-1 flex items-center justify-center py-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-5 w-full max-w-2xl">
            {/* Upload Option */}
            <motion.button
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ delay: 0.1, duration: 0.3 }}
              onClick={() => setMode('upload')}
              className="glass action-card p-7 text-left"
              style={{ borderColor: 'rgba(61, 189, 184, 0.15)' }}
            >
              <div className="w-14 h-14 rounded-2xl icon-cyan flex items-center justify-center mb-5">
                <svg className="w-7 h-7 text-[#3DBDB8]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                </svg>
              </div>
              <h2 className="text-lg font-semibold text-white mb-1.5">Upload to Cloud</h2>
              <p className="text-slate-400 text-sm leading-relaxed">
                Sync folders from your Mac to the cloud
              </p>
            </motion.button>

            {/* Download Option */}
            <motion.button
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ delay: 0.15, duration: 0.3 }}
              onClick={() => setMode('download')}
              className="glass action-card p-7 text-left"
              style={{ borderColor: 'rgba(245, 184, 65, 0.15)' }}
            >
              <div className="w-14 h-14 rounded-2xl icon-golden flex items-center justify-center mb-5">
                <svg className="w-7 h-7 text-[#F5B841]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10" />
                </svg>
              </div>
              <h2 className="text-lg font-semibold text-white mb-1.5">Download from Cloud</h2>
              <p className="text-slate-400 text-sm leading-relaxed">
                Restore folders from the cloud to your Mac
              </p>
            </motion.button>
          </div>

          {/* Delete All Button */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.4 }}
            className="mt-10 text-center"
          >
            <button
              onClick={() => setShowDeleteConfirm(true)}
              className="text-slate-500 hover:text-red-400 text-xs flex items-center gap-1.5 mx-auto transition-colors duration-200"
            >
              <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
              </svg>
              Delete All Cloud Files
            </button>

            {/* Delete Result Message */}
            {deleteResult && (
              <motion.div
                initial={{ opacity: 0, y: -10 }}
                animate={{ opacity: 1, y: 0 }}
                className={`mt-3 text-sm ${deleteResult.success ? 'text-[#3DBDB8]' : 'text-red-400'}`}
              >
                {deleteResult.success 
                  ? `Successfully deleted ${deleteResult.count} file(s)`
                  : `Error: ${deleteResult.error}`
                }
              </motion.div>
            )}
          </motion.div>
        </div>

        {/* Delete Confirmation Modal */}
        <AnimatePresence>
          {showDeleteConfirm && (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="fixed inset-0 bg-black/70 backdrop-blur-md flex items-center justify-center p-4 z-50"
              onClick={() => !isDeleting && setShowDeleteConfirm(false)}
            >
              <motion.div
                initial={{ scale: 0.95, opacity: 0 }}
                animate={{ scale: 1, opacity: 1 }}
                exit={{ scale: 0.95, opacity: 0 }}
                onClick={(e) => e.stopPropagation()}
                className="glass p-8 max-w-sm w-full rounded-3xl"
              >
                <div className="w-16 h-16 rounded-2xl bg-red-500/15 flex items-center justify-center mx-auto mb-5">
                  <svg className="w-8 h-8 text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </div>
                
                <h3 className="text-xl font-semibold text-white text-center mb-2">
                  Delete All Files?
                </h3>
                <p className="text-slate-400 text-center mb-6">
                  This will permanently delete all files from your cloud storage. This action cannot be undone.
                </p>

                <div className="flex gap-3">
                  <button
                    onClick={() => setShowDeleteConfirm(false)}
                    disabled={isDeleting}
                    className="btn-secondary flex-1"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={handleDeleteAll}
                    disabled={isDeleting}
                    className="btn-danger flex-1 flex items-center justify-center gap-2"
                  >
                    {isDeleting ? (
                      <>
                        <svg className="w-4 h-4 animate-spin" viewBox="0 0 24 24">
                          <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
                          <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                        </svg>
                        Deleting...
                      </>
                    ) : (
                      'Delete All'
                    )}
                  </button>
                </div>
              </motion.div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    );
  }

  // Upload mode
  if (mode === 'upload') {
    return (
      <div className="min-h-screen flex flex-col p-8">
        <motion.header
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          className="flex items-center gap-4 mb-8"
        >
          <button onClick={handleBack} className="w-10 h-10 rounded-xl glass flex items-center justify-center hover:bg-white/10 transition">
            <svg className="w-5 h-5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
            </svg>
          </button>
          <div>
            <h1 className="text-xl font-semibold text-white">Upload to Cloud</h1>
            <p className="text-sm text-slate-400">Select folders to sync</p>
          </div>
        </motion.header>

        <div className="flex-1 flex flex-col gap-6 max-w-2xl mx-auto w-full">
          {/* Source Folders */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="glass p-6"
          >
            <div className="flex items-center justify-between mb-4">
              <h2 className="font-medium text-white">Source Folders</h2>
              <button onClick={handleSelectSourceFolders} className="btn-secondary text-sm py-2">
                + Add Folders
              </button>
            </div>

            {selectedSourceFolders.length === 0 ? (
              <div className="text-center py-8 text-slate-500">
                <svg className="w-12 h-12 mx-auto mb-3 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                </svg>
                <p>No folders selected</p>
                <p className="text-sm mt-1">Click &quot;Add Folders&quot; to select folders to upload</p>
              </div>
            ) : (
              <div className="space-y-2 max-h-64 overflow-y-auto">
                <AnimatePresence>
                  {selectedSourceFolders.map((folder) => (
                    <motion.div
                      key={folder}
                      initial={{ opacity: 0, x: -20 }}
                      animate={{ opacity: 1, x: 0 }}
                      exit={{ opacity: 0, x: 20 }}
                      className="flex items-center justify-between p-3 glass-subtle folder-item"
                    >
                      <div className="flex items-center gap-3 min-w-0">
                        <svg className="w-5 h-5 text-blue-400 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                        </svg>
                        <span className="text-sm text-white truncate">{folder}</span>
                      </div>
                      <button
                        onClick={() => removeSourceFolder(folder)}
                        className="p-1 hover:bg-red-500/20 rounded-lg transition"
                      >
                        <svg className="w-4 h-4 text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                        </svg>
                      </button>
                    </motion.div>
                  ))}
                </AnimatePresence>
              </div>
            )}
          </motion.div>

          {/* Start Button */}
          <motion.button
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.2 }}
            onClick={handleStartUpload}
            disabled={selectedSourceFolders.length === 0}
            className="btn-primary py-4 text-lg"
          >
            Start Upload
          </motion.button>
        </div>
      </div>
    );
  }

  // Download mode
  if (mode === 'download') {
    return (
      <div className="min-h-screen flex flex-col p-8">
        <motion.header
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          className="flex items-center gap-4 mb-8"
        >
          <button onClick={handleBack} className="w-10 h-10 rounded-xl glass flex items-center justify-center hover:bg-white/10 transition">
            <svg className="w-5 h-5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
            </svg>
          </button>
          <div>
            <h1 className="text-xl font-semibold text-white">Download from Cloud</h1>
            <p className="text-sm text-slate-400">Select a folder to restore</p>
          </div>
        </motion.header>

        <div className="flex-1 flex flex-col gap-6 max-w-2xl mx-auto w-full">
          {/* Cloud Folders */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="glass p-6"
          >
            <div className="flex items-center justify-between mb-4">
              <h2 className="font-medium text-white">Cloud Folders</h2>
              <button onClick={loadCloudFolders} className="btn-secondary text-sm py-2">
                Refresh
              </button>
            </div>

            {isLoading ? (
              <div className="text-center py-8">
                <svg className="w-8 h-8 mx-auto animate-spin text-blue-400" viewBox="0 0 24 24">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                </svg>
                <p className="text-slate-400 mt-3">Loading folders...</p>
              </div>
            ) : cloudFolders.length === 0 ? (
              <div className="text-center py-8 text-slate-500">
                <svg className="w-12 h-12 mx-auto mb-3 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1} d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z" />
                </svg>
                <p>No folders in cloud</p>
                <p className="text-sm mt-1">Upload some folders first</p>
              </div>
            ) : (
              <div className="space-y-2 max-h-64 overflow-y-auto">
                {cloudFolders.map((folder) => (
                  <button
                    key={folder.path}
                    onClick={() => setSelectedCloudFolder(folder.path)}
                    className={`w-full flex items-center justify-between p-3 glass-subtle folder-item text-left ${
                      selectedCloudFolder === folder.path ? 'border-blue-500 bg-blue-500/10' : ''
                    }`}
                  >
                    <div className="flex items-center gap-3">
                      <svg className="w-5 h-5 text-purple-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                      </svg>
                      <div>
                        <span className="text-sm text-white">{folder.name}</span>
                        <span className="text-xs text-slate-500 ml-2">
                          {folder.file_count} files • {formatBytes(folder.total_size)}
                        </span>
                      </div>
                    </div>
                    {selectedCloudFolder === folder.path && (
                      <svg className="w-5 h-5 text-blue-400" fill="currentColor" viewBox="0 0 20 20">
                        <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                      </svg>
                    )}
                  </button>
                ))}
              </div>
            )}
          </motion.div>

          {/* Target Folder */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 }}
            className="glass p-6"
          >
            <h2 className="font-medium text-white mb-4">Download Location</h2>
            
            {selectedTargetFolder ? (
              <div className="flex items-center justify-between p-3 glass-subtle">
                <div className="flex items-center gap-3 min-w-0">
                  <svg className="w-5 h-5 text-emerald-400 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                  </svg>
                  <span className="text-sm text-white truncate">{selectedTargetFolder}</span>
                </div>
                <button onClick={handleSelectTargetFolder} className="text-blue-400 text-sm hover:underline">
                  Change
                </button>
              </div>
            ) : (
              <button
                onClick={handleSelectTargetFolder}
                className="w-full p-4 border-2 border-dashed border-slate-600 rounded-xl text-center hover:border-slate-500 transition"
              >
                <svg className="w-8 h-8 mx-auto mb-2 text-slate-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                </svg>
                <span className="text-slate-400">Select download location</span>
              </button>
            )}
          </motion.div>

          {/* Start Button */}
          <motion.button
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.2 }}
            onClick={handleStartDownload}
            disabled={!selectedCloudFolder || !selectedTargetFolder}
            className="btn-primary py-4 text-lg"
          >
            Start Download
          </motion.button>
        </div>
      </div>
    );
  }

  return null;
}

