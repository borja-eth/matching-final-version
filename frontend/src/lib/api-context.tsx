'use client';

import React, { createContext, useContext, useState, useEffect } from 'react';
import { matchingEngineApi, Instrument, Depth, Trade, Order } from './api-client';
import { toast } from 'sonner';

interface ApiContextType {
  isConnected: boolean;
  instruments: Instrument[];
  selectedInstrument: string | null;
  selectInstrument: (id: string) => void;
  orderbook: Depth | null;
  trades: Trade[];
  orders: Order[];
  refreshInstruments: () => Promise<void>;
  refreshOrderbook: () => Promise<void>;
  refreshTrades: () => Promise<void>;
  createInstrument: (name: string, base: string, quote: string) => Promise<Instrument | null>;
  placeOrder: (order: any) => Promise<Order | null>;
  cancelOrder: (orderId: string) => Promise<boolean>;
}

const ApiContext = createContext<ApiContextType | undefined>(undefined);

export function ApiProvider({ children }: { children: React.ReactNode }) {
  const [isConnected, setIsConnected] = useState(false);
  const [instruments, setInstruments] = useState<Instrument[]>([]);
  const [selectedInstrument, setSelectedInstrument] = useState<string | null>(null);
  const [orderbook, setOrderbook] = useState<Depth | null>(null);
  const [trades, setTrades] = useState<Trade[]>([]);
  const [orders, setOrders] = useState<Order[]>([]);

  // Check API connection on mount
  useEffect(() => {
    const checkConnection = async () => {
      const connected = await matchingEngineApi.healthCheck();
      setIsConnected(connected);
      
      if (connected) {
        await refreshInstruments();
      } else {
        toast.error('Could not connect to the matching engine');
      }
    };
    
    checkConnection();
    
    const interval = setInterval(checkConnection, 10000);
    return () => clearInterval(interval);
  }, []);

  // Load orderbook and trades when instrument changes
  useEffect(() => {
    if (selectedInstrument) {
      refreshOrderbook();
      refreshTrades();
      
      // Set up polling intervals
      const orderbookInterval = setInterval(refreshOrderbook, 1000);
      const tradesInterval = setInterval(refreshTrades, 5000);
      
      return () => {
        clearInterval(orderbookInterval);
        clearInterval(tradesInterval);
      };
    }
  }, [selectedInstrument]);

  // Get all instruments
  const refreshInstruments = async () => {
    const fetchedInstruments = await matchingEngineApi.getInstruments();
    
    // Create a default instrument if none exist
    if (fetchedInstruments.length === 0) {
      const defaultInstrument = await matchingEngineApi.createInstrument(
        "BTC/USD", 
        "BTC", 
        "USD"
      );
      
      if (defaultInstrument) {
        setInstruments([defaultInstrument]);
        setSelectedInstrument(defaultInstrument.id);
        toast.success('Created default BTC/USD instrument');
        return;
      } else {
        toast.error('Failed to create default instrument');
      }
    }
    
    setInstruments(fetchedInstruments);
    
    // Select the first instrument if none is selected
    if (fetchedInstruments.length > 0 && !selectedInstrument) {
      setSelectedInstrument(fetchedInstruments[0].id);
      
      // Also refresh trades data for this instrument
      if (fetchedInstruments[0].id) {
        // Wait a moment to ensure the instrument is selected
        setTimeout(() => {
          refreshTrades();
        }, 500);
      }
    }
  };

  // Refresh orderbook data
  const refreshOrderbook = async () => {
    if (!selectedInstrument) return;
    
    const depth = await matchingEngineApi.getDepth(selectedInstrument, 20);
    if (depth) {
      setOrderbook(depth);
    }
    
    // Also refresh trades data when we refresh the orderbook
    refreshTrades();
  };

  // Refresh trades data
  const refreshTrades = async () => {
    if (!selectedInstrument) return;
    
    const fetchedTrades = await matchingEngineApi.getTrades(selectedInstrument);
    setTrades(fetchedTrades);
  };

  // Create a new instrument
  const createInstrument = async (name: string, base: string, quote: string) => {
    const instrument = await matchingEngineApi.createInstrument(name, base, quote);
    if (instrument) {
      await refreshInstruments();
      toast.success(`Created instrument ${name}`);
    }
    return instrument;
  };

  // Place a new order
  const placeOrder = async (orderData: any) => {
    if (!selectedInstrument) {
      toast.error('No instrument selected');
      return null;
    }
    
    // Ensure instrument ID is set
    const order = {
      ...orderData,
      instrument_id: selectedInstrument,
    };
    
    const result = await matchingEngineApi.placeOrder(order);
    if (result) {
      toast.success(`Order placed: ${result.id}`);
      await refreshOrderbook();
      await refreshTrades();
    }
    return result;
  };

  // Cancel an order
  const cancelOrder = async (orderId: string) => {
    if (!selectedInstrument) {
      toast.error('No instrument selected');
      return false;
    }
    
    const result = await matchingEngineApi.cancelOrder(orderId, selectedInstrument);
    if (result) {
      toast.success(`Order cancelled: ${orderId}`);
      await refreshOrderbook();
      return true;
    }
    return false;
  };

  return (
    <ApiContext.Provider
      value={{
        isConnected,
        instruments,
        selectedInstrument,
        selectInstrument: setSelectedInstrument,
        orderbook,
        trades,
        orders,
        refreshInstruments,
        refreshOrderbook,
        refreshTrades,
        createInstrument,
        placeOrder,
        cancelOrder,
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