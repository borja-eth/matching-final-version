'use client';

import ThemeToggle from '@/components/theme-toggle';

export default function Header() {
  return (
    <header className="toolbar dark:backdrop-blur-xl bg-background border-b border-border/20 py-3 px-6">
      <div className="container flex items-center justify-between max-w-7xl mx-auto">
        <h1 className="text-lg font-bold">Ultimate Matching</h1>
        <div className="flex items-center gap-4">
          <ThemeToggle />
        </div>
      </div>
    </header>
  );
} 