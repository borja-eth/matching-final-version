'use client';

import { usePathname } from 'next/navigation';
import Link from 'next/link';
import { Home, BarChart2, Settings, ChevronRight, Layers, AreaChart, Activity } from 'lucide-react';

import {
  Sidebar,
  SidebarContent,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  SidebarProvider,
  SidebarTrigger,
} from '@/components/ui/sidebar';

export default function MainSidebar() {
  const pathname = usePathname();
  
  return (
    <SidebarProvider defaultOpen={true}>
      <Sidebar className="border-r border-border">
        <SidebarHeader className="flex items-center gap-2 p-4">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="h-6 w-6"
          >
            <path d="M12 3L1 9l11 6 2-1.18" />
            <path d="M17.8 14 12 12l-11 6 11 6 11-6-5.2-2.9" />
            <path d="M3 16v4" />
            <path d="M16 3h-2c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h6c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2h-2" />
          </svg>
          <span className="text-lg font-bold">UMatching</span>
        </SidebarHeader>
        <SidebarContent>
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton
                asChild
                isActive={pathname === '/'}
                tooltip="Dashboard"
              >
                <Link href="/">
                  <Home className="mr-2" />
                  <span>Dashboard</span>
                </Link>
              </SidebarMenuButton>
            </SidebarMenuItem>
            <SidebarMenuItem>
              <SidebarMenuButton
                asChild
                isActive={pathname === '/exchange'}
                tooltip="Exchange"
              >
                <Link href="/exchange">
                  <BarChart2 className="mr-2" />
                  <span>Exchange</span>
                </Link>
              </SidebarMenuButton>
            </SidebarMenuItem>
            <SidebarMenuItem>
              <SidebarMenuButton
                asChild
                isActive={pathname === '/portfolio'}
                tooltip="Portfolio"
              >
                <Link href="/portfolio">
                  <Layers className="mr-2" />
                  <span>Portfolio</span>
                </Link>
              </SidebarMenuButton>
            </SidebarMenuItem>
            <SidebarMenuItem>
              <SidebarMenuButton
                asChild
                isActive={pathname === '/markets'}
                tooltip="Markets"
              >
                <Link href="/markets">
                  <AreaChart className="mr-2" />
                  <span>Markets</span>
                </Link>
              </SidebarMenuButton>
            </SidebarMenuItem>
            <SidebarMenuItem>
              <SidebarMenuButton
                asChild
                isActive={pathname === '/activity'}
                tooltip="Activity"
              >
                <Link href="/activity">
                  <Activity className="mr-2" />
                  <span>Activity</span>
                </Link>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarContent>
      </Sidebar>
    </SidebarProvider>
  );
} 