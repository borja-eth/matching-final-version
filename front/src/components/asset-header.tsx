import React from 'react';

export default function AssetHeader() {
  return (
    <div className="flex items-center px-4 py-3 border-b border-[#2B2B43] bg-[#1F1F1F]">
      {/* Asset Info */}
      <div className="flex items-center gap-3">
        <div className="w-8 h-8 bg-red-500 rounded-full flex items-center justify-center">
          M
        </div>
        <div>
          <h1 className="text-lg font-semibold">MSTR/BTC</h1>
          <p className="text-sm text-gray-400">MicroStrategy Inc</p>
        </div>
      </div>

      {/* Price Stats */}
      <div className="flex items-center gap-8 ml-12">
        <div>
          <p className="text-sm text-gray-400">24h Change</p>
          <p className="text-green-500">+0.29%</p>
        </div>
        <div>
          <p className="text-sm text-gray-400">24h High</p>
          <p>₿ 0.0022776</p>
        </div>
        <div>
          <p className="text-sm text-gray-400">24h Low</p>
          <p>₿ 0.0021771</p>
        </div>
      </div>

      {/* Current Price */}
      <div className="ml-auto">
        <p className="text-2xl font-semibold">₿ 0.00383141</p>
      </div>
    </div>
  );
} 