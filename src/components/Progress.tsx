'use client';

import { motion } from 'framer-motion';
import type { SyncProgress } from '@/lib/types';
import { formatBytes, formatSpeed, formatEta, getStatusText } from '@/lib/tauri';

interface ProgressProps {
  progress: SyncProgress;
  onPause: () => void;
  onResume: () => void;
  onCancel: () => void;
}

export default function Progress({ progress, onPause, onResume, onCancel }: ProgressProps) {
  const isPaused = progress.status === 'Paused';
  const isSyncing = progress.status === 'Syncing' || progress.status === 'Scanning';
  const isCompleted = progress.status === 'Completed';
  const hasError = typeof progress.status === 'object' && 'Error' in progress.status;

  const percentage = progress.total_bytes > 0
    ? Math.round((progress.transferred_bytes / progress.total_bytes) * 100)
    : 0;

  const fileProgress = progress.total_files > 0
    ? `${progress.completed_files} / ${progress.total_files} files`
    : '';

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      className={`glass p-6 ${isSyncing && !isPaused ? 'syncing' : ''}`}
    >
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          {/* Status Icon */}
          <div className={`w-10 h-10 rounded-xl flex items-center justify-center ${
            isCompleted ? 'bg-emerald-500/20' :
            hasError ? 'bg-red-500/20' :
            isPaused ? 'bg-amber-500/20' :
            'bg-blue-500/20'
          }`}>
            {isCompleted ? (
              <svg className="w-5 h-5 text-emerald-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
              </svg>
            ) : hasError ? (
              <svg className="w-5 h-5 text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            ) : isPaused ? (
              <svg className="w-5 h-5 text-amber-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 9v6m4-6v6" />
              </svg>
            ) : (
              <svg className="w-5 h-5 text-blue-400 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
              </svg>
            )}
          </div>
          
          <div>
            <h3 className="font-medium text-white">
              {progress.direction === 'LocalToCloud' ? 'Uploading' : 'Downloading'}
            </h3>
            <p className="text-sm text-slate-400">
              {getStatusText(progress.status)}
            </p>
          </div>
        </div>

        {/* Percentage */}
        <div className="text-right">
          <div className="text-2xl font-semibold text-white">
            {percentage}%
          </div>
          <div className="text-sm text-slate-400">
            {fileProgress}
          </div>
        </div>
      </div>

      {/* Progress Bar */}
      <div className="progress-bar mb-4">
        <motion.div
          className="progress-fill"
          initial={{ width: 0 }}
          animate={{ width: `${percentage}%` }}
          transition={{ duration: 0.3 }}
        />
      </div>

      {/* Stats Row */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <div className="glass-subtle p-3 text-center">
          <div className="text-sm text-slate-400 mb-1">Transferred</div>
          <div className="font-medium text-white">
            {formatBytes(progress.transferred_bytes)}
          </div>
        </div>
        <div className="glass-subtle p-3 text-center">
          <div className="text-sm text-slate-400 mb-1">Speed</div>
          <div className="font-medium text-white">
            {formatSpeed(progress.bytes_per_second)}
          </div>
        </div>
        <div className="glass-subtle p-3 text-center">
          <div className="text-sm text-slate-400 mb-1">ETA</div>
          <div className="font-medium text-white">
            {formatEta(progress.eta_seconds)}
          </div>
        </div>
      </div>

      {/* Current File */}
      {progress.current_file && (
        <div className="mb-6">
          <div className="text-xs text-slate-500 mb-1">Current file:</div>
          <div className="text-sm text-slate-300 truncate font-mono bg-black/20 px-3 py-2 rounded-lg">
            {progress.current_file}
          </div>
        </div>
      )}

      {/* Control Buttons */}
      <div className="flex gap-3">
        {!isCompleted && !hasError && (
          <>
            {isPaused ? (
              <button onClick={onResume} className="btn-primary flex-1 flex items-center justify-center gap-2">
                <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M8 5v14l11-7z" />
                </svg>
                Resume
              </button>
            ) : (
              <button onClick={onPause} className="btn-secondary flex-1 flex items-center justify-center gap-2">
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 9v6m4-6v6" />
                </svg>
                Pause
              </button>
            )}
            <button onClick={onCancel} className="btn-danger flex items-center justify-center gap-2 px-6">
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
              Cancel
            </button>
          </>
        )}
        {(isCompleted || hasError) && (
          <button onClick={onCancel} className="btn-secondary flex-1">
            Close
          </button>
        )}
      </div>
    </motion.div>
  );
}

