'use client';

import { useState } from 'react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Slider } from '@/components/ui/slider';
import { useApi } from '@/lib/api-context';
import { toast } from 'sonner';
import { v4 as uuidv4 } from 'uuid';
import { cn } from '@/lib/utils';

export default function OrderForm() {
  const { client, selectedInstrument, instruments, orderbook, refreshOrderbook } = useApi();
  
  const [side, setSide] = useState<'Buy' | 'Sell'>('Buy');
  const [orderType, setOrderType] = useState<'Limit' | 'Market'>('Limit');
  const [amount, setAmount] = useState('');
  const [price, setPrice] = useState('');
  const [total, setTotal] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  
  const instrument = selectedInstrument 
    ? instruments.find(i => i.id === selectedInstrument) 
    : null;
  
  const updateTotal = (newAmount: string, newPrice: string) => {
    if (!newAmount || !newPrice) {
      setTotal('');
      return;
    }
    try {
      const calculatedTotal = parseFloat(newAmount) * parseFloat(newPrice);
      setTotal(calculatedTotal.toFixed(2));
    } catch (error) {
      setTotal('');
    }
  };
  
  const handleAmountChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newAmount = e.target.value;
    setAmount(newAmount);
    updateTotal(newAmount, price);
  };
  
  const handlePriceChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newPrice = e.target.value;
    setPrice(newPrice);
    updateTotal(amount, newPrice);
  };
  
  const fillBestPrice = () => {
    if (!orderbook) return;
    let bestPrice = '';
    if (side === 'Buy' && orderbook.asks.length > 0) {
      bestPrice = orderbook.asks[0].price;
    } else if (side === 'Sell' && orderbook.bids.length > 0) {
      bestPrice = orderbook.bids[0].price;
    }
    if (bestPrice) {
      setPrice(parseFloat(bestPrice).toFixed(2));
      updateTotal(amount, bestPrice);
    }
  };
  
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!selectedInstrument || !amount || (orderType === 'Limit' && !price)) {
      toast("Validation Error", {
        description: "Please fill in all required fields",
        className: "destructive"
      });
      return;
    }
    
    setIsSubmitting(true);
    
    try {
      await client.createOrder({
        ext_id: uuidv4().slice(0, 8),
        account_id: "00000000-0000-0000-0000-000000000000",
        order_type: orderType,
        instrument_id: selectedInstrument,
        side,
        base_amount: amount,
        limit_price: orderType === 'Limit' ? price : undefined,
        time_in_force: 'GTC',
      });
      
      toast("Order Placed", {
        description: `${side} ${amount} at ${price || 'market price'}`,
      });
      
      setAmount('');
      setPrice('');
      setTotal('');
      refreshOrderbook();
      
    } catch (error) {
      toast("Order Failed", {
        description: error instanceof Error ? error.message : "Failed to place order",
        className: "destructive"
      });
    } finally {
      setIsSubmitting(false);
    }
  };
  
  return (
    <div className="h-full flex flex-col">
      <div className="flex items-center justify-between px-4 py-2 border-b border-border bg-muted/5">
        <h2 className="text-sm font-medium">Place Order</h2>
      </div>

      <div className="flex-1 overflow-hidden">
        {!selectedInstrument ? (
          <div className="text-center text-muted-foreground py-10">
            Select an instrument to place orders
          </div>
        ) : (
          <form onSubmit={handleSubmit} className="h-full flex flex-col">
            <div className="p-4 space-y-4">
              {/* Order type tabs */}
              <div className="flex gap-2">
                <Button 
                  type="button" 
                  variant={orderType === 'Limit' ? "default" : "outline"}
                  size="sm"
                  onClick={() => setOrderType('Limit')}
                  className="flex-1 h-8 font-medium"
                >
                  Limit
                </Button>
                <Button 
                  type="button" 
                  variant={orderType === 'Market' ? "default" : "outline"}
                  size="sm"
                  onClick={() => setOrderType('Market')}
                  className="flex-1 h-8 font-medium"
                >
                  Market
                </Button>
              </div>

              {/* Buy/Sell tabs */}
              <div className="flex gap-2">
                <Button 
                  type="button" 
                  variant={side === 'Buy' ? "default" : "outline"}
                  size="sm"
                  onClick={() => setSide('Buy')}
                  className={cn(
                    "flex-1 h-8 font-medium",
                    side === 'Buy' ? "bg-green-500 hover:bg-green-600" : ""
                  )}
                >
                  Buy
                </Button>
                <Button 
                  type="button" 
                  variant={side === 'Sell' ? "default" : "outline"}
                  size="sm"
                  onClick={() => setSide('Sell')}
                  className={cn(
                    "flex-1 h-8 font-medium",
                    side === 'Sell' ? "bg-red-500 hover:bg-red-600" : ""
                  )}
                >
                  Sell
                </Button>
              </div>

              {/* Form fields */}
              <div className="space-y-3">
                <div>
                  <div className="flex justify-between mb-1.5">
                    <Label htmlFor="amount" className="text-xs">Amount ({instrument?.base_currency})</Label>
                    <span className="text-[11px] text-muted-foreground">Available: 0.0000</span>
                  </div>
                  <Input
                    id="amount"
                    type="number"
                    placeholder="0.00"
                    step="0.0001"
                    min="0.0001"
                    value={amount}
                    onChange={handleAmountChange}
                    className="h-8 text-sm font-mono"
                  />
                </div>
                
                {orderType === 'Limit' && (
                  <div>
                    <div className="flex justify-between mb-1.5">
                      <Label htmlFor="price" className="text-xs">Price ({instrument?.quote_currency})</Label>
                      <button 
                        type="button" 
                        onClick={fillBestPrice}
                        className="text-[11px] text-primary hover:underline"
                      >
                        Best Market
                      </button>
                    </div>
                    <Input
                      id="price"
                      type="number"
                      placeholder="0.00"
                      step="0.01"
                      min="0.01"
                      value={price}
                      onChange={handlePriceChange}
                      className="h-8 text-sm font-mono"
                    />
                  </div>
                )}
                
                <div>
                  <Label htmlFor="total" className="text-xs block mb-1.5">
                    Total ({instrument?.quote_currency})
                  </Label>
                  <Input
                    id="total"
                    type="number"
                    placeholder="0.00"
                    readOnly
                    value={total}
                    className="h-8 text-sm font-mono bg-muted/5"
                  />
                </div>
              </div>
            </div>

            {/* Submit button - fixed to bottom */}
            <div className="mt-auto p-4 pt-0">
              <Button 
                type="submit" 
                className={cn(
                  "w-full h-9 text-sm font-medium",
                  side === 'Buy' 
                    ? "bg-green-500 hover:bg-green-600" 
                    : "bg-red-500 hover:bg-red-600"
                )}
                disabled={isSubmitting || !amount || (orderType === 'Limit' && !price)}
              >
                {isSubmitting ? 'Processing...' : `${side} ${instrument?.base_currency}`}
              </Button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
} 