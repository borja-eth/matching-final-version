'use client';

import { useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { useApi } from '@/lib/api-context';
import { cn } from '@/lib/utils';

export default function OrderBook() {
  const { orderbook, refreshOrderbook, selectedInstrument } = useApi();
  
  useEffect(() => {
    if (selectedInstrument) {
      // Initial fetch
      refreshOrderbook();
      
      // Set up polling
      const interval = setInterval(refreshOrderbook, 2000);
      return () => clearInterval(interval);
    }
  }, [selectedInstrument, refreshOrderbook]);
  
  // Format price for display
  const formatPrice = (price: string) => {
    return parseFloat(price).toFixed(2);
  };
  
  // Format volume for display
  const formatVolume = (volume: string) => {
    return parseFloat(volume).toFixed(4);
  };
  
  // Calculate total volume
  const calculateTotal = (price: string, volume: string) => {
    return (parseFloat(price) * parseFloat(volume)).toFixed(2);
  };
  
  // Get the spread between highest bid and lowest ask
  const spread = orderbook && orderbook.bids.length > 0 && orderbook.asks.length > 0
    ? Math.abs(parseFloat(orderbook.asks[0].price) - parseFloat(orderbook.bids[0].price)).toFixed(2)
    : '0.00';
  
  const spreadPercent = orderbook && orderbook.bids.length > 0 && orderbook.asks.length > 0
    ? (Math.abs(parseFloat(orderbook.asks[0].price) - parseFloat(orderbook.bids[0].price)) / parseFloat(orderbook.asks[0].price) * 100).toFixed(2)
    : '0.00';
  
  return (
    <Card className="h-full">
      <CardHeader className="pb-3">
        <CardTitle className="text-lg font-medium">Order Book</CardTitle>
      </CardHeader>
      <CardContent className="px-0 text-sm">
        {!selectedInstrument ? (
          <div className="text-center text-muted-foreground py-10">
            Select an instrument to view order book
          </div>
        ) : !orderbook ? (
          <div className="text-center text-muted-foreground py-10">
            Loading order book...
          </div>
        ) : (
          <>
            <div className="flex justify-between px-4 text-xs text-muted-foreground mb-1">
              <span>Price</span>
              <span>Amount</span>
              <span>Total</span>
            </div>
            
            {/* Asks (sell orders) - reversed so highest price is at the bottom */}
            <div className="border-b border-border/20 pb-2">
              {orderbook.asks.slice().reverse().map((level, i) => (
                <div 
                  key={`ask-${i}`} 
                  className="flex justify-between px-4 py-0.5 hover:bg-muted/30"
                >
                  <span className="font-mono text-red-500">{formatPrice(level.price)}</span>
                  <span className="font-mono">{formatVolume(level.volume)}</span>
                  <span className="font-mono text-xs text-muted-foreground">
                    {calculateTotal(level.price, level.volume)}
                  </span>
                </div>
              ))}
              
              {orderbook.asks.length === 0 && (
                <div className="text-center text-xs text-muted-foreground py-2">
                  No asks
                </div>
              )}
            </div>
            
            {/* Spread */}
            <div className="flex justify-between items-center px-4 py-1 bg-muted/20 text-xs text-muted-foreground">
              <span>Spread:</span>
              <span>{spread} ({spreadPercent}%)</span>
            </div>
            
            {/* Bids (buy orders) */}
            <div className="pt-2">
              {orderbook.bids.map((level, i) => (
                <div 
                  key={`bid-${i}`} 
                  className="flex justify-between px-4 py-0.5 hover:bg-muted/30"
                >
                  <span className="font-mono text-green-500">{formatPrice(level.price)}</span>
                  <span className="font-mono">{formatVolume(level.volume)}</span>
                  <span className="font-mono text-xs text-muted-foreground">
                    {calculateTotal(level.price, level.volume)}
                  </span>
                </div>
              ))}
              
              {orderbook.bids.length === 0 && (
                <div className="text-center text-xs text-muted-foreground py-2">
                  No bids
                </div>
              )}
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
} 