use crate::app_state;
use anyhow::{Context as _, Result};
use once_cell::sync::Lazy;
use serenity::{
    client::Context,
    model::{
        id::{ChannelId, GuildId},
        voice::VoiceState,
    },
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

static AUTOJOIN_SETTINGS: Lazy<Arc<RwLock<HashMap<u64, bool>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));
static AUTOJOIN_DEFAULT_VC: Lazy<Arc<RwLock<HashMap<u64, ChannelId>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));
static AUTOJOIN_DEFAULT_TEXT: Lazy<Arc<RwLock<HashMap<u64, ChannelId>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

pub async fn toggle_autojoin_for_guild(guild_id: u64) -> bool {
    let mut settings = AUTOJOIN_SETTINGS.write().await;
    let current = settings.get(&guild_id).copied().unwrap_or(true);
    let new_setting = !current;
    settings.insert(guild_id, new_setting);
    new_setting
}

pub async fn is_autojoin_enabled_for_guild(guild_id: u64) -> bool {
    let settings = AUTOJOIN_SETTINGS.read().await;
    settings.get(&guild_id).copied().unwrap_or(true)
}

pub async fn set_default_vc(guild_id: u64, channel_id: ChannelId) {
    let mut m = AUTOJOIN_DEFAULT_VC.write().await;
    m.insert(guild_id, channel_id);
}

pub async fn get_default_vc(guild_id: u64) -> Option<ChannelId> {
    let m = AUTOJOIN_DEFAULT_VC.read().await;
    m.get(&guild_id).copied()
}

pub async fn set_default_text_ch(guild_id: u64, channel_id: ChannelId) {
    let mut m = AUTOJOIN_DEFAULT_TEXT.write().await;
    m.insert(guild_id, channel_id);
}

pub async fn get_default_text_ch(guild_id: u64) -> Option<ChannelId> {
    let m = AUTOJOIN_DEFAULT_TEXT.read().await;
    m.get(&guild_id).copied()
}

pub async fn on_voice_state_update(ctx: &Context, new: &VoiceState) -> Result<()> {
    let guild_id: GuildId = match new.guild_id {
        Some(id) => id,
        None => return Ok(()),
    };
    let new_ch = match new.channel_id {
        Some(ch) => ch,
        None => return Ok(()),
    };

    if !is_autojoin_enabled_for_guild(guild_id.into()).await {
        return Ok(());
    }

    if let Some(default_vc) = get_default_vc(guild_id.into()).await {
        if default_vc == new_ch {
            if !bot_call::is_connected(ctx, guild_id).await? {
                bot_call::join_deaf(ctx, guild_id, new_ch).await?;
                let state = app_state::get(ctx).await?;
                let bind_text = get_default_text_ch(guild_id.into())
                    .await
                    .or_else(|| {
                        guild_id
                            .to_guild_cached(&ctx.cache)
                            .and_then(|g| g.system_channel_id)
                    })
                    .context("No system channel to bind for autojoin")?;
                state.connected_guild_states.insert(
                    guild_id,
                    app_state::ConnectedGuildState {
                        bound_text_channel: bind_text,
                        last_message_read: None,
                        joined_voice_channel: Some(new_ch),
                    },
                );
            }
        }
    }

    Ok(())
}
