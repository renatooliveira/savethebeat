use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_host")]
    pub host: String,

    // Future phases (Optional fields)
    pub database_url: Option<String>,
    pub slack_signing_secret: Option<String>,
    pub slack_bot_token: Option<String>,
    pub spotify_client_id: Option<String>,
    pub spotify_client_secret: Option<String>,
    pub spotify_redirect_uri: Option<String>,
    pub base_url: Option<String>,

    #[serde(default = "default_rust_log")]
    pub rust_log: String,
}

fn default_port() -> u16 {
    3000
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_rust_log() -> String {
    "info,savethebeat=debug".to_string()
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();
        envy::from_env::<Config>().map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))
    }
}
