'use client';

import React, { useState, useEffect } from 'react';
import { useApi } from '@/lib/api-context';
import { toast } from 'sonner';
import { OrderType as ApiOrderType, TimeInForce } from '@/lib/api';

type OrderType = 'limit' | 'market' | 'stop';

export default function TradingForm() {
  const { selectedInstrument, client, refreshOrderbook, refreshTrades } = useApi();
  const [orderType, setOrderType] = useState<OrderType>('limit');
  const [price, setPrice] = useState('0.00383141');
  const [amount, setAmount] = useState('');
  const [total, setTotal] = useState('0.00');
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Calculate total when price or amount changes
  useEffect(() => {
    if (price && amount) {
      const calculatedTotal = (parseFloat(price) * parseFloat(amount)).toFixed(8);
      setTotal(calculatedTotal);
    } else {
      setTotal('0.00');
    }
  }, [price, amount]);

  const handlePriceChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    if (value === '' || /^\d*\.?\d*$/.test(value)) {
      setPrice(value);
    }
  };

  const handleAmountChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    if (value === '' || /^\d*\.?\d*$/.test(value)) {
      setAmount(value);
    }
  };

  const handlePercentageClick = (percentage: number) => {
    // TODO: Calculate based on available balance
    const maxAmount = 1.0; // This should come from available balance
    const calculatedAmount = (maxAmount * percentage / 100).toFixed(8);
    setAmount(calculatedAmount);
  };

  const validateForm = (): boolean => {
    if (!selectedInstrument) {
      toast.error("Please select an instrument");
      return false;
    }

    if (orderType === 'limit' && (!price || parseFloat(price) <= 0)) {
      toast.error("Please enter a valid price");
      return false;
    }

    if (!amount || parseFloat(amount) <= 0) {
      toast.error("Please enter a valid amount");
      return false;
    }

    return true;
  };

  const placeOrder = async (side: 'Buy' | 'Sell') => {
    if (!validateForm()) return;
    
    setIsSubmitting(true);
    
    try {
      // Create order request
      const orderRequest = {
        ext_id: undefined,
        account_id: "11111111-1111-1111-1111-111111111111", // Default account ID
        order_type: orderType === 'limit' ? 'Limit' as ApiOrderType : 'Market' as ApiOrderType,
        instrument_id: selectedInstrument!,
        side: side,
        limit_price: orderType === 'limit' ? price : undefined,
        trigger_price: orderType === 'stop' ? price : undefined,
        base_amount: amount,
        time_in_force: 'GTC' as TimeInForce
      };
      
      // Place the order
      const order = await client.createOrder(orderRequest);
      
      // Show success message
      toast.success(`${side} order placed successfully`, {
        description: `Order ID: ${order.id}`
      });
      
      // Refresh orderbook and trades
      await refreshOrderbook();
      await refreshTrades();
      
      // Reset form if market order
      if (orderType === 'market') {
        setAmount('');
      }
    } catch (error) {
      console.error('Failed to place order:', error);
      toast.error("Failed to place order", {
        description: error instanceof Error ? error.message : "Unknown error"
      });
    } finally {
      setIsSubmitting(false);
    }
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
          <button 
            className="py-3 rounded bg-[#26A69A] text-white font-medium hover:bg-[#219387] disabled:opacity-50 disabled:cursor-not-allowed"
            onClick={() => placeOrder('Buy')}
            disabled={isSubmitting}
          >
            {isSubmitting ? 'Processing...' : 'Buy'}
          </button>
          <button 
            className="py-3 rounded bg-[#EF454A] text-white font-medium hover:bg-[#D63F44] disabled:opacity-50 disabled:cursor-not-allowed"
            onClick={() => placeOrder('Sell')}
            disabled={isSubmitting}
          >
            {isSubmitting ? 'Processing...' : 'Sell'}
          </button>
        </div>
      </div>
    </div>
  );
} 