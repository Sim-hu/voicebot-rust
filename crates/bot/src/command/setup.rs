use anyhow::{Context as _, Result};
use log::info;
use serenity::{
    client::Context,
    model::{application::command::CommandOptionType, id::GuildId},
};

pub async fn setup_guild_commands(ctx: &Context, guild_id: GuildId) -> Result<()> {
    info!("Setting up guild commands for guild {}", guild_id);
    
    // Clear existing commands and set new ones
    guild_id
        .set_application_commands(&ctx.http, |commands| {
            commands
                .create_application_command(|command| {
                    command.name("help").description("使い方を表示")
                })
                .create_application_command(|command| {
                    command
                        .name("v")
                        .description("ボイスチャンネル参加/退出を切り替え")
                })
                .create_application_command(|command| {
                    command
                        .name("time")
                        .description("時報機能の設定")
                        .create_option(|option| {
                            option
                                .name("toggle")
                                .description("時報機能のON/OFFを切り替え")
                                .kind(CommandOptionType::SubCommand)
                        })
                        .create_option(|option| {
                            option
                                .name("channel")
                                .description("時報の出力チャンネルを設定")
                                .kind(CommandOptionType::SubCommand)
                                .create_sub_option(|option| {
                                    option
                                        .name("channel")
                                        .description("時報を出力するチャンネル")
                                        .kind(CommandOptionType::Channel)
                                        .required(true)
                                })
                        })
                })
                .create_application_command(|command| {
                    command
                        .name("skip")
                        .description("読み上げ中のメッセージをスキップ")
                })
                .create_application_command(|command| {
                    command
                        .name("dict")
                        .description("読み上げ辞書の閲覧と編集")
                        .create_option(|option| {
                            option
                                .name("add")
                                .description("辞書に項目を追加")
                                .kind(CommandOptionType::SubCommand)
                                .create_sub_option(|option| {
                                    option
                                        .name("word")
                                        .description("読み方を指定したい語句")
                                        .kind(CommandOptionType::String)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("read-as")
                                        .description("語句の読み方")
                                        .kind(CommandOptionType::String)
                                        .required(true)
                                })
                        })
                        .create_option(|option| {
                            option
                                .name("remove")
                                .description("辞書から項目を削除")
                                .kind(CommandOptionType::SubCommand)
                                .create_sub_option(|option| {
                                    option
                                        .name("word")
                                        .description("削除したい語句")
                                        .kind(CommandOptionType::String)
                                        .required(true)
                                })
                        })
                        .create_option(|option| {
                            option
                                .name("list")
                                .description("辞書を表示")
                                .kind(CommandOptionType::SubCommand)
                        })
                })
        })
        .await
        .context("Failed to set guild application commands")?;

    info!("Successfully set up commands for guild {}", guild_id);
    Ok(())
}

pub async fn clear_guild_commands(ctx: &Context, guild_id: GuildId) -> Result<()> {
    info!("Clearing guild commands for guild {}", guild_id);
    
    // Clear all guild commands by setting an empty list
    guild_id
        .set_application_commands(&ctx.http, |commands| commands)
        .await
        .context("Failed to clear guild application commands")?;
    
    info!("Successfully cleared guild commands for guild {}", guild_id);
    Ok(())
}

pub async fn clear_global_commands(ctx: &Context) -> Result<()> {
    info!("Clearing global commands");
    
    // Clear all global commands by setting an empty list using serde_json::json!
    ctx.http
        .create_global_application_commands(&serde_json::json!([]))
        .await
        .context("Failed to clear global application commands")?;
    
    info!("Successfully cleared global commands");
    Ok(())
}
