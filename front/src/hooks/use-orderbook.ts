'use client';

import { useState, useEffect } from 'react';
import { useApi } from '@/lib/api-context';
import { Depth } from '@/lib/api';

export function useOrderbook(updateInterval = 3000) {
  const { api, selectedInstrument } = useApi();
  const [depth, setDepth] = useState<Depth | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Only fetch orderbook if we have a selected instrument
    if (!selectedInstrument) {
      console.log("No instrument selected, skipping orderbook fetch");
      return;
    }

    console.log("Fetching orderbook for instrument: ", selectedInstrument);
    
    // Function to fetch the orderbook
    const fetchOrderbook = async () => {
      try {
        setIsLoading(true);
        setError(null);
        const data = await api.getOrderbook(selectedInstrument);
        console.log("Received orderbook data:", data);
        setDepth(data);
      } catch (err) {
        console.error("Error fetching orderbook:", err);
        setError(`Failed to load orderbook: ${err instanceof Error ? err.message : String(err)}`);
      } finally {
        setIsLoading(false);
      }
    };
    
    // Initial fetch
    fetchOrderbook();
    
    // Setup interval for regular updates
    const interval = setInterval(fetchOrderbook, updateInterval);
    
    // Cleanup on unmount or when selectedInstrument changes
    return () => clearInterval(interval);
  }, [api, selectedInstrument, updateInterval]);
  
  return { depth, isLoading, error };
} 