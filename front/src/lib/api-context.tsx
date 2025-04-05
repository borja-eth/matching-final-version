'use client';

import React, { createContext, useState, useContext, useEffect, ReactNode } from 'react';
import { ApiClient, Instrument, Depth, Trade } from './api';
import { toast } from "sonner";

interface ApiContextType {
  client: ApiClient;
  instruments: Instrument[];
  selectedInstrument: string | null;
  selectInstrument: (id: string) => void;
  refreshInstruments: () => Promise<Instrument[]>;
  orderbook: Depth | null;
  trades: Trade[];
  refreshOrderbook: () => Promise<void>;
  refreshTrades: () => Promise<void>;
  createDefaultInstrument: () => Promise<void>;
  isApiConnected: boolean;
}

const ApiContext = createContext<ApiContextType | undefined>(undefined);

interface ApiProviderProps {
  children: ReactNode;
  baseUrl?: string;
}

export function ApiProvider({ children, baseUrl = 'http://127.0.0.1:3001' }: ApiProviderProps) {
  const [client] = useState<ApiClient>(() => new ApiClient(baseUrl));
  const [instruments, setInstruments] = useState<Instrument[]>([]);
  const [selectedInstrument, setSelectedInstrument] = useState<string | null>(null);
  const [orderbook, setOrderbook] = useState<Depth | null>(null);
  const [trades, setTrades] = useState<Trade[]>([]);
  const [isApiConnected, setIsApiConnected] = useState(true);
  
  const refreshInstruments = async (): Promise<Instrument[]> => {
    try {
      const fetchedInstruments = await client.listInstruments();
      setInstruments(fetchedInstruments);
      
      // If no instrument is selected and we have instruments, select the first one
      if (!selectedInstrument && fetchedInstruments.length > 0) {
        setSelectedInstrument(fetchedInstruments[0].id);
      }
      
      return fetchedInstruments;
    } catch (error) {
      console.error('Failed to fetch instruments:', error);
      toast("Error", {
        description: "Failed to load instruments - API may be down",
        className: "destructive"
      });
      
      // Return an empty array instead of throwing
      return [];
    }
  };
  
  const createDefaultInstrument = async () => {
    try {
      // Create a default BTC/USD instrument
      const instrument = await client.createInstrument({
        name: "BTC/USD",
        base_currency: "BTC",
        quote_currency: "USD"
      });
      
      toast("Success", {
        description: `Created instrument: ${instrument.name}`,
      });
      
      // Refresh the instrument list
      await refreshInstruments();
      
    } catch (error) {
      console.error('Failed to create instrument:', error);
      toast("Error", {
        description: "Failed to create default instrument",
        className: "destructive"
      });
    }
  };
  
  const refreshOrderbook = async () => {
    if (!selectedInstrument) return;
    
    try {
      const data = await client.getOrderbook(selectedInstrument);
      setOrderbook(data);
    } catch (error) {
      console.error('Failed to fetch orderbook:', error);
    }
  };
  
  const refreshTrades = async () => {
    if (!selectedInstrument) return;
    
    try {
      const data = await client.getTrades(selectedInstrument);
      setTrades(data);
    } catch (error) {
      console.error('Failed to fetch trades:', error);
    }
  };
  
  // Load instruments on mount
  useEffect(() => {
    refreshInstruments();
  }, []);
  
  // Load orderbook and trades when instrument changes
  useEffect(() => {
    if (selectedInstrument) {
      refreshOrderbook();
      refreshTrades();
    }
  }, [selectedInstrument]);
  
  // Set up periodic refresh
  useEffect(() => {
    if (!selectedInstrument) return;
    
    const orderbookInterval = setInterval(refreshOrderbook, 5000);
    const tradesInterval = setInterval(refreshTrades, 10000);
    
    return () => {
      clearInterval(orderbookInterval);
      clearInterval(tradesInterval);
    };
  }, [selectedInstrument]);
  
  // When we load and find no instruments, create a default one
  useEffect(() => {
    const checkAndCreateDefaultInstrument = async () => {
      const instruments = await refreshInstruments();
      if (instruments.length === 0) {
        await createDefaultInstrument();
      }
    };
    
    checkAndCreateDefaultInstrument();
  }, []);

  useEffect(() => {
    // This is a separate check just to see if API is available
    const checkApiConnection = async () => {
      try {
        const isHealthy = await client.healthCheck();
        if (isHealthy && !isApiConnected) {
          console.log("API connected successfully");
          // If connection is restored, disable mock mode
          client.setMockMode(false);
        }
        setIsApiConnected(isHealthy);
      } catch (error) {
        console.error('API connection failed:', error);
        setIsApiConnected(false);
      }
    };
    
    // Initial check
    checkApiConnection();
    
    // Setup periodic checking
    const interval = setInterval(checkApiConnection, 10000); // Check every 10 seconds
    
    return () => clearInterval(interval);
  }, [client, isApiConnected]);
  
  return (
    <ApiContext.Provider
      value={{
        client,
        instruments,
        selectedInstrument,
        selectInstrument: setSelectedInstrument,
        refreshInstruments,
        orderbook,
        trades,
        refreshOrderbook,
        refreshTrades,
        createDefaultInstrument,
        isApiConnected,
      }}
    >
      {children}
    </ApiContext.Provider>
  );
}

export function useApi() {
  const context = useContext(ApiContext);
  
  if (context === undefined) {
    throw new Error('useApi must be used within an ApiProvider');
  }
  
  return context;
} 