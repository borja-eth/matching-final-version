import "@/app/globals.css";
import { Inter } from "next/font/google";
import { ThemeProvider } from "@/components/theme-provider";
import { Toaster } from "sonner";
import type { Metadata } from "next";

const inter = Inter({ 
  subsets: ["latin"],
  variable: "--font-sans" 
});

export const metadata: Metadata = {
  title: "UMatching - Cryptocurrency Exchange",
  description: "A high-performance cryptocurrency matching engine and exchange",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className={`${inter.variable} font-sans antialiased`}>
        <ThemeProvider attribute="class" defaultTheme="system" enableSystem>
          {children}
          <Toaster />
        </ThemeProvider>
      </body>
    </html>
  );
}
