"use client";

import React, { useEffect, useState } from 'react';
import { ChevronRight, TrendingUp, TrendingDown, DollarSign, BarChart4 } from 'lucide-react';
import { motion } from 'framer-motion';
import { convertToSatoshis } from '../lib/utils';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "./ui/tooltip";

interface MarketPrice {
  symbol: string;
  price: string;
  priceInSats: string;
  change: string;
  isPositive: boolean;
  type: 'crypto' | 'stock';
}

const TickerBar: React.FC = () => {
  const [allPrices, setAllPrices] = useState<MarketPrice[]>([
    // Default crypto data (placeholder values until API fetch)
    { symbol: 'BTC/USDT', price: '82,608.90', priceInSats: 'Loading...', change: '+8.95%', isPositive: true, type: 'crypto' },
    { symbol: 'ETH/USDT', price: '3,072.50', priceInSats: 'Loading...', change: '+4.21%', isPositive: true, type: 'crypto' },
    { symbol: 'SOL/USDT', price: '128.47', priceInSats: 'Loading...', change: '+12.35%', isPositive: true, type: 'crypto' },
    { symbol: 'XRP/USDT', price: '0.5073', priceInSats: 'Loading...', change: '-2.13%', isPositive: false, type: 'crypto' },
    // Default stock data (placeholder values until API fetch)
    { symbol: 'AAPL', price: '182.64', priceInSats: 'Loading...', change: '+0.87%', isPositive: true, type: 'stock' },
    { symbol: 'MSFT', price: '420.39', priceInSats: 'Loading...', change: '+1.23%', isPositive: true, type: 'stock' },
    { symbol: 'GOOGL', price: '164.55', priceInSats: 'Loading...', change: '-0.45%', isPositive: false, type: 'stock' },
    { symbol: 'AMZN', price: '178.32', priceInSats: 'Loading...', change: '+2.14%', isPositive: true, type: 'stock' },
  ]);
  const [cryptoLoading, setCryptoLoading] = useState(true);
  const [stockLoading, setStockLoading] = useState(true);
  const [selectedTab, setSelectedTab] = useState<'all' | 'crypto' | 'stocks'>('all');
  const [bitcoinPrice, setBitcoinPrice] = useState<number>(82608.90); // Default BTC price

  // List of cryptocurrency IDs for the CoinGecko API
  const cryptoIds = [
    'bitcoin',
    'ethereum',
    'solana',
    'ripple',
    'cardano',
    'dogecoin',
    'polkadot',
    'chainlink',
    'binancecoin',
    'avalanche-2',
    'matic-network',
    'cosmos'
  ];

  // List of top US stock symbols
  const stockSymbols = [
    'AAPL',  // Apple
    'MSFT',  // Microsoft
    'GOOGL', // Alphabet (Google)
    'AMZN',  // Amazon
    'META',  // Meta Platforms (Facebook)
    'TSLA',  // Tesla
    'NVDA',  // NVIDIA
    'JPM',   // JPMorgan Chase
    'V',     // Visa
    'WMT',   // Walmart
  ];

  // Get symbol name from ID
  const getSymbolName = (id: string): string => {
    const symbolMap: Record<string, string> = {
      'bitcoin': 'BTC/USDT',
      'ethereum': 'ETH/USDT',
      'solana': 'SOL/USDT',
      'ripple': 'XRP/USDT',
      'cardano': 'ADA/USDT',
      'dogecoin': 'DOGE/USDT',
      'polkadot': 'DOT/USDT',
      'chainlink': 'LINK/USDT',
      'binancecoin': 'BNB/USDT',
      'avalanche-2': 'AVAX/USDT',
      'matic-network': 'MATIC/USDT',
      'cosmos': 'ATOM/USDT'
    };
    return symbolMap[id] || id.toUpperCase() + '/USDT';
  };

  // Fetch Bitcoin price specifically for Satoshi conversion
  const fetchBitcoinPrice = async () => {
    try {
      const response = await fetch(
        'https://api.coingecko.com/api/v3/coins/bitcoin?localization=false&tickers=false&market_data=true&community_data=false&developer_data=false&sparkline=false'
      );
      
      if (!response.ok) {
        throw new Error('Network response was not ok');
      }
      
      const data = await response.json();
      const btcPrice = data.market_data.current_price.usd;
      setBitcoinPrice(btcPrice);
      
      return btcPrice;
    } catch (error) {
      console.error('Error fetching Bitcoin price data:', error);
      return bitcoinPrice; // Return current state as fallback
    }
  };

  // Fetch cryptocurrency data from CoinGecko API
  const fetchCryptoData = async () => {
    try {
      setCryptoLoading(true);
      
      // First get the current Bitcoin price for Satoshi conversion
      const btcPrice = await fetchBitcoinPrice();
      
      const response = await fetch(
        `https://api.coingecko.com/api/v3/coins/markets?vs_currency=usd&ids=${cryptoIds.join(',')}&order=market_cap_desc&per_page=100&page=1&sparkline=false&price_change_percentage=24h`
      );
      
      if (!response.ok) {
        throw new Error('Network response was not ok');
      }
      
      const data = await response.json();
      
      const formattedCryptoPrices: MarketPrice[] = data.map((crypto: any) => {
        const priceChange = crypto.price_change_percentage_24h || 0;
        const isPositive = priceChange >= 0;
        const price = crypto.current_price;
        const isBitcoin = crypto.id === 'bitcoin';
        
        return {
          symbol: getSymbolName(crypto.id),
          price: price.toLocaleString(undefined, {
            minimumFractionDigits: price < 1 ? 4 : 2,
            maximumFractionDigits: price < 1 ? 4 : 2
          }),
          priceInSats: convertToSatoshis(price, btcPrice, isBitcoin),
          change: `${isPositive ? '+' : ''}${priceChange.toFixed(2)}%`,
          isPositive,
          type: 'crypto'
        };
      });
      
      // Update allPrices by merging with existing stock prices
      setAllPrices(prevPrices => {
        const stockPrices = prevPrices.filter(price => price.type === 'stock');
        return [...formattedCryptoPrices, ...stockPrices];
      });
    } catch (error) {
      console.error('Error fetching cryptocurrency data:', error);
      // Keep the existing prices if there's an error
    } finally {
      setCryptoLoading(false);
    }
  };

  // Fetch stock data from Alpha Vantage API or use simulated data
  const fetchStockData = async () => {
    try {
      setStockLoading(true);
      
      // Get current Bitcoin price for Satoshi conversion
      const btcPrice = bitcoinPrice;
      
      // IMPORTANT: Due to Alpha Vantage API limitations, we're using simulated data
      // Set this to false to attempt using the real API with your key
      const USE_SIMULATED_DATA = true;
      
      // Alpha Vantage API key
      const ALPHA_VANTAGE_API_KEY = '1J8QXNKR1BXDY050';
      
      // Simulated data for development to avoid API rate limits
      const simulatedStockData: Record<string, { price: string, numericPrice: number, change: string, isPositive: boolean }> = {
        'AAPL': { price: '182.64', numericPrice: 182.64, change: '+0.87%', isPositive: true },
        'MSFT': { price: '420.39', numericPrice: 420.39, change: '+1.23%', isPositive: true },
        'GOOGL': { price: '164.55', numericPrice: 164.55, change: '-0.45%', isPositive: false },
        'AMZN': { price: '178.32', numericPrice: 178.32, change: '+2.14%', isPositive: true },
        'META': { price: '479.28', numericPrice: 479.28, change: '+0.65%', isPositive: true },
        'TSLA': { price: '222.18', numericPrice: 222.18, change: '-1.32%', isPositive: false },
        'NVDA': { price: '926.46', numericPrice: 926.46, change: '+3.27%', isPositive: true },
        'JPM': { price: '198.53', numericPrice: 198.53, change: '+0.42%', isPositive: true },
        'V': { price: '275.64', numericPrice: 275.64, change: '+0.18%', isPositive: true },
        'WMT': { price: '68.42', numericPrice: 68.42, change: '-0.24%', isPositive: false },
      };
      
      // Only fetch a batch to respect Alpha Vantage's rate limiting
      const currentTime = new Date().getTime();
      const batchIndex = Math.floor(currentTime / 60000) % 2; // Switch batch every minute
      const stockBatch = stockSymbols.slice(batchIndex * 5, (batchIndex * 5) + 5);
      
      console.log(`Processing stock batch ${batchIndex + 1}: ${stockBatch.join(', ')}`);
      
      const stockData: MarketPrice[] = [];
      
      if (USE_SIMULATED_DATA) {
        // Use simulated data
        for (const symbol of stockBatch) {
          const data = simulatedStockData[symbol];
          if (data) {
            stockData.push({
              symbol: symbol,
              price: data.price,
              priceInSats: convertToSatoshis(data.numericPrice, btcPrice),
              change: data.change,
              isPositive: data.isPositive,
              type: 'stock'
            });
          }
        }
      } else {
        // Use the actual Alpha Vantage API
        for (const symbol of stockBatch) {
          try {
            console.log(`Fetching data for ${symbol}...`);
            const response = await fetch(
              `https://www.alphavantage.co/query?function=GLOBAL_QUOTE&symbol=${symbol}&apikey=${ALPHA_VANTAGE_API_KEY}`
            );
            
            if (!response.ok) {
              throw new Error(`Network response was not ok for ${symbol} (${response.status})`);
            }
            
            const data = await response.json();
            console.log(`Response for ${symbol}:`, JSON.stringify(data).substring(0, 200) + '...');
            
            // Check if we have valid data and handle Alpha Vantage's error messages
            if (data['Global Quote'] && Object.keys(data['Global Quote']).length > 0) {
              const quote = data['Global Quote'];
              
              if (quote['05. price'] && quote['09. change percent']) {
                const currentPrice = parseFloat(quote['05. price']);
                const priceChange = parseFloat(quote['09. change percent'].replace(/[+%]/g, ''));
                const isPositive = parseFloat(quote['09. change']) >= 0;
                
                stockData.push({
                  symbol: symbol,
                  price: currentPrice.toLocaleString(undefined, {
                    minimumFractionDigits: 2,
                    maximumFractionDigits: 2
                  }),
                  priceInSats: convertToSatoshis(currentPrice, btcPrice),
                  change: `${isPositive ? '+' : ''}${priceChange.toFixed(2)}%`,
                  isPositive,
                  type: 'stock'
                });
              } else {
                throw new Error(`Missing price data for ${symbol}`);
              }
            } else if (data['Note'] && data['Note'].includes('API call frequency')) {
              console.warn(`Rate limit hit for ${symbol}: ${data['Note']}`);
              throw new Error(`Rate limit exceeded for ${symbol}`);
            } else {
              console.error(`Invalid data structure for ${symbol}:`, data);
              throw new Error(`Invalid data structure for ${symbol}`);
            }
            
            // Add delay to respect Alpha Vantage's rate limit (5 calls per minute for free tier)
            await new Promise(resolve => setTimeout(resolve, 12500));
          } catch (err) {
            console.error(`Error fetching data for ${symbol}:`, err);
            
            // Use fallback data
            const fallback = simulatedStockData[symbol];
            if (fallback) {
              stockData.push({
                symbol: symbol,
                price: fallback.price,
                priceInSats: convertToSatoshis(fallback.numericPrice, btcPrice),
                change: fallback.change,
                isPositive: fallback.isPositive,
                type: 'stock'
              });
            } else {
              stockData.push({
                symbol: symbol,
                price: "0.00",
                priceInSats: "0.00 SATS",
                change: "0.00%",
                isPositive: true,
                type: 'stock'
              });
            }
            
            // Add smaller delay for failed requests
            await new Promise(resolve => setTimeout(resolve, 200));
          }
        }
      }
      
      // Update allPrices with the current batch data while keeping other stocks from previous data
      setAllPrices(prevPrices => {
        // Keep existing stock data for stocks not in current batch
        const otherStockPrices = prevPrices.filter(
          price => price.type === 'stock' && !stockBatch.includes(price.symbol)
        );
        const cryptoPrices = prevPrices.filter(price => price.type === 'crypto');
        return [...cryptoPrices, ...otherStockPrices, ...stockData];
      });
    } catch (error) {
      console.error('Error in fetchStockData:', error);
      // Keep existing stock data if there's an error
      setAllPrices(prevPrices => prevPrices);
    } finally {
      setStockLoading(false);
    }
  };

  useEffect(() => {
    // Initial data fetches
    fetchCryptoData();
    fetchStockData();
    
    // Set up intervals to fetch data periodically
    const cryptoInterval = setInterval(fetchCryptoData, 30000);
    const stockInterval = setInterval(fetchStockData, 60000);
    
    return () => {
      clearInterval(cryptoInterval);
      clearInterval(stockInterval);
    };
  }, []);

  // Filter prices based on selected tab
  const filteredPrices = allPrices.filter(price => {
    if (selectedTab === 'all') return true;
    return price.type === selectedTab;
  });

  return (
    <div className="border-t border-b border-border/30 bg-background/60 py-1.5 overflow-hidden">
      <div className="flex items-center px-3">
        <div className="flex items-center text-muted-foreground mr-3 shrink-0 border-r border-border/30 pr-3">
          <BarChart4 size={11} className="mr-1" />
          <span className="text-[11px] font-medium">Markets</span>
          <ChevronRight size={11} className="ml-1 text-muted-foreground/50" />
        </div>
        
        <div className="flex gap-2 mr-4 shrink-0">
          <button 
            onClick={() => setSelectedTab('all')}
            className={`text-[11px] px-2 py-0.5 rounded-sm ${
              selectedTab === 'all' 
              ? 'bg-secondary/40 text-foreground' 
              : 'text-muted-foreground hover:bg-secondary/20'
            }`}
          >
            All
          </button>
          <button 
            onClick={() => setSelectedTab('crypto')}
            className={`text-[11px] px-2 py-0.5 rounded-sm flex items-center ${
              selectedTab === 'crypto' 
              ? 'bg-secondary/40 text-foreground' 
              : 'text-muted-foreground hover:bg-secondary/20'
            }`}
          >
            <TrendingUp size={9} className="mr-1" />
            Crypto
          </button>
          <button 
            onClick={() => setSelectedTab('stocks')}
            className={`text-[11px] px-2 py-0.5 rounded-sm flex items-center ${
              selectedTab === 'stocks' 
              ? 'bg-secondary/40 text-foreground' 
              : 'text-muted-foreground hover:bg-secondary/20'
            }`}
          >
            <DollarSign size={9} className="mr-1" />
            Stocks
          </button>
        </div>
        
        <div className="overflow-hidden relative w-full">
          {(cryptoLoading && stockLoading && allPrices.length === 0) ? (
            <div className="text-[11px] text-muted-foreground">Loading market data...</div>
          ) : (
            <motion.div 
              animate={{ 
                x: [0, -7500], 
              }}
              transition={{ 
                repeat: Infinity, 
                duration: 120,
                ease: "linear"
              }}
              className="flex space-x-4 whitespace-nowrap"
            >
              {[...filteredPrices, ...filteredPrices, ...filteredPrices].map((item, index) => (
                <div key={`${item.symbol}-${index}`} className="flex items-center">
                  <span className={`text-[11px] font-medium flex items-center ${
                    item.type === 'stock' ? 'text-blue-400' : 'text-foreground'
                  }`}>
                    {item.type === 'stock' && <DollarSign size={8} className="mr-0.5" />}
                    {item.symbol}
                  </span>
                  <TooltipProvider>
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <span className="ml-1 text-[11px] text-muted-foreground cursor-help">{item.priceInSats}</span>
                      </TooltipTrigger>
                      <TooltipContent side="bottom" sideOffset={5} className="bg-[#1e1e1e] border-border text-white p-2 rounded">
                        <div className="space-y-1 text-xs">
                          <div className="grid grid-cols-2 gap-2">
                            <span className="text-muted-foreground">Symbol:</span>
                            <span className={`text-right font-medium ${
                              item.type === 'stock' ? 'text-blue-400' : 'text-foreground'
                            }`}>
                              {item.symbol}
                            </span>
                          </div>
                          <div className="grid grid-cols-2 gap-2">
                            <span className="text-muted-foreground">USD Price:</span>
                            <span className="text-right font-mono">{item.price}</span>
                          </div>
                          <div className="grid grid-cols-2 gap-2">
                            <span className="text-muted-foreground">Satoshi Price:</span>
                            <span className="text-right font-medium text-orange-400">{item.priceInSats}</span>
                          </div>
                          <div className="grid grid-cols-2 gap-2">
                            <span className="text-muted-foreground">24h Change:</span>
                            <span className={`text-right font-medium flex items-center justify-end ${
                              item.isPositive ? 'text-green-500' : 'text-red-500'
                            }`}>
                              {item.isPositive ? 
                                <TrendingUp size={9} className="mr-0.5" /> : 
                                <TrendingDown size={9} className="mr-0.5" />
                              }
                              {item.change}
                            </span>
                          </div>
                          <div className="grid grid-cols-2 gap-2">
                            <span className="text-muted-foreground">Type:</span>
                            <span className="text-right font-medium capitalize">{item.type}</span>
                          </div>
                        </div>
                      </TooltipContent>
                    </Tooltip>
                  </TooltipProvider>
                  <span className={`ml-1 flex items-center text-[11px] ${item.isPositive ? 'text-green-500' : 'text-red-500'}`}>
                    {item.isPositive ? 
                      <TrendingUp size={9} className="mr-0.5" /> : 
                      <TrendingDown size={9} className="mr-0.5" />
                    }
                    {item.change}
                  </span>
                </div>
              ))}
            </motion.div>
          )}
        </div>
      </div>
    </div>
  );
};

export default TickerBar; 