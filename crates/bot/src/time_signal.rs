use crate::app_state::{self, AppState};
use anyhow::{anyhow, Result};
use bot_audio::EncodedAudio;
use chrono::Timelike;
use log::{error, info};
use once_cell::sync::OnceCell;
use serenity::client::Context as SerenityContext;
use serenity::model::id::{ChannelId, GuildId, UserId};
use std::sync::Arc;
use tokio::time::{interval, Duration};

#[derive(Clone)]
pub struct TimeSignalAudio {
    pub source_url: String,
    pub pcm: Arc<Vec<u8>>,
}

#[derive(Clone, Default)]
pub struct TimeSignalConfig {
    pub enabled: bool,
    pub audio: Option<TimeSignalAudio>,
    pub last_announced_hour: Option<u8>,
}

static SERVICE_ONCE: OnceCell<()> = OnceCell::new();
static HTTP_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();

pub fn spawn_service(ctx: SerenityContext) {
    if SERVICE_ONCE.set(()).is_err() {
        return;
    }

    tokio::spawn(async move {
        if let Err(err) = start_time_signal_loop(ctx).await {
            error!("Time signal service exited: {}", err);
        }
    });
}

async fn start_time_signal_loop(ctx: SerenityContext) -> Result<()> {
    let mut ticker = interval(Duration::from_secs(60));

    loop {
        ticker.tick().await;

        let now_jst = current_jst();
        if now_jst.minute() != 0 {
            continue;
        }
        let hour = now_jst.hour() as u8;

        let state = match app_state::get(&ctx).await {
            Ok(state) => state,
            Err(err) => {
                error!("Time signal: failed to get app state: {}", err);
                continue;
            }
        };

        let current_user = ctx.cache.current_user_id();

        let targets = state
            .connected_guild_states
            .iter()
            .filter_map(|entry| {
                let guild_id = *entry.key();
                let config = state
                    .time_signal_settings
                    .get(&guild_id)
                    .map(|cfg| cfg.clone())
                    .unwrap_or_default();

                if !config.enabled || config.last_announced_hour == Some(hour) {
                    return None;
                }

                let voice_channel = entry.joined_voice_channel?;
                Some((guild_id, entry.bound_text_channel, voice_channel, config))
            })
            .collect::<Vec<_>>();

        for (guild_id, text_channel, voice_channel, config) in targets {
            if !has_listeners(&ctx, guild_id, voice_channel, current_user).await {
                continue;
            }

            let announcement = format!("{}時をお知らせします。", hour);
            if let Err(err) = text_channel.say(&ctx.http, &announcement).await {
                error!(
                    "Time signal: failed to send message in guild {}: {}",
                    guild_id, err
                );
                continue;
            }

            if let Some(audio) = config.audio.clone() {
                if let Err(err) = bot_call::enqueue(&ctx, guild_id, (*audio.pcm).clone()).await {
                    error!(
                        "Time signal: failed to enqueue audio in guild {}: {}",
                        guild_id, err
                    );
                }
            }

            state
                .time_signal_settings
                .entry(guild_id)
                .and_modify(|entry| entry.last_announced_hour = Some(hour))
                .or_insert_with(|| {
                    let mut new_config = TimeSignalConfig::default();
                    new_config.enabled = true;
                    new_config.last_announced_hour = Some(hour);
                    new_config
                });

            info!("Time signal executed for guild {} at {}時", guild_id, hour);
        }
    }
}

async fn has_listeners(
    ctx: &SerenityContext,
    guild_id: GuildId,
    voice_channel: ChannelId,
    current_user: UserId,
) -> bool {
    ctx.cache
        .guild_field(guild_id, |guild| {
            guild
                .voice_states
                .values()
                .filter(|state| {
                    state.channel_id == Some(voice_channel) && state.user_id != current_user
                })
                .count()
        })
        .map_or(false, |listeners| listeners > 0)
}

fn current_jst() -> chrono::DateTime<chrono::FixedOffset> {
    let offset = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
    chrono::Utc::now().with_timezone(&offset)
}

pub async fn toggle_for_guild(state: &AppState, guild_id: GuildId) -> bool {
    let mut entry = state
        .time_signal_settings
        .entry(guild_id)
        .or_insert_with(TimeSignalConfig::default);
    entry.enabled = !entry.enabled;
    entry.last_announced_hour = None;
    entry.enabled
}

pub async fn set_audio_from_url(state: &AppState, guild_id: GuildId, url: &str) -> Result<()> {
    const MAX_BYTES: usize = 10 * 1024 * 1024; // 10MB

    let client = HTTP_CLIENT.get_or_init(reqwest::Client::new);
    let response = client.get(url).send().await?.error_for_status()?;
    let bytes = response.bytes().await?;

    if bytes.is_empty() {
        return Err(anyhow!("音声データが空です"));
    }
    if bytes.len() > MAX_BYTES {
        return Err(anyhow!("音声ファイルが大きすぎます (最大10MB)"));
    }

    let encoded = EncodedAudio::from(bytes.to_vec());
    let decoded: Vec<u8> = encoded.decode().await?.into();

    let audio = TimeSignalAudio {
        source_url: url.to_string(),
        pcm: Arc::new(decoded),
    };

    state
        .time_signal_settings
        .entry(guild_id)
        .or_insert_with(TimeSignalConfig::default)
        .audio = Some(audio);

    Ok(())
}

pub fn clear_audio(state: &AppState, guild_id: GuildId) {
    if let Some(mut entry) = state.time_signal_settings.get_mut(&guild_id) {
        entry.audio = None;
    }
}
