'use client';

import React from 'react';
import dynamic from 'next/dynamic';

// Dynamically import TradingView's lightweight charts with no SSR
const TradingViewChart = dynamic(() => import('./TradingViewChart'), {
  ssr: false,
});

export default function TradingChart() {
  const timeframes = ['1s', '1m', '5m', '15m', '30m', '1h'];

  return (
    <div className="h-full flex flex-col">
      <div className="p-3 border-b border-[#2B2B43] flex items-center justify-between">
        <div className="flex gap-2">
          {timeframes.map((tf) => (
            <button
              key={tf}
              className={`px-3 py-1 text-sm rounded ${
                tf === '1s' ? 'bg-[#2B2B43] text-white' : 'text-gray-400'
              }`}
            >
              {tf}
            </button>
          ))}
        </div>

        <div className="flex gap-2">
          <button className="p-2 rounded hover:bg-[#2B2B43]">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M3 3h18v18H3z"/>
              <path d="M21 9H3M21 15H3M12 3v18"/>
            </svg>
          </button>
          <button className="p-2 rounded hover:bg-[#2B2B43]">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/>
            </svg>
          </button>
        </div>
      </div>

      <div className="flex-1 relative">
        <TradingViewChart />
      </div>
    </div>
  );
} 