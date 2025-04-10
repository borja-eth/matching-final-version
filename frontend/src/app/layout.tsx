import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";
import { ApiProvider } from "../lib/api-context";
import { Toaster } from "sonner";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Roxom Exchange",
  description: "Next-generation Bitcoin exchange platform",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="dark">
      <body className={`${inter.className} custom-scrollbar`}>
        <ApiProvider>
          {children}
          <Toaster />
        </ApiProvider>
      </body>
    </html>
  );
}
