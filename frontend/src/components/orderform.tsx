"use client";

import React, { useState, useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Tabs, TabsContent, TabsList, TabsTrigger } from './ui/tabs';
import { Slider } from './ui/slider';
import { Switch } from './ui/switch';
import { Label } from './ui/label';
import { Expand, Maximize, MoreVertical } from 'lucide-react';
import { cn, convertToSatoshis, btcToSatoshis } from '../lib/utils';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "./ui/tooltip";
import { useApi } from '../lib/api-context';
import { toast } from 'sonner';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";

interface CryptoData {
  price: number;
  priceString: string;
  priceInSats: string;
  change24h: number;
  isPositive: boolean;
}

const Orderform: React.FC = () => {
  const { isConnected, orderbook, selectedInstrument, selectInstrument, instruments, placeOrder } = useApi();
  const [orderType, setOrderType] = useState<'buy' | 'sell'>('buy');
  const [orderMode, setOrderMode] = useState<'limit' | 'market'>('limit');
  const [amount, setAmount] = useState<string>('');
  const [price, setPrice] = useState<string>('');
  const [total, setTotal] = useState<string>('0');
  const [totalInSats, setTotalInSats] = useState<string>('0 SATS');
  const [sliderValue, setSliderValue] = useState<number>(0);
  const [cryptoData, setCryptoData] = useState<CryptoData>({
    price: 0,
    priceString: '0',
    priceInSats: '0 SATS',
    change24h: 0,
    isPositive: true
  });
  const [loading, setLoading] = useState(false);
  const [placing, setPlacing] = useState(false);
  const [priceUpdated, setPriceUpdated] = useState(false);
  const prevPriceRef = useRef<string>('');

  // Animation variants
  const containerVariants = {
    hidden: { opacity: 0 },
    visible: { 
      opacity: 1,
      transition: { 
        duration: 0.3,
        when: "beforeChildren",
        staggerChildren: 0.05
      }
    }
  };

  const itemVariants = {
    hidden: { opacity: 0, y: 5 },
    visible: { 
      opacity: 1, 
      y: 0,
      transition: { duration: 0.2 }
    }
  };

  const buttonVariants = {
    idle: { scale: 1 },
    hover: { scale: 1.02, transition: { duration: 0.2 } },
    tap: { scale: 0.98, transition: { duration: 0.1 } }
  };

  // Listen for orderbook price selection events
  useEffect(() => {
    const handleOrderbookPriceSelected = (event: Event) => {
      const customEvent = event as CustomEvent;
      const { price: selectedPrice } = customEvent.detail;
      
      // Format and set the price
      const formattedPrice = selectedPrice.toLocaleString(undefined, {
        minimumFractionDigits: 2,
        maximumFractionDigits: 8
      });
      
      setPrice(formattedPrice);

      // Trigger price flash animation
      setPriceUpdated(true);
      setTimeout(() => setPriceUpdated(false), 1000);
      
      // Recalculate total if amount exists
      if (amount && !isNaN(parseFloat(amount))) {
        calculateTotal(amount, selectedPrice);
      }
    };

    // Add event listener
    window.addEventListener('order-price-selected', handleOrderbookPriceSelected);
    
    // Cleanup
    return () => {
      window.removeEventListener('order-price-selected', handleOrderbookPriceSelected);
    };
  }, [amount]); // Re-run if amount changes to ensure total recalculation works

  // Update price from orderbook when it changes
  useEffect(() => {
    if (!orderbook || orderbook.bids.length === 0 || orderbook.asks.length === 0) return;

    try {
      // Get the mid price from orderbook
      const highestBid = parseFloat(orderbook.bids[0].price);
      const lowestAsk = parseFloat(orderbook.asks[0].price);
      const midPrice = (highestBid + lowestAsk) / 2;
      
      // Format the price
      const formattedPrice = midPrice.toLocaleString(undefined, {
        minimumFractionDigits: 2,
        maximumFractionDigits: 8
      });

      // Compare with previous price to trigger animation
      if (prevPriceRef.current && prevPriceRef.current !== formattedPrice) {
        setPriceUpdated(true);
        setTimeout(() => setPriceUpdated(false), 1000);
      }
      
      // Only update price if no manual input has been made
      if (!price || price === prevPriceRef.current) {
        setPrice(formattedPrice);
      }
      
      prevPriceRef.current = formattedPrice;
      
      // Update total if amount exists
      if (amount && !isNaN(parseFloat(amount))) {
        calculateTotal(amount, midPrice);
      }
      
      // Update crypto data
      setCryptoData({
        price: midPrice,
        priceString: formattedPrice,
        priceInSats: convertToSatoshis(midPrice, midPrice, true),
        change24h: 0, // We don't have this data
        isPositive: true
      });
    } catch (error) {
      console.error('Error processing orderbook data:', error);
    }
  }, [orderbook, amount]);

  // Calculate total based on amount and price
  const calculateTotal = (amountStr: string, priceValue: number) => {
    if (amountStr && !isNaN(parseFloat(amountStr))) {
      const totalValue = parseFloat(amountStr) * priceValue;
      setTotal(totalValue.toLocaleString(undefined, {
        minimumFractionDigits: 2,
        maximumFractionDigits: 8
      }));

      // Calculate the total in SATS (this is expressing the USD value in terms of Satoshis)
      setTotalInSats(convertToSatoshis(totalValue, priceValue));
    }
  };

  // Handle order type change
  const handleOrderTypeChange = (value: string) => {
    setOrderType(value as 'buy' | 'sell');
  };

  // Handle order mode change
  const handleOrderModeChange = (mode: 'limit' | 'market') => {
    setOrderMode(mode);
    
    // Update price for market orders
    if (mode === 'market' && cryptoData.price) {
      setPrice(cryptoData.priceString);
      
      if (amount && !isNaN(parseFloat(amount))) {
        calculateTotal(amount, cryptoData.price);
      }
    }
  };

  // Handle price change
  const handlePriceChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newPrice = e.target.value;
    setPrice(newPrice);
    
    if (amount && !isNaN(parseFloat(amount))) {
      const numericPrice = parseFloat(newPrice.replace(/,/g, ''));
      if (!isNaN(numericPrice)) {
        calculateTotal(amount, numericPrice);
      }
    }
  };

  // Handle amount change
  const handleAmountChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newAmount = e.target.value;
    setAmount(newAmount);
    
    if (!isNaN(parseFloat(newAmount))) {
      const numericPrice = parseFloat(price.replace(/,/g, ''));
      if (!isNaN(numericPrice)) {
        calculateTotal(newAmount, numericPrice);
      }
    }
  };

  // Handle slider change
  const handleSliderChange = (newValue: number[]) => {
    const value = newValue[0];
    setSliderValue(value);
    
    // Calculate amount based on slider percentage
    if (value > 0) {
      const mockBalance = 1;
      const newAmount = (mockBalance * value / 100).toFixed(6);
      setAmount(newAmount);
      
      const numericPrice = parseFloat(price.replace(/,/g, ''));
      if (!isNaN(numericPrice)) {
        calculateTotal(newAmount, numericPrice);
      }
    } else {
      setAmount('');
      setTotal('0');
      setTotalInSats('0 SATS');
    }
  };

  // Submit order
  const handleSubmitOrder = async () => {
    if (!selectedInstrument) {
      toast.error('No instrument selected');
      return;
    }
    
    if (!amount || parseFloat(amount) <= 0) {
      toast.error('Please enter a valid amount');
      return;
    }
    
    if (orderMode === 'limit' && (!price || parseFloat(price.replace(/,/g, '')) <= 0)) {
      toast.error('Please enter a valid price');
      return;
    }
    
    try {
      setPlacing(true);
      
      // Create order request
      const orderRequest = {
        account_id: "11111111-1111-1111-1111-111111111111", // Default account ID
        order_type: orderMode === 'limit' ? 'Limit' : 'Market',
        side: orderType === 'buy' ? 'Bid' : 'Ask',
        base_amount: amount.replace(/,/g, ''),
        time_in_force: 'GTC'
      };
      
      // Add limit price for limit orders
      if (orderMode === 'limit') {
        orderRequest['limit_price'] = price.replace(/,/g, '');
      }
      
      // Place the order
      const result = await placeOrder(orderRequest);
      
      if (result) {
        toast.success(`${orderType === 'buy' ? 'Buy' : 'Sell'} order placed successfully`);
        
        // Reset form for next order
        setAmount('');
        setSliderValue(0);
      }
    } catch (error) {
      console.error('Error placing order:', error);
      toast.error('Failed to place order', {
        description: error instanceof Error ? error.message : 'An unknown error occurred'
      });
    } finally {
      setPlacing(false);
    }
  };

  // Get current instrument info
  const getCurrentInstrumentInfo = () => {
    if (!selectedInstrument || instruments.length === 0) {
      return { base: 'BTC', quote: 'USD' };
    }
    
    const instrument = instruments.find(i => i.id === selectedInstrument);
    if (!instrument) {
      return { base: 'BTC', quote: 'USD' };
    }
    
    return {
      base: instrument.base_currency,
      quote: instrument.quote_currency
    };
  };
  
  const { base, quote } = getCurrentInstrumentInfo();

  // Prepare action button text and styling
  const actionButtonText = orderType === 'buy' 
    ? `Buy ${base}` 
    : `Sell ${base}`;
    
  const actionButtonClass = orderType === 'buy' 
    ? 'bg-green-500 hover:bg-green-600 text-white' 
    : 'bg-red-500 hover:bg-red-600 text-white';

  // Handler for instrument selection
  const handleInstrumentSelect = (instrumentId: string) => {
    selectInstrument(instrumentId);
  };

  return (
    <motion.div
      variants={containerVariants}
      initial="hidden"
      animate="visible"
      className="h-full flex flex-col bg-[#141414] min-w-[220px]"
    >
      <div className="border-b border-border/20 py-1.5 px-2 flex items-center justify-between">
        <h2 className="text-sm font-bold text-white">Trade</h2>
        <div className="flex items-center space-x-2">
          {isConnected ? (
            <div className="w-2 h-2 rounded-full bg-green-500" title="Connected to API" />
          ) : (
            <div className="w-2 h-2 rounded-full bg-red-500" title="Not connected to API" />
          )}
          <Maximize className="h-4 w-4 text-muted-foreground cursor-pointer" />
        </div>
      </div>
      
      <div className="flex-1 flex flex-col p-0">
        {/* Instrument Selector */}
        {instruments.length > 0 && (
          <motion.div variants={itemVariants} className="px-2 pt-1.5 pb-0.5">
            <div className="bg-[#1e1e1e] rounded-md p-2 mb-2">
              <label className="text-xs text-muted-foreground block mb-1">
                Trading Pair
              </label>
              <select 
                className="w-full bg-[#2a2a2a] text-white text-sm rounded p-1 border border-border/20"
                value={selectedInstrument || ''}
                onChange={(e) => handleInstrumentSelect(e.target.value)}
              >
                {instruments.map(instrument => (
                  <option key={instrument.id} value={instrument.id}>
                    {instrument.name} ({instrument.base_currency}/{instrument.quote_currency})
                  </option>
                ))}
              </select>
            </div>
          </motion.div>
        )}
        
        {/* Top tabs section */}
        <motion.div variants={itemVariants} className="px-2 pt-1.5 pb-0.5">
          <div className="flex border-b border-border/20">
            <div className="relative px-2.5 py-1.5 text-xs font-medium text-white after:absolute after:bottom-0 after:left-0 after:right-0 after:h-0.5 after:bg-orange-400">
              Spot
            </div>
            <div className="px-2.5 py-1.5 text-xs font-medium text-muted-foreground">
              Margin 10X
            </div>
            <div className="px-2.5 py-1.5 text-xs font-medium text-muted-foreground">
              Convert
            </div>
            <div className="ml-auto">
              <MoreVertical className="h-4 w-4 text-muted-foreground cursor-pointer" />
            </div>
          </div>
        </motion.div>
        
        {/* Buy/Sell section */}
        <motion.div variants={itemVariants} className="px-2 pt-2 pb-1.5">
          <div className="grid grid-cols-2 gap-1">
            <motion.button
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              onClick={() => handleOrderTypeChange('buy')}
              className={cn(
                "py-2 px-3 text-center text-xs font-medium rounded-md transition-colors duration-200 relative z-10",
                orderType === 'buy' ? "bg-green-500 text-white" : "bg-[#2b2b2b] text-muted-foreground"
              )}
            >
              Buy
            </motion.button>
            <motion.button
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              onClick={() => handleOrderTypeChange('sell')}
              className={cn(
                "py-2 px-3 text-center text-xs font-medium rounded-md transition-colors duration-200 relative z-10",
                orderType === 'sell' ? "bg-red-500 text-white" : "bg-[#2b2b2b] text-muted-foreground"
              )}
            >
              Sell
            </motion.button>
          </div>
        </motion.div>
        
        {/* Order type section */}
        <motion.div variants={itemVariants} className="px-2 pt-0.5 pb-1">
          <div className="flex space-x-3 text-xs">
            <button
              onClick={() => handleOrderModeChange('limit')}
              className={cn(
                "font-medium transition-colors relative z-10",
                orderMode === 'limit' 
                  ? "text-orange-400 border-b-2 border-orange-400 pb-0.5" 
                  : "text-muted-foreground pb-0.5 border-b-2 border-transparent"
              )}
            >
              Limit
            </button>
            <button
              onClick={() => handleOrderModeChange('market')}
              className={cn(
                "font-medium transition-colors relative z-10",
                orderMode === 'market' 
                  ? "text-orange-400 border-b-2 border-orange-400 pb-0.5" 
                  : "text-muted-foreground pb-0.5 border-b-2 border-transparent"
              )}
            >
              Market
            </button>
            <div className="flex items-center ml-auto">
              <span className="text-white text-xs font-medium mr-2">TP/SL</span>
              <svg className="h-3.5 w-3.5 text-muted-foreground" viewBox="0 0 24 24" fill="none">
                <path d="M6 9l6 6 6-6" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
              </svg>
            </div>
          </div>
        </motion.div>
        
        {/* Available Balance */}
        <motion.div variants={itemVariants} className="px-2 py-1 flex justify-between items-center">
          <span className="text-xs text-muted-foreground">Available Balance</span>
          <span className="text-xs text-white">-- {quote}</span>
        </motion.div>
        
        {/* Price Input */}
        <motion.div variants={itemVariants} className="px-2 py-0.5">
          <div className="bg-[#1e1e1e] rounded-md flex flex-col overflow-hidden relative">
            <div className="px-2 pt-1.5 pb-0.5 text-xs text-muted-foreground">
              Price
            </div>
            <div className="flex items-center px-2 pb-1.5 relative">
              <motion.div 
                className="absolute inset-0 bg-green-500/5 pointer-events-none"
                initial={{ opacity: 0 }}
                animate={{ opacity: priceUpdated ? 1 : 0 }}
                transition={{ duration: 0.3 }}
              />
              <input 
                value={loading ? "Loading..." : price} 
                onChange={handlePriceChange} 
                disabled={orderMode === 'market' || loading}
                className="bg-transparent flex-1 focus:outline-none text-white text-sm z-10 relative"
                placeholder="0"
              />
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span className="text-white text-xs ml-1 relative z-10 cursor-help">{quote}</span>
                  </TooltipTrigger>
                  <TooltipContent side="right" sideOffset={5} className="bg-[#1e1e1e] border-border text-white p-2 rounded">
                    <div className="space-y-1 text-xs">
                      <div className="grid grid-cols-2 gap-2">
                        <span className="text-muted-foreground">{base} Price ({quote}):</span>
                        <span className="text-right font-medium">
                          {loading ? "Loading..." : price}
                        </span>
                      </div>
                      {orderMode === 'market' && (
                        <div className="pt-1 text-center text-xs text-muted-foreground border-t border-border/30">
                          Market orders execute at current market price
                        </div>
                      )}
                    </div>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          </div>
        </motion.div>
        
        {/* Quantity Input */}
        <motion.div variants={itemVariants} className="px-2 py-0.5">
          <div className="bg-[#1e1e1e] rounded-md flex flex-col overflow-hidden relative">
            <div className="px-2 pt-1.5 pb-0.5 text-xs text-muted-foreground">
              Quantity
            </div>
            <div className="flex items-center px-2 pb-1.5 relative">
              <input 
                value={amount} 
                onChange={handleAmountChange} 
                disabled={loading}
                className="bg-transparent flex-1 focus:outline-none text-white text-sm z-10 relative"
                placeholder="0"
              />
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span className="text-white text-xs ml-1 relative z-10 cursor-help">{base}</span>
                  </TooltipTrigger>
                  <TooltipContent side="right" sideOffset={5} className="bg-[#1e1e1e] border-border text-white p-2 rounded">
                    <div className="space-y-1 text-xs">
                      <div className="grid grid-cols-2 gap-2">
                        <span className="text-muted-foreground">Asset:</span>
                        <span className="text-right font-medium">{base}</span>
                      </div>
                      <div className="grid grid-cols-2 gap-2">
                        <span className="text-muted-foreground">Current Price:</span>
                        <span className="text-right font-medium">
                          {loading ? "Loading..." : cryptoData.priceString} {quote}
                        </span>
                      </div>
                      <div className="grid grid-cols-2 gap-2">
                        <span className="text-muted-foreground">Quantity:</span>
                        <span className="text-right font-mono">{amount || '0'} {base}</span>
                      </div>
                    </div>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          </div>
        </motion.div>
        
        {/* Slider */}
        <motion.div variants={itemVariants} className="px-2 py-3 pb-4">
          <div className="space-y-4">
            <div className="relative">
              <Slider
                value={[sliderValue]}
                onValueChange={handleSliderChange}
                max={100}
                step={1}
                className={cn(
                  "relative z-10 w-full",
                  orderType === 'buy' ? "[&_[data-slot=slider-range]]:bg-green-500" : "[&_[data-slot=slider-range]]:bg-red-500"
                )}
              />
              
              <div className="flex justify-between mt-2">
                {[0, 25, 50, 75, 100].map((percent) => (
                  <button
                    type="button"
                    key={percent}
                    onClick={() => handleSliderChange([percent])}
                    className="text-xs text-muted-foreground hover:text-white"
                  >
                    {percent}%
                  </button>
                ))}
              </div>
            </div>
          </div>
        </motion.div>
        
        {/* Order Value */}
        <motion.div variants={itemVariants} className="px-2 py-0.5">
          <div className="bg-[#1e1e1e] rounded-md flex flex-col overflow-hidden relative">
            <div className="px-2 pt-1.5 pb-0.5 text-xs text-muted-foreground">
              Order Value
            </div>
            <div className="flex items-center px-2 pb-1.5 relative">
              <input 
                value={total} 
                readOnly 
                className="bg-transparent flex-1 focus:outline-none text-white text-sm relative z-10 cursor-default"
                placeholder="0"
              />
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span className="text-white text-xs ml-1 relative z-10 cursor-help">{quote}</span>
                  </TooltipTrigger>
                  <TooltipContent side="right" sideOffset={5} className="bg-[#1e1e1e] border-border text-white p-2 rounded">
                    <div className="space-y-1 text-xs">
                      <div className="grid grid-cols-2 gap-2">
                        <span className="text-muted-foreground">Order Value ({quote}):</span>
                        <span className="text-right font-medium">{total}</span>
                      </div>
                      <div className="grid grid-cols-2 gap-2">
                        <span className="text-muted-foreground">Quantity ({base}):</span>
                        <span className="text-right font-mono">{amount || '0'}</span>
                      </div>
                    </div>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          </div>
        </motion.div>
        
        {/* Action Buttons */}
        <div className="mt-auto px-2 py-1.5 space-y-1.5">
          <motion.button
            variants={buttonVariants}
            initial="idle"
            whileHover="hover"
            whileTap="tap"
            disabled={!isConnected || placing || !amount || amount === '0' || (orderMode === 'limit' && (!price || price === '0'))}
            onClick={handleSubmitOrder}
            className={cn(
              "w-full py-2 rounded-md font-medium text-xs transition-all duration-200 relative z-10",
              "disabled:opacity-50 disabled:cursor-not-allowed",
              orderType === 'buy' ? "bg-green-500 hover:bg-green-600 text-white" : "bg-red-500 hover:bg-red-600 text-white"
            )}
          >
            {placing ? (
              <div className="flex items-center justify-center">
                <motion.div 
                  className="h-3 w-3 border-2 border-white/30 border-t-white rounded-full mr-1.5"
                  animate={{ rotate: 360 }}
                  transition={{ 
                    duration: 1, 
                    repeat: Infinity, 
                    ease: "linear" 
                  }}
                />
                Processing...
              </div>
            ) : (
              actionButtonText
            )}
          </motion.button>
          
          {!isConnected && (
            <div className="text-xs text-center text-amber-400">
              Not connected to matching engine
            </div>
          )}
        </div>
      </div>
    </motion.div>
  );
};

export default Orderform; 