'use client';

import React, { useEffect, useRef, useState } from 'react';
import { createChart, ColorType, CrosshairMode, CandlestickSeries, Time } from 'lightweight-charts';
import { useApi } from '@/lib/api-context';
import { Trade } from '@/lib/api';

interface CandlestickData {
  time: Time;
  open: number;
  high: number;
  low: number;
  close: number;
  volume?: number;
}

interface Candle {
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  trades: number;
}

export default function TradingViewChart() {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<any>(null);
  const candlestickSeriesRef = useRef<any>(null);
  const volumeSeriesRef = useRef<any>(null);
  const currentCandleRef = useRef<Candle | null>(null);
  const candlesRef = useRef<Map<number, CandlestickData>>(new Map());
  const { trades, orderbook } = useApi();

  // Initialize chart
  useEffect(() => {
    if (!chartContainerRef.current) return;

    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: '#1F1F1F' },
        textColor: '#B2B5BE',
      },
      grid: {
        vertLines: { color: '#2B2B43' },
        horzLines: { color: '#2B2B43' },
      },
      crosshair: {
        mode: CrosshairMode.Normal,
        vertLine: {
          color: '#758696',
          width: 1,
          style: 1,
        },
        horzLine: {
          color: '#758696',
          width: 1,
          style: 1,
        },
      },
      timeScale: {
        borderColor: '#2B2B43',
        timeVisible: true,
        secondsVisible: true,
      },
      rightPriceScale: {
        borderColor: '#2B2B43',
        scaleMargins: {
          top: 0.1,
          bottom: 0.1,
        },
        autoScale: true,
      },
      width: chartContainerRef.current.clientWidth,
      height: chartContainerRef.current.clientHeight,
    });

    const candlestickSeries = chart.addSeries(CandlestickSeries, {
      upColor: '#26A69A',
      downColor: '#EF454A',
      borderUpColor: '#26A69A',
      borderDownColor: '#EF454A',
      wickUpColor: '#26A69A',
      wickDownColor: '#EF454A',
    });

    chartRef.current = chart;
    candlestickSeriesRef.current = candlestickSeries;

    const handleResize = () => {
      if (chartContainerRef.current) {
        chart.applyOptions({
          width: chartContainerRef.current.clientWidth,
          height: chartContainerRef.current.clientHeight,
        });
      }
    };

    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      chart.remove();
    };
  }, []);

  // Process trades and update candles
  useEffect(() => {
    if (!trades || !candlestickSeriesRef.current) return;

    // Process each trade
    trades.forEach(trade => {
      const timestamp = Math.floor(new Date(trade.created_at).getTime() / 1000);
      const price = parseFloat(trade.price);
      const volume = parseFloat(trade.base_amount);

      // Get or create candle for this timestamp
      if (!candlesRef.current.has(timestamp)) {
        candlesRef.current.set(timestamp, {
          time: timestamp as Time,
          open: price,
          high: price,
          low: price,
          close: price,
          volume: volume,
        });
      } else {
        const candle = candlesRef.current.get(timestamp)!;
        candlesRef.current.set(timestamp, {
          ...candle,
          high: Math.max(candle.high, price),
          low: Math.min(candle.low, price),
          close: price,
          volume: (candle.volume || 0) + volume,
        });
      }
    });

    // Convert Map to sorted array for the chart
    const sortedData = Array.from(candlesRef.current.values())
      .sort((a, b) => Number(a.time) - Number(b.time));

    // Keep only last 1000 candles
    if (sortedData.length > 1000) {
      const newData = sortedData.slice(-1000);
      candlesRef.current = new Map(newData.map(candle => [Number(candle.time), candle]));
    }

    // Update the chart
    candlestickSeriesRef.current.setData(sortedData);

    // If we have new data, scroll to it
    if (sortedData.length > 0) {
      chartRef.current.timeScale().scrollToRealTime();
    }
  }, [trades]);

  // Update current candle with orderbook data between trades
  useEffect(() => {
    if (!orderbook || !candlestickSeriesRef.current) return;

    const now = Math.floor(Date.now() / 1000);
    const bestBid = orderbook.bids[0]?.price ? parseFloat(orderbook.bids[0].price) : 0;
    const bestAsk = orderbook.asks[0]?.price ? parseFloat(orderbook.asks[0].price) : 0;
    
    if (!bestBid || !bestAsk) return;

    const midPrice = (bestBid + bestAsk) / 2;
    
    // Update or create current candle
    if (!candlesRef.current.has(now)) {
      candlesRef.current.set(now, {
        time: now as Time,
        open: midPrice,
        high: midPrice,
        low: midPrice,
        close: midPrice,
        volume: 0,
      });
    } else {
      const candle = candlesRef.current.get(now)!;
      if (candle.volume === 0) { // Only update if no trades in this candle
        candlesRef.current.set(now, {
          ...candle,
          high: Math.max(candle.high, midPrice),
          low: Math.min(candle.low, midPrice),
          close: midPrice,
        });
      }
    }

    // Update the chart with latest data
    const sortedData = Array.from(candlesRef.current.values())
      .sort((a, b) => Number(a.time) - Number(b.time));
    
    candlestickSeriesRef.current.setData(sortedData);
    chartRef.current.timeScale().scrollToRealTime();
  }, [orderbook]);

  return <div ref={chartContainerRef} className="w-full h-full" />;
} 