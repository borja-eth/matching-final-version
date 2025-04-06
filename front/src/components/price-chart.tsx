'use client';

import { useEffect, useRef } from 'react';
import {
  createChart,
  ColorType,
  CrosshairMode,
  LineStyle,
  IChartApi,
  DeepPartial,
  ChartOptions,
  SeriesOptionsCommon,
  LineWidth,
  Time,
  CandlestickData,
  UTCTimestamp,
  CandlestickSeries
} from 'lightweight-charts';
import { useApi } from '@/lib/api-context';

export default function PriceChart() {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const { orderbook, selectedInstrument } = useApi();
  const candlesRef = useRef<CandlestickData[]>([]);
  const currentCandleRef = useRef<CandlestickData | null>(null);
  
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
        borderColor: '#2B2B43',
        timeVisible: true,
        secondsVisible: true,
        fixLeftEdge: true,
        fixRightEdge: true,
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

    // Create candlestick series
    const candlestickSeries = chart.addSeries(CandlestickSeries, {
      upColor: '#26A69A',
      downColor: '#EF454A',
      borderVisible: false,
      wickUpColor: '#26A69A',
      wickDownColor: '#EF454A',
      priceLineVisible: false,
    });

    // Initialize with current orderbook mid price
    const currentTime = Math.floor(Date.now() / 1000);
    if (orderbook.bids.length > 0 && orderbook.asks.length > 0) {
      const bestBid = parseFloat(orderbook.bids[0].price);
      const bestAsk = parseFloat(orderbook.asks[0].price);
      const midPrice = (bestBid + bestAsk) / 2;
      
      currentCandleRef.current = {
        time: currentTime as UTCTimestamp,
        open: midPrice,
        high: midPrice,
        low: midPrice,
        close: midPrice,
      };
      candlesRef.current = [currentCandleRef.current];
      candlestickSeries.setData(candlesRef.current);
    }

    // Update candle every 100ms
    const updateInterval = setInterval(() => {
      if (!currentCandleRef.current || !orderbook.bids.length || !orderbook.asks.length) return;

      const now = Math.floor(Date.now() / 1000);
      const currentCandleTime = currentCandleRef.current.time as number;
      
      if (now > currentCandleTime) {
        // Start a new candle
        const lastClose = currentCandleRef.current.close;
        const newCandle = {
          time: now as UTCTimestamp,
          open: lastClose,
          high: lastClose,
          low: lastClose,
          close: lastClose,
        };
        
        // Add the completed candle to history
        candlesRef.current.push(newCandle);
        // Keep last 100 candles
        if (candlesRef.current.length > 100) {
          candlesRef.current = candlesRef.current.slice(-100);
        }
        
        currentCandleRef.current = newCandle;
        candlestickSeries.setData(candlesRef.current);
      } else {
        // Update current candle
        const bestBid = parseFloat(orderbook.bids[0].price);
        const bestAsk = parseFloat(orderbook.asks[0].price);
        const currentPrice = (bestBid + bestAsk) / 2;
        
        currentCandleRef.current.high = Math.max(currentCandleRef.current.high, currentPrice);
        currentCandleRef.current.low = Math.min(currentCandleRef.current.low, currentPrice);
        currentCandleRef.current.close = currentPrice;
        
        candlestickSeries.update(currentCandleRef.current);
      }
    }, 100); // Update more frequently for smoother animation

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
      clearInterval(updateInterval);
      chart.remove();
    };
  }, [orderbook, selectedInstrument]);

  return (
    <div className="h-[400px] w-full bg-[var(--bds-gray-bg-card)]">
      <div className="flex items-center gap-4 px-4 py-2 border-b border-[var(--bds-gray-ele-border)]">
        <div className="flex items-center gap-2">
          <select className="h-6 px-2 text-[var(--bds-font-size-12)] bg-[var(--bds-gray-bg-float)] border-none rounded text-[var(--bds-gray-t2)]">
            <option>1s</option>
            <option>30m</option>
            <option>1h</option>
            <option>4h</option>
            <option>1d</option>
          </select>
          <div className="flex items-center gap-1">
            <button className="h-6 px-2 text-[var(--bds-font-size-12)] bg-[var(--bds-gray-bg-float)] rounded text-[var(--bds-gray-t2)] hover:bg-[var(--bds-trans-hover)]">
              Standard
            </button>
            <button className="h-6 px-2 text-[var(--bds-font-size-12)] bg-[var(--bds-gray-bg-float)] rounded text-[var(--bds-gray-t2)] hover:bg-[var(--bds-trans-hover)]">
              TradingView
            </button>
            <button className="h-6 px-2 text-[var(--bds-font-size-12)] bg-[var(--bds-gray-bg-float)] rounded text-[var(--bds-gray-t2)] hover:bg-[var(--bds-trans-hover)]">
              Depth
            </button>
          </div>
        </div>
        <div className="flex items-center gap-1">
          <button className="w-6 h-6 flex items-center justify-center text-[var(--bds-gray-t2)] hover:bg-[var(--bds-trans-hover)] rounded">
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7" />
            </svg>
          </button>
          <button className="w-6 h-6 flex items-center justify-center text-[var(--bds-gray-t2)] hover:bg-[var(--bds-trans-hover)] rounded">
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M4 14h6v6M14 4h6v6M4 14l7-7M20 10l-7 7" />
            </svg>
          </button>
          <button className="w-6 h-6 flex items-center justify-center text-[var(--bds-gray-t2)] hover:bg-[var(--bds-trans-hover)] rounded">
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M21 21l-6-6m-2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
          </button>
        </div>
      </div>
      <div ref={chartContainerRef} className="w-full h-full" />
    </div>
  );
} 