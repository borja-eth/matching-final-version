/**
 * API Client for the Ultimate Matching Engine
 */

// Define the base URL for the API
const API_BASE_URL = 'http://localhost:3001';

// Types based on the Rust API DTOs
export interface PriceLevel {
  price: string;
  volume: string;
  order_count: number;
}

export interface Depth {
  bids: PriceLevel[];
  asks: PriceLevel[];
  timestamp: string;
  instrument_id: string;
}

export interface Trade {
  id: string;
  instrument_id: string;
  maker_order_id: string;
  taker_order_id: string;
  base_amount: string;
  quote_amount: string;
  price: string;
  created_at: string;
}

export interface Instrument {
  id: string;
  name: string;
  base_currency: string;
  quote_currency: string;
}

export interface Order {
  id: string;
  ext_id?: string;
  account_id: string;
  order_type: 'Limit' | 'Market';
  instrument_id: string;
  side: 'Buy' | 'Sell';
  limit_price?: string;
  trigger_price?: string;
  base_amount: string;
  remaining_base: string;
  filled_base: string;
  filled_quote: string;
  status: 'New' | 'PartiallyFilled' | 'Filled' | 'Cancelled' | 'Rejected';
  created_at: string;
  updated_at: string;
}

export interface CreateOrderRequest {
  ext_id?: string;
  account_id: string;
  order_type: 'Limit' | 'Market';
  instrument_id: string;
  side: 'Buy' | 'Sell';
  limit_price?: string;
  trigger_price?: string;
  base_amount: string;
  time_in_force?: 'GTC' | 'IOC' | 'FOK';
}

/**
 * API Client for interacting with the matching engine
 */
export class MatchingEngineApi {
  private baseUrl: string;

  constructor(baseUrl: string = API_BASE_URL) {
    this.baseUrl = baseUrl;
  }

  /**
   * Check if the API is healthy
   */
  async healthCheck(): Promise<boolean> {
    try {
      const response = await fetch(`${this.baseUrl}/health`);
      return response.ok;
    } catch (error) {
      console.error('Health check failed:', error);
      return false;
    }
  }

  /**
   * Get a list of available instruments
   */
  async getInstruments(): Promise<Instrument[]> {
    try {
      const response = await fetch(`${this.baseUrl}/instruments`);
      if (!response.ok) {
        throw new Error(`Failed to fetch instruments: ${response.status}`);
      }
      return await response.json();
    } catch (error) {
      console.error('Error fetching instruments:', error);
      return [];
    }
  }

  /**
   * Create a new instrument
   */
  async createInstrument(name: string, base: string, quote: string): Promise<Instrument | null> {
    try {
      const response = await fetch(`${this.baseUrl}/instruments`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          name,
          base_currency: base,
          quote_currency: quote,
        }),
      });

      if (!response.ok) {
        throw new Error(`Failed to create instrument: ${response.status}`);
      }

      return await response.json();
    } catch (error) {
      console.error('Error creating instrument:', error);
      return null;
    }
  }

  /**
   * Get orderbook data for an instrument
   */
  async getOrderbook(instrumentId: string): Promise<Depth | null> {
    try {
      const response = await fetch(`${this.baseUrl}/instruments/${instrumentId}/orderbook`);
      if (!response.ok) {
        throw new Error(`Failed to fetch orderbook: ${response.status}`);
      }
      return await response.json();
    } catch (error) {
      console.error('Error fetching orderbook:', error);
      return null;
    }
  }

  /**
   * Get market depth for an instrument
   */
  async getDepth(instrumentId: string, level: number = 10): Promise<Depth | null> {
    try {
      const response = await fetch(`${this.baseUrl}/instruments/${instrumentId}/depth?level=${level}`);
      if (!response.ok) {
        throw new Error(`Failed to fetch depth: ${response.status}`);
      }
      return await response.json();
    } catch (error) {
      console.error('Error fetching depth:', error);
      return null;
    }
  }

  /**
   * Get recent trades for an instrument
   */
  async getTrades(instrumentId: string, limit: number = 50): Promise<Trade[]> {
    try {
      const response = await fetch(`${this.baseUrl}/instruments/${instrumentId}/trades?limit=${limit}`);
      if (!response.ok) {
        throw new Error(`Failed to fetch trades: ${response.status}`);
      }
      return await response.json();
    } catch (error) {
      console.error('Error fetching trades:', error);
      return [];
    }
  }

  /**
   * Place a new order
   */
  async placeOrder(order: CreateOrderRequest): Promise<Order | null> {
    try {
      // Log the order we're about to send
      console.log('Placing order with data:', JSON.stringify(order, null, 2));
      
      const response = await fetch(`${this.baseUrl}/orders`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(order),
      });

      if (!response.ok) {
        // Try to extract the error message from the response
        let errorMessage = `Failed to place order: ${response.status}`;
        
        try {
          const errorText = await response.text();
          console.error('API error response:', errorText);
          
          try {
            const errorData = JSON.parse(errorText);
            if (errorData.error && errorData.error.message) {
              errorMessage = `Failed to place order (${response.status}): ${errorData.error.message}`;
            }
          } catch (jsonError) {
            // If JSON parsing fails, use the raw text
            if (errorText) {
              errorMessage = `Failed to place order (${response.status}): ${errorText}`;
            }
          }
        } catch (textError) {
          console.error('Could not read error response text:', textError);
        }
        
        throw new Error(errorMessage);
      }

      return await response.json();
    } catch (error) {
      console.error('Error placing order:', error);
      throw error; // Rethrow to allow handling at the component level
    }
  }

  /**
   * Cancel an order
   */
  async cancelOrder(orderId: string, instrumentId: string): Promise<Order | null> {
    try {
      const response = await fetch(`${this.baseUrl}/orders/${orderId}?instrument_id=${instrumentId}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        throw new Error(`Failed to cancel order: ${response.status}`);
      }

      return await response.json();
    } catch (error) {
      console.error('Error cancelling order:', error);
      return null;
    }
  }

  /**
   * Get order details
   */
  async getOrder(orderId: string, instrumentId: string): Promise<Order | null> {
    try {
      const response = await fetch(`${this.baseUrl}/orders/${orderId}?instrument_id=${instrumentId}`);
      if (!response.ok) {
        throw new Error(`Failed to fetch order: ${response.status}`);
      }
      return await response.json();
    } catch (error) {
      console.error('Error fetching order:', error);
      return null;
    }
  }
}

// Export a singleton instance
export const matchingEngineApi = new MatchingEngineApi(); 