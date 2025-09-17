use crate::app_state::AppState;
use crate::command::actions;
use crate::command::actions::VoiceToggleOutcome;
use anyhow::{anyhow, Result};
use bot_db::dict;
use serde_json::Value;
use serenity::builder::CreateEmbed;
use serenity::client::Context as SerenityContext;
use serenity::model::application::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption,
};
use serenity::model::application::interaction::autocomplete::AutocompleteInteraction;
use serenity::model::application::interaction::InteractionResponseType;

pub async fn handle(
    ctx: &SerenityContext,
    interaction: &ApplicationCommandInteraction,
    state: &AppState,
) -> Result<()> {
    match interaction.data.name.as_str() {
        "v" => handle_voice(ctx, interaction, state).await?,
        "s" => handle_skip(ctx, interaction, state).await?,
        "time" => handle_time(ctx, interaction, state).await?,
        "dict" => handle_dict(ctx, interaction, state).await?,
        "help" => handle_help(ctx, interaction).await?,
        _ => respond_text(ctx, interaction, "未対応のコマンドです。").await?,
    }

    Ok(())
}

pub async fn handle_autocomplete(
    ctx: &SerenityContext,
    interaction: &AutocompleteInteraction,
    state: &AppState,
) -> Result<()> {
    if interaction.data.name != "dict" {
        return Ok(());
    }

    let Some(guild_id) = interaction.guild_id else {
        return Ok(());
    };

    let Some(option) = find_focused_option(&interaction.data.options) else {
        return Ok(());
    };

    let query = option.value.as_ref().and_then(Value::as_str).unwrap_or("");

    let mut words = match actions::dict_words(state, guild_id).await {
        Ok(words) => words,
        Err(_) => Vec::new(),
    };
    words.sort();

    let lower_query = query.to_lowercase();
    let suggestions = words
        .into_iter()
        .filter(|word| lower_query.is_empty() || word.to_lowercase().contains(&lower_query))
        .take(25)
        .collect::<Vec<_>>();

    interaction
        .create_autocomplete_response(&ctx.http, |response| {
            for word in &suggestions {
                response.add_string_choice(word, word.clone());
            }
            response
        })
        .await?;

    Ok(())
}

async fn handle_voice(
    ctx: &SerenityContext,
    interaction: &ApplicationCommandInteraction,
    state: &AppState,
) -> Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        respond_text(
            ctx,
            interaction,
            "このコマンドはサーバー内で使用してください。",
        )
        .await?;
        return Ok(());
    };

    let outcome = actions::toggle_voice(
        ctx,
        state,
        guild_id,
        interaction.user.id,
        interaction.channel_id,
    )
    .await?;

    match outcome {
        VoiceToggleOutcome::Joined { voice_channel } => {
            respond_text(
                ctx,
                interaction,
                &format!("<#{}> に参加しました。", voice_channel),
            )
            .await?
        }
        VoiceToggleOutcome::Left => {
            respond_text(ctx, interaction, "ボイスチャンネルから退出しました。").await?
        }
        VoiceToggleOutcome::MissingUserChannel => {
            respond_text(
                ctx,
                interaction,
                "まずボイスチャンネルに参加してから実行してください。",
            )
            .await?
        }
    }

    Ok(())
}

async fn handle_skip(
    ctx: &SerenityContext,
    interaction: &ApplicationCommandInteraction,
    _state: &AppState,
) -> Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        respond_text(
            ctx,
            interaction,
            "このコマンドはサーバー内で使用してください。",
        )
        .await?;
        return Ok(());
    };

    if !bot_call::is_connected(ctx, guild_id).await? {
        respond_text(ctx, interaction, "再生中の読み上げはありません。").await?;
        return Ok(());
    }

    actions::skip_current_track(ctx, guild_id).await?;
    respond_text(ctx, interaction, "再生中の読み上げをスキップしました。").await?;
    Ok(())
}

async fn handle_time(
    ctx: &SerenityContext,
    interaction: &ApplicationCommandInteraction,
    state: &AppState,
) -> Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        respond_text(
            ctx,
            interaction,
            "このコマンドはサーバー内で使用してください。",
        )
        .await?;
        return Ok(());
    };

    let subcommand = interaction
        .data
        .options
        .first()
        .map(|option| option.name.as_str())
        .unwrap_or("toggle");

    match subcommand {
        "toggle" => {
            let enabled = actions::toggle_time_signal(state, guild_id).await;
            let status = if enabled { "ON" } else { "OFF" };
            respond_text(
                ctx,
                interaction,
                &format!("時報を{}に切り替えました。", status),
            )
            .await?
        }
        "audio_set" => {
            let Some(option) = interaction.data.options.first() else {
                respond_text(ctx, interaction, "URL を指定してください。").await?;
                return Ok(());
            };
            let Some(url_option) = option.options.first() else {
                respond_text(ctx, interaction, "URL を指定してください。").await?;
                return Ok(());
            };
            let Some(url) = url_option.value.as_ref().and_then(Value::as_str) else {
                respond_text(ctx, interaction, "不正な URL です。").await?;
                return Ok(());
            };

            match actions::set_time_signal_audio(state, guild_id, url).await {
                Ok(()) => respond_text(ctx, interaction, "時報の音声URLを更新しました。").await?,
                Err(err) => {
                    respond_text(
                        ctx,
                        interaction,
                        &format!("音声の設定に失敗しました: {}", err),
                    )
                    .await?
                }
            }
        }
        "audio_clear" => {
            actions::clear_time_signal_audio(state, guild_id);
            respond_text(ctx, interaction, "時報の音声設定を削除しました。").await?
        }
        _ => respond_text(ctx, interaction, "未対応のサブコマンドです。").await?,
    }

    Ok(())
}

async fn handle_autojoin(
    ctx: &SerenityContext,
    interaction: &ApplicationCommandInteraction,
) -> Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        respond_text(
            ctx,
            interaction,
            "このコマンドはサーバー内で使用してください。",
        )
        .await?;
        return Ok(());
    };

    let enabled = actions::toggle_autojoin(guild_id).await;
    let status = if enabled { "ON" } else { "OFF" };
    respond_text(
        ctx,
        interaction,
        &format!("Autojoin を {} に切り替えました。", status),
    )
    .await?;
    Ok(())
}

async fn handle_dict(
    ctx: &SerenityContext,
    interaction: &ApplicationCommandInteraction,
    state: &AppState,
) -> Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        respond_text(
            ctx,
            interaction,
            "このコマンドはサーバー内で使用してください。",
        )
        .await?;
        return Ok(());
    };

    let Some(subcommand) = interaction.data.options.first() else {
        respond_text(ctx, interaction, "サブコマンドを指定してください。").await?;
        return Ok(());
    };

    match subcommand.name.as_str() {
        "add" => {
            let word = extract_string_option(subcommand, "word")?;
            let read_as = extract_string_option(subcommand, "read_as")?;

            match actions::dict_add(state, guild_id, &word, &read_as).await? {
                dict::InsertResponse::Success => {
                    respond_text(
                        ctx,
                        interaction,
                        &format!("辞書に登録しました: {} → {}", word, read_as),
                    )
                    .await?
                }
                dict::InsertResponse::WordAlreadyExists => {
                    respond_text(
                        ctx,
                        interaction,
                        "すでに登録済みです。上書きする場合はいったん削除してください。",
                    )
                    .await?
                }
            }
        }
        "remove" => {
            let word = extract_string_option(subcommand, "word")?;
            match actions::dict_remove(state, guild_id, &word).await? {
                dict::RemoveResponse::Success => {
                    respond_text(ctx, interaction, &format!("辞書から削除しました: {}", word))
                        .await?
                }
                dict::RemoveResponse::WordDoesNotExist => {
                    respond_text(ctx, interaction, "指定された単語は登録されていません。").await?
                }
            }
        }
        "list" => {
            let json = actions::dict_list(state, guild_id).await?;
            if json.len() <= 1900 {
                respond_text(ctx, interaction, &format!("```json\n{}\n```", json)).await?
            } else {
                respond_text(
                    ctx,
                    interaction,
                    "件数が多すぎるため表示できません。登録内容を絞ってください。",
                )
                .await?
            }
        }
        _ => respond_text(ctx, interaction, "未対応のサブコマンドです。").await?,
    }

    Ok(())
}

async fn handle_help(
    ctx: &SerenityContext,
    interaction: &ApplicationCommandInteraction,
) -> Result<()> {
    let embed = actions::build_help_embed();
    respond_embed(ctx, interaction, embed).await
}

fn extract_string_option(option: &CommandDataOption, name: &str) -> Result<String> {
    option
        .options
        .iter()
        .find(|opt| opt.name == name)
        .and_then(|opt| opt.value.as_ref())
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Missing required option: {}", name))
}

fn find_focused_option<'a>(options: &'a [CommandDataOption]) -> Option<&'a CommandDataOption> {
    for option in options {
        if option.focused {
            return Some(option);
        }

        if let Some(inner) = find_focused_option(&option.options) {
            return Some(inner);
        }
    }

    None
}

async fn respond_text(
    ctx: &SerenityContext,
    interaction: &ApplicationCommandInteraction,
    content: impl AsRef<str>,
) -> Result<()> {
    interaction
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(content.as_ref()))
        })
        .await?;
    Ok(())
}

async fn respond_embed(
    ctx: &SerenityContext,
    interaction: &ApplicationCommandInteraction,
    embed: CreateEmbed,
) -> Result<()> {
    interaction
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.add_embed(embed.clone());
                    message
                })
        })
        .await?;
    Ok(())
}
