'use client';

import { useEffect } from 'react';
import { motion } from 'framer-motion';
import { useAppStore } from '@/lib/store';
import { checkStoredKey, getUserInfo, isTauri } from '@/lib/tauri';
import KeyEntry from '@/components/KeyEntry';
import SyncPanel from '@/components/SyncPanel';

export default function Home() {
  const { screen, setScreen, setUser } = useAppStore();

  useEffect(() => {
    const init = async () => {
      // Check if running in browser (development)
      if (!isTauri()) {
        console.log('Running in browser mode - showing key entry');
        setScreen('key-entry');
        return;
      }

      // Check for stored key
      const hasKey = await checkStoredKey();
      
      if (hasKey) {
        // Get user info
        const user = await getUserInfo();
        if (user) {
          setUser(user);
          setScreen('main');
          return;
        }
      }
      
      setScreen('key-entry');
    };

    init();
  }, [setScreen, setUser]);

  // Loading screen
  if (screen === 'loading') {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <motion.div
          initial={{ opacity: 0, scale: 0.9 }}
          animate={{ opacity: 1, scale: 1 }}
          className="text-center"
        >
          <div className="w-20 h-20 rounded-2xl glass mx-auto mb-6 flex items-center justify-center">
            <svg
              className="w-10 h-10 text-blue-400 animate-pulse"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z"
              />
            </svg>
          </div>
          <h1 className="text-2xl font-semibold text-white mb-2">Sync2Bucket</h1>
          <p className="text-slate-400">Loading...</p>
        </motion.div>
      </div>
    );
  }

  // Key entry screen
  if (screen === 'key-entry') {
    return <KeyEntry />;
  }

  // Main sync panel
  if (screen === 'main') {
    return <SyncPanel />;
  }

  return null;
}
