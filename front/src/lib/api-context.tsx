'use client';

import React, { createContext, useState, useContext, useEffect, ReactNode } from 'react';
import { ApiClient, Instrument, Depth, Trade, PriceLevel } from './api';
import { toast } from "sonner";

// Mock data for development
const generateMockPriceLevel = (basePrice: number, index: number, isAsk: boolean): PriceLevel => ({
  price: (basePrice + (isAsk ? 1 : -1) * index * 0.00000100).toFixed(8),
  volume: (0.67 + Math.random() * 0.5).toFixed(8),
  order_count: 1
});

const MOCK_ORDERBOOK: Depth = {
  bids: Array.from({ length: 10 }, (_, i) => generateMockPriceLevel(0.00383141, i, false)),
  asks: Array.from({ length: 10 }, (_, i) => generateMockPriceLevel(0.00383141, i, true)),
  timestamp: new Date().toISOString(),
  instrument_id: 'MSTR/BTC'
};

const MOCK_INSTRUMENTS: Instrument[] = [{
  id: 'MSTR/BTC',
  name: 'MSTR/BTC',
  base_currency: 'MSTR',
  quote_currency: 'BTC'
}];

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
  const [isApiConnected, setIsApiConnected] = useState(false);
  const [useMockData, setUseMockData] = useState(false);
  
  const refreshInstruments = async (): Promise<Instrument[]> => {
    try {
      if (useMockData) {
        setInstruments(MOCK_INSTRUMENTS);
        return MOCK_INSTRUMENTS;
      }
      
      const fetchedInstruments = await client.listInstruments();
      setInstruments(fetchedInstruments);
      
      if (!selectedInstrument && fetchedInstruments.length > 0) {
        setSelectedInstrument(fetchedInstruments[0].id);
      }
      
      return fetchedInstruments;
    } catch (error) {
      console.error('Failed to fetch instruments:', error);
      if (!useMockData) {
        setUseMockData(true);
        setInstruments(MOCK_INSTRUMENTS);
        if (!selectedInstrument) {
          setSelectedInstrument(MOCK_INSTRUMENTS[0].id);
        }
      }
      return MOCK_INSTRUMENTS;
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
      if (useMockData) {
        // Update mock data slightly to simulate changes
        const mockData = {
          ...MOCK_ORDERBOOK,
          bids: MOCK_ORDERBOOK.bids.map(bid => ({
            ...bid,
            volume: (parseFloat(bid.volume) + (Math.random() * 0.1 - 0.05)).toFixed(8)
          })),
          asks: MOCK_ORDERBOOK.asks.map(ask => ({
            ...ask,
            volume: (parseFloat(ask.volume) + (Math.random() * 0.1 - 0.05)).toFixed(8)
          }))
        };
        setOrderbook(mockData);
        return;
      }

      const data = await client.getOrderbook(selectedInstrument);
      setOrderbook(data);
    } catch (error) {
      console.error('Failed to fetch orderbook:', error);
      if (!useMockData) {
        setUseMockData(true);
        setOrderbook(MOCK_ORDERBOOK);
      }
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
    const checkApiConnection = async () => {
      try {
        const isHealthy = await client.healthCheck();
        setIsApiConnected(isHealthy);
        setUseMockData(!isHealthy);
        
        if (isHealthy && useMockData) {
          console.log("API connected successfully");
          client.setMockMode(false);
          refreshInstruments();
        }
      } catch (error) {
        console.error('API connection failed:', error);
        setIsApiConnected(false);
        setUseMockData(true);
      }
    };
    
    checkApiConnection();
    const interval = setInterval(checkApiConnection, 10000);
    return () => clearInterval(interval);
  }, [client, useMockData]);
  
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