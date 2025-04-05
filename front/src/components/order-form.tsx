'use client';

import { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Slider } from '@/components/ui/slider';
import { useApi } from '@/lib/api-context';
import { toast } from 'sonner';
import { v4 as uuidv4 } from 'uuid';

export default function OrderForm() {
  const { client, selectedInstrument, instruments, orderbook, refreshOrderbook } = useApi();
  
  const [side, setSide] = useState<'Buy' | 'Sell'>('Buy');
  const [orderType, setOrderType] = useState<'Limit' | 'Market'>('Limit');
  const [amount, setAmount] = useState('');
  const [price, setPrice] = useState('');
  const [total, setTotal] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  
  // Get the selected instrument details
  const instrument = selectedInstrument 
    ? instruments.find(i => i.id === selectedInstrument) 
    : null;
  
  // Update total when amount or price changes
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
  
  // Handle amount change
  const handleAmountChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newAmount = e.target.value;
    setAmount(newAmount);
    updateTotal(newAmount, price);
  };
  
  // Handle price change
  const handlePriceChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newPrice = e.target.value;
    setPrice(newPrice);
    updateTotal(amount, newPrice);
  };
  
  // Fill in best price from orderbook
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
  
  // Handle order submission
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
      const order = await client.createOrder({
        ext_id: uuidv4().slice(0, 8), // Generate a short reference ID
        account_id: "00000000-0000-0000-0000-000000000000", // Demo account
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
      
      // Reset form
      setAmount('');
      setPrice('');
      setTotal('');
      
      // Refresh orderbook
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
    <Card className="h-full">
      <CardHeader className="pb-3">
        <CardTitle className="text-lg font-medium">Place Order</CardTitle>
      </CardHeader>
      <CardContent>
        {!selectedInstrument ? (
          <div className="text-center text-muted-foreground py-10">
            Select an instrument to place orders
          </div>
        ) : (
          <form onSubmit={handleSubmit}>
            <Tabs defaultValue={side} onValueChange={(v) => setSide(v as 'Buy' | 'Sell')}>
              <TabsList className="grid grid-cols-2 mb-4">
                <TabsTrigger value="Buy" className="data-[state=active]:bg-green-500 data-[state=active]:text-white">Buy</TabsTrigger>
                <TabsTrigger value="Sell" className="data-[state=active]:bg-red-500 data-[state=active]:text-white">Sell</TabsTrigger>
              </TabsList>
              
              <div className="space-y-4">
                <div className="flex gap-2">
                  <Button 
                    type="button" 
                    variant={orderType === 'Limit' ? "default" : "outline"}
                    size="sm"
                    onClick={() => setOrderType('Limit')}
                    className="flex-1"
                  >
                    Limit
                  </Button>
                  <Button 
                    type="button" 
                    variant={orderType === 'Market' ? "default" : "outline"}
                    size="sm"
                    onClick={() => setOrderType('Market')}
                    className="flex-1"
                  >
                    Market
                  </Button>
                </div>
                
                <div className="space-y-2">
                  <div className="flex justify-between">
                    <Label htmlFor="amount">Amount ({instrument?.base_currency})</Label>
                    <span className="text-xs text-muted-foreground">Available: 0.0000</span>
                  </div>
                  <Input
                    id="amount"
                    type="number"
                    placeholder="0.00"
                    step="0.0001"
                    min="0.0001"
                    value={amount}
                    onChange={handleAmountChange}
                  />
                </div>
                
                {orderType === 'Limit' && (
                  <div className="space-y-2">
                    <div className="flex justify-between">
                      <Label htmlFor="price">Price ({instrument?.quote_currency})</Label>
                      <button 
                        type="button" 
                        onClick={fillBestPrice}
                        className="text-xs text-primary hover:underline"
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
                    />
                  </div>
                )}
                
                <div className="space-y-2">
                  <Label htmlFor="total">Total ({instrument?.quote_currency})</Label>
                  <Input
                    id="total"
                    type="number"
                    placeholder="0.00"
                    readOnly
                    value={total}
                  />
                </div>
                
                <Button 
                  type="submit" 
                  className={`w-full ${side === 'Buy' ? 'bg-green-500 hover:bg-green-600' : 'bg-red-500 hover:bg-red-600'}`}
                  disabled={isSubmitting || !amount || (orderType === 'Limit' && !price)}
                >
                  {isSubmitting ? 'Processing...' : `${side} ${instrument?.base_currency}`}
                </Button>
              </div>
            </Tabs>
          </form>
        )}
      </CardContent>
    </Card>
  );
} 