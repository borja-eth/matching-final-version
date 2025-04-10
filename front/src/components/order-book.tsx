'use client';

import { useEffect, useState } from 'react';
import { useApi } from '@/lib/api-context';

interface Instrument {
  quote_currency: string;
  base_currency: string;
}

interface Order {
  price: string;
  volume: string;
}

export default function OrderBook() {
  const { orderbook, refreshOrderbook, selectedInstrument } = useApi();
  const [grouping, setGrouping] = useState('0.1');
  const [displayMode, setDisplayMode] = useState<'amount' | 'total'>('amount');
  
  useEffect(() => {
    if (selectedInstrument) {
      refreshOrderbook();
      const interval = setInterval(refreshOrderbook, 1000);
      return () => clearInterval(interval);
    }
  }, [selectedInstrument, refreshOrderbook]);

  const formatNumber = (num: string | number, decimals: number = 8) => {
    try {
      return Number(num).toFixed(decimals);
    } catch (e) {
      return '0.00000000';
    }
  };

  const getMaxVolume = (orders: Order[]) => {
    if (!orders?.length) return 1;
    return Math.max(...orders.map(order => parseFloat(order.volume)));
  };

  const maxAskVolume = orderbook?.asks ? getMaxVolume(orderbook.asks) : 1;
  const maxBidVolume = orderbook?.bids ? getMaxVolume(orderbook.bids) : 1;

  return (
    <div className="h-full flex flex-col bg-[#141414]">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-[#2B2B43]">
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1">
            <svg className="w-4 h-4" viewBox="0 0 20 20" fill="none">
              <path fill="#EF454A" d="M2 3h6v6H2z"/>
              <path fill="#26A69A" d="M2 11h6v6H2z"/>
              <g fill="#B2B5BE" opacity=".8">
                <path d="M10 3h8v2h-8zM10 7h8v2h-8zM10 11h8v2h-8zM10 15h8v2h-8z"/>
              </g>
            </svg>
          </div>
          <span className="text-[14px] font-medium text-white">Order Book</span>
        </div>
        <div className="flex gap-2">
          <select 
            value={grouping}
            onChange={(e) => setGrouping(e.target.value)}
            className="h-6 px-2 text-[12px] bg-[#2B2B43] text-[#B2B5BE] border-none rounded"
          >
            <option value="0.1">0.1</option>
            <option value="0.01">0.01</option>
            <option value="0.001">0.001</option>
          </select>
          <select
            value={displayMode}
            onChange={(e) => setDisplayMode(e.target.value as 'amount' | 'total')}
            className="h-6 px-2 text-[12px] bg-[#2B2B43] text-[#B2B5BE] border-none rounded"
          >
            <option value="amount">Amount</option>
            <option value="total">Total</option>
          </select>
        </div>
      </div>

      {/* Order Book Content */}
      <div className="flex-1 overflow-hidden">
        {!orderbook ? (
          <div className="text-center text-[#B2B5BE] py-10">Loading...</div>
        ) : (
          <div className="h-full flex flex-col">
            {/* Column Headers */}
            <div className="grid grid-cols-3 text-[12px] text-[#B2B5BE] px-4 py-2">
              <div>Price (BTC)</div>
              <div className="text-right">Size (MSTR)</div>
              <div className="text-right">Total</div>
            </div>

            {/* Asks */}
            <div className="flex-1 overflow-y-auto">
              {orderbook.asks && [...orderbook.asks].reverse().map((ask, i) => (
                <div 
                  key={`${ask.price}-${i}`}
                  className="grid grid-cols-3 text-[12px] px-4 py-[2px] relative group"
                >
                  <div className="text-[#EF454A] font-mono z-10">{formatNumber(ask.price)}</div>
                  <div className="text-right font-mono text-[#B2B5BE] z-10">{formatNumber(ask.volume)}</div>
                  <div className="text-right font-mono text-[#B2B5BE] z-10">
                    {formatNumber(parseFloat(ask.price) * parseFloat(ask.volume))}
                  </div>
                  <div 
                    className="absolute right-0 top-0 bottom-0 bg-[#EF454A] opacity-10"
                    style={{ width: `${(parseFloat(ask.volume) / maxAskVolume) * 100}%` }}
                  />
                </div>
              ))}
            </div>

            {/* Spread */}
            <div className="px-4 py-2 border-y border-[#2B2B43] flex justify-between items-center">
              <div className="text-[#26A69A] font-mono text-[14px]">
                {orderbook.bids?.[0]?.price && formatNumber(orderbook.bids[0].price)}
              </div>
              <div className="text-[12px] text-[#B2B5BE]">
                Spread: {formatNumber(
                  Math.abs(
                    parseFloat(orderbook.asks?.[0]?.price || '0') - 
                    parseFloat(orderbook.bids?.[0]?.price || '0')
                  )
                )}
              </div>
            </div>

            {/* Bids */}
            <div className="flex-1 overflow-y-auto">
              {orderbook.bids && orderbook.bids.map((bid, i) => (
                <div 
                  key={`${bid.price}-${i}`}
                  className="grid grid-cols-3 text-[12px] px-4 py-[2px] relative group"
                >
                  <div className="text-[#26A69A] font-mono z-10">{formatNumber(bid.price)}</div>
                  <div className="text-right font-mono text-[#B2B5BE] z-10">{formatNumber(bid.volume)}</div>
                  <div className="text-right font-mono text-[#B2B5BE] z-10">
                    {formatNumber(parseFloat(bid.price) * parseFloat(bid.volume))}
                  </div>
                  <div 
                    className="absolute right-0 top-0 bottom-0 bg-[#26A69A] opacity-10"
                    style={{ width: `${(parseFloat(bid.volume) / maxBidVolume) * 100}%` }}
                  />
                </div>
              ))}
            </div>

            {/* Buy/Sell Distribution */}
            <div className="h-1 flex">
              <div className="bg-[#26A69A] h-full" style={{ width: '76%' }} />
              <div className="bg-[#EF454A] h-full" style={{ width: '24%' }} />
            </div>
          </div>
        )}
      </div>
    </div>
  );
} 