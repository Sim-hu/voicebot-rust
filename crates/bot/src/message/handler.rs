use super::read::build_read_text;
use crate::app_state::{self, AppState};
use crate::command::actions;
use crate::command::actions::VoiceToggleOutcome;
use anyhow::{anyhow, Context as _, Result};
use bot_db::{dict, voice::GetOption};
use bot_speech::speech::{list_preset_ids, make_speech, SpeechRequest};
const ZUNDAMON_UUID: &str = "388f246b-8c41-4ac1-8e2d-5d79f3ff56d9";
use log::trace;
use serenity::{client::Context, model::channel::Message};

pub async fn handle(ctx: &Context, msg: Message) -> Result<()> {
    println!(
        "DEBUG: Message received: '{}' from user: {}",
        msg.content, msg.author.name
    );

    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            println!("DEBUG: No guild_id, skipping");
            return Ok(());
        }
    };

    if msg.author.id == ctx.cache.current_user_id() {
        println!("DEBUG: Message from bot itself, skipping");
        return Ok(());
    }

    // Skip messages from other bots
    if msg.author.bot {
        println!("DEBUG: Message from other bot, skipping");
        return Ok(());
    }

    let state = match app_state::get(ctx).await {
        Ok(state) => state,
        Err(e) => {
            trace!("Failed to get app state: {}", e);
            return Ok(()); // Skip processing if app state is not available
        }
    };

    // Handle prefix commands first - these work regardless of connection status
    if msg.content.starts_with('!') {
        println!(
            "DEBUG: Processing prefix command: {} from user: {} in guild: {}",
            msg.content, msg.author.name, guild_id
        );
        match handle_prefix_command(ctx, &msg, &state).await {
            Ok(()) => {
                println!("DEBUG: Prefix command processed successfully");
                return Ok(());
            }
            Err(e) => {
                println!("DEBUG: Error handling prefix command: {}", e);
                // Try to send error message to user, but don't fail if that fails too
                let _ = msg
                    .reply(ctx, "コマンドの処理中にエラーが発生しました。")
                    .await;
                return Ok(());
            }
        }
    }

    // For TTS processing: only process if bot is connected to voice channel
    if !bot_call::is_connected(ctx, guild_id).await? {
        if crate::autojoin::is_autojoin_enabled_for_guild(guild_id.into()).await {
            let channel_id = guild_id.to_guild_cached(&ctx.cache).and_then(|guild| {
                guild
                    .voice_states
                    .get(&msg.author.id)
                    .and_then(|vs| vs.channel_id)
            });
            if let Some(channel_id) = channel_id {
                bot_call::join_deaf(ctx, guild_id, channel_id).await?;
                let state = app_state::get(ctx).await?;
                state.connected_guild_states.insert(
                    guild_id,
                    crate::app_state::ConnectedGuildState {
                        bound_text_channel: msg.channel_id,
                        last_message_read: None,
                        joined_voice_channel: Some(channel_id),
                    },
                );
            } else {
                return Ok(());
            }
        } else {
            return Ok(());
        }
    }

    // Check if this guild has a connected state
    let mut guild_state = match state.connected_guild_states.get_mut(&guild_id) {
        Some(status) => status,
        None => return Ok(()), // No guild state - skip TTS processing
    };

    // Only process messages from the bound text channel
    {
        let mut ok = false;
        if guild_state.bound_text_channel == msg.channel_id {
            ok = true;
        }
        if let Some(vc) = guild_state.joined_voice_channel {
            if vc == msg.channel_id {
                ok = true;
            }
        }
        if !ok {
            if let Some(cfg) = crate::autojoin::get_default_vc(guild_id.into()).await {
                if cfg == msg.channel_id {
                    ok = true;
                }
            }
        }
        if !ok {
            return Ok(());
        }
    }

    trace!(
        "Processing TTS message: '{}' from user: {} in guild: {}",
        msg.content,
        msg.author.name,
        guild_id
    );

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

    let available_preset_ids = match list_preset_ids(&state.voicevox_client).await {
        Ok(ids) => ids,
        Err(_) => Vec::new(),
    };

    if let Ok(speakers) = state.voicevox_client.speakers().await {
        if let Some(z) = speakers
            .iter()
            .find(|s| s.speaker_uuid == ZUNDAMON_UUID || s.name.contains("ずんだもん"))
        {
            if let Some(style) = z
                .styles
                .iter()
                .find(|st| st.name.contains("ノーマル"))
                .or_else(|| z.styles.get(0))
            {
                let encoded_audio = bot_speech::speech::make_speech_by_style(
                    &state.voicevox_client,
                    text.clone(),
                    style.id,
                )
                .await
                .context("Failed to execute TTS (Zundamon Normal)")?;
                let raw_audio = encoded_audio.decode().await?.into();
                bot_call::enqueue(ctx, guild_id, raw_audio).await?;
                guild_state.last_message_read = Some(msg);
                return Ok(());
            }
        }
    };
    if available_preset_ids.is_empty() {
        let style_ids = bot_speech::speech::list_style_ids(&state.voicevox_client).await?;
        let style_id = *style_ids
            .first()
            .ok_or_else(|| anyhow!("No styles available"))?;

        let encoded_audio = bot_speech::speech::make_speech_by_style(
            &state.voicevox_client,
            text.clone(),
            style_id,
        )
        .await
        .context("Failed to execute Text-to-Speech (style fallback)")?;
        let raw_audio = encoded_audio.decode().await?.into();

        bot_call::enqueue(ctx, guild_id, raw_audio).await?;
        guild_state.last_message_read = Some(msg);
        return Ok(());
    }
    let fallback_preset_id = available_preset_ids
        .first()
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
    let Some(guild_id) = msg.guild_id else {
        return Ok(());
    };

    let mut segments = msg.content.trim_start_matches('!').split_whitespace();
    let command = match segments.next() {
        Some(cmd) => cmd.to_lowercase(),
        None => return Ok(()),
    };
    let args: Vec<&str> = segments.collect();

    match command.as_str() {
        "v" => {
            let outcome =
                actions::toggle_voice(ctx, state, guild_id, msg.author.id, msg.channel_id).await?;
            match outcome {
                VoiceToggleOutcome::Joined { voice_channel } => {
                    msg.reply(ctx, format!("<#{}> に参加しました。", voice_channel))
                        .await?;
                }
                VoiceToggleOutcome::Left => {
                    msg.reply(ctx, "ボイスチャンネルから退出しました。").await?;
                }
                VoiceToggleOutcome::MissingUserChannel => {
                    msg.reply(ctx, "まずボイスチャンネルに参加してから実行してください。")
                        .await?;
                }
            }
        }
        "s" => {
            if !bot_call::is_connected(ctx, guild_id).await? {
                msg.reply(ctx, "再生中の読み上げはありません。").await?;
                return Ok(());
            }
            actions::skip_current_track(ctx, guild_id).await?;
            msg.reply(ctx, "再生中の読み上げをスキップしました。")
                .await?;
        }
        "time" => {
            let sub = args.first().copied().unwrap_or("toggle");
            match sub {
                "toggle" => {
                    let enabled = actions::toggle_time_signal(state, guild_id).await;
                    let status = if enabled { "ON" } else { "OFF" };
                    msg.reply(ctx, format!("時報を{}に切り替えました。", status))
                        .await?;
                }
                "audio" => {
                    let mode = args.get(1).copied().unwrap_or("");
                    match mode {
                        "set" => {
                            let Some(url) = args.get(2) else {
                                msg.reply(ctx, "使い方: !time audio set <URL>").await?;
                                return Ok(());
                            };
                            match actions::set_time_signal_audio(state, guild_id, url).await {
                                Ok(()) => {
                                    msg.reply(ctx, "時報の音声URLを更新しました。").await?;
                                }
                                Err(err) => {
                                    msg.reply(ctx, format!("音声の設定に失敗しました: {}", err))
                                        .await?;
                                }
                            }
                        }
                        "clear" => {
                            actions::clear_time_signal_audio(state, guild_id);
                            msg.reply(ctx, "時報の音声設定を削除しました。").await?;
                        }
                        _ => {
                            msg.reply(
                                ctx,
                                "使い方: !time toggle / !time audio set <URL> / !time audio clear",
                            )
                            .await?;
                        }
                    }
                }
                _ => {
                    msg.reply(
                        ctx,
                        "使い方: !time toggle / !time audio set <URL> / !time audio clear",
                    )
                    .await?;
                }
            }
        }
        "autojoin" => {
            let enabled = actions::toggle_autojoin(guild_id).await;
            let status = if enabled { "ON" } else { "OFF" };
            msg.reply(ctx, format!("Autojoin を {} に切り替えました。", status))
                .await?;
        }
        "dict" => {
            let sub = args.first().copied().unwrap_or("");
            match sub {
                "add" => {
                    let Some(word) = args.get(1) else {
                        msg.reply(ctx, "使い方: !dict add <単語> <読み>").await?;
                        return Ok(());
                    };
                    let read_as = args.iter().skip(2).cloned().collect::<Vec<_>>().join(" ");
                    if read_as.is_empty() {
                        msg.reply(ctx, "使い方: !dict add <単語> <読み>").await?;
                        return Ok(());
                    }
                    match actions::dict_add(state, guild_id, word, &read_as).await? {
                        dict::InsertResponse::Success => {
                            msg.reply(ctx, format!("辞書に登録しました: {} → {}", word, read_as))
                                .await?;
                        }
                        dict::InsertResponse::WordAlreadyExists => {
                            msg.reply(
                                ctx,
                                "すでに登録済みです。上書きする場合はいったん削除してください。",
                            )
                            .await?;
                        }
                    }
                }
                "remove" => {
                    let Some(word) = args.get(1) else {
                        msg.reply(ctx, "使い方: !dict remove <単語>").await?;
                        return Ok(());
                    };
                    match actions::dict_remove(state, guild_id, word).await? {
                        dict::RemoveResponse::Success => {
                            msg.reply(ctx, format!("辞書から削除しました: {}", word))
                                .await?;
                        }
                        dict::RemoveResponse::WordDoesNotExist => {
                            msg.reply(ctx, "指定された単語は登録されていません。")
                                .await?;
                        }
                    }
                }
                "list" => {
                    let json = actions::dict_list(state, guild_id).await?;
                    if json.len() <= 1900 {
                        msg.reply(ctx, format!("```json\n{}\n```", json)).await?;
                    } else {
                        msg.reply(
                            ctx,
                            "件数が多すぎるため表示できません。登録内容を絞ってください。",
                        )
                        .await?;
                    }
                }
                _ => {
                    msg.reply(
                        ctx,
                        "使い方: !dict add <単語> <読み> / !dict remove <単語> / !dict list",
                    )
                    .await?;
                }
            }
        }
        "help" => {
            let embed = actions::build_help_embed();
            msg.channel_id
                .send_message(ctx, |m| {
                    m.set_embed(embed.clone());
                    m
                })
                .await?;
        }
        _ => {
            msg.reply(ctx, "不明なコマンドです。!help で一覧を確認してください。")
                .await?;
        }
    }

    Ok(())
}
