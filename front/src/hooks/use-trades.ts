'use client';

import { useState, useEffect } from 'react';
import { useApi } from '@/lib/api-context';
import { Trade } from '@/lib/api';

export function useTrades(limit = 20, updateInterval = 3000) {
  const { api, selectedInstrument } = useApi();
  const [trades, setTrades] = useState<Trade[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Only fetch trades if we have a selected instrument
    if (!selectedInstrument) {
      console.log("No instrument selected, skipping trades fetch");
      return;
    }

    console.log("Fetching trades for instrument: ", selectedInstrument);
    
    // Function to fetch trades
    const fetchTrades = async () => {
      try {
        setIsLoading(true);
        setError(null);
        const data = await api.getTrades(selectedInstrument, limit);
        console.log("Received trades data:", data);
        setTrades(data);
      } catch (err) {
        console.error("Error fetching trades:", err);
        setError(`Failed to load trades: ${err instanceof Error ? err.message : String(err)}`);
      } finally {
        setIsLoading(false);
      }
    };
    
    // Initial fetch
    fetchTrades();
    
    // Setup interval for regular updates
    const interval = setInterval(fetchTrades, updateInterval);
    
    // Cleanup on unmount or when params change
    return () => clearInterval(interval);
  }, [api, selectedInstrument, limit, updateInterval]);
  
  return { trades, isLoading, error };
} 