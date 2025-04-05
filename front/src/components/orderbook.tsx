'use client';

import { useOrderbook } from '@/hooks/use-orderbook';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import Decimal from 'decimal.js';

export default function OrderBook() {
  const { depth, isLoading, error } = useOrderbook(1000); // Faster updates

  // Format number with commas and fixed decimals
  const formatNumber = (numStr: string, decimals: number = 8) => {
    try {
      const num = new Decimal(numStr);
      return num.toFixed(decimals).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
    } catch (e) {
      return numStr;
    }
  };

  // Calculate maximum volume for depth visualization
  const findMaxVolume = () => {
    if (!depth) return 1;
    
    let maxBid = 0;
    let maxAsk = 0;
    
    depth.bids.forEach(bid => {
      const volume = new Decimal(bid.volume).toNumber();
      if (volume > maxBid) maxBid = volume;
    });
    
    depth.asks.forEach(ask => {
      const volume = new Decimal(ask.volume).toNumber();
      if (volume > maxAsk) maxAsk = volume;
    });
    
    return Math.max(maxBid, maxAsk, 1);
  };

  const maxVolume = findMaxVolume();

  if (error) {
    return (
      <Card className="mac-window">
        <CardHeader className="mac-header">
          <CardTitle className="text-lg">Order Book</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-destructive">{error}</div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="mac-window">
      <CardHeader className="mac-header justify-between flex-row">
        <CardTitle className="text-lg font-medium">Order Book</CardTitle>
        <div className="text-xs text-muted-foreground">
          <span className="ask mr-2">Asks</span>
          <span className="bid">Bids</span>
        </div>
      </CardHeader>
      <CardContent className="p-0">
        {isLoading && !depth ? (
          <div className="space-y-2 p-4">
            {Array(10).fill(0).map((_, i) => (
              <Skeleton key={i} className="h-5 w-full" />
            ))}
          </div>
        ) : (
          <div className="grid grid-cols-1 text-sm">
            {/* Asks (Sell orders) - displayed in reverse order (highest to lowest) */}
            <div className="order-book mac-section">
              <table className="w-full">
                <thead>
                  <tr className="text-xs text-muted-foreground border-b">
                    <th className="py-2 pl-4 text-left">Price</th>
                    <th className="py-2 text-right">Amount</th>
                    <th className="py-2 pr-4 text-right">Total</th>
                  </tr>
                </thead>
                <tbody>
                  {depth?.asks.slice().reverse().map((level, idx) => {
                    const volume = new Decimal(level.volume);
                    const price = new Decimal(level.price);
                    const total = price.mul(volume);
                    const volumePercentage = (volume.toNumber() / maxVolume) * 100;
                    
                    return (
                      <tr key={`ask-${idx}`} className="relative hover:bg-secondary/20">
                        <td className="py-1 pl-4 mono ask font-medium">
                          {formatNumber(level.price, 2)}
                        </td>
                        <td className="py-1 pr-2 text-right mono">
                          {formatNumber(level.volume, 6)}
                        </td>
                        <td className="py-1 pr-4 text-right mono">
                          {formatNumber(total.toString(), 2)}
                        </td>
                        <td 
                          className="depth-visualization depth-ask" 
                          style={{ width: `${volumePercentage}%` }} 
                        />
                      </tr>
                    );
                  })}
                  {(!depth?.asks || depth.asks.length === 0) && (
                    <tr>
                      <td colSpan={3} className="text-center py-8 text-muted-foreground">
                        No asks
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>

            {/* Spread indicator */}
            {depth && depth.bids.length > 0 && depth.asks.length > 0 && (
              <div className="text-xs text-center py-2 bg-secondary/30 mac-section">
                <span className="text-muted-foreground">
                  Spread: {formatNumber(
                    new Decimal(depth.asks[0].price).sub(depth.bids[0].price).toString(), 
                    2
                  )} ({formatNumber(
                    new Decimal(depth.asks[0].price)
                      .sub(depth.bids[0].price)
                      .div(depth.asks[0].price)
                      .mul(100)
                      .toString(),
                    2
                  )}%)
                </span>
              </div>
            )}

            {/* Bids (Buy orders) */}
            <div className="order-book">
              <table className="w-full">
                <tbody>
                  {depth?.bids.map((level, idx) => {
                    const volume = new Decimal(level.volume);
                    const price = new Decimal(level.price);
                    const total = price.mul(volume);
                    const volumePercentage = (volume.toNumber() / maxVolume) * 100;
                    
                    return (
                      <tr key={`bid-${idx}`} className="relative hover:bg-secondary/20">
                        <td className="py-1 pl-4 mono bid font-medium">
                          {formatNumber(level.price, 2)}
                        </td>
                        <td className="py-1 pr-2 text-right mono">
                          {formatNumber(level.volume, 6)}
                        </td>
                        <td className="py-1 pr-4 text-right mono">
                          {formatNumber(total.toString(), 2)}
                        </td>
                        <td 
                          className="depth-visualization depth-bid" 
                          style={{ width: `${volumePercentage}%` }} 
                        />
                      </tr>
                    );
                  })}
                  {(!depth?.bids || depth.bids.length === 0) && (
                    <tr>
                      <td colSpan={3} className="text-center py-8 text-muted-foreground">
                        No bids
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
} 