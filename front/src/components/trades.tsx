'use client';

import { useEffect } from 'react';
import { useApi } from '@/lib/api-context';
import { cn } from '@/lib/utils';
import { ArrowDown, ArrowUp } from 'lucide-react';

export default function Trades() {
  const { trades, refreshTrades, selectedInstrument } = useApi();
  
  useEffect(() => {
    if (selectedInstrument) {
      // Initial fetch
      refreshTrades();
      
      // Set up polling
      const interval = setInterval(refreshTrades, 5000);
      return () => clearInterval(interval);
    }
  }, [selectedInstrument, refreshTrades]);
  
  // Helper to format a date
  const formatTime = (dateStr: string) => {
    const date = new Date(dateStr);
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  };
  
  // Helper to format a price
  const formatPrice = (price: string) => {
    return parseFloat(price).toFixed(2);
  };
  
  // Helper to determine trade side - this is an approximation since we don't have side directly
  const isBuyTrade = (index: number) => {
    if (index === 0) return true; // Default first trade to buy for demo
    if (!trades[index - 1]) return true;
    
    const prevPrice = parseFloat(trades[index - 1].price);
    const currentPrice = parseFloat(trades[index].price);
    
    return currentPrice >= prevPrice;
  };
  
  return (
    <div className="h-full flex flex-col">
      <div className="flex items-center justify-between px-4 py-2 border-b border-border bg-muted/5">
        <h2 className="text-sm font-medium">Market Trades</h2>
        <div className="flex gap-4 text-xs">
          <span className="text-green-500">Buy</span>
          <span className="text-red-500">Sell</span>
        </div>
      </div>

      <div className="flex-1 overflow-hidden">
        {!selectedInstrument ? (
          <div className="text-center text-muted-foreground py-10">
            Select an instrument to view trades
          </div>
        ) : trades.length === 0 ? (
          <div className="text-center text-muted-foreground py-10">
            No trades yet
          </div>
        ) : (
          <div className="h-full flex flex-col">
            {/* Headers */}
            <div className="grid grid-cols-3 px-4 py-1 text-[11px] text-muted-foreground bg-muted/5">
              <span>Price</span>
              <span className="text-right">Size</span>
              <span className="text-right">Time</span>
            </div>
            
            {/* Trades list */}
            <div className="flex-1 overflow-y-auto">
              <div className="space-y-[1px]">
                {trades.map((trade, i) => {
                  const isBuy = isBuyTrade(i);
                  return (
                    <div 
                      key={trade.id} 
                      className="grid grid-cols-3 px-4 py-[2px] text-xs hover:bg-muted/10"
                    >
                      <span className={cn(
                        "font-mono flex items-center gap-1",
                        isBuy ? "text-green-500" : "text-red-500"
                      )}>
                        {isBuy ? 
                          <ArrowUp className="h-3 w-3" /> : 
                          <ArrowDown className="h-3 w-3" />
                        }
                        {formatPrice(trade.price)}
                      </span>
                      <span className="font-mono text-right">
                        {parseFloat(trade.base_amount).toFixed(4)}
                      </span>
                      <span className="text-right text-muted-foreground">
                        {formatTime(trade.created_at)}
                      </span>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
} 