use super::{
    model::{Command, DictAddOption, DictRemoveOption, TimeChannelOption},
    parser::parse,
};
use crate::{app_state, component_interaction::custom_id};
use anyhow::{anyhow, bail, Context as _, Result};
use bot_db::{
    dict::{GetAllOption, InsertOption, InsertResponse, RemoveOption, RemoveResponse},
    voice::GetOption,
};
use rand::seq::SliceRandom;
use serenity::{
    builder::{
        CreateActionRow, CreateComponents, CreateEmbed, CreateSelectMenu, CreateSelectMenuOption,
    },
    client::Context,
    model::{
        application::interaction::{
            application_command::ApplicationCommandInteraction, InteractionResponseType,
            MessageFlags,
        },
        id::{ChannelId, GuildId, UserId},
    },
};

pub async fn handle(ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<()> {
    match parse(cmd) {
        Command::VoiceToggle => handle_voice_toggle(ctx, cmd)
            .await
            .context("Failed to execute /v")?,
        Command::Skip => handle_skip(ctx, cmd)
            .await
            .context("Failed to execute /skip")?,
        Command::DictAdd(option) => handle_dict_add(ctx, cmd, option)
            .await
            .context("Failed to execute /dict add")?,
        Command::DictRemove(option) => handle_dict_remove(ctx, cmd, option)
            .await
            .context("Failed to execute /dict remove")?,
        Command::DictList => handle_dict_list(ctx, cmd)
            .await
            .context("Failed to execute /dict list")?,
        Command::Help => handle_help(ctx, cmd)
            .await
            .context("Failed to execute /help")?,
        Command::TimeToggle => handle_time_toggle(ctx, cmd)
            .await
            .context("Failed to execute /time")?,
        Command::TimeChannel(option) => handle_time_channel(ctx, cmd, option)
            .await
            .context("Failed to execute /time channel")?,
        Command::Unknown => {
            bail!("Unknown command: {:?}", cmd);
        }
    };

    Ok(())
}

async fn handle_voice_toggle(ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<()> {
    let guild_id = match cmd.guild_id {
        Some(id) => id,
        None => {
            r(ctx, cmd, "`/v` はサーバー内でのみ使えます。").await?;
            return Ok(());
        }
    };

    if bot_call::is_connected(ctx, guild_id).await? {
        // Leave voice channel
        bot_call::leave(ctx, guild_id).await?;
        let state = app_state::get(ctx).await?;
        state.connected_guild_states.remove(&guild_id);
        r(ctx, cmd, "ボイスチャンネルから退出しました。").await?;
    } else {
        // Join voice channel
        let user_id = cmd.user.id;
        let text_channel_id = cmd.channel_id;

        let voice_channel_id = match get_user_voice_channel(ctx, &guild_id, &user_id)? {
            Some(channel) => channel,
            None => {
                r(ctx, cmd, "まずボイスチャンネルに参加してください。").await?;
                return Ok(());
            }
        };

        bot_call::join_deaf(ctx, guild_id, voice_channel_id).await?;

        let state = app_state::get(ctx).await?;
        state.connected_guild_states.insert(
            guild_id,
            app_state::ConnectedGuildState {
                bound_text_channel: text_channel_id,
                last_message_read: None,
            },
        );

        r(ctx, cmd, "ボイスチャンネルに参加しました。").await?;
    }

    Ok(())
}

async fn handle_time_toggle(ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<()> {
    let guild_id = match cmd.guild_id {
        Some(id) => id,
        None => {
            r(ctx, cmd, "`/time toggle` はサーバー内でのみ使えます。").await?;
            return Ok(());
        }
    };

    let enabled = crate::time_signal::toggle_time_signal_for_guild(guild_id.into()).await;
    let status = if enabled { "有効" } else { "無効" };
    r(ctx, cmd, format!("時報機能を{}にしました。毎時0分に時刻をお知らせします。", status)).await?;
    Ok(())
}

async fn handle_time_channel(ctx: &Context, cmd: &ApplicationCommandInteraction, option: TimeChannelOption) -> Result<()> {
    let guild_id = match cmd.guild_id {
        Some(id) => id,
        None => {
            r(ctx, cmd, "`/time channel` はサーバー内でのみ使えます。").await?;
            return Ok(());
        }
    };

    // TODO: Store the time signal channel in database
    // For now, just respond that the feature will be implemented
    let channel_mention = format!("<#{}>", option.channel_id);
    r(ctx, cmd, format!("時報の出力チャンネルを{}に設定しました。", channel_mention)).await?;
    Ok(())
}

async fn handle_skip(ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<()> {
    let guild_id = match cmd.guild_id {
        Some(id) => id,
        None => {
            r(ctx, cmd, "`/skip` はサーバー内でのみ使えます。").await?;
            return Ok(());
        }
    };

    if !bot_call::is_connected(ctx, guild_id).await? {
        {
            r(ctx, cmd, "どのボイスチャンネルにも接続していません。").await?;
            return Ok(());
        };
    }

    bot_call::skip(ctx, guild_id).await?;

    r(ctx, cmd, "読み上げ中のメッセージをスキップしました。").await?;
    Ok(())
}


async fn handle_dict_add(
    ctx: &Context,
    cmd: &ApplicationCommandInteraction,
    option: DictAddOption,
) -> Result<()> {
    let guild_id = match cmd.guild_id {
        Some(id) => id,
        None => {
            r(ctx, cmd, "`/dict add` はサーバー内でのみ使えます。").await?;
            return Ok(());
        }
    };

    let state = app_state::get(ctx).await?;
    let mut conn = state.redis_client.get_async_connection().await?;

    let resp = bot_db::dict::insert(
        &mut conn,
        InsertOption {
            guild_id: guild_id.into(),
            word: option.word.clone(),
            read_as: option.read_as.clone(),
        },
    )
    .await?;

    let msg = match resp {
        InsertResponse::Success => format!(
            "{}の読み方を{}として辞書に登録しました。",
            sanitize_response(&option.word),
            sanitize_response(&option.read_as)
        ),
        InsertResponse::WordAlreadyExists => format!(
            "すでに{}は辞書に登録されています。",
            sanitize_response(&option.word)
        ),
    };
    r(ctx, cmd, msg).await?;
    Ok(())
}

async fn handle_dict_remove(
    ctx: &Context,
    cmd: &ApplicationCommandInteraction,
    option: DictRemoveOption,
) -> Result<()> {
    let guild_id = match cmd.guild_id {
        Some(id) => id,
        None => {
            r(ctx, cmd, "`/dict remove` はサーバー内でのみ使えます。").await?;
            return Ok(());
        }
    };

    let state = app_state::get(ctx).await?;
    let mut conn = state.redis_client.get_async_connection().await?;

    let resp = bot_db::dict::remove(
        &mut conn,
        RemoveOption {
            guild_id: guild_id.into(),
            word: option.word.clone(),
        },
    )
    .await?;

    let msg = match resp {
        RemoveResponse::Success => format!(
            "辞書から{}を削除しました。",
            sanitize_response(&option.word)
        ),
        RemoveResponse::WordDoesNotExist => format!(
            "{}は辞書に登録されていません。",
            sanitize_response(&option.word)
        ),
    };
    r(ctx, cmd, msg).await?;
    Ok(())
}

async fn handle_dict_list(ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<()> {
    let guild_id = match cmd.guild_id {
        Some(id) => id,
        None => {
            r(ctx, cmd, "`/dict list` はサーバー内でのみ使えます。").await?;
            return Ok(());
        }
    };

    let state = app_state::get(ctx).await?;
    let mut conn = state.redis_client.get_async_connection().await?;

    let dict = bot_db::dict::get_all(
        &mut conn,
        GetAllOption {
            guild_id: guild_id.into(),
        },
    )
    .await?;

    {
        let mut embed = CreateEmbed::default();

        let guild_name = guild_id
            .name(&ctx.cache)
            .unwrap_or_else(|| "サーバー".to_string());
        embed.title(format!("📕 {}の辞書", guild_name));

        embed.fields(
            dict.into_iter()
                .map(|(word, read_as)| (word, sanitize_response(&read_as), false)),
        );

        cmd.create_interaction_response(&ctx.http, |create_response| {
            create_response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|create_message| create_message.add_embed(embed))
        })
        .await
        .context("Failed to create interaction response")?;
    };

    Ok(())
}

async fn handle_help(ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<()> {
    let help_message = r#"**🎤 あすとら - Discord読み上げBot**

**基本的な使い方:**
• `/v` または `!v` - ボイスチャンネルに参加/退出
• `/skip` または `!s` - 読み上げ中のメッセージをスキップ
• `/time toggle` - 時報機能のON/OFF切り替え
• `/time channel` - 時報の出力チャンネル設定

**辞書機能:**
• `/dict add <語句> <読み方>` - 読み方を辞書に追加
• `/dict remove <語句>` - 辞書から削除
• `/dict list` - 辞書一覧を表示

**ご利用方法:**
1. あなたがボイスチャンネルに参加
2. `/v` または `!v` でボットを呼び出し
3. テキストチャンネルでメッセージを送信すると読み上げます

ステータス: !vでvcに参加"#;
    
    r(ctx, cmd, help_message).await?;
    Ok(())
}

fn get_user_voice_channel(
    ctx: &Context,
    guild_id: &GuildId,
    user_id: &UserId,
) -> Result<Option<ChannelId>> {
    let guild = guild_id
        .to_guild_cached(&ctx.cache)
        .context("Failed to find guild in the cache")?;

    let channel_id = guild
        .voice_states
        .get(user_id)
        .and_then(|voice_state| voice_state.channel_id);

    Ok(channel_id)
}

// Helper function to create text message response
async fn r(ctx: &Context, cmd: &ApplicationCommandInteraction, text: impl ToString) -> Result<()> {
    cmd.create_interaction_response(&ctx.http, |create_response| {
        create_response
            .kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|create_message| create_message.content(text))
    })
    .await
    .context("Failed to create interaction response")?;

    Ok(())
}

fn sanitize_response(text: &str) -> String {
    format!("`{}`", text.replace('`', ""))
}
