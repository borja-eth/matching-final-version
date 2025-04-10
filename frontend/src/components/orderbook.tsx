"use client";

import React, { useState, useEffect, useRef } from 'react';
import { motion, AnimatePresence, LayoutGroup } from 'framer-motion';
import { Table, TableBody, TableCell, TableHead, TableRow } from './ui/table';
import { Button } from './ui/button';
import { cn, convertToSatoshis, btcToSatoshis } from '../lib/utils';
import { ChevronUp, ChevronDown, BarChart2 } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "./ui/tooltip";
import { useApi } from '../lib/api-context';

interface OrderbookEntry {
  price: number;
  quantity: number;
  total: number;
  id?: string; // Unique identifier for animation
}

interface TradeEntry {
  id: string;
  price: number;
  amount: number;
  side: 'buy' | 'sell';
  time: string;
}

// Create a custom event for price selection
export const selectOrderPrice = (price: number) => {
  const event = new CustomEvent('order-price-selected', { 
    detail: { price } 
  });
  window.dispatchEvent(event);
};

const Orderbook: React.FC = () => {
  const { orderbook, refreshOrderbook, trades, isConnected } = useApi();
  const [selectedTab, setSelectedTab] = useState<'bids' | 'asks' | 'both' | 'trades'>('both');
  const [bids, setBids] = useState<OrderbookEntry[]>([]);
  const [asks, setAsks] = useState<OrderbookEntry[]>([]);
  const [processedTrades, setProcessedTrades] = useState<TradeEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [spread, setSpread] = useState({ value: 0, percentage: 0 });
  const [maxQuantity, setMaxQuantity] = useState(0);
  const [currentPrice, setCurrentPrice] = useState<number | null>(null);
  const [bidPercentage, setBidPercentage] = useState(50);
  const [askPercentage, setAskPercentage] = useState(50);
  const [priceDirection, setPriceDirection] = useState<'up' | 'down' | null>(null);
  const prevPriceRef = useRef<number | null>(null);
  const [bitcoinPrice, setBitcoinPrice] = useState<number>(0);

  // State to track previous bids and asks for price change detection
  const prevBidsRef = useRef<Record<number, number>>({});
  const prevAsksRef = useRef<Record<number, number>>({});
  const [changedPrices, setChangedPrices] = useState<Record<string, 'increased' | 'decreased' | null>>({});

  // Animation variants for container
  const containerVariants = {
    hidden: { opacity: 0 },
    visible: { 
      opacity: 1,
      transition: { 
        duration: 0.2
      }
    }
  };
  
  // Much clearer directional animations with fixed positions rather than relative
  // Asks animation - fade in from top with more pronounced direction
  const asksRowVariants = {
    hidden: { 
      opacity: 0, 
      y: -10,
      height: 0,
      overflow: 'hidden'
    },
    visible: (i: number) => ({
      opacity: 1,
      y: 0,
      height: 'auto',
      overflow: 'visible',
      transition: {
        y: { duration: 0.2, delay: i * 0.015 },
        opacity: { duration: 0.15, delay: i * 0.015 },
        height: { duration: 0.1, delay: i * 0.015 }
      }
    }),
    exit: { 
      opacity: 0,
      scale: 0.96,
      transition: { duration: 0.05 }
    }
  };

  // Bids animation - fade in from bottom with more pronounced direction
  const bidsRowVariants = {
    hidden: { 
      opacity: 0, 
      y: 10,
      height: 0,
      overflow: 'hidden'
    },
    visible: (i: number) => ({
      opacity: 1,
      y: 0,
      height: 'auto',
      overflow: 'visible',
      transition: {
        y: { duration: 0.2, delay: i * 0.015 },
        opacity: { duration: 0.15, delay: i * 0.015 },
        height: { duration: 0.1, delay: i * 0.015 }
      }
    }),
    exit: { 
      opacity: 0,
      scale: 0.96,
      transition: { duration: 0.05 }
    }
  };
  
  // Flash animation for price updates
  const flashAnimationVariants = {
    flash: {
      backgroundColor: ["rgba(0,0,0,0)", "rgba(255,255,255,0.1)", "rgba(0,0,0,0)"],
      transition: { duration: 0.7, ease: "easeInOut" }
    },
    neutral: { 
      backgroundColor: "rgba(0, 0, 0, 0)",
      transition: { duration: 0.5 }
    }
  };

  const priceMoveVariants = {
    up: { 
      backgroundColor: "rgba(74, 222, 128, 0.15)",
      transition: { duration: 1.5, ease: [0.25, 0.1, 0.25, 1.0] }
    },
    down: { 
      backgroundColor: "rgba(249, 115, 115, 0.15)", 
      transition: { duration: 1.5, ease: [0.25, 0.1, 0.25, 1.0] }
    },
    neutral: { 
      backgroundColor: "rgba(0, 0, 0, 0)",
      transition: { duration: 0.5 }
    }
  };

  // Various animation variants for different price changes
  const priceFlashAnimation = {
    up: { 
      backgroundColor: ["rgba(0,0,0,0)", "rgba(16, 185, 129, 0.12)", "rgba(0,0,0,0)"],
      transition: { 
        duration: 1.2, 
        ease: [0.25, 0.1, 0.25, 1.0], // Custom easing for smoother animation
        times: [0, 0.3, 1] // Control timing of the keyframes
      }
    },
    down: { 
      backgroundColor: ["rgba(0,0,0,0)", "rgba(239, 68, 68, 0.12)", "rgba(0,0,0,0)"],
      transition: { 
        duration: 1.2, 
        ease: [0.25, 0.1, 0.25, 1.0],
        times: [0, 0.3, 1]
      }
    },
    neutral: { 
      backgroundColor: "rgba(0,0,0,0)",
      transition: { duration: 0.5 }
    }
  };

  // Flash-only animation variant for "all" view - no directional movement
  const flashOnlyVariants = {
    hidden: { 
      opacity: 0,
      scale: 0.98,
      y: -2
    },
    visible: (i: number) => ({
      opacity: 1,
      scale: 1,
      y: 0,
      transition: {
        opacity: { duration: 0.3, delay: i * 0.015, ease: "easeOut" },
        scale: { duration: 0.25, delay: i * 0.015, ease: "easeOut" },
        y: { duration: 0.2, delay: i * 0.015, ease: "easeOut" }
      }
    }),
    exit: { 
      opacity: 0,
      scale: 0.98,
      y: -2,
      transition: { duration: 0.15, ease: "easeIn" }
    }
  };
  
  // Flash animation specifically for price/quantity changes - more subtle, smoother transitions
  const highlightFlashAnimation = {
    increased: {
      backgroundColor: ["rgba(0,0,0,0)", "rgba(16, 185, 129, 0.18)", "rgba(0,0,0,0)"],
      transition: { 
        duration: 1.2, 
        ease: "circOut"
      }
    },
    decreased: {
      backgroundColor: ["rgba(0,0,0,0)", "rgba(239, 68, 68, 0.18)", "rgba(0,0,0,0)"],
      transition: { 
        duration: 1.2, 
        ease: "circOut"
      }
    },
    none: {
      backgroundColor: "rgba(0,0,0,0)",
      transition: { duration: 0.3 }
    }
  };

  // Process orderbook data from API
  useEffect(() => {
    if (!orderbook) {
      setLoading(true);
      return;
    }

    try {
      // Create maps of current bids/asks for change detection
      const currentBidPrices: Record<number, number> = {};
      const currentAskPrices: Record<number, number> = {};
      const newChangedPrices: Record<string, 'increased' | 'decreased' | null> = {};
      
      // Process bids (buy orders)
      const processedBids: OrderbookEntry[] = orderbook.bids.map((bid, index) => {
        const price = parseFloat(bid.price);
        const quantity = parseFloat(bid.volume);
        currentBidPrices[price] = quantity;
        
        // Check if quantity changed for this price
        if (prevBidsRef.current[price] !== undefined) {
          if (quantity > prevBidsRef.current[price]) {
            newChangedPrices[`bid-${price}`] = 'increased';
          } else if (quantity < prevBidsRef.current[price]) {
            newChangedPrices[`bid-${price}`] = 'decreased';
          }
        }
        
        return {
          price,
          quantity,
          total: price * quantity,
          id: `bid-${price}-${Date.now()}-${index}` // Create unique ID for animation
        };
      });
      
      // Process asks (sell orders)
      const processedAsks: OrderbookEntry[] = orderbook.asks.map((ask, index) => {
        const price = parseFloat(ask.price);
        const quantity = parseFloat(ask.volume);
        currentAskPrices[price] = quantity;
        
        // Check if quantity changed for this price
        if (prevAsksRef.current[price] !== undefined) {
          if (quantity > prevAsksRef.current[price]) {
            newChangedPrices[`ask-${price}`] = 'increased';
          } else if (quantity < prevAsksRef.current[price]) {
            newChangedPrices[`ask-${price}`] = 'decreased';
          }
        }
        
        return {
          price,
          quantity,
          total: price * quantity,
          id: `ask-${price}-${Date.now()}-${index}` // Create unique ID for animation
        };
      });
      
      // Update refs with current prices
      prevBidsRef.current = currentBidPrices;
      prevAsksRef.current = currentAskPrices;
      
      // Update changed prices (will auto-reset after animation)
      setChangedPrices(newChangedPrices);
      setTimeout(() => setChangedPrices({}), 500);
      
      // Sort bids in descending order by price
      processedBids.sort((a, b) => b.price - a.price);
      // Sort asks in ascending order by price
      processedAsks.sort((a, b) => a.price - b.price);
      
      setBids(processedBids);
      setAsks(processedAsks);
      
      // Calculate spread
      if (processedBids.length > 0 && processedAsks.length > 0) {
        const highestBid = processedBids[0].price;
        const lowestAsk = processedAsks[0].price;
        const spreadValue = lowestAsk - highestBid;
        const spreadPercentage = (spreadValue / lowestAsk) * 100;
        
        setSpread({
          value: spreadValue,
          percentage: spreadPercentage
        });
        
        // Determine current price (mid price)
        const midPrice = (highestBid + lowestAsk) / 2;
        
        // Determine price direction
        if (prevPriceRef.current !== null) {
          if (midPrice > prevPriceRef.current) {
            setPriceDirection('up');
          } else if (midPrice < prevPriceRef.current) {
            setPriceDirection('down');
          }
          
          // Reset direction after a delay
          setTimeout(() => {
            setPriceDirection(null);
          }, 2000);
        }
        
        prevPriceRef.current = midPrice;
        setCurrentPrice(midPrice);
      }
      
      // Calculate max quantity for visualization
      const allQuantities = [...processedBids.map(bid => bid.quantity), ...processedAsks.map(ask => ask.quantity)];
      if (allQuantities.length > 0) {
        setMaxQuantity(Math.max(...allQuantities));
      }
      
      // Calculate bid/ask percentages
      const totalBidQty = processedBids.reduce((sum, bid) => sum + bid.quantity, 0);
      const totalAskQty = processedAsks.reduce((sum, ask) => sum + ask.quantity, 0);
      const totalQty = totalBidQty + totalAskQty;
      
      if (totalQty > 0) {
        setBidPercentage(Math.round((totalBidQty / totalQty) * 100));
        setAskPercentage(Math.round((totalAskQty / totalQty) * 100));
      }
      
      setLoading(false);
    } catch (error) {
      console.error('Error processing orderbook data:', error);
      // Use sample data if there's an error
      useSampleData();
    }
  }, [orderbook]);

  // Process trades data from API
  useEffect(() => {
    if (!trades || trades.length === 0) return;

    const formatted = trades.map(trade => {
      // Calculate price information for display
      const price = parseFloat(trade.price);
      const amount = parseFloat(trade.base_amount);
      
      // Determine the trade side (buy or sell) based on market dynamics
      // Standard convention: trades are labeled based on the taker's action
      // - A buy (green) means a buy order executed against a sell order in the book
      // - A sell (red) means a sell order executed against a buy order in the book
      //
      // Without direct access to order details, we use price position relative to orderbook:
      let side: 'buy' | 'sell' = 'buy';
      
      if (orderbook && orderbook.bids.length > 0 && orderbook.asks.length > 0) {
        const bestBid = parseFloat(orderbook.bids[0].price);
        const bestAsk = parseFloat(orderbook.asks[0].price);
        
        // If price is closer to or at the bid, it was likely a sell hitting a bid
        // If price is closer to or at the ask, it was likely a buy lifting an offer
        if (Math.abs(price - bestBid) <= Math.abs(price - bestAsk)) {
          side = 'sell'; // Executed against a bid (someone sold to a buyer)
        } else {
          side = 'buy';  // Executed against an ask (someone bought from a seller)
        }
      }
      
      return {
        id: trade.id,
        price,
        amount,
        side,
        time: new Date(trade.created_at).toLocaleTimeString()
      };
    });

    setProcessedTrades(formatted);
  }, [trades, orderbook]);

  // Manually refresh orderbook data
  const handleRefresh = () => {
    setLoading(true);
    refreshOrderbook();
  };

  // Use sample data if API call fails
  const useSampleData = () => {
    const sampleBids = [
      { price: 82608.9, quantity: 0.121085, total: 10002.56, id: `bid-82608.9-${Date.now()}-0` },
      { price: 82608.5, quantity: 0.109868, total: 9076.52, id: `bid-82608.5-${Date.now()}-1` },
      { price: 82608.0, quantity: 3.399650, total: 280843.56, id: `bid-82608.0-${Date.now()}-2` },
      { price: 82607.6, quantity: 0.617287, total: 50994.24, id: `bid-82607.6-${Date.now()}-3` },
      { price: 82607.3, quantity: 0.001333, total: 110.12, id: `bid-82607.3-${Date.now()}-4` },
      { price: 82606.9, quantity: 0.000196, total: 16.19, id: `bid-82606.9-${Date.now()}-5` },
      { price: 82606.5, quantity: 0.047987, total: 3964.29, id: `bid-82606.5-${Date.now()}-6` },
      { price: 82606.1, quantity: 0.000456, total: 37.67, id: `bid-82606.1-${Date.now()}-7` },
      { price: 82605.8, quantity: 0.000240, total: 19.83, id: `bid-82605.8-${Date.now()}-8` },
      { price: 82605.4, quantity: 0.012540, total: 1035.87, id: `bid-82605.4-${Date.now()}-9` },
    ];

    const sampleAsks = [
      { price: 82610.6, quantity: 0.000456, total: 37.67, id: `ask-82610.6-${Date.now()}-0` },
      { price: 82610.5, quantity: 0.003000, total: 247.83, id: `ask-82610.5-${Date.now()}-1` },
      { price: 82610.3, quantity: 0.000456, total: 37.67, id: `ask-82610.3-${Date.now()}-2` },
      { price: 82610.0, quantity: 0.600061, total: 49570.04, id: `ask-82610.0-${Date.now()}-3` },
      { price: 82609.5, quantity: 0.121085, total: 10002.56, id: `ask-82609.5-${Date.now()}-4` },
      { price: 82609.0, quantity: 0.085185, total: 7037.12, id: `ask-82609.0-${Date.now()}-5` },
      { price: 82613.5, quantity: 0.033000, total: 2726.15, id: `ask-82613.5-${Date.now()}-6` },
      { price: 82614.0, quantity: 0.000456, total: 37.67, id: `ask-82614.0-${Date.now()}-7` },
      { price: 82614.5, quantity: 0.000196, total: 16.19, id: `ask-82614.5-${Date.now()}-8` },
      { price: 82615.0, quantity: 0.001450, total: 119.75, id: `ask-82615.0-${Date.now()}-9` },
    ];

    setCurrentPrice(82609.0); // Sample current price
    setPriceDirection(null);
    prevPriceRef.current = 82609.0;
    
    setBids(sampleBids);
    setAsks(sampleAsks);

    // Calculate sample spread
    const highestBid = sampleBids[0].price;
    const lowestAsk = sampleAsks[0].price;
    const spreadValue = lowestAsk - highestBid;
    const spreadPercentage = (spreadValue / lowestAsk) * 100;
    
    setSpread({
      value: spreadValue,
      percentage: spreadPercentage
    });

    // Calculate max quantity for visualization
    const allQuantities = [...sampleBids.map(bid => bid.quantity), ...sampleAsks.map(ask => ask.quantity)];
    setMaxQuantity(Math.max(...allQuantities));
    
    // Set sample bid/ask percentages
    setBidPercentage(45);
    setAskPercentage(55);
    
    setLoading(false);
  };

  // Get max quantity for the visible orders only
  const getVisibleMaxQuantity = () => {
    let visibleOrders = [];
    
    // Get the visible orders based on the current tab
    if (selectedTab === 'asks') {
      visibleOrders = [...asks];
    } else if (selectedTab === 'bids') {
      visibleOrders = [...bids];
    } else {
      // In 'both' mode, take only the displayed orders (limited to 15 each)
      visibleOrders = [...asks.slice(0, 15), ...bids.slice(0, 15)];
    }
    
    // Extract quantities
    const visibleQuantities = visibleOrders.map(order => order.quantity);
    
    // Return max or 0 if no orders
    return visibleQuantities.length > 0 ? Math.max(...visibleQuantities) : 0;
  };
  
  // Function to ensure equal display of bids and asks
  const getBalancedOrderCount = () => {
    // Calculate how many orders we can show based on the highest count
    // but ensure equal representation on both sides
    const maxCount = Math.max(asks.length, bids.length);
    // Limit to a maximum of 15 per side to prevent excessive entries
    return Math.min(15, maxCount);
  };
  
  // Calculate the width for depth visualization with minimum width to ensure visibility
  const calculateDepthWidth = (quantity: number) => {
    const maxQty = getVisibleMaxQuantity();
    if (maxQty === 0) return 0;
    
    // Calculate percentage with a minimum of 5% to ensure all orders have a visible background
    const percentage = (quantity / maxQty) * 100;
    return Math.max(percentage, 5);
  };

  // Handler for price click
  const handlePriceClick = (price: number) => {
    selectOrderPrice(price);
  };

  return (
    <motion.div
      initial="hidden"
      animate="visible"
      variants={containerVariants}
      className="h-full flex flex-col bg-background/95 relative"
    >
      <div className="border-b border-border/40 p-2 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <BarChart2 className="h-4 w-4 text-muted-foreground" />
          <h2 className="text-sm font-bold">Order Book</h2>
          <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`} 
            title={isConnected ? 'Connected to API' : 'Not connected to API'} />
        </div>
        <div className="flex gap-1">
          <TooltipProvider delayDuration={300}>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button 
                  variant="outline" 
                  size="sm"
                  className={cn(
                    "text-xs h-7 px-3 py-1 rounded transition-all duration-200", 
                    selectedTab === 'bids' 
                      ? "bg-background border-border shadow-sm" 
                      : "bg-transparent hover:bg-background/50"
                  )}
                  onClick={() => setSelectedTab('bids')}
                >
                  Bids
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                <p className="text-xs">Show only buy orders</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
          
          <TooltipProvider delayDuration={300}>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button 
                  variant="outline" 
                  size="sm"
                  className={cn(
                    "text-xs h-7 px-3 py-1 rounded transition-all duration-200", 
                    selectedTab === 'both' 
                      ? "bg-background border-border shadow-sm" 
                      : "bg-transparent hover:bg-background/50"
                  )}
                  onClick={() => setSelectedTab('both')}
                >
                  All
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                <p className="text-xs">Show both buy and sell orders</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
          
          <TooltipProvider delayDuration={300}>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button 
                  variant="outline" 
                  size="sm"
                  className={cn(
                    "text-xs h-7 px-3 py-1 rounded transition-all duration-200", 
                    selectedTab === 'asks' 
                      ? "bg-background border-border shadow-sm" 
                      : "bg-transparent hover:bg-background/50"
                  )}
                  onClick={() => setSelectedTab('asks')}
                >
                  Asks
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                <p className="text-xs">Show only sell orders</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>

          <TooltipProvider delayDuration={300}>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button 
                  variant="outline" 
                  size="sm"
                  className={cn(
                    "text-xs h-7 px-3 py-1 rounded transition-all duration-200", 
                    selectedTab === 'trades' 
                      ? "bg-background border-border shadow-sm" 
                      : "bg-transparent hover:bg-background/50"
                  )}
                  onClick={() => setSelectedTab('trades')}
                >
                  Trades
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                <p className="text-xs">Show recent trades</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
      </div>

      <div className="flex-1 overflow-hidden orderbook-container">
        {loading && bids.length === 0 && asks.length === 0 && processedTrades.length === 0 ? (
          <div className="flex h-full items-center justify-center">
            <motion.div 
              className="flex flex-col items-center gap-2"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ duration: 0.3 }}
            >
              <motion.div 
                className="h-6 w-6 border-2 border-primary/30 border-t-primary rounded-full"
                animate={{ rotate: 360 }}
                transition={{ 
                  duration: 1.5, 
                  repeat: Infinity, 
                  ease: "linear" 
                }}
              />
              <p className="text-xs text-muted-foreground">Loading order book data...</p>
            </motion.div>
          </div>
        ) : selectedTab === 'trades' ? (
          // Trades Tab Content
          <div className="trades-table h-full flex flex-col">
            <div className="grid grid-cols-4 text-xs text-muted-foreground py-1 border-b border-border">
              <div className="text-left pl-3">Price</div>
              <div className="text-right">Amount</div>
              <div className="text-center">Side</div>
              <div className="text-right pr-3">Time</div>
            </div>

            <div className="flex-1 overflow-auto scrollbar-thin scrollbar-thumb-border scrollbar-track-background">
              {processedTrades.length === 0 ? (
                <div className="flex h-full items-center justify-center">
                  <p className="text-xs text-muted-foreground">No trades available</p>
                </div>
              ) : (
                <AnimatePresence mode="popLayout">
                  {processedTrades.map((trade, index) => (
                    <motion.div
                      key={trade.id}
                      onClick={() => handlePriceClick(trade.price)}
                      className="grid grid-cols-4 text-xs py-[2px] cursor-pointer hover:bg-secondary/10 relative"
                      variants={flashOnlyVariants}
                      custom={index}
                      initial="hidden"
                      animate="visible"
                      exit="exit"
                    >
                      <div className={cn(
                        "font-medium text-left pl-3 flex items-center",
                        trade.side === 'buy' ? "text-green-500" : "text-red-500"
                      )}>
                        {trade.price.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 8 })}
                      </div>
                      <div className="text-right font-mono">{trade.amount.toFixed(8)}</div>
                      <div className={cn(
                        "text-center font-medium",
                        trade.side === 'buy' ? "text-green-500" : "text-red-500"
                      )}>
                        {trade.side === 'buy' ? 'BUY' : 'SELL'}
                      </div>
                      <div className="text-right pr-3 font-mono">{trade.time}</div>
                    </motion.div>
                  ))}
                </AnimatePresence>
              )}
            </div>
          </div>
        ) : (
          // Order Book Content
          <div className="orderbook-table h-full">
            <LayoutGroup>
              {/* Table headers */}
              <div className="grid grid-cols-3 text-xs text-muted-foreground py-1 border-b border-border">
                <div className="text-left pl-3">Price</div>
                <div className="text-right">Amount</div>
                <div className="text-right pr-3">Total</div>
              </div>

              <div className="flex-grow overflow-auto scrollbar-thin scrollbar-thumb-border scrollbar-track-background">
                <div className="grid grid-rows-[minmax(0,1fr)_auto_minmax(0,1fr)] h-full">
                  {/* Asks (sell orders) */}
                  {(selectedTab === 'asks' || selectedTab === 'both') && (
                    <div className={`${selectedTab === 'both' ? 'flex flex-col' : 'flex flex-col-reverse'} overflow-y-auto max-h-full`}>
                      <AnimatePresence mode={selectedTab === 'both' ? 'wait' : 'popLayout'}>
                        {(selectedTab === 'both' ? 
                          // In 'both' view, ensure equal representation of orders
                          asks.slice(0, getBalancedOrderCount()) : 
                          asks).map((ask, index) => (
                          <motion.div
                            key={ask.id}
                            onClick={() => handlePriceClick(ask.price)}
                            className="grid grid-cols-3 text-xs py-[2px] cursor-pointer hover:bg-secondary/10 relative"
                            variants={selectedTab === 'both' ? flashOnlyVariants : asksRowVariants}
                            custom={selectedTab === 'both' ? index : asks.length - index - 1}
                            initial="hidden"
                            animate="visible"
                            exit="exit"
                          >
                            <motion.div 
                              className="absolute inset-0 pointer-events-none"
                              animate={changedPrices[`ask-${ask.price}`] || 'none'}
                              variants={highlightFlashAnimation}
                            />
                            <TooltipProvider>
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <div className="text-red-500 font-medium text-left pl-3 flex items-center">
                                    {ask.price.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 8 })}
                                  </div>
                                </TooltipTrigger>
                                <TooltipContent side="right" sideOffset={5} className="bg-[#1e1e1e] border-border text-white p-2 rounded">
                                  <div className="space-y-1 text-xs">
                                    <div className="grid grid-cols-2 gap-2">
                                      <span className="text-muted-foreground">Price:</span>
                                      <span className="text-right font-medium text-red-500">
                                        {ask.price.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 8 })}
                                      </span>
                                    </div>
                                    <div className="grid grid-cols-2 gap-2">
                                      <span className="text-muted-foreground">Quantity:</span>
                                      <span className="text-right font-mono">{ask.quantity.toFixed(8)}</span>
                                    </div>
                                    <div className="grid grid-cols-2 gap-2">
                                      <span className="text-muted-foreground">Total:</span>
                                      <span className="text-right font-mono">
                                        {ask.total.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 8 })}
                                      </span>
                                    </div>
                                  </div>
                                </TooltipContent>
                              </Tooltip>
                            </TooltipProvider>
                            <div className="text-right font-mono">{ask.quantity.toFixed(8)}</div>
                            <div className="relative text-right pr-3 font-mono">
                              {ask.total.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 8 })}
                              <div
                                className="absolute right-0 top-0 h-full bg-red-500/10 z-0"
                                style={{ width: `${calculateDepthWidth(ask.quantity)}%` }}
                              />
                            </div>
                          </motion.div>
                        ))}
                      </AnimatePresence>
                    </div>
                  )}

                  {/* Current Price Display */}
                  {selectedTab === 'both' && (
                    <motion.div
                      variants={priceFlashAnimation}
                      animate={priceDirection || "neutral"}
                      className="flex justify-between items-center py-1 px-3 bg-secondary-foreground/5 border-y border-border"
                    >
                      <div className="text-sm font-medium flex items-center gap-1">
                        <span className="text-xs text-muted-foreground mr-1">Last Price:</span>
                        <span className={cn(
                          priceDirection === 'up' ? 'text-green-500' : priceDirection === 'down' ? 'text-red-500' : 'text-white'
                        )}>
                          {currentPrice?.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 8 })}
                        </span>
                      </div>

                      <div className="text-xs text-muted-foreground">
                        Spread: {spread.value.toFixed(8)} ({spread.percentage.toFixed(4)}%)
                      </div>
                    </motion.div>
                  )}

                  {/* Bids (buy orders) */}
                  {(selectedTab === 'bids' || selectedTab === 'both') && (
                    <div className="flex flex-col overflow-y-auto max-h-full">
                      <AnimatePresence mode={selectedTab === 'both' ? 'wait' : 'popLayout'}>
                        {(selectedTab === 'both' ? 
                          // In 'both' view, ensure equal representation of orders
                          bids.slice(0, getBalancedOrderCount()) : 
                          bids).map((bid, index) => (
                          <motion.div
                            key={bid.id}
                            onClick={() => handlePriceClick(bid.price)}
                            className="grid grid-cols-3 text-xs py-[2px] cursor-pointer hover:bg-secondary/10 relative"
                            variants={selectedTab === 'both' ? flashOnlyVariants : bidsRowVariants}
                            custom={index}
                            initial="hidden"
                            animate="visible"
                            exit="exit"
                          >
                            <motion.div 
                              className="absolute inset-0 pointer-events-none"
                              animate={changedPrices[`bid-${bid.price}`] || 'none'}
                              variants={highlightFlashAnimation}
                            />
                            <TooltipProvider>
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <div className="text-green-500 font-medium text-left pl-3 flex items-center">
                                    {bid.price.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 8 })}
                                  </div>
                                </TooltipTrigger>
                                <TooltipContent side="right" sideOffset={5} className="bg-[#1e1e1e] border-border text-white p-2 rounded">
                                  <div className="space-y-1 text-xs">
                                    <div className="grid grid-cols-2 gap-2">
                                      <span className="text-muted-foreground">Price:</span>
                                      <span className="text-right font-medium text-green-500">
                                        {bid.price.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 8 })}
                                      </span>
                                    </div>
                                    <div className="grid grid-cols-2 gap-2">
                                      <span className="text-muted-foreground">Quantity:</span>
                                      <span className="text-right font-mono">{bid.quantity.toFixed(8)}</span>
                                    </div>
                                    <div className="grid grid-cols-2 gap-2">
                                      <span className="text-muted-foreground">Total:</span>
                                      <span className="text-right font-mono">
                                        {bid.total.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 8 })}
                                      </span>
                                    </div>
                                  </div>
                                </TooltipContent>
                              </Tooltip>
                            </TooltipProvider>
                            <div className="text-right font-mono">{bid.quantity.toFixed(8)}</div>
                            <div className="relative text-right pr-3 font-mono">
                              {bid.total.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 8 })}
                              <div
                                className="absolute right-0 top-0 h-full bg-green-500/10 z-0"
                                style={{ width: `${calculateDepthWidth(bid.quantity)}%` }}
                              />
                            </div>
                          </motion.div>
                        ))}
                      </AnimatePresence>
                    </div>
                  )}
                </div>
              </div>
            </LayoutGroup>
          </div>
        )}
      </div>
    </motion.div>
  );
};

export default Orderbook; 