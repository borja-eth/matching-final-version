'use client';

import { useEffect, useRef } from 'react';
import {
  createChart,
  ColorType,
  CrosshairMode,
  LineStyle,
  DeepPartial,
  ChartOptions,
  LineWidth,
  AreaData,
  UTCTimestamp,
  AreaSeries,
} from 'lightweight-charts';
import { useApi } from '@/lib/api-context';

interface CumulativeOrder {
  price: number;
  total: number;
}

interface DepthData {
  bids: CumulativeOrder[];
  asks: CumulativeOrder[];
  midPrice: number;
}

export default function DepthChart() {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const { orderbook, selectedInstrument } = useApi();

  // Process orderbook data into cumulative depth
  const processDepthData = (): DepthData => {
    if (!orderbook) return { bids: [], asks: [], midPrice: 0 };

    // Get the mid price to center the chart
    const midPrice = orderbook.bids.length && orderbook.asks.length
      ? (parseFloat(orderbook.bids[0].price) + parseFloat(orderbook.asks[0].price)) / 2
      : 0;

    // Sort orders by price
    const bids = [...orderbook.bids]
      .sort((a, b) => parseFloat(a.price) - parseFloat(b.price)) // Sort ascending for chart
      .map(order => ({
        price: parseFloat(order.price),
        amount: parseFloat(order.volume)
      }));

    const asks = [...orderbook.asks]
      .sort((a, b) => parseFloat(a.price) - parseFloat(b.price))
      .map(order => ({
        price: parseFloat(order.price),
        amount: parseFloat(order.volume)
      }));

    // Calculate cumulative totals (reverse for bids to show correct depth)
    let bidTotal = 0;
    const bidDepth: CumulativeOrder[] = [...bids].reverse().map(bid => {
      bidTotal += bid.amount;
      return { price: bid.price, total: bidTotal };
    }).reverse(); // Reverse back to ascending price order

    let askTotal = 0;
    const askDepth: CumulativeOrder[] = asks.map(ask => {
      askTotal += ask.amount;
      return { price: ask.price, total: askTotal };
    });

    return {
      bids: bidDepth,
      asks: askDepth,
      midPrice,
    };
  };

  useEffect(() => {
    if (!chartContainerRef.current || !orderbook || !selectedInstrument) return;

    const chartOptions: DeepPartial<ChartOptions> = {
      layout: {
        background: { type: ColorType.Solid, color: '#1C1C28' },
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
          width: 1 as LineWidth,
          style: LineStyle.Dashed,
        },
        horzLine: {
          color: '#758696',
          width: 1 as LineWidth,
          style: LineStyle.Dashed,
        },
      },
      timeScale: {
        visible: true,
        borderColor: '#2B2B43',
      },
      rightPriceScale: {
        borderColor: '#2B2B43',
        autoScale: true,
      },
    };

    const chart = createChart(chartContainerRef.current, {
      ...chartOptions,
      width: chartContainerRef.current.clientWidth,
      height: 400,
    });

    // Create bid and ask area series
    const bidSeries = chart.addSeries(AreaSeries, {
      topColor: 'rgba(38, 166, 154, 0.4)',
      bottomColor: 'rgba(38, 166, 154, 0.0)',
      lineColor: 'rgba(38, 166, 154, 1)',
      lineWidth: 2,
      priceFormat: {
        type: 'price',
        precision: 2,
        minMove: 0.01,
      },
    });

    const askSeries = chart.addSeries(AreaSeries, {
      topColor: 'rgba(239, 69, 74, 0.4)',
      bottomColor: 'rgba(239, 69, 74, 0.0)',
      lineColor: 'rgba(239, 69, 74, 1)',
      lineWidth: 2,
      priceFormat: {
        type: 'price',
        precision: 2,
        minMove: 0.01,
      },
    });

    // Update function
    const updateDepthChart = () => {
      const { bids, asks, midPrice } = processDepthData();

      // Convert to the format expected by lightweight-charts
      const bidData: AreaData[] = bids.map(bid => ({
        time: bid.price as UTCTimestamp,
        value: bid.total,
      }));

      const askData: AreaData[] = asks.map(ask => ({
        time: ask.price as UTCTimestamp,
        value: ask.total,
      }));

      bidSeries.setData(bidData);
      askSeries.setData(askData);

      // Fit content and center around mid price
      chart.timeScale().fitContent();
      
      // Set visible range to show reasonable depth
      if (midPrice > 0) {
        const priceRange = midPrice! * 0.02; // Show ±2% of mid price
        chart.timeScale().setVisibleRange({
          from: (midPrice! - priceRange) as UTCTimestamp,
          to: (midPrice! + priceRange) as UTCTimestamp,
        });
      }
    };

    // Initial update
    updateDepthChart();

    // Update on orderbook changes
    const interval = setInterval(updateDepthChart, 1000);

    // Handle window resize
    const handleResize = () => {
      if (chartContainerRef.current) {
        chart.applyOptions({
          width: chartContainerRef.current.clientWidth,
        });
      }
    };

    window.addEventListener('resize', handleResize);

    // Cleanup
    return () => {
      window.removeEventListener('resize', handleResize);
      clearInterval(interval);
      chart.remove();
    };
  }, [orderbook, selectedInstrument]);

  return (
    <div className="h-[400px] w-full bg-[var(--bds-gray-bg-card)]">
      <div className="flex items-center justify-between px-4 py-2 border-b border-[var(--bds-gray-ele-border)]">
        <h2 className="text-sm font-medium">Depth Chart</h2>
      </div>
      <div ref={chartContainerRef} className="w-full h-full" />
    </div>
  );
} 