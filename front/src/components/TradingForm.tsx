'use client';

import React, { useState } from 'react';
import { useApi } from '@/lib/api-context';

export default function TradingForm() {
  const { selectedInstrument } = useApi();
  const [orderType, setOrderType] = useState<'limit' | 'market' | 'stop'>('limit');
  const [price, setPrice] = useState('0.00383141');
  const [amount, setAmount] = useState('');
  const [total, setTotal] = useState('0.00');

  const handlePriceChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newPrice = e.target.value;
    setPrice(newPrice);
    if (amount) {
      setTotal((parseFloat(newPrice) * parseFloat(amount)).toFixed(8));
    }
  };

  const handleAmountChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newAmount = e.target.value;
    setAmount(newAmount);
    if (price) {
      setTotal((parseFloat(price) * parseFloat(newAmount)).toFixed(8));
    }
  };

  const handlePercentageClick = (percentage: number) => {
    // TODO: Calculate amount based on available balance and percentage
    const newAmount = '0.67'; // This should be calculated from balance
    setAmount(newAmount);
    setTotal((parseFloat(price) * parseFloat(newAmount)).toFixed(8));
  };

  return (
    <div className="h-full flex flex-col">
      <div className="p-3 border-b border-[#2B2B43] flex items-center justify-between">
        <h2 className="text-sm font-medium">Trade</h2>
        <div className="flex gap-2">
          {(['limit', 'market', 'stop'] as const).map((type) => (
            <button
              key={type}
              onClick={() => setOrderType(type)}
              className={`px-3 py-1 text-sm rounded capitalize ${
                orderType === type ? 'bg-[#2B2B43] text-white' : 'text-gray-400'
              }`}
            >
              {type}
            </button>
          ))}
        </div>
      </div>

      <div className="p-4 flex-1">
        {/* Price Input */}
        <div className="mb-4">
          <label className="block text-sm text-gray-400 mb-2">Price</label>
          <div className="relative">
            <input
              type="text"
              className="w-full bg-[#2B2B43] rounded px-3 py-2 text-white"
              placeholder="0.00"
              value={price}
              onChange={handlePriceChange}
              disabled={orderType === 'market'}
            />
            <span className="absolute right-3 top-2 text-gray-400">BTC</span>
          </div>
        </div>

        {/* Amount Input */}
        <div className="mb-4">
          <label className="block text-sm text-gray-400 mb-2">Amount</label>
          <div className="relative">
            <input
              type="text"
              className="w-full bg-[#2B2B43] rounded px-3 py-2 text-white"
              placeholder="0.00"
              value={amount}
              onChange={handleAmountChange}
            />
            <span className="absolute right-3 top-2 text-gray-400">MSTR</span>
          </div>
        </div>

        {/* Percentage Selector */}
        <div className="grid grid-cols-4 gap-2 mb-6">
          {[25, 50, 75, 100].map((percent) => (
            <button
              key={percent}
              onClick={() => handlePercentageClick(percent)}
              className="px-2 py-1 text-sm rounded bg-[#2B2B43] text-gray-400 hover:bg-[#3B3B53]"
            >
              {percent}%
            </button>
          ))}
        </div>

        {/* Total */}
        <div className="mb-6">
          <label className="block text-sm text-gray-400 mb-2">Total</label>
          <div className="relative">
            <input
              type="text"
              className="w-full bg-[#2B2B43] rounded px-3 py-2 text-white"
              placeholder="0.00"
              value={total}
              readOnly
            />
            <span className="absolute right-3 top-2 text-gray-400">BTC</span>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="grid grid-cols-2 gap-3">
          <button className="py-3 rounded bg-[#26A69A] text-white font-medium hover:bg-[#219387]">
            Buy
          </button>
          <button className="py-3 rounded bg-[#EF454A] text-white font-medium hover:bg-[#D63F44]">
            Sell
          </button>
        </div>
      </div>
    </div>
  );
} 