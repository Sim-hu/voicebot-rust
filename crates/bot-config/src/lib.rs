use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub discord: DiscordConfig,
    pub voicevox: VoicevoxConfig,
    pub redis: RedisConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordConfig {
    pub client_id: u64,
    pub bot_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VoicevoxConfig {
    pub api_base: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
}

pub async fn load() -> Result<Config> {
    // Try to load from environment variables first
    if let (Ok(client_id_str), Ok(bot_token), Ok(redis_url)) = (
        std::env::var("DISCORD_CLIENT_ID"),
        std::env::var("DISCORD_BOT_TOKEN"), 
        std::env::var("REDIS_URL")
    ) {
        let client_id = client_id_str.parse::<u64>()
            .context("Failed to parse DISCORD_CLIENT_ID as u64")?;
        
        let voicevox_api_base = std::env::var("VOICEVOX_API_BASE")
            .unwrap_or_else(|_| "http://voicevox:50021".to_string());

        return Ok(Config {
            discord: DiscordConfig { client_id, bot_token },
            voicevox: VoicevoxConfig { api_base: voicevox_api_base },
            redis: RedisConfig { url: redis_url },
        });
    }

    // Fall back to YAML file if environment variables are not available
    let config_path = std::env::var("BOT_CONFIG").unwrap_or_else(|_| "/etc/bot.yaml".to_string());

    let yaml = tokio::fs::read_to_string(&config_path)
        .await
        .with_context(|| format!("Failed to load config file from {}", config_path))?;

    let config = serde_yaml::from_str::<Config>(&yaml).context("Failed to parse config file")?;

    Ok(config)
}
