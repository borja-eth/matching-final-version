use dotenv::dotenv;
use std::env;

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
        // Load .env file
        dotenv().ok();

        let rabbit_url =
            env::var(RABBIT_URL).expect("failed to load environment variable RABBIT_URL");
        let instruments = env::var(INSTRUMENTS)
            .expect("failed to load environment variable INSTRUMENTS")
            .split(',')
            .map(|s| {
                Uuid::parse_str(s).unwrap_or_else(|_| panic!("failed to parse instrument: {}", s))
            })
            .collect();
        let app_id = env::var(APP_ID).expect("failed to load environment variable APP_ID");

        Config {
            rabbit_url,
            instruments,
            app_id,
        }
    }
}
