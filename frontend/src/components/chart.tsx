"use client";

import React, { useEffect, useRef, useState, useCallback } from 'react';
import { createChart, CandlestickSeries, ColorType, Time, UTCTimestamp, HistogramSeries } from 'lightweight-charts';
import { useApi } from '../lib/api-context';
import { Trade } from '../lib/api-client';
import { ToggleGroup, ToggleGroupItem } from './ui/toggle-group';
import { Button } from './ui/button';
import { RotateCcw } from 'lucide-react';

// Interface for candlestick data
interface CandlestickData {
  time: Time;
  open: number;
  high: number;
  low: number;
  close: number;
  volume?: number;
}

// Interface for volume data
interface VolumeData {
  time: Time;
  value: number;
  color?: string;
}

// Interface for trade aggregated by time period
interface AggregatedTrade {
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  trades: number;
}

// Available timeframes in milliseconds
type TimeframeKey = '100ms' | '1s' | '1m' | '5m' | '15m' | '1h' | '4h' | '1d';

const timeframes: Record<TimeframeKey, number> = {
  '100ms': 100, // 100 milliseconds for ultra-high-frequency trading
  '1s': 1000,
  '1m': 60 * 1000,
  '5m': 5 * 60 * 1000,
  '15m': 15 * 60 * 1000,
  '1h': 60 * 60 * 1000,
  '4h': 4 * 60 * 60 * 1000,
  '1d': 24 * 60 * 60 * 1000,
};

const Chart: React.FC = () => {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<any>(null);
  const candleSeriesRef = useRef<any>(null);
  const volumeSeriesRef = useRef<any>(null);
  const prevTradesRef = useRef<Trade[]>([]);
  const { trades } = useApi();
  const [candleData, setCandleData] = useState<CandlestickData[]>([]);
  const [volumeData, setVolumeData] = useState<VolumeData[]>([]);
  const [timeframe, setTimeframe] = useState<TimeframeKey>('100ms');

  // Function to check if there are new trades
  const hasNewTrades = useCallback((currentTrades: Trade[], previousTrades: Trade[]) => {
    if (currentTrades.length !== previousTrades.length) return true;
    if (currentTrades.length === 0) return false;
    
    // Check if the latest trade ID is different
    return currentTrades[0].id !== previousTrades[0].id;
  }, []);

  // Efficient update of latest candle with new trade data
  const updateLatestCandle = useCallback((
    tradesByTime: Map<number, AggregatedTrade>,
    lastCandleTime: number,
    existingCandles: CandlestickData[],
    existingVolumes: VolumeData[]
  ): [CandlestickData[], VolumeData[]] => {
    if (!tradesByTime.has(lastCandleTime)) return [existingCandles, existingVolumes];
    
    const updatedCandles = [...existingCandles];
    const updatedVolumes = [...existingVolumes];
    
    // Find the latest candle
    const latestCandleIndex = updatedCandles.findIndex(
      candle => (candle.time as number) === lastCandleTime
    );
    
    if (latestCandleIndex === -1) return [existingCandles, existingVolumes];
    
    // Get updated aggregated data
    const updatedData = tradesByTime.get(lastCandleTime)!;
    
    // Update the latest candle
    updatedCandles[latestCandleIndex] = {
      ...updatedCandles[latestCandleIndex],
      high: updatedData.high,
      low: updatedData.low,
      close: updatedData.close,
      volume: updatedData.volume
    };
    
    // Update the latest volume
    updatedVolumes[latestCandleIndex] = {
      ...updatedVolumes[latestCandleIndex],
      value: updatedData.volume,
      color: updatedData.close >= updatedData.open ? 'rgba(16, 185, 129, 0.5)' : 'rgba(239, 68, 68, 0.5)'
    };
    
    return [updatedCandles, updatedVolumes];
  }, []);

  // Aggregate trades into candlestick data based on the selected timeframe
  useEffect(() => {
    if (!trades || trades.length === 0) return;
    
    // Only process trades if there are new ones
    if (!hasNewTrades(trades, prevTradesRef.current)) return;
    prevTradesRef.current = trades;

    // Get the current timeframe in milliseconds
    const timeframeDuration = timeframes[timeframe];
    
    // Create a map to aggregate trades by time period
    const tradesByTime = new Map<number, AggregatedTrade>();
    
    // Track the current candle time
    const now = Date.now();
    const currentCandleTimestamp = Math.floor(now / timeframeDuration) * timeframeDuration / 1000;
    
    // Process all trades
    trades.forEach(trade => {
      // Parse trade data
      const price = parseFloat(trade.price);
      const amount = parseFloat(trade.base_amount);
      const tradeTime = new Date(trade.created_at).getTime();
      
      // Round down to the nearest timeframe interval
      const periodTimestamp = Math.floor(tradeTime / timeframeDuration) * timeframeDuration / 1000;
      
      // Get or create the aggregated trade for this period
      if (!tradesByTime.has(periodTimestamp)) {
        tradesByTime.set(periodTimestamp, {
          open: price,
          high: price,
          low: price,
          close: price,
          volume: amount,
          trades: 1
        });
      } else {
        const existingData = tradesByTime.get(periodTimestamp)!;
        tradesByTime.set(periodTimestamp, {
          open: existingData.open, // First price is the open
          high: Math.max(existingData.high, price),
          low: Math.min(existingData.low, price),
          close: price, // Last price is the close
          volume: existingData.volume + amount,
          trades: existingData.trades + 1
        });
      }
    });
    
    // Check if we need to update just the latest candle or rebuild all
    if (candleData.length > 0 && candleData.some(candle => (candle.time as number) === currentCandleTimestamp)) {
      // Efficient update - just update the current period's candle
      const [updatedCandles, updatedVolumes] = updateLatestCandle(
        tradesByTime, 
        currentCandleTimestamp, 
        candleData, 
        volumeData
      );
      
      setCandleData(updatedCandles);
      setVolumeData(updatedVolumes);
    } else {
      // Full rebuild of all candles
      const candles: CandlestickData[] = [];
      const volumes: VolumeData[] = [];
      
      Array.from(tradesByTime.entries())
        .sort((a, b) => (a[0]) - (b[0]))
        .forEach(([time, data]) => {
          candles.push({
            time: time as UTCTimestamp,
            open: data.open,
            high: data.high,
            low: data.low,
            close: data.close,
            volume: data.volume
          });
          
          volumes.push({
            time: time as UTCTimestamp,
            value: data.volume,
            // Green if close is higher than open, otherwise red
            color: data.close >= data.open ? 'rgba(16, 185, 129, 0.5)' : 'rgba(239, 68, 68, 0.5)'
          });
        });
      
      setCandleData(candles);
      setVolumeData(volumes);
    }
  }, [trades, timeframe, hasNewTrades, updateLatestCandle, candleData, volumeData]);

  // Create and update chart
  useEffect(() => {
    if (!chartContainerRef.current) return;

    // Create chart with dark theme
    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: 'transparent' },
        textColor: '#9ca3af',
        fontSize: 12,
        fontFamily: 'Inter, system-ui, sans-serif',
      },
      grid: {
        vertLines: { color: 'rgba(55, 65, 81, 0.3)' },
        horzLines: { color: 'rgba(55, 65, 81, 0.3)' },
      },
      crosshair: {
        horzLine: {
          color: '#9ca3af',
          labelBackgroundColor: '#374151',
        },
        vertLine: {
          color: '#9ca3af',
          labelBackgroundColor: '#374151',
        },
      },
      rightPriceScale: {
        borderColor: 'rgba(55, 65, 81, 0.5)',
        textColor: '#9ca3af',
        autoScale: true,
        mode: 1, // Normal price scale mode (not logarithmic)
        alignLabels: true,
        borderVisible: true,
        entireTextOnly: false,
        scaleMargins: {
          top: 0.1,  // 10% margin for top
          bottom: 0.2, // 20% margin for bottom 
        },
      },
      timeScale: {
        borderColor: 'rgba(55, 65, 81, 0.5)',
        timeVisible: true,
        secondsVisible: true,
        tickMarkFormatter: (time: number) => {
          const date = new Date(time * 1000);
          if (timeframe === '100ms') {
            return date.getHours() + ':' + 
                   date.getMinutes().toString().padStart(2, '0') + ':' +
                   date.getSeconds().toString().padStart(2, '0') + '.' +
                   date.getMilliseconds().toString().padStart(3, '0');
          } else if (timeframe === '1s') {
            return date.getHours() + ':' + 
                   date.getMinutes().toString().padStart(2, '0') + ':' +
                   date.getSeconds().toString().padStart(2, '0');
          }
          return date.getHours() + ':' + date.getMinutes().toString().padStart(2, '0');
        }
      },
      handleScroll: {
        mouseWheel: true,
        pressedMouseMove: true,
      },
      handleScale: {
        axisPressedMouseMove: true,
        mouseWheel: true,
        pinch: true,
      },
    });

    // Create candlestick series
    const candlestickSeries = chart.addSeries(CandlestickSeries, {
      upColor: '#10b981', // green
      downColor: '#ef4444', // red
      borderUpColor: '#10b981',
      borderDownColor: '#ef4444',
      wickUpColor: '#10b981',
      wickDownColor: '#ef4444',
      priceLineVisible: true,
      priceLineWidth: 1,
      priceLineColor: '#10b981',
      priceLineStyle: 2, // dashed
    });

    // Create volume histogram series
    const volumeSeries = chart.addSeries(HistogramSeries, {
      color: '#26a69a',
      priceFormat: {
        type: 'volume',
      },
      priceScaleId: '', // Create a separate scale for volume
    });

    // Configure the price scale for the volume series
    volumeSeries.priceScale().applyOptions({
      scaleMargins: {
        top: 0.8, // Position the volume series at the bottom 20% of the chart
        bottom: 0,
      },
      visible: false, // Hide the volume price scale
    });

    // Ensure the chart fits the container
    chart.applyOptions({
      width: chartContainerRef.current.clientWidth,
      height: chartContainerRef.current.clientHeight - 30, // Adjust height for timeframe selector
    });

    // Store references
    chartRef.current = chart;
    candleSeriesRef.current = candlestickSeries;
    volumeSeriesRef.current = volumeSeries;

    // Fit content to see all data
    chart.timeScale().fitContent();

    // Handle resize
    const handleResize = () => {
      if (chartContainerRef.current && chartRef.current) {
        chartRef.current.applyOptions({
          width: chartContainerRef.current.clientWidth,
          height: chartContainerRef.current.clientHeight - 30, // Adjust height for timeframe selector
        });
      }
    };

    window.addEventListener('resize', handleResize);

    // Create a ResizeObserver to detect container size changes
    const resizeObserver = new ResizeObserver(() => {
      handleResize();
    });
    
    if (chartContainerRef.current) {
      resizeObserver.observe(chartContainerRef.current);
    }

    // Cleanup
    return () => {
      window.removeEventListener('resize', handleResize);
      resizeObserver.disconnect();
      if (chartRef.current) {
        chartRef.current.remove();
        chartRef.current = null;
        candleSeriesRef.current = null;
        volumeSeriesRef.current = null;
      }
    };
  }, []);

  // Update chart data when candleData changes - optimized to only update the chart when needed
  useEffect(() => {
    if (!candleSeriesRef.current) return;
    
    if (candleData.length > 0) {
      // Use efficient update method if available to update only latest candle
      if (candleSeriesRef.current.update && candleData.length > 1) {
        const latestCandle = candleData[candleData.length - 1];
        candleSeriesRef.current.update(latestCandle);
        
        // Only fit content periodically for 1-second candles to avoid performance issues
        if (timeframe === '1s') {
          const now = new Date();
          if (now.getSeconds() % 5 === 0) { // Only fit every 5 seconds
            if (chartRef.current) {
              chartRef.current.timeScale().fitContent();
            }
          }
        } else {
          // For larger timeframes, fit content on each update
          if (chartRef.current) {
            chartRef.current.timeScale().fitContent();
          }
        }
      } else {
        // Fall back to setting all data
        candleSeriesRef.current.setData(candleData);
        
        // If we have data, fit content to see all data
        if (chartRef.current) {
          chartRef.current.timeScale().fitContent();
        }
      }
    } else if (candleSeriesRef.current && candleData.length === 0) {
      // If we have no data, use placeholder data to show the chart
      const currentTime = Math.floor(Date.now() / 1000);
      const timeframeDuration = timeframes[timeframe] / 1000;
      
      const placeholderCandleData = [
        { time: (currentTime - 5 * timeframeDuration) as UTCTimestamp, open: 267.50, high: 267.92, low: 267.30, close: 267.70 },
        { time: (currentTime - 4 * timeframeDuration) as UTCTimestamp, open: 267.70, high: 268.10, low: 267.65, close: 267.91 },
        { time: (currentTime - 3 * timeframeDuration) as UTCTimestamp, open: 267.91, high: 268.25, low: 267.88, close: 267.93 },
        { time: (currentTime - 2 * timeframeDuration) as UTCTimestamp, open: 267.93, high: 268.35, low: 267.90, close: 267.94 },
        { time: (currentTime - timeframeDuration) as UTCTimestamp, open: 267.94, high: 268.40, low: 267.85, close: 268.25 }
      ];
      candleSeriesRef.current.setData(placeholderCandleData);
    }
  }, [candleData, timeframe]);

  // Update volume data - optimized to only update when needed
  useEffect(() => {
    if (!volumeSeriesRef.current) return;
    
    if (volumeData.length > 0) {
      // Use efficient update method if available to update only latest volume
      if (volumeSeriesRef.current.update && volumeData.length > 1) {
        const latestVolume = volumeData[volumeData.length - 1];
        volumeSeriesRef.current.update(latestVolume);
      } else {
        // Fall back to setting all data
        volumeSeriesRef.current.setData(volumeData);
      }
    } else if (volumeSeriesRef.current && volumeData.length === 0) {
      // If we have no volume data, use placeholder data
      const currentTime = Math.floor(Date.now() / 1000);
      const timeframeDuration = timeframes[timeframe] / 1000;
      
      const placeholderVolumeData = [
        { time: (currentTime - 5 * timeframeDuration) as UTCTimestamp, value: 0.45, color: 'rgba(16, 185, 129, 0.5)' },
        { time: (currentTime - 4 * timeframeDuration) as UTCTimestamp, value: 0.62, color: 'rgba(16, 185, 129, 0.5)' },
        { time: (currentTime - 3 * timeframeDuration) as UTCTimestamp, value: 0.51, color: 'rgba(16, 185, 129, 0.5)' },
        { time: (currentTime - 2 * timeframeDuration) as UTCTimestamp, value: 0.39, color: 'rgba(239, 68, 68, 0.5)' },
        { time: (currentTime - timeframeDuration) as UTCTimestamp, value: 0.57, color: 'rgba(16, 185, 129, 0.5)' }
      ];
      volumeSeriesRef.current.setData(placeholderVolumeData);
    }
  }, [volumeData, timeframe]);

  // Handle timeframe change
  const handleTimeframeChange = (value: string) => {
    if (value in timeframes) {
      setTimeframe(value as TimeframeKey);
    }
  };

  // Function to scroll chart to real-time data
  const scrollToRealtime = useCallback(() => {
    if (chartRef.current) {
      chartRef.current.timeScale().scrollToRealTime();
    }
  }, []);

  return (
    <div className="h-full w-full flex flex-col">
      <div className="px-2 pt-1 pb-1 flex justify-between items-center">
        <ToggleGroup 
          type="single" 
          value={timeframe} 
          onValueChange={handleTimeframeChange}
          className="bg-background/20 rounded-sm p-0.5 text-xs"
        >
          <ToggleGroupItem value="100ms" size="sm" className="h-5 px-1.5 text-xs">100ms</ToggleGroupItem>
          <ToggleGroupItem value="1s" size="sm" className="h-5 px-1.5 text-xs">1s</ToggleGroupItem>
          <ToggleGroupItem value="1m" size="sm" className="h-5 px-1.5 text-xs">1m</ToggleGroupItem>
          <ToggleGroupItem value="5m" size="sm" className="h-5 px-1.5 text-xs">5m</ToggleGroupItem>
          <ToggleGroupItem value="15m" size="sm" className="h-5 px-1.5 text-xs">15m</ToggleGroupItem>
          <ToggleGroupItem value="1h" size="sm" className="h-5 px-1.5 text-xs">1h</ToggleGroupItem>
          <ToggleGroupItem value="4h" size="sm" className="h-5 px-1.5 text-xs">4h</ToggleGroupItem>
          <ToggleGroupItem value="1d" size="sm" className="h-5 px-1.5 text-xs">1d</ToggleGroupItem>
        </ToggleGroup>
        
        <Button 
          variant="ghost" 
          size="sm" 
          className="h-5 text-xs flex items-center gap-1"
          onClick={scrollToRealtime}
        >
          <RotateCcw className="h-3 w-3" />
          <span>Real-time</span>
        </Button>
      </div>
      <div 
        ref={chartContainerRef} 
        className="flex-1 w-full"
      />
    </div>
  );
};

export default Chart; 