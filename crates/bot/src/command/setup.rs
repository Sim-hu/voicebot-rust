use anyhow::Result;
use serenity::client::Context;
use serenity::model::application::command::{Command, CommandOptionType};

pub async fn setup_commands(ctx: &Context) -> Result<()> {
    Command::set_global_application_commands(&ctx.http, |commands| {
        commands
            .create_application_command(|command| {
                command
                    .name("v")
                    .description("ボイスチャンネルへの参加／退出を切り替えます。")
            })
            .create_application_command(|command| {
                command
                    .name("s")
                    .description("現在再生中の読み上げをスキップします。")
            })
            .create_application_command(|command| {
                command
                    .name("time")
                    .description("時報機能を設定します。")
                    .create_option(|option| {
                        option
                            .name("toggle")
                            .description("時報のON/OFFを切り替えます。")
                            .kind(CommandOptionType::SubCommand)
                    })
                    .create_option(|option| {
                        option
                            .name("audio_set")
                            .description("時報で再生する音声ファイルのURLを設定します。")
                            .kind(CommandOptionType::SubCommand)
                            .create_sub_option(|sub_option| {
                                sub_option
                                    .name("url")
                                    .description("音声ファイルのURL (MP3/WAVなど)")
                                    .kind(CommandOptionType::String)
                                    .required(true)
                            })
                    })
                    .create_option(|option| {
                        option
                            .name("audio_clear")
                            .description("設定済みの時報音声を解除します。")
                            .kind(CommandOptionType::SubCommand)
                    })
            })
            .create_application_command(|command| {
                command.name("autojoin").description(
                    "ユーザーのVC参加を検知してBotを自動参加させる機能を切り替えます。",
                )
            })
            .create_application_command(|command| {
                command
                    .name("dict")
                    .description("読み替え辞書を管理します。")
                    .create_option(|option| {
                        option
                            .name("add")
                            .description("読み替えを追加します。")
                            .kind(CommandOptionType::SubCommand)
                            .create_sub_option(|sub_option| {
                                sub_option
                                    .name("word")
                                    .description("読み替え対象の単語")
                                    .kind(CommandOptionType::String)
                                    .required(true)
                            })
                            .create_sub_option(|sub_option| {
                                sub_option
                                    .name("read_as")
                                    .description("読み上げる際の読み仮名")
                                    .kind(CommandOptionType::String)
                                    .required(true)
                            })
                    })
                    .create_option(|option| {
                        option
                            .name("remove")
                            .description("登録済みの読み替えを削除します。")
                            .kind(CommandOptionType::SubCommand)
                            .create_sub_option(|sub_option| {
                                sub_option
                                    .name("word")
                                    .description("削除する単語")
                                    .kind(CommandOptionType::String)
                                    .required(true)
                                    .set_autocomplete(true)
                            })
                    })
                    .create_option(|option| {
                        option
                            .name("list")
                            .description("登録済みの読み替え一覧を表示します。")
                            .kind(CommandOptionType::SubCommand)
                    })
            })
            .create_application_command(|command| {
                command
                    .name("help")
                    .description("使い方のヘルプを表示します。")
            })
    })
    .await?;

    Ok(())
}
