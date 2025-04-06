'use client';

import { useEffect } from 'react';
import { useApi } from '@/lib/api-context';
import { cn } from '@/lib/utils';

interface Instrument {
  quote_currency: string;
  base_currency: string;
}

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
  
  // Cast selectedInstrument to Instrument type since we know it's not null in the render
  const instrument = selectedInstrument as unknown as Instrument;
  
  return (
    <div className="h-full flex flex-col">
      <div className="flex items-center justify-between px-4 py-2 border-b border-[var(--bds-gray-ele-border)]">
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1">
            <svg className="w-4 h-4" viewBox="0 0 20 20" fill="none">
              <path fill="var(--bds-red-700-normal)" d="M2 3h6v6H2z"/>
              <path fill="var(--bds-green-700-normal)" d="M2 11h6v6H2z"/>
              <g fill="var(--bds-gray-t1-title)" opacity=".8">
                <path d="M10 3h8v2h-8zM10 7h8v2h-8zM10 11h8v2h-8zM10 15h8v2h-8z"/>
              </g>
            </svg>
            <svg className="w-4 h-4 opacity-40" viewBox="0 0 20 20" fill="none">
              <path fill="var(--bds-red-700-normal)" d="M17 2v6h-6V2z"/>
              <path fill="var(--bds-green-700-normal)" d="M9 2v6H3V2z"/>
              <g fill="var(--bds-gray-t1-title)" opacity=".8">
                <path d="M17 10v8h-2v-8zM13 10v8h-2v-8zM9 10v8H7v-8zM5 10v8H3v-8z"/>
              </g>
            </svg>
          </div>
          <h2 className="text-[var(--bds-font-size-14)] font-[var(--bds-font-weight-medium)] text-[var(--bds-gray-t1-title)]">Order Book</h2>
        </div>
        <div className="flex items-center gap-2">
          <select className="h-6 px-2 text-[var(--bds-font-size-12)] bg-[var(--bds-gray-bg-float)] border-none rounded text-[var(--bds-gray-t2)]">
            <option>0.1</option>
          </select>
          <select className="h-6 px-2 text-[var(--bds-font-size-12)] bg-[var(--bds-gray-bg-float)] border-none rounded text-[var(--bds-gray-t2)]">
            <option>Total(BTC)</option>
          </select>
        </div>
      </div>

      <div className="flex-1 overflow-hidden">
        {!selectedInstrument ? (
          <div className="text-center text-[var(--bds-gray-t3)] py-10">
            Select an instrument to view order book
          </div>
        ) : !orderbook ? (
          <div className="text-center text-[var(--bds-gray-t3)] py-10">
            Loading order book...
          </div>
        ) : (
          <div className="h-full flex flex-col">
            {/* Asks (Sell Orders) */}
            <div className="flex-1 overflow-y-auto">
              <div className="grid grid-cols-3 text-[var(--bds-font-size-12)] text-[var(--bds-gray-t3)] px-4 py-1 bg-[var(--bds-gray-bg-float)]">
                <div>Price({instrument.quote_currency})</div>
                <div className="text-right">Qty({instrument.base_currency})</div>
                <div className="text-right">Total({instrument.base_currency})</div>
              </div>
              <div className="space-y-[1px]">
                {[...orderbook.asks].reverse().map((ask, index) => (
                  <div 
                    key={index}
                    className="grid grid-cols-3 text-[var(--bds-font-size-12)] px-4 py-1 hover:bg-[var(--bds-trans-hover)] cursor-pointer relative group"
                  >
                    <div className="text-[var(--bds-red-700-normal)] font-mono">{formatPrice(ask.price)}</div>
                    <div className="text-right font-mono text-[var(--bds-gray-t2)]">{formatVolume(ask.volume)}</div>
                    <div className="text-right font-mono text-[var(--bds-gray-t2)]">{calculateTotal(ask.price, ask.volume)}</div>
                    <div 
                      className="absolute inset-0 bg-[var(--bds-red-100-bg)] -z-10"
                      style={{ width: `${(parseFloat(ask.volume) / parseFloat(orderbook.asks[0].volume)) * 100}%` }}
                    />
                  </div>
                ))}
              </div>
            </div>

            {/* Current Price */}
            <div className="px-4 py-2 bg-muted/5 border-y border-border">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <svg className="w-4 h-4 text-green-500" viewBox="0 0 1024 1024">
                    <path fill="currentColor" d="M554.667 316.33L780.5 542.166a42.667 42.667 0 0 0 60.331-60.33L542.464 183.467a43.093 43.093 0 0 0-60.928 0L183.168 481.835a42.667 42.667 0 1 0 60.33 60.33l225.835-225.834v496.981c0 22.101 19.115 40.021 42.667 40.021s42.667-17.92 42.667-40.021V316.331z"/>
                  </svg>
                  <span className="text-green-500 font-mono">{formatPrice(orderbook.bids[0].price)}</span>
                </div>
                <div className="text-xs text-muted-foreground">
                  Spread: {formatPrice(spread)} ({spreadPercent}%)
                </div>
              </div>
            </div>

            {/* Bids (Buy Orders) */}
            <div className="flex-1 overflow-y-auto">
              <div className="space-y-[1px]">
                {orderbook.bids.map((bid, index) => (
                  <div 
                    key={index}
                    className="grid grid-cols-3 text-xs px-4 py-1 hover:bg-muted/10 cursor-pointer relative group"
                  >
                    <div className="text-green-500 font-mono">{formatPrice(bid.price)}</div>
                    <div className="text-right font-mono">{formatVolume(bid.volume)}</div>
                    <div className="text-right font-mono">{calculateTotal(bid.price, bid.volume)}</div>
                    <div 
                      className="absolute inset-0 bg-green-500/5 -z-10"
                      style={{ width: `${(parseFloat(bid.volume) / parseFloat(orderbook.bids[0].volume)) * 100}%` }}
                    />
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
} 