'use client';

import { useEffect, useState } from 'react';
import { ApiClient } from '@/lib/api';

export default function Footer() {
  const [apiConnected, setApiConnected] = useState(false);
  
  useEffect(() => {
    const api = new ApiClient('http://127.0.0.1:3001');
    
    async function checkApiStatus() {
      try {
        const isHealthy = await api.healthCheck();
        setApiConnected(isHealthy);
      } catch (err) {
        setApiConnected(false);
      }
    }
    
    checkApiStatus();
    const interval = setInterval(checkApiStatus, 30000); // Check every 30 seconds
    
    return () => clearInterval(interval);
  }, []);
  
  return (
    <footer className="border-t border-border/40 py-4 px-6 bg-background/80">
      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <div className="flex items-center gap-2">
          <span>© {new Date().getFullYear()} Ultimate Matching Engine</span>
          <span className={`${apiConnected ? 'bg-green-500' : 'bg-red-500'} rounded-full text-[10px] px-1.5 py-0.5 text-white`}>
            API {apiConnected ? 'CONNECTED' : 'DISCONNECTED'}
          </span>
        </div>
        <div>
          {apiConnected && <span>Ready for trading</span>}
        </div>
      </div>
    </footer>
  );
} 