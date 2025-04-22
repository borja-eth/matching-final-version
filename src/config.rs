use dotenv::dotenv;
use std::env;
use tracing::info;
use uuid::Uuid;

const RABBIT_URL: &str = "RABBIT_URL";
const INSTRUMENTS: &str = "INSTRUMENTS";
const APP_ID: &str = "APP_ID";

#[derive(Clone)]
pub struct Config {
    pub rabbit_url: String,
    pub instruments: Vec<Uuid>,
    pub app_id: String,
}

impl Config {
    pub fn from_env() -> Config {
        match Self::try_from_env() {
            Ok(config) => config,
            Err(err) => panic!("{}", err),
        }
    }

    pub fn try_from_env() -> Result<Config, String> {
        // Load .env file
        dotenv().ok();

        let rabbit_url = env::var(RABBIT_URL)
            .map_err(|_| format!("failed to load environment variable {}", RABBIT_URL))?;
            
        let instruments_str = env::var(INSTRUMENTS)
            .map_err(|_| format!("failed to load environment variable {}", INSTRUMENTS))?;
        
        // Clean up the instruments string (remove trailing % or other unexpected characters)
        let cleaned_instruments_str = instruments_str.trim_end_matches('%').trim();
        info!("Parsed instruments from env: {}", cleaned_instruments_str);
        
        let instruments = cleaned_instruments_str
            .split(',')
            .map(|s| {
                let trimmed = s.trim();
                Uuid::parse_str(trimmed)
                    .map_err(|_| format!("failed to parse instrument: {}", trimmed))
            })
            .collect::<Result<Vec<Uuid>, String>>()?;
        
        let app_id = env::var(APP_ID)
            .unwrap_or_else(|_| "matching-engine".to_string());

        Ok(Config {
            rabbit_url,
            instruments,
            app_id,
        })
    }

    pub fn default() -> Config {
        Config {
            rabbit_url: "amqp://guest:guest@localhost:5672".to_string(),
            instruments: vec![Uuid::new_v4()], // Create one random instrument ID
            app_id: "matching-engine".to_string(),
        }
    }
}
