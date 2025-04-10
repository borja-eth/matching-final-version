"use client";

import React, { useEffect, useRef } from 'react';
import { createChart, LineSeries, ColorType } from 'lightweight-charts';

const Chart: React.FC = () => {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<any>(null);
  const seriesRef = useRef<any>(null);

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
      },
      timeScale: {
        borderColor: 'rgba(55, 65, 81, 0.5)',
        timeVisible: true,
        secondsVisible: false,
        tickMarkFormatter: (time: number) => {
          const date = new Date(time * 1000);
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

    // Create line series with green color for price line
    const lineSeries = chart.addSeries(LineSeries, {
      color: '#10b981',
      lineWidth: 2,
      crosshairMarkerVisible: true,
      crosshairMarkerRadius: 4,
      crosshairMarkerBorderWidth: 1,
      crosshairMarkerBackgroundColor: '#10b981',
      crosshairMarkerBorderColor: '#ffffff',
      lastPriceAnimation: 1,
      priceLineVisible: true,
      priceLineWidth: 1,
      priceLineColor: '#10b981',
      priceLineStyle: 2, // dashed
    });

    // Sample data with timestamps
    const sampleData = [
      { time: '2023-01-01', value: 82000 },
      { time: '2023-01-02', value: 81500 },
      { time: '2023-01-03', value: 82400 },
      { time: '2023-01-04', value: 83100 },
      { time: '2023-01-05', value: 82900 },
      { time: '2023-01-06', value: 83200 },
      { time: '2023-01-07', value: 84100 },
      { time: '2023-01-08', value: 83800 },
      { time: '2023-01-09', value: 84300 },
      { time: '2023-01-10', value: 84200 },
      { time: '2023-01-11', value: 83900 },
      { time: '2023-01-12', value: 84600 },
      { time: '2023-01-13', value: 84500 },
      { time: '2023-01-14', value: 84100 },
      { time: '2023-01-15', value: 82600 },
      { time: '2023-01-16', value: 83100 },
      { time: '2023-01-17', value: 84500 },
      { time: '2023-01-18', value: 82600 },
      { time: '2023-01-19', value: 81400 },
      { time: '2023-01-20', value: 82608.9 },
    ];

    lineSeries.setData(sampleData);

    // Ensure the chart fits the container
    chart.applyOptions({
      width: chartContainerRef.current.clientWidth,
      height: chartContainerRef.current.clientHeight,
    });

    // Store references
    chartRef.current = chart;
    seriesRef.current = lineSeries;

    // Fit content to see all data
    chart.timeScale().fitContent();

    // Handle resize
    const handleResize = () => {
      if (chartContainerRef.current && chartRef.current) {
        chartRef.current.applyOptions({
          width: chartContainerRef.current.clientWidth,
          height: chartContainerRef.current.clientHeight,
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
        seriesRef.current = null;
      }
    };
  }, []);

  return (
    <div 
      ref={chartContainerRef} 
      className="h-full w-full"
    />
  );
};

export default Chart; 