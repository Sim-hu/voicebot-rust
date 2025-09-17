use crate::time_signal::TimeSignalConfig;
use anyhow::{anyhow, Result};
use bot_db::redis;
use bot_speech::voicevox::VoicevoxClient;
use dashmap::DashMap;
use serenity::{
    client::{Client, Context},
    model::{
        channel::Message,
        id::{ChannelId, GuildId},
    },
    prelude::TypeMapKey,
};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub redis_client: redis::Client,
    pub voicevox_client: VoicevoxClient,
    pub connected_guild_states: DashMap<GuildId, ConnectedGuildState>,
    pub preferred_style_id: RwLock<Option<i64>>,
    pub time_signal_settings: DashMap<GuildId, TimeSignalConfig>,
}

pub struct ConnectedGuildState {
    pub bound_text_channel: ChannelId,
    pub last_message_read: Option<Message>,
    pub joined_voice_channel: Option<ChannelId>,
}

impl TypeMapKey for AppState {
    type Value = Arc<AppState>;
}

pub async fn initialize(client: &Client, state: AppState) {
    let mut data = client.data.write().await;
    data.insert::<AppState>(Arc::new(state));
}

pub async fn get(ctx: &Context) -> Result<Arc<AppState>> {
    let data = ctx.data.read().await;

    let state_ref = data
        .get::<AppState>()
        .ok_or_else(|| anyhow!("AppState is not initialized"))?;

    Ok(state_ref.clone())
}
