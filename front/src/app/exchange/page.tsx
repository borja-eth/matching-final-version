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
import { Activity, AlertCircle } from 'lucide-react';
import Footer from '@/components/footer';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';

export default function ExchangePage() {
  return (
    <ApiProvider>
      <div className="min-h-screen bg-background flex flex-col">
        <header className="border-b border-border py-4 px-6">
          <div className="flex flex-col gap-4">
            <div className="flex justify-between items-center">
              <h1 className="text-2xl font-bold">UMatching</h1>
              <ThemeToggle />
            </div>
            <div className="flex justify-between items-center">
              <NavBar />
              <InstrumentSelector />
            </div>
          </div>
        </header>
        
        <main className="flex-1 px-6 py-4">
          <ApiStatus />
          
          <div className="grid grid-cols-12 gap-4 mt-4">
            {/* Chart section */}
            <div className="col-span-12 lg:col-span-8">
              <Card className="h-[400px]">
                <div className="p-4">
                  <h2 className="text-lg font-medium mb-2">Price Chart</h2>
                  <div className="flex items-center justify-center h-[350px]">
                    <div className="text-muted-foreground flex flex-col items-center">
                      <Activity className="h-12 w-12 mb-2 opacity-20" />
                      <p>Trading chart would appear here</p>
                    </div>
                  </div>
                </div>
              </Card>
            </div>
            
            {/* Order form section */}
            <div className="col-span-12 lg:col-span-4">
              <OrderForm />
            </div>
            
            {/* Order book section */}
            <div className="col-span-12 lg:col-span-6">
              <Card>
                <OrderBook />
              </Card>
            </div>
            
            {/* Trades section */}
            <div className="col-span-12 lg:col-span-6">
              <Card>
                <Trades />
              </Card>
            </div>
          </div>
        </main>
        <Footer />
        <Toaster />
      </div>
    </ApiProvider>
  );
} 