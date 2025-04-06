'use client';

import { ApiProvider } from '@/lib/api-context';
import { Toaster } from 'sonner';
import { Card } from '@/components/ui/card';
import OrderBook from '@/components/order-book';
import Trades from '@/components/trades';
import OrderForm from '@/components/order-form';
import InstrumentSelector from '@/components/instrument-selector';
import ThemeToggle from '@/components/theme-toggle';
import NavBar from '@/components/nav-bar';
import ApiStatus from '@/components/api-status';
import { Activity } from 'lucide-react';
import Footer from '@/components/footer';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import PriceChart from '@/components/price-chart';

export default function ExchangePage() {
  return (
    <ApiProvider>
      <div className="min-h-screen bg-background flex flex-col">
        {/* Fixed header - 48px height */}
        <header className="h-12 border-b border-border flex items-center px-4 fixed w-full top-0 z-50 bg-background">
          <div className="flex items-center justify-between w-full">
            <div className="flex items-center gap-8">
              <h1 className="text-xl font-bold">UMatching</h1>
              <NavBar />
            </div>
            <div className="flex items-center gap-4">
              <ApiStatus />
              <InstrumentSelector />
              <ThemeToggle />
            </div>
          </div>
        </header>

        {/* Main content area */}
        <main className="flex-1 pt-12">
          <div className="grid grid-cols-12 h-[calc(100vh-3rem)]">
            {/* Left column: Chart and Trades */}
            <div className="col-span-8 border-r border-border">
              {/* Market info bar */}
              <div className="h-14 border-b border-border px-4 flex items-center">
                <div className="flex items-center gap-8">
                  <div className="flex items-center gap-2">
                    <span className="text-lg font-semibold">BTC/USDT</span>
                    <span className="text-sm text-muted-foreground">24h Change: +2.45%</span>
                  </div>
                  <div className="flex items-center gap-6 text-sm">
                    <div>
                      <div className="text-muted-foreground">24h High</div>
                      <div>$45,123.45</div>
                    </div>
                    <div>
                      <div className="text-muted-foreground">24h Low</div>
                      <div>$43,789.12</div>
                    </div>
                    <div>
                      <div className="text-muted-foreground">24h Volume (BTC)</div>
                      <div>1,234.56</div>
                    </div>
                  </div>
                </div>
              </div>

              {/* Chart section */}
              <div className="h-[60%] border-b border-border">
                <PriceChart />
              </div>
              
              {/* Trades section */}
              <div className="h-[40%]">
                <Trades />
              </div>
            </div>

            {/* Right column: Order Form and Order Book */}
            <div className="col-span-4 flex flex-col">
              {/* Order Form section */}
              <div className="h-[60%] border-b border-border">
                <OrderForm />
              </div>
              
              {/* Order Book section */}
              <div className="h-[40%]">
                <OrderBook />
              </div>
            </div>
          </div>
        </main>
        <Footer />
        <Toaster />
      </div>
    </ApiProvider>
  );
} 