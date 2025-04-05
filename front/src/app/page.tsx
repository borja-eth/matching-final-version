'use client';

import { ApiProvider } from '@/lib/api-context';
import ThemeToggle from '@/components/theme-toggle';
import DashboardCard from '@/components/dashboard-card';
import NavBar from '@/components/nav-bar';
import { Toaster } from 'sonner';
import Footer from '@/components/footer';
import { 
  Activity, 
  ArrowUpDown, 
  BarChart4, 
  CircleDollarSign, 
  Clock, 
  DatabaseZap,
  Users
} from 'lucide-react';
import Link from 'next/link';
import { Button } from '@/components/ui/button';
import ApiStatus from '@/components/api-status';

export default function Home() {
  return (
    <ApiProvider>
      <div className="min-h-screen bg-background flex flex-col">
        <header className="border-b border-border py-4 px-6">
          <div className="flex flex-col gap-4">
            <div className="flex justify-between items-center">
              <h1 className="text-2xl font-bold">UMatching</h1>
              <ThemeToggle />
            </div>
            <NavBar />
          </div>
        </header>
        
        <main className="flex-1 px-6 py-4">
          <ApiStatus />
          
          <div className="grid grid-cols-12 gap-4 mt-4">
            <div className="col-span-12 md:col-span-6 lg:col-span-3">
              <DashboardCard
                title="API Status"
                value="Online"
                icon={DatabaseZap}
                description="Server is running at port 3001"
                trend="up"
                trendValue="99.9% uptime"
              />
            </div>
            <div className="col-span-12 md:col-span-6 lg:col-span-3">
              <DashboardCard
                title="BTC/USD Price"
                value="$36,428.52"
                icon={CircleDollarSign}
                trend="up"
                trendValue="+1.2% in 24h"
              />
            </div>
            <div className="col-span-12 md:col-span-6 lg:col-span-3">
              <DashboardCard
                title="Trading Volume"
                value="$1.2M"
                icon={BarChart4}
                description="Last 24 hours"
                trend="up"
                trendValue="+5.4% from yesterday"
              />
            </div>
            <div className="col-span-12 md:col-span-6 lg:col-span-3">
              <DashboardCard
                title="Active Orders"
                value="42"
                icon={Activity}
                description="Across all markets"
              />
            </div>

            <div className="col-span-12 md:col-span-6 mt-4">
              <div className="bg-card rounded-lg p-6 border h-full">
                <h2 className="text-xl font-bold mb-4">Recent Activity</h2>
                <div className="space-y-3">
                  <div className="flex items-center text-sm">
                    <Clock className="h-4 w-4 mr-2 text-muted-foreground" />
                    <span className="text-muted-foreground mr-2">09:45</span>
                    <span>Order placed: Buy 0.25 BTC @ $36,400</span>
                  </div>
                  <div className="flex items-center text-sm">
                    <Clock className="h-4 w-4 mr-2 text-muted-foreground" />
                    <span className="text-muted-foreground mr-2">09:30</span>
                    <span>Order filled: Sell 0.15 BTC @ $36,380</span>
                  </div>
                  <div className="flex items-center text-sm">
                    <Clock className="h-4 w-4 mr-2 text-muted-foreground" />
                    <span className="text-muted-foreground mr-2">09:15</span>
                    <span>Deposit confirmed: 0.5 BTC</span>
                  </div>
                </div>
              </div>
            </div>

            <div className="col-span-12 md:col-span-6 mt-4">
              <div className="bg-card rounded-lg p-6 border h-full">
                <h2 className="text-xl font-bold mb-4">Popular Markets</h2>
                <div className="space-y-3">
                  <div className="flex justify-between items-center">
                    <div className="flex items-center">
                      <ArrowUpDown className="h-4 w-4 mr-2" />
                      <span>BTC/USD</span>
                    </div>
                    <div className="text-green-500">$36,428.52 (+1.2%)</div>
                  </div>
                  <div className="flex justify-between items-center">
                    <div className="flex items-center">
                      <ArrowUpDown className="h-4 w-4 mr-2" />
                      <span>ETH/USD</span>
                    </div>
                    <div className="text-green-500">$1,945.32 (+0.8%)</div>
                  </div>
                  <div className="flex justify-between items-center">
                    <div className="flex items-center">
                      <ArrowUpDown className="h-4 w-4 mr-2" />
                      <span>SOL/USD</span>
                    </div>
                    <div className="text-red-500">$95.23 (-0.4%)</div>
                  </div>
                </div>
                <div className="mt-4">
                  <Link href="/exchange">
                    <Button size="sm">
                      Trade Now
                    </Button>
                  </Link>
                </div>
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
