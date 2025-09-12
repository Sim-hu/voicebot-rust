use super::read::build_read_text;
use crate::app_state::{self, AppState};
use anyhow::{anyhow, Context as _, Result};
use bot_db::{dict, voice::GetOption};
use bot_speech::speech::{list_preset_ids, make_speech, SpeechRequest};
use log::trace;
use rand::seq::SliceRandom;
use serenity::{client::Context, model::channel::Message};

pub async fn handle(ctx: &Context, msg: Message) -> Result<()> {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => return Ok(()),
    };

    if !bot_call::is_connected(ctx, guild_id).await? {
        return Ok(());
    }

    let state = app_state::get(ctx).await?;
    let mut guild_state = match state.connected_guild_states.get_mut(&guild_id) {
        Some(status) => status,
        None => return Ok(()),
    };

    if guild_state.bound_text_channel != msg.channel_id {
        return Ok(());
    }

    // Skip message from Koe itself
    if msg.author.id == ctx.cache.current_user_id() {
        return Ok(());
    }

    // Handle prefix commands
    if msg.content.starts_with('!') {
        return handle_prefix_command(ctx, &msg, &state).await;
    }

    let mut conn = state.redis_client.get_async_connection().await?;

    let text = build_read_text(
        ctx,
        &mut conn,
        guild_id,
        &msg,
        &guild_state.last_message_read,
    )
    .await?;
    trace!("Built text: {:?}", &text);

    if text.is_empty() {
        trace!("Text is empty");
        return Ok(());
    }

    let available_preset_ids = list_preset_ids(&state.voicevox_client).await?;
    let fallback_preset_id = available_preset_ids
        .choose(&mut rand::thread_rng())
        .ok_or_else(|| anyhow!("No presets available"))?
        .into();
    let preset_id = bot_db::voice::get(
        &mut conn,
        GetOption {
            guild_id: guild_id.into(),
            user_id: msg.author.id.into(),
            fallback: fallback_preset_id,
        },
    )
    .await?
    .into();

    let encoded_audio = make_speech(&state.voicevox_client, SpeechRequest { text, preset_id })
        .await
        .context("Failed to execute Text-to-Speech")?;
    let raw_audio = encoded_audio.decode().await?.into();

    bot_call::enqueue(ctx, guild_id, raw_audio).await?;

    guild_state.last_message_read = Some(msg);

    Ok(())
}

async fn handle_prefix_command(ctx: &Context, msg: &Message, state: &AppState) -> Result<()> {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => return Ok(()),
    };

    let command = msg.content.trim_start_matches('!').to_lowercase();
    let command_parts: Vec<&str> = command.split_whitespace().collect();
    let cmd = command_parts.get(0).unwrap_or(&"");

    match *cmd {
        "s" => {
            // Skip command - just return without processing
            return Ok(());
        },
        "v" => {
            // Voice toggle command - join/leave voice channel
            if bot_call::is_connected(ctx, guild_id).await? {
                // Leave voice channel
                bot_call::leave(ctx, guild_id).await?;
                state.connected_guild_states.remove(&guild_id);
                msg.reply(ctx, "ボイスチャンネルから退出しました。").await?;
            } else {
                // Join voice channel
                let channel_id = guild_id
                    .to_guild_cached(ctx)
                    .and_then(|guild| guild.voice_states.get(&msg.author.id).map(|vs| vs.channel_id))
                    .flatten();
                if let Some(channel_id) = channel_id
                {
                    bot_call::join_deaf(ctx, guild_id, channel_id).await?;
                    state.connected_guild_states.insert(
                        guild_id,
                        crate::app_state::ConnectedGuildState {
                            bound_text_channel: msg.channel_id,
                            last_message_read: None,
                        },
                    );
                    msg.reply(ctx, "ボイスチャンネルに参加しました。").await?;
                } else {
                    msg.reply(ctx, "まずボイスチャンネルに参加してください。").await?;
                }
            }
            return Ok(());
        },
        "dict" => {
            return handle_dict_command(ctx, msg, state, &command_parts[1..]).await;
        },
        "time" => {
            let guild_id = guild_id.into();
            let enabled = crate::time_signal::toggle_time_signal_for_guild(guild_id).await;
            let status = if enabled { "有効" } else { "無効" };
            msg.reply(ctx, format!("時報機能を{}にしました。毎時0分に時刻をお知らせします。", status)).await?;
            return Ok(());
        },
        _ => {
            // Unknown command - skip reading aloud
            return Ok(());
        }
    }
}

async fn handle_dict_command(ctx: &Context, msg: &Message, state: &AppState, args: &[&str]) -> Result<()> {
    let guild_id = msg.guild_id.unwrap();
    let mut conn = state.redis_client.get_async_connection().await?;
    
    if args.is_empty() {
        msg.reply(ctx, "使用方法: `!dict add <単語> <読み方>` | `!dict remove <単語>` | `!dict list`").await?;
        return Ok(());
    }

    match args[0] {
        "add" => {
            if args.len() < 3 {
                msg.reply(ctx, "使用方法: `!dict add <単語> <読み方>`").await?;
                return Ok(());
            }
            
            let word = args[1].to_string();
            let read_as = args[2..].join(" ");
            
            let result = dict::insert(
                &mut conn, 
                dict::InsertOption {
                    guild_id: guild_id.into(),
                    word: word.clone(),
                    read_as: read_as.clone(),
                }
            ).await?;
            
            match result {
                dict::InsertResponse::Success => {
                    msg.reply(ctx, format!("辞書に追加しました: {} → {}", word, read_as)).await?;
                },
                dict::InsertResponse::WordAlreadyExists => {
                    msg.reply(ctx, format!("「{}」は既に辞書に登録されています。", word)).await?;
                }
            }
        },
        "remove" => {
            if args.len() < 2 {
                msg.reply(ctx, "使用方法: `!dict remove <単語>`").await?;
                return Ok(());
            }
            
            let word = args[1].to_string();
            
            let result = dict::remove(
                &mut conn,
                dict::RemoveOption {
                    guild_id: guild_id.into(),
                    word: word.clone(),
                }
            ).await?;
            
            match result {
                dict::RemoveResponse::Success => {
                    msg.reply(ctx, format!("辞書から削除しました: {}", word)).await?;
                },
                dict::RemoveResponse::WordDoesNotExist => {
                    msg.reply(ctx, format!("「{}」は辞書に登録されていません。", word)).await?;
                }
            }
        },
        "list" => {
            let dict_entries = dict::get_all(
                &mut conn,
                dict::GetAllOption {
                    guild_id: guild_id.into(),
                }
            ).await?;
            
            if dict_entries.is_empty() {
                msg.reply(ctx, "辞書は空です。").await?;
            } else {
                // JSON形式で返す
                let json_entries: serde_json::Value = dict_entries
                    .into_iter()
                    .map(|(word, read_as)| serde_json::json!({
                        "word": word,
                        "read_as": read_as
                    }))
                    .collect::<Vec<_>>()
                    .into();
                
                let json_str = serde_json::to_string_pretty(&json_entries)?;
                let response = format!("```json\n{}\n```", json_str);
                msg.reply(ctx, response).await?;
            }
        },
        _ => {
            msg.reply(ctx, "使用方法: `!dict add <単語> <読み方>` | `!dict remove <単語>` | `!dict list`").await?;
        }
    }
    
    Ok(())
}
