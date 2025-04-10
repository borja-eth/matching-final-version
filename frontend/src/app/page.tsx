"use client";

import React, { useState, useEffect } from 'react';
import Chart from '../components/chart';
import Orderbook from '../components/orderbook';
import Orderform from '../components/orderform';
import OrderManagementPanel from '../components/orderManagementPanel';
import Footer from '../components/footer';
import TickerBar from '../components/ticker-bar';
import { convertToSatoshis } from '../lib/utils';

interface BitcoinData {
  price: string;
  priceInSats: string;
  priceChange24h: string;
  isPositive: boolean;
}

const HomePage: React.FC = () => {
  const [bitcoinData, setBitcoinData] = useState<BitcoinData>({
    price: '$82,608.90',
    priceInSats: 'Loading...',
    priceChange24h: '+8.24%',
    isPositive: true
  });
  const [loading, setLoading] = useState(true);

  // Fetch Bitcoin price data from CoinGecko API
  const fetchBitcoinPrice = async () => {
    try {
      setLoading(true);
      const response = await fetch(
        'https://api.coingecko.com/api/v3/coins/bitcoin?localization=false&tickers=false&market_data=true&community_data=false&developer_data=false&sparkline=false'
      );
      
      if (!response.ok) {
        throw new Error('Network response was not ok');
      }
      
      const data = await response.json();
      
      const btcPrice = data.market_data.current_price.usd;
      const priceChange24h = data.market_data.price_change_percentage_24h || 0;
      const isPositive = priceChange24h >= 0;
      
      setBitcoinData({
        price: `$${btcPrice.toLocaleString(undefined, {
          minimumFractionDigits: 2,
          maximumFractionDigits: 2
        })}`,
        priceInSats: convertToSatoshis(btcPrice, btcPrice, true), // Calculate dynamically
        priceChange24h: `${isPositive ? '+' : ''}${priceChange24h.toFixed(2)}%`,
        isPositive
      });
    } catch (error) {
      console.error('Error fetching Bitcoin price data:', error);
      // Keep the existing values if there's an error
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    // Initial data fetch
    fetchBitcoinPrice();
    
    // Set up interval to fetch data periodically (every 30 seconds)
    const interval = setInterval(() => {
      fetchBitcoinPrice();
    }, 30000);
    
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="h-screen flex flex-col bg-background text-foreground exchange-page overflow-hidden">
      {/* Header */}
      <header className="border-b border-border/40 px-6 py-2 flex items-center justify-between shrink-0 bg-background/95 backdrop-blur-sm">
        <h1 className="text-xl font-bold text-foreground flex items-center">
          <span className="text-green-500 mr-1.5">$</span>
          Roxom Exchange
        </h1>
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium text-foreground">BTC/USDT:</span>
            <span className={`text-sm font-medium ${bitcoinData.isPositive ? 'text-green-500' : 'text-red-500'}`}>
              {loading ? "Loading..." : bitcoinData.price}
            </span>
            <span className={`text-sm ${bitcoinData.isPositive ? 'text-green-500' : 'text-red-500'}`}>
              {loading ? "..." : bitcoinData.priceChange24h}
            </span>
          </div>
          <div className="flex gap-2">
            <button className="bg-background border border-border hover:bg-secondary/20 text-foreground px-3 py-1 rounded-md text-sm">
              Log In
            </button>
            <button className="bg-green-600 hover:bg-green-700 text-white px-3 py-1 rounded-md text-sm">
              Sign Up
            </button>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex-1 p-2 overflow-hidden">
        <div className="grid grid-rows-[65%_auto] gap-2 h-full">
          {/* Top Row - Chart, Orderbook, Order Form */}
          <div className="grid grid-cols-12 gap-2 h-full">
            {/* Chart Section */}
            <div className="col-span-7 card-glass rounded-lg overflow-hidden border border-border/40">
              <div className="p-2 border-b border-border/40 flex items-center justify-between">
                <h2 className="text-sm font-bold">BTC/USDT</h2>
                <div className="flex items-center text-xs">
                  <div className="flex flex-col items-end">
                    <span className={`font-medium ${bitcoinData.isPositive ? 'text-green-500' : 'text-red-500'}`}>
                      {loading ? "Loading..." : bitcoinData.price}
                    </span>
                    <span className="text-[10px] text-muted-foreground">
                      {loading ? "..." : bitcoinData.priceInSats}
                    </span>
                  </div>
                  <span className={`ml-2 ${bitcoinData.isPositive ? 'text-green-500' : 'text-red-500'}`}>
                    {loading ? "..." : bitcoinData.priceChange24h}
                  </span>
                </div>
              </div>
              <div className="h-[calc(100%-32px)]">
                <Chart />
              </div>
            </div>

            {/* Orderbook Section */}
            <div className="col-span-3 card-glass rounded-lg overflow-hidden border border-border/40">
              <Orderbook />
            </div>

            {/* Order Form Section */}
            <div className="col-span-2 card-glass rounded-lg overflow-hidden border border-border/40">
              <Orderform />
            </div>
          </div>

          {/* Bottom Row - Order Management Panel */}
          <div className="card-glass rounded-lg overflow-hidden border border-border/40">
            <OrderManagementPanel />
          </div>
        </div>
      </div>
      
      {/* Ticker Bar */}
      <div className="shrink-0">
        <TickerBar />
      </div>
      
      {/* Footer */}
      <div className="shrink-0">
        <Footer />
      </div>
    </div>
  );
};

export default HomePage;
