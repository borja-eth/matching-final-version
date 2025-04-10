'use client';

import { ApiProvider } from '@/lib/api-context';
import { Toaster } from 'sonner';
import TradingChart from '@/components/trading-chart';
import OrderBook from '@/components/order-book';
import TradingForm from '@/components/trading-form';
import AssetHeader from '@/components/asset-header';
import InstrumentSelector from '@/components/instrument-selector';
import ThemeToggle from '@/components/theme-toggle';
import NavBar from '@/components/nav-bar';
import ApiStatus from '@/components/api-status';
import Footer from '@/components/footer';

export default function ExchangePage() {
  return (
    <ApiProvider>
      <div className="min-h-screen bg-[#141414] flex flex-col text-white">
        {/* Fixed header */}
        <header className="h-12 border-b border-[#2B2B43] flex items-center px-4 fixed w-full top-0 z-50 bg-[#141414]">
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
          <div className="flex flex-col h-[calc(100vh-3rem)]">
            {/* Asset Header */}
            <AssetHeader />
            
            {/* Trading Interface */}
            <div className="flex flex-1 gap-4 p-4">
              {/* Chart */}
              <div className="flex-[3] bg-[#1F1F1F] rounded-lg overflow-hidden">
                <TradingChart />
              </div>

              {/* Order Book */}
              <div className="w-[280px] bg-[#1F1F1F] rounded-lg overflow-hidden">
                <OrderBook />
              </div>
              
              {/* Trading Form */}
              <div className="w-[320px] bg-[#1F1F1F] rounded-lg overflow-hidden">
                <TradingForm />
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