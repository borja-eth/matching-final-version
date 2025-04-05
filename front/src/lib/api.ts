import Decimal from 'decimal.js';
import { v4 as uuidv4 } from 'uuid';

// Types based on the Rust API DTOs
export type OrderType = 'Limit' | 'Market';
export type OrderSide = 'Buy' | 'Sell';
export type BackendOrderSide = 'Bid' | 'Ask';
export type OrderStatus = 'New' | 'PartiallyFilled' | 'Filled' | 'Cancelled' | 'Rejected';
export type TimeInForce = 'GTC' | 'IOC' | 'FOK';

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
  order_type: OrderType;
  instrument_id: string;
  side: OrderSide;
  limit_price?: string;
  trigger_price?: string;
  base_amount: string;
  remaining_base: string;
  filled_base: string;
  filled_quote: string;
  status: OrderStatus;
  created_at: string;
  updated_at: string;
}

export interface CreateOrderRequest {
  ext_id?: string;
  account_id: string;
  order_type: OrderType;
  instrument_id: string;
  side: OrderSide;
  limit_price?: string;
  trigger_price?: string;
  base_amount: string;
  time_in_force?: TimeInForce;
}

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

export interface CreateInstrumentRequest {
  id?: string;
  name: string;
  base_currency: string;
  quote_currency: string;
}

// API client class
export class ApiClient {
  private baseUrl: string;
  private useMock: boolean = false;
  private mockInstrumentId: string = "11111111-1111-1111-1111-111111111111";

  constructor(baseUrl: string = 'http://127.0.0.1:3001') {
    this.baseUrl = baseUrl;
  }

  private async request<T>(
    endpoint: string,
    method: string = 'GET',
    body?: any
  ): Promise<T> {
    if (this.useMock) {
      return this.handleMockRequest<T>(endpoint, method, body);
    }

    try {
      const options: RequestInit = {
        method,
        headers: {
          'Content-Type': 'application/json',
          'Accept': 'application/json',
        },
        credentials: 'include',
        mode: 'cors',
      };

      if (body) {
        options.body = JSON.stringify(body);
      }

      const url = `${this.baseUrl}${endpoint}`;
      console.log(`API Request: ${method} ${url}`, body ? body : '');
      
      const response = await fetch(url, options);
      
      if (!response.ok) {
        const errorText = await response.text();
        console.error(`API Error: ${response.status} ${errorText}`);
        throw new Error(`API Error ${response.status}: ${errorText}`);
      }
      
      const data = await response.json();
      console.log(`API Response:`, data);
      
      // Check if this is an order or list of orders and convert side field
      const processedData = this.convertOrderSidesInResponse(data);
      
      return processedData as T;
    } catch (error) {
      console.error("API request failed:", error);
      // Only switch to mock mode after showing the error
      this.useMock = true;
      return this.handleMockRequest<T>(endpoint, method, body);
    }
  }

  // Convert order side fields in API responses
  private convertOrderSidesInResponse(data: any): any {
    // If it's an array, process each item
    if (Array.isArray(data)) {
      return data.map(item => this.convertOrderSidesInResponse(item));
    }
    
    // If it's an object and has a 'side' field with valid backend values
    if (data && typeof data === 'object' && 'side' in data) {
      if (data.side === 'Bid' || data.side === 'Ask') {
        return {
          ...data,
          side: convertOrderSideToFrontend(data.side as BackendOrderSide)
        };
      }
    }
    
    // Otherwise return as is
    return data;
  }

  private handleMockRequest<T>(endpoint: string, method: string, body?: any): Promise<T> {
    if (endpoint === '/instruments' && method === 'GET') {
      const mockInstruments = [{
        id: this.mockInstrumentId,
        name: 'BTC/USD (Mock)',
        base_currency: 'BTC',
        quote_currency: 'USD',
      }];
      return Promise.resolve(mockInstruments as unknown as T);
    }
    
    if (endpoint === '/instruments' && method === 'POST') {
      const req = body as CreateInstrumentRequest;
      const mockInstrument: Instrument = {
        id: this.mockInstrumentId,
        name: req.name || 'BTC/USD (Mock)',
        base_currency: req.base_currency || 'BTC',
        quote_currency: req.quote_currency || 'USD',
      };
      return Promise.resolve(mockInstrument as unknown as T);
    }
    
    if (endpoint.includes('/orderbook') || endpoint.includes('/depth')) {
      const mockDepth: Depth = {
        bids: [{ 
          price: "99.0", 
          volume: "1.0", 
          order_count: 1 
        }],
        asks: [{ 
          price: "100.0", 
          volume: "0.5", 
          order_count: 1 
        }],
        timestamp: new Date().toISOString(),
        instrument_id: this.mockInstrumentId,
      };
      return Promise.resolve(mockDepth as unknown as T);
    }
    
    if (endpoint.includes('/trades')) {
      const mockTrades: Trade[] = [{
        id: "22222222-2222-2222-2222-222222222222",
        instrument_id: this.mockInstrumentId,
        maker_order_id: "33333333-3333-3333-3333-333333333333",
        taker_order_id: "44444444-4444-4444-4444-444444444444",
        base_amount: "0.5",
        quote_amount: "50.0",
        price: "100.0",
        created_at: new Date().toISOString(),
      }];
      return Promise.resolve(mockTrades as unknown as T);
    }
    
    if (endpoint === '/orders' && method === 'POST') {
      const req = body as CreateOrderRequest;
      const mockOrder: Order = {
        id: this.generateUuid(),
        account_id: req.account_id,
        order_type: req.order_type,
        instrument_id: req.instrument_id,
        side: req.side,
        limit_price: req.limit_price,
        base_amount: req.base_amount,
        remaining_base: req.base_amount,
        filled_base: "0",
        filled_quote: "0",
        status: "New",
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
      return Promise.resolve(mockOrder as unknown as T);
    }
    
    return Promise.resolve({} as T);
  }

  // Health check
  async healthCheck(): Promise<boolean> {
    try {
      const response = await fetch(`${this.baseUrl}/health`, {
        // Add timeout to prevent long waiting
        signal: AbortSignal.timeout(3000)
      });
      return response.ok;
    } catch (error) {
      console.error("Health check failed:", error);
      return false;
    }
  }

  // Enable or disable mock mode
  setMockMode(useMock: boolean): void {
    this.useMock = useMock;
    console.log(`API mock mode ${this.useMock ? 'enabled' : 'disabled'}`);
  }

  // Get the base URL
  getBaseUrl(): string {
    return this.baseUrl;
  }

  // Check if mock mode is enabled
  isMockModeEnabled(): boolean {
    return this.useMock;
  }

  // Instrument endpoints
  async createInstrument(req: CreateInstrumentRequest): Promise<Instrument> {
    return this.request<Instrument>('/instruments', 'POST', req);
  }

  async listInstruments(): Promise<Instrument[]> {
    try {
      return await this.request<Instrument[]>('/instruments');
    } catch (error) {
      console.error("Failed to fetch instruments:", error);
      this.useMock = true;
      return this.handleMockRequest<Instrument[]>('/instruments', 'GET');
    }
  }

  // Order endpoints
  async createOrder(req: CreateOrderRequest): Promise<Order> {
    // Convert the order side to backend format
    const backendReq = {
      ...req,
      side: convertOrderSideToBackend(req.side)
    };
    return this.request<Order>('/orders', 'POST', backendReq);
  }

  async cancelOrder(orderId: string, instrumentId: string): Promise<Order> {
    return this.request<Order>(`/orders/${orderId}?instrument_id=${instrumentId}`, 'DELETE');
  }

  async getOrder(orderId: string, instrumentId: string): Promise<Order> {
    return this.request<Order>(`/orders/${orderId}?instrument_id=${instrumentId}`).then(order => {
      // Convert backend side to frontend format if needed
      if ((order.side as unknown) === 'Bid' || (order.side as unknown) === 'Ask') {
        return {
          ...order,
          side: convertOrderSideToFrontend(order.side as unknown as BackendOrderSide)
        };
      }
      return order;
    });
  }

  // Market data endpoints
  async getOrderbook(instrumentId: string): Promise<Depth> {
    if (!instrumentId || typeof instrumentId !== 'string') {
      console.error("Invalid instrument ID passed to getOrderbook:", instrumentId);
      return this.handleMockRequest<Depth>('/orderbook', 'GET');
    }
    return this.request<Depth>(`/instruments/${instrumentId}/orderbook`);
  }

  async getDepth(instrumentId: string, level: number = 10): Promise<Depth> {
    if (!instrumentId || typeof instrumentId !== 'string') {
      console.error("Invalid instrument ID passed to getDepth:", instrumentId);
      return this.handleMockRequest<Depth>('/depth', 'GET');
    }
    return this.request<Depth>(`/instruments/${instrumentId}/depth?level=${level}`);
  }

  async getTrades(instrumentId: string, limit: number = 20): Promise<Trade[]> {
    if (!instrumentId || typeof instrumentId !== 'string') {
      console.error("Invalid instrument ID passed to getTrades:", instrumentId);
      return this.handleMockRequest<Trade[]>('/trades', 'GET');
    }
    return this.request<Trade[]>(`/instruments/${instrumentId}/trades?limit=${limit}`);
  }

  // Utility methods
  private generateUuid(): string {
    return uuidv4();
  }
}

// Convert frontend order side to backend format
function convertOrderSideToBackend(side: OrderSide): BackendOrderSide {
  return side === 'Buy' ? 'Bid' : 'Ask';
}

// Convert backend order side to frontend format
function convertOrderSideToFrontend(side: BackendOrderSide): OrderSide {
  return side === 'Bid' ? 'Buy' : 'Sell';
} 