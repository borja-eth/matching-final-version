'use client';

import { useEffect } from 'react';
import { useApi } from '@/lib/api-context';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
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
    <Card className="h-full">
      <CardHeader className="pb-3">
        <CardTitle className="text-lg font-medium">Recent Trades</CardTitle>
      </CardHeader>
      <CardContent className="px-0 text-sm">
        {!selectedInstrument ? (
          <div className="text-center text-muted-foreground py-10">
            Select an instrument to view trades
          </div>
        ) : trades.length === 0 ? (
          <div className="text-center text-muted-foreground py-10">
            No trades yet
          </div>
        ) : (
          <>
            <div className="flex justify-between px-4 text-xs text-muted-foreground mb-1">
              <span>Price</span>
              <span>Size</span>
              <span>Time</span>
            </div>
            
            <div className="space-y-0.5">
              {trades.map((trade, i) => {
                const isBuy = isBuyTrade(i);
                
                return (
                  <div 
                    key={trade.id} 
                    className="flex justify-between px-4 py-0.5 hover:bg-muted/30"
                  >
                    <span className={cn("font-mono flex items-center gap-1", 
                      isBuy ? "text-green-500" : "text-red-500"
                    )}>
                      {isBuy ? <ArrowUp className="h-3 w-3" /> : <ArrowDown className="h-3 w-3" />}
                      {formatPrice(trade.price)}
                    </span>
                    <span className="font-mono">{parseFloat(trade.base_amount).toFixed(4)}</span>
                    <span className="text-xs text-muted-foreground">{formatTime(trade.created_at)}</span>
                  </div>
                );
              })}
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
} 