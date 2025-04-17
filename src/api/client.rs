use std::time::Duration;
use reqwest::{Client, StatusCode};
use anyhow::{Result, anyhow};
use uuid::Uuid;

use crate::api::dto::{CreateOrderRequest, OrderResponse, DepthResponse, CreateInstrumentRequest, InstrumentResponse};

/// API client for interacting with the matching engine
#[derive(Debug, Clone)]
pub struct ApiClient {
    /// HTTP client
    client: Client,
    
    /// Base URL for the API
    base_url: String,
}

impl ApiClient {
    /// Creates a new API client
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(100)
            .pool_idle_timeout(Some(Duration::from_secs(30)))
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to build HTTP client");
            
        Self {
            client,
            base_url: base_url.to_string(),
        }
    }
    
    /// Places an order through the API
    pub async fn place_order(&self, order: CreateOrderRequest) -> Result<OrderResponse> {
        let url = format!("{}/orders", self.base_url);
        
        let resp = self.client.post(&url)
            .json(&order)
            .send()
            .await?;
            
        match resp.status() {
            StatusCode::CREATED => {
                let order_resp = resp.json::<OrderResponse>().await?;
                Ok(order_resp)
            },
            status => {
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(anyhow!("Failed to place order: {} - {}", status, error_text))
            }
        }
    }
    
    /// Cancels an order through the API
    pub async fn cancel_order(&self, order_id: Uuid, instrument_id: Uuid) -> Result<OrderResponse> {
        let url = format!("{}/orders/{}", self.base_url, order_id);
        
        let resp = self.client.delete(&url)
            .query(&[("instrument_id", instrument_id.to_string())])
            .send()
            .await?;
            
        match resp.status() {
            StatusCode::OK => {
                let order_resp = resp.json::<OrderResponse>().await?;
                Ok(order_resp)
            },
            status => {
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(anyhow!("Failed to cancel order: {} - {}", status, error_text))
            }
        }
    }
    
    /// Gets market depth through the API
    pub async fn get_depth(&self, instrument_id: Uuid, levels: usize) -> Result<DepthResponse> {
        let url = format!("{}/instruments/{}/depth", self.base_url, instrument_id);
        
        let resp = self.client.get(&url)
            .query(&[("level", levels.to_string())])
            .send()
            .await?;
            
        match resp.status() {
            StatusCode::OK => {
                let depth_resp = resp.json::<DepthResponse>().await?;
                Ok(depth_resp)
            },
            status => {
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(anyhow!("Failed to get depth: {} - {}", status, error_text))
            }
        }
    }
    
    /// Ensures an instrument exists, creating it if necessary
    pub async fn ensure_instrument(&self, name: &str, base_currency: &str, quote_currency: &str) -> Result<Uuid> {
        // First try to list instruments to find an existing one
        let url = format!("{}/instruments", self.base_url);
        let resp = self.client.get(&url).send().await?;
        
        if resp.status() == StatusCode::OK {
            let instruments: Vec<InstrumentResponse> = resp.json().await?;
            if let Some(instrument) = instruments.iter().find(|i| 
                i.name == name && 
                i.base_currency == base_currency && 
                i.quote_currency == quote_currency
            ) {
                return Ok(instrument.id);
            }
        }
        
        // If not found, create a new one
        let create_req = CreateInstrumentRequest {
            id: None,
            name: name.to_string(),
            base_currency: base_currency.to_string(),
            quote_currency: quote_currency.to_string(),
        };
        
        let url = format!("{}/instruments", self.base_url);
        let resp = self.client.post(&url)
            .json(&create_req)
            .send()
            .await?;
            
        match resp.status() {
            StatusCode::CREATED => {
                let instrument = resp.json::<InstrumentResponse>().await?;
                Ok(instrument.id)
            },
            status => {
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(anyhow!("Failed to create instrument: {} - {}", status, error_text))
            }
        }
    }
} 