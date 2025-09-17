use crate::app_state::{AppState, ConnectedGuildState};
use crate::{autojoin, time_signal};
use anyhow::Context as _;
use anyhow::Result;
use bot_db::{dict, redis};
use serenity::builder::CreateEmbed;
use serenity::client::Context as SerenityContext;
use serenity::model::id::{ChannelId, GuildId, UserId};
use std::collections::BTreeMap;
use std::path::Path;
use tokio::fs;

const DICT_JSON_PATH: &str = "deployment/dict.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceToggleOutcome {
    Joined { voice_channel: ChannelId },
    Left,
    MissingUserChannel,
}

pub async fn toggle_voice(
    ctx: &SerenityContext,
    state: &AppState,
    guild_id: GuildId,
    user_id: UserId,
    bind_text_channel: ChannelId,
) -> Result<VoiceToggleOutcome> {
    if bot_call::is_connected(ctx, guild_id).await? {
        bot_call::leave(ctx, guild_id).await?;
        state.connected_guild_states.remove(&guild_id);
        Ok(VoiceToggleOutcome::Left)
    } else {
        let channel_id = guild_id.to_guild_cached(&ctx.cache).and_then(|guild| {
            guild
                .voice_states
                .get(&user_id)
                .and_then(|vs| vs.channel_id)
        });

        let Some(channel_id) = channel_id else {
            return Ok(VoiceToggleOutcome::MissingUserChannel);
        };

        bot_call::join_deaf(ctx, guild_id, channel_id).await?;

        state.connected_guild_states.insert(
            guild_id,
            ConnectedGuildState {
                bound_text_channel: bind_text_channel,
                last_message_read: None,
                joined_voice_channel: Some(channel_id),
            },
        );

        Ok(VoiceToggleOutcome::Joined {
            voice_channel: channel_id,
        })
    }
}

pub async fn skip_current_track(ctx: &SerenityContext, guild_id: GuildId) -> Result<bool> {
    bot_call::skip(ctx, guild_id).await?;
    Ok(true)
}

pub async fn toggle_autojoin(guild_id: GuildId) -> bool {
    autojoin::toggle_autojoin_for_guild(guild_id.into()).await
}

pub async fn toggle_time_signal(state: &AppState, guild_id: GuildId) -> bool {
    time_signal::toggle_for_guild(state, guild_id).await
}

pub async fn set_time_signal_audio(state: &AppState, guild_id: GuildId, url: &str) -> Result<()> {
    time_signal::set_audio_from_url(state, guild_id, url).await
}

pub fn clear_time_signal_audio(state: &AppState, guild_id: GuildId) {
    time_signal::clear_audio(state, guild_id);
}

pub async fn dict_add(
    state: &AppState,
    guild_id: GuildId,
    word: &str,
    read_as: &str,
) -> Result<dict::InsertResponse> {
    let mut conn = get_redis_connection(state).await?;
    let result = dict::insert(
        &mut conn,
        dict::InsertOption {
            guild_id: guild_id.into(),
            word: word.to_string(),
            read_as: read_as.to_string(),
        },
    )
    .await
    .context("Failed to insert dictionary entry")?;

    if let dict::InsertResponse::Success = result {
        sync_dict_json(state, guild_id).await?;
    }

    Ok(result)
}

pub async fn dict_remove(
    state: &AppState,
    guild_id: GuildId,
    word: &str,
) -> Result<dict::RemoveResponse> {
    let mut conn = get_redis_connection(state).await?;
    let result = dict::remove(
        &mut conn,
        dict::RemoveOption {
            guild_id: guild_id.into(),
            word: word.to_string(),
        },
    )
    .await
    .context("Failed to remove dictionary entry")?;

    if let dict::RemoveResponse::Success = result {
        sync_dict_json(state, guild_id).await?;
    }

    Ok(result)
}

pub async fn dict_list(state: &AppState, guild_id: GuildId) -> Result<String> {
    let mut conn = get_redis_connection(state).await?;
    let entries = dict::get_all(
        &mut conn,
        dict::GetAllOption {
            guild_id: guild_id.into(),
        },
    )
    .await
    .context("Failed to list dictionary entries")?;

    let map: BTreeMap<String, String> = entries.into_iter().collect();
    write_dict_json(&map).await?;

    Ok(serde_json::to_string_pretty(&map)?)
}

pub async fn dict_words(state: &AppState, guild_id: GuildId) -> Result<Vec<String>> {
    let mut conn = get_redis_connection(state).await?;
    let entries = dict::get_all(
        &mut conn,
        dict::GetAllOption {
            guild_id: guild_id.into(),
        },
    )
    .await?;

    Ok(entries.into_iter().map(|(word, _)| word).collect())
}

pub fn build_help_embed() -> CreateEmbed {
    let mut embed = CreateEmbed::default();
    embed
        .title("読み上げBot コマンド一覧")
        .description("スラッシュコマンドとプレフィックスコマンドは同様に動作します。")
        .color(0x1abc9c);

    embed.field(
        "`/v`, `!v`",
        "ボイスチャンネルへの参加／退出を切り替えます。",
        false,
    );
    embed.field(
        "`/s`, `!s`",
        "現在再生中の読み上げ音声をスキップします。",
        false,
    );
    embed.field(
        "`/time`, `!time`",
        "`toggle` で時報のON/OFFを切り替え、`audio set` で音声URL設定、`audio clear` で解除します。",
        false,
    );
    embed.field(
        "`/autojoin`, `!autojoin`",
        "ユーザーのVC参加を検出してBotを自動参加させる機能を切り替えます。",
        false,
    );
    embed.field(
        "`/dict add`, `!dict add`",
        "読み替えを辞書に追加します。",
        false,
    );
    embed.field(
        "`/dict remove`, `!dict remove`",
        "読み替えを削除します。スラッシュコマンドでは補完が利用できます。",
        false,
    );
    embed.field(
        "`/dict list`, `!dict list`",
        "登録済みの読み替え一覧をJSON形式で表示します。",
        false,
    );
    embed.field("`/help`, `!help`", "このヘルプを表示します。", false);

    embed
}

async fn sync_dict_json(state: &AppState, guild_id: GuildId) -> Result<()> {
    let mut conn = get_redis_connection(state).await?;
    let entries = dict::get_all(
        &mut conn,
        dict::GetAllOption {
            guild_id: guild_id.into(),
        },
    )
    .await?;

    let map: BTreeMap<String, String> = entries.into_iter().collect();
    write_dict_json(&map).await
}

async fn write_dict_json(map: &BTreeMap<String, String>) -> Result<()> {
    let json = serde_json::to_string_pretty(map)?;
    let path = Path::new(DICT_JSON_PATH);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    fs::write(path, json).await?;
    Ok(())
}

async fn get_redis_connection(state: &AppState) -> Result<redis::aio::Connection> {
    state
        .redis_client
        .get_async_connection()
        .await
        .context("Failed to acquire Redis connection")
}
