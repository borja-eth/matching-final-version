"use client";

import React, { useState } from 'react';
import { motion } from 'framer-motion';
import { Tabs, TabsContent, TabsList, TabsTrigger } from './ui/tabs';
import { Table, TableBody, TableCell, TableHead, TableRow } from './ui/table';
import { Button } from './ui/button';
import { Badge } from './ui/badge';

interface Order {
  id: string;
  pair: string;
  type: 'buy' | 'sell';
  orderType: 'limit' | 'market';
  price: number;
  amount: number;
  filled: number;
  total: number;
  status: 'open' | 'partial' | 'filled' | 'cancelled';
  date: Date;
}

interface Trade {
  id: string;
  pair: string;
  type: 'buy' | 'sell';
  price: number;
  amount: number;
  total: number;
  fee: number;
  date: Date;
}

const OrderManagementPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState('current');
  
  // Sample data
  const currentOrders: Order[] = [
    {
      id: '1234569',
      pair: 'BTC/USDT',
      type: 'buy',
      orderType: 'limit',
      price: 82540.0,
      amount: 0.05,
      filled: 0.0,
      total: 4127.0,
      status: 'open',
      date: new Date('2024-05-11T10:15:00')
    }
  ];
  
  const orderHistory: Order[] = [
    {
      id: '1234567',
      pair: 'BTC/USDT',
      type: 'buy',
      orderType: 'limit',
      price: 82745.4,
      amount: 0.15,
      filled: 0.15,
      total: 12411.81,
      status: 'filled',
      date: new Date('2024-05-10T14:30:00')
    },
    {
      id: '1234568',
      pair: 'BTC/USDT',
      type: 'sell',
      orderType: 'market',
      price: 82746.4,
      amount: 0.08,
      filled: 0.08,
      total: 6619.71,
      status: 'filled',
      date: new Date('2024-05-09T09:15:00')
    }
  ];
  
  const tradeHistory: Trade[] = [
    {
      id: 'T1234567',
      pair: 'BTC/USDT',
      type: 'buy',
      price: 82745.4,
      amount: 0.15,
      total: 12411.81,
      fee: 12.41,
      date: new Date('2024-05-10T14:30:00')
    },
    {
      id: 'T1234568',
      pair: 'BTC/USDT',
      type: 'sell',
      price: 82746.4,
      amount: 0.08,
      total: 6619.71,
      fee: 6.62,
      date: new Date('2024-05-09T09:15:00')
    }
  ];

  // Format date helper
  const formatDate = (date: Date) => {
    return date.toLocaleString('en-US', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      hour12: false
    });
  };

  // Animation variants
  const itemVariants = {
    hidden: { opacity: 0 },
    visible: (i: number) => ({
      opacity: 1,
      transition: {
        delay: i * 0.05,
        duration: 0.2,
      },
    }),
  };

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 0.3 }}
      className="h-full flex flex-col"
    >
      <Tabs 
        defaultValue="current" 
        value={activeTab}
        onValueChange={setActiveTab}
        className="flex flex-col h-full"
      >
        <div className="border-b border-border/40 p-2 flex justify-between items-center">
          <h2 className="text-sm font-bold">Orders & History</h2>
          <TabsList className="bg-transparent border border-border/40 p-0.5 h-7">
            <TabsTrigger 
              value="current" 
              className="text-xs px-2 py-0.5 h-full data-[state=active]:bg-secondary/80 data-[state=active]:text-foreground rounded-sm"
            >
              Open Orders
            </TabsTrigger>
            <TabsTrigger 
              value="history" 
              className="text-xs px-2 py-0.5 h-full data-[state=active]:bg-secondary/80 data-[state=active]:text-foreground rounded-sm"
            >
              Order History
            </TabsTrigger>
            <TabsTrigger 
              value="trades" 
              className="text-xs px-2 py-0.5 h-full data-[state=active]:bg-secondary/80 data-[state=active]:text-foreground rounded-sm"
            >
              Trade History
            </TabsTrigger>
          </TabsList>
        </div>
        
        <div className="flex-1 overflow-hidden">
          <TabsContent value="current" className="m-0 h-full">
            {currentOrders.length === 0 ? (
              <div className="flex justify-center items-center h-full">
                <p className="text-sm text-muted-foreground">No open orders</p>
              </div>
            ) : (
              <div className="overflow-x-auto h-full">
                <Table>
                  <thead className="text-xs text-muted-foreground sticky top-0 bg-background border-b border-border/30">
                    <tr>
                      <TableHead className="py-1">Date/Time</TableHead>
                      <TableHead>Pair</TableHead>
                      <TableHead>Type</TableHead>
                      <TableHead className="text-right">Price</TableHead>
                      <TableHead className="text-right">Amount</TableHead>
                      <TableHead className="text-right">Filled</TableHead>
                      <TableHead className="text-right">Total</TableHead>
                      <TableHead>Action</TableHead>
                    </tr>
                  </thead>
                  <tbody>
                    {currentOrders.map((order, index) => (
                      <motion.tr
                        key={order.id}
                        custom={index}
                        initial="hidden"
                        animate="visible"
                        variants={itemVariants}
                        className="border-b border-border/10 hover:bg-accent/10"
                      >
                        <TableCell className="text-xs py-1">{formatDate(order.date)}</TableCell>
                        <TableCell className="text-xs">{order.pair}</TableCell>
                        <TableCell>
                          <Badge 
                            variant={order.type === 'buy' ? 'default' : 'destructive'} 
                            className={`text-xs py-0 px-2 ${
                              order.type === 'buy' 
                                ? 'bg-green-600 hover:bg-green-700' 
                                : 'bg-red-500 hover:bg-red-600'
                            }`}
                          >
                            {order.type.toUpperCase()}
                          </Badge>
                        </TableCell>
                        <TableCell className="text-xs text-right">{order.price.toLocaleString()}</TableCell>
                        <TableCell className="text-xs text-right">{order.amount.toFixed(5)}</TableCell>
                        <TableCell className="text-xs text-right">{(order.filled / order.amount * 100).toFixed(1)}%</TableCell>
                        <TableCell className="text-xs text-right">{order.total.toLocaleString()}</TableCell>
                        <TableCell>
                          <Button size="sm" variant="ghost" className="text-xs h-6 px-2 text-red-400 hover:text-red-300 hover:bg-red-900/20">
                            Cancel
                          </Button>
                        </TableCell>
                      </motion.tr>
                    ))}
                  </tbody>
                </Table>
              </div>
            )}
          </TabsContent>
          
          <TabsContent value="history" className="m-0 h-full">
            {orderHistory.length === 0 ? (
              <div className="flex justify-center items-center h-full">
                <p className="text-sm text-muted-foreground">No order history</p>
              </div>
            ) : (
              <div className="overflow-x-auto h-full">
                <Table>
                  <thead className="text-xs text-muted-foreground sticky top-0 bg-background border-b border-border/30">
                    <tr>
                      <TableHead className="py-2">Date/Time</TableHead>
                      <TableHead>Pair</TableHead>
                      <TableHead>Type</TableHead>
                      <TableHead className="text-right">Price</TableHead>
                      <TableHead className="text-right">Amount</TableHead>
                      <TableHead className="text-right">Total</TableHead>
                      <TableHead>Status</TableHead>
                    </tr>
                  </thead>
                  <tbody>
                    {orderHistory.map((order, index) => (
                      <motion.tr
                        key={order.id}
                        custom={index}
                        initial="hidden"
                        animate="visible"
                        variants={itemVariants}
                        className="border-b border-border/10 hover:bg-accent/10"
                      >
                        <TableCell className="text-xs py-2">{formatDate(order.date)}</TableCell>
                        <TableCell className="text-xs">{order.pair}</TableCell>
                        <TableCell>
                          <Badge 
                            variant={order.type === 'buy' ? 'default' : 'destructive'} 
                            className={`text-xs py-0 px-2 ${
                              order.type === 'buy' 
                                ? 'bg-green-600 hover:bg-green-700' 
                                : 'bg-red-500 hover:bg-red-600'
                            }`}
                          >
                            {order.type.toUpperCase()}
                          </Badge>
                        </TableCell>
                        <TableCell className="text-xs text-right">{order.price.toLocaleString()}</TableCell>
                        <TableCell className="text-xs text-right">{order.amount.toFixed(5)}</TableCell>
                        <TableCell className="text-xs text-right">{order.total.toLocaleString()}</TableCell>
                        <TableCell>
                          <Badge 
                            variant="outline" 
                            className="text-xs py-0 px-2 border-green-500/30 text-green-500"
                          >
                            {order.status.toUpperCase()}
                          </Badge>
                        </TableCell>
                      </motion.tr>
                    ))}
                  </tbody>
                </Table>
              </div>
            )}
          </TabsContent>
          
          <TabsContent value="trades" className="m-0 h-full">
            {tradeHistory.length === 0 ? (
              <div className="flex justify-center items-center h-full">
                <p className="text-sm text-muted-foreground">No trade history</p>
              </div>
            ) : (
              <div className="overflow-x-auto h-full">
                <Table>
                  <thead className="text-xs text-muted-foreground sticky top-0 bg-background border-b border-border/30">
                    <tr>
                      <TableHead className="py-2">Date/Time</TableHead>
                      <TableHead>Pair</TableHead>
                      <TableHead>Side</TableHead>
                      <TableHead className="text-right">Price</TableHead>
                      <TableHead className="text-right">Amount</TableHead>
                      <TableHead className="text-right">Total</TableHead>
                      <TableHead className="text-right">Fee</TableHead>
                    </tr>
                  </thead>
                  <tbody>
                    {tradeHistory.map((trade, index) => (
                      <motion.tr
                        key={trade.id}
                        custom={index}
                        initial="hidden"
                        animate="visible"
                        variants={itemVariants}
                        className="border-b border-border/10 hover:bg-accent/10"
                      >
                        <TableCell className="text-xs py-2">{formatDate(trade.date)}</TableCell>
                        <TableCell className="text-xs">{trade.pair}</TableCell>
                        <TableCell>
                          <Badge 
                            variant={trade.type === 'buy' ? 'default' : 'destructive'} 
                            className={`text-xs py-0 px-2 ${
                              trade.type === 'buy' 
                                ? 'bg-green-600 hover:bg-green-700' 
                                : 'bg-red-500 hover:bg-red-600'
                            }`}
                          >
                            {trade.type.toUpperCase()}
                          </Badge>
                        </TableCell>
                        <TableCell className="text-xs text-right">{trade.price.toLocaleString()}</TableCell>
                        <TableCell className="text-xs text-right">{trade.amount.toFixed(5)}</TableCell>
                        <TableCell className="text-xs text-right">{trade.total.toLocaleString()}</TableCell>
                        <TableCell className="text-xs text-right">{trade.fee.toFixed(2)}</TableCell>
                      </motion.tr>
                    ))}
                  </tbody>
                </Table>
              </div>
            )}
          </TabsContent>
        </div>
      </Tabs>
    </motion.div>
  );
};

export default OrderManagementPanel; 