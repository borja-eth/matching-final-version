'use client';

import { usePathname } from 'next/navigation';
import Link from 'next/link';
import { Home, BarChart2, Layers, AreaChart, Activity } from 'lucide-react';
import { cn } from '@/lib/utils';

const navItems = [
  { href: '/', label: 'Dashboard', icon: Home },
  { href: '/exchange', label: 'Exchange', icon: BarChart2 },
  { href: '/portfolio', label: 'Portfolio', icon: Layers },
  { href: '/markets', label: 'Markets', icon: AreaChart },
  { href: '/activity', label: 'Activity', icon: Activity },
];

export default function NavBar() {
  const pathname = usePathname();
  
  return (
    <nav className="flex items-center space-x-4 lg:space-x-6">
      {navItems.map((item) => {
        const Icon = item.icon;
        const isActive = pathname === item.href;
        
        return (
          <Link
            key={item.href}
            href={item.href}
            className={cn(
              "flex items-center text-sm font-medium transition-colors hover:text-primary",
              isActive 
                ? "text-primary" 
                : "text-muted-foreground"
            )}
          >
            <Icon className="h-4 w-4 mr-2" />
            <span>{item.label}</span>
          </Link>
        );
      })}
    </nav>
  );
} 