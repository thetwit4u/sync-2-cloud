'use client';

import { useState } from 'react';
import { motion } from 'framer-motion';
import { validateKey } from '@/lib/tauri';
import { useAppStore } from '@/lib/store';

export default function KeyEntry() {
  const [key, setKey] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { setScreen, setUser } = useAppStore();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!key.trim()) {
      setError('Please enter your sync key');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const result = await validateKey(key.trim());
      
      if (result.valid && result.user_name) {
        setUser({
          uid: '',
          name: result.user_name,
          created: Date.now() / 1000,
        });
        setScreen('main');
      } else {
        setError(result.error || 'Invalid key');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Validation failed');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center p-8">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.6 }}
        className="w-full max-w-md"
      >
        {/* Logo/Header */}
        <div className="text-center mb-10">
          <motion.div
            initial={{ scale: 0.8, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            transition={{ duration: 0.5, delay: 0.2 }}
            className="inline-flex items-center justify-center w-24 h-24 rounded-3xl glass mb-6"
          >
            <svg className="w-14 h-14" viewBox="0 0 48 48" fill="none">
              <circle cx="16" cy="28" r="10" stroke="#3DBDB8" strokeWidth="4" fill="none"/>
              <circle cx="32" cy="20" r="10" stroke="#F5B841" strokeWidth="4" fill="none"/>
              <circle cx="16" cy="28" r="3" fill="#3DBDB8"/>
              <circle cx="32" cy="20" r="3" fill="#F5B841"/>
            </svg>
          </motion.div>
          
          <motion.h1 
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.3 }}
            className="text-3xl font-bold text-white mb-3 tracking-tight"
          >
            Sync2Bucket
          </motion.h1>
          <motion.p 
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.4 }}
            className="text-slate-400"
          >
            Enter your sync key to get started
          </motion.p>
        </div>

        {/* Key Input Form */}
        <motion.form
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.4 }}
          onSubmit={handleSubmit}
          className="glass p-8 rounded-3xl space-y-6"
        >
          <div>
            <label 
              htmlFor="key" 
              className="block text-sm font-medium text-slate-300 mb-3"
            >
              Sync Key
            </label>
            <input
              id="key"
              type="text"
              value={key}
              onChange={(e) => setKey(e.target.value)}
              placeholder="EXAD-XXXXXXXXXXXXXXXXXXXXXXXX"
              className="input-glass font-mono text-sm tracking-wide"
              autoFocus
              autoComplete="off"
              spellCheck={false}
            />
            {error && (
              <motion.p
                initial={{ opacity: 0, y: -10 }}
                animate={{ opacity: 1, y: 0 }}
                className="mt-3 text-sm text-red-400 flex items-center gap-2"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                {error}
              </motion.p>
            )}
          </div>

          <button
            type="submit"
            disabled={isLoading}
            className="btn-primary w-full flex items-center justify-center gap-2 py-4 text-base"
          >
            {isLoading ? (
              <>
                <svg className="animate-spin h-5 w-5" viewBox="0 0 24 24">
                  <circle
                    className="opacity-25"
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    strokeWidth="4"
                    fill="none"
                  />
                  <path
                    className="opacity-75"
                    fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                  />
                </svg>
                Validating...
              </>
            ) : (
              <>
                <svg
                  className="w-5 h-5"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
                  />
                </svg>
                Activate
              </>
            )}
          </button>
        </motion.form>

        {/* Help text */}
        <motion.p
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.5, delay: 0.6 }}
          className="text-center text-sm text-slate-500 mt-8"
        >
          Don&apos;t have a key? Contact your administrator.
        </motion.p>
      </motion.div>
    </div>
  );
}
