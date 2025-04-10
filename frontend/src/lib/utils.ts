import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/**
 * Converts a price in USD to Satoshis (1 BTC = 100,000,000 SATS)
 * @param priceUSD - The price in USD
 * @param bitcoinPriceUSD - The current Bitcoin price in USD
 * @param isBitcoin - Whether the asset being converted is Bitcoin itself
 * @returns Formatted string representing the price in Satoshis
 */
export function convertToSatoshis(priceUSD: number, bitcoinPriceUSD: number, isBitcoin: boolean = false): string {
  // 1 BTC = 100,000,000 Satoshis
  const SATS_PER_BTC = 100000000;
  
  if (!bitcoinPriceUSD || bitcoinPriceUSD === 0) {
    return "N/A";
  }
  
  // If it's Bitcoin itself, calculate dynamically based on the amount
  if (isBitcoin) {
    // For 1 BTC, it's 100M SATS, but we want to be dynamic based on amount
    const satoshiValue = (priceUSD / bitcoinPriceUSD) * SATS_PER_BTC;
    
    // Format based on the amount
    if (priceUSD === bitcoinPriceUSD) {
      // If it's exactly 1 BTC
      return "100M SATS";
    } else {
      // Otherwise format normally based on the amount
      if (satoshiValue >= 1000000) {
        return `${(satoshiValue / 1000000).toLocaleString(undefined, { maximumFractionDigits: 2 })}M SATS`;
      } else if (satoshiValue >= 1000) {
        return `${(satoshiValue / 1000).toLocaleString(undefined, { maximumFractionDigits: 2 })}K SATS`;
      } else {
        return `${satoshiValue.toLocaleString(undefined, { maximumFractionDigits: 2 })} SATS`;
      }
    }
  }
  
  // Calculate the price in Satoshis: (asset_price_usd / btc_price_usd) * 100M
  const satoshiValue = (priceUSD / bitcoinPriceUSD) * SATS_PER_BTC;
  
  // Format based on size of the value
  if (satoshiValue >= 1000000) {
    return `${(satoshiValue / 1000000).toLocaleString(undefined, { maximumFractionDigits: 2 })}M SATS`;
  } else if (satoshiValue >= 1000) {
    return `${(satoshiValue / 1000).toLocaleString(undefined, { maximumFractionDigits: 2 })}K SATS`;
  } else {
    return `${satoshiValue.toLocaleString(undefined, { maximumFractionDigits: 2 })} SATS`;
  }
}

/**
 * Converts BTC amount to Satoshis
 * @param btcAmount - The amount in BTC
 * @returns Formatted string representing the amount in Satoshis
 */
export function btcToSatoshis(btcAmount: number): string {
  const SATS_PER_BTC = 100000000;
  const satoshiValue = btcAmount * SATS_PER_BTC;
  
  if (satoshiValue >= 1000000) {
    return `${(satoshiValue / 1000000).toLocaleString(undefined, { maximumFractionDigits: 2 })}M SATS`;
  } else if (satoshiValue >= 1000) {
    return `${(satoshiValue / 1000).toLocaleString(undefined, { maximumFractionDigits: 2 })}K SATS`;
  } else {
    return `${satoshiValue.toLocaleString(undefined, { maximumFractionDigits: 2 })} SATS`;
  }
}

