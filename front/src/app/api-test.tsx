'use client';

import { useEffect, useState } from 'react';
import { ApiClient, Instrument, Depth, Trade } from '@/lib/api';

export default function ApiTest() {
  const [instruments, setInstruments] = useState<Instrument[]>([]);
  const [orderbook, setOrderbook] = useState<Depth | null>(null);
  const [trades, setTrades] = useState<Trade[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [apiConnected, setApiConnected] = useState(false);
  
  const api = new ApiClient('http://127.0.0.1:3001');
  
  useEffect(() => {
    async function testApiConnection() {
      try {
        setLoading(true);
        setError(null);
        
        // Test health endpoint
        const isHealthy = await api.healthCheck();
        setApiConnected(isHealthy);
        
        if (isHealthy) {
          // Get instruments
          const instrumentsList = await api.listInstruments();
          setInstruments(instrumentsList);
          
          // If we have instruments, get orderbook and trades for the first one
          if (instrumentsList.length > 0) {
            const firstInstrument = instrumentsList[0];
            const depth = await api.getOrderbook(firstInstrument.id);
            setOrderbook(depth);
            
            const tradesList = await api.getTrades(firstInstrument.id);
            setTrades(tradesList);
          }
        }
      } catch (err) {
        setError(`Error connecting to API: ${err instanceof Error ? err.message : String(err)}`);
        console.error("API connection error:", err);
      } finally {
        setLoading(false);
      }
    }
    
    testApiConnection();
  }, []);
  
  return (
    <div className="border rounded-lg p-6 my-6">
      <h2 className="text-xl font-bold mb-4">API Connection Test</h2>
      
      {loading ? (
        <p>Testing API connection...</p>
      ) : error ? (
        <div className="text-red-600">
          <p>{error}</p>
        </div>
      ) : (
        <div className="space-y-4">
          <div className="bg-green-100 dark:bg-green-900 p-3 rounded">
            <p>API Status: {apiConnected ? '✅ Connected' : '❌ Disconnected'}</p>
          </div>
          
          <div>
            <h3 className="font-bold">Instruments ({instruments.length})</h3>
            <ul className="list-disc pl-6">
              {instruments.map(instrument => (
                <li key={instrument.id}>
                  {instrument.name} ({instrument.base_currency}/{instrument.quote_currency}) - ID: {instrument.id}
                </li>
              ))}
            </ul>
          </div>
          
          {orderbook && (
            <div>
              <h3 className="font-bold">Orderbook</h3>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <h4 className="font-semibold">Bids</h4>
                  {orderbook.bids.length === 0 ? (
                    <p className="italic">No bids</p>
                  ) : (
                    <ul>
                      {orderbook.bids.map((level, i) => (
                        <li key={i}>{level.price} - {level.volume}</li>
                      ))}
                    </ul>
                  )}
                </div>
                <div>
                  <h4 className="font-semibold">Asks</h4>
                  {orderbook.asks.length === 0 ? (
                    <p className="italic">No asks</p>
                  ) : (
                    <ul>
                      {orderbook.asks.map((level, i) => (
                        <li key={i}>{level.price} - {level.volume}</li>
                      ))}
                    </ul>
                  )}
                </div>
              </div>
            </div>
          )}
          
          <div>
            <h3 className="font-bold">Recent Trades ({trades.length})</h3>
            {trades.length === 0 ? (
              <p className="italic">No trades yet</p>
            ) : (
              <ul>
                {trades.map(trade => (
                  <li key={trade.id}>
                    {trade.price} - {trade.base_amount} - {new Date(trade.created_at).toLocaleTimeString()}
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      )}
    </div>
  );
} 