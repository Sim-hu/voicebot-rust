use crate::regex::{custom_emoji_regex, url_regex};
use aho_corasick::{AhoCorasickBuilder, MatchKind};
use anyhow::Result;
use discord_md::generate::{ToMarkdownString, ToMarkdownStringOption};
use bot_db::{dict::GetAllOption, redis};
use serenity::{
    client::Context,
    model::{channel::Message, id::GuildId},
    utils::ContentSafeOptions,
};

pub async fn build_read_text(
    ctx: &Context,
    conn: &mut redis::aio::Connection,
    guild_id: GuildId,
    msg: &Message,
    last_msg: &Option<Message>,
) -> Result<String> {
    let author_name = build_author_name(ctx, msg).await;

    let content = plain_content(ctx, msg);
    let content = replace_custom_emojis(&content);
    let content = discord_md::parse(&content).to_markdown_string(
        &ToMarkdownStringOption::new()
            .omit_format(true)
            .omit_spoiler(true),
    );
    let content = improved_text_processing(&content);

    let text = content;

    let text = replace_words_on_dict(conn, guild_id, &text).await?;

    // 文字数を60文字に制限
    if text.chars().count() > 60 {
        Ok(text.chars().take(60 - 4).collect::<String>() + "、以下略")
    } else {
        Ok(text)
    }
}

fn should_read_author_name(msg: &Message, last_msg: &Option<Message>) -> bool {
    let last_msg = match last_msg {
        Some(msg) => msg,
        None => return true,
    };

    msg.author != last_msg.author
        || (msg.timestamp.unix_timestamp() - last_msg.timestamp.unix_timestamp()) > 10
}

async fn build_author_name(ctx: &Context, msg: &Message) -> String {
    msg.author_nick(&ctx.http)
        .await
        // FIXME: `User::name`はユーザーの表示名ではなく一意のユーザー名を返す。現在のSerenityの実装では、ユーザーの表示名を取得する方法がない。
        // cf. https://github.com/serenity-rs/serenity/discussions/2500
        .unwrap_or_else(|| msg.author.name.clone())
}

/// [Message]の内容を返す。ID表記されたメンションやチャンネル名は読める形に書き換える。
fn plain_content(ctx: &Context, msg: &Message) -> String {
    let mut options = ContentSafeOptions::new()
        .clean_channel(true)
        .clean_role(true)
        .clean_user(true)
        .show_discriminator(false)
        .clean_here(false)
        .clean_everyone(false);

    if let Some(guild_id) = msg.guild_id {
        options = options.display_as_member_from(guild_id);
    }

    serenity::utils::content_safe(&ctx.cache, &msg.content, &options, &msg.mentions)
}

/// カスタム絵文字を読める形に置き換える
fn replace_custom_emojis(text: &str) -> String {
    custom_emoji_regex().replace_all(text, "$1").into()
}

async fn replace_words_on_dict(
    conn: &mut redis::aio::Connection,
    guild_id: GuildId,
    text: &str,
) -> Result<String> {
    let dict = bot_db::dict::get_all(
        conn,
        GetAllOption {
            guild_id: guild_id.into(),
        },
    )
    .await?;

    let word_list = dict.iter().map(|(word, _)| word).collect::<Vec<_>>();
    let read_as_list = dict.iter().map(|(_, read_as)| read_as).collect::<Vec<_>>();

    let ac = AhoCorasickBuilder::new()
        .match_kind(MatchKind::LeftmostLongest)
        .build(word_list)?;

    Ok(ac.replace_all(text, &read_as_list))
}

/// 改善されたテキスト処理（仕様書の要求に基づく）
fn improved_text_processing(text: &str) -> String {
    let mut result = text.to_string();
    
    // URL を省略
    result = regex::Regex::new(r"https?://\S+")
        .unwrap()
        .replace_all(&result, "リンク省略")
        .to_string();
    
    // カスタム絵文字を除去
    result = regex::Regex::new(r"<a?:\w+:\d+>")
        .unwrap()
        .replace_all(&result, "")
        .to_string();
    
    // Unicode絵文字を除去
    result = regex::Regex::new(r":[^:\s]{1,20}:")
        .unwrap()
        .replace_all(&result, "")
        .to_string();
    
    // xaero関連をウェイポイント共有に置換
    result = regex::Regex::new(r"\bxaero[^\s]*")
        .unwrap()
        .replace_all(&result, "ウェイポイント共有")
        .to_string();
    
    // ファイル/画像の言及を除去
    result = regex::Regex::new(r"(?i)(画像|ファイル|画像ファイル)")
        .unwrap()
        .replace_all(&result, "")
        .to_string();
    
    // Discordメンション（@everyone, @here, @user）を除去
    result = regex::Regex::new(r"@\w+")
        .unwrap()
        .replace_all(&result, "")
        .to_string();
    
    // 括弧類を除去
    result = regex::Regex::new(r"[（()（]")
        .unwrap()
        .replace_all(&result, "")
        .to_string();
    
    // 英語の読み上げを改善
    result = improve_english_pronunciation(&result);
    
    result
}

/// 英語の読み上げを改善（アルファベットをより自然に）
fn improve_english_pronunciation(text: &str) -> String {
    let mut result = text.to_string();
    
    // 一般的な英単語をカタカナに置換
    let english_replacements = [
        ("hello", "ハロー"),
        ("thanks", "サンクス"),
        ("thank you", "サンキュー"),
        ("yes", "イエス"),
        ("no", "ノー"),
        ("ok", "オーケー"),
        ("okay", "オーケー"),
        ("good", "グッド"),
        ("bad", "バッド"),
        ("nice", "ナイス"),
        ("cool", "クール"),
        ("wow", "ワオ"),
        ("sorry", "ソーリー"),
        ("please", "プリーズ"),
        ("welcome", "ウェルカム"),
        ("you", "ユー"),
        ("me", "ミー"),
        ("help", "ヘルプ"),
        ("stop", "ストップ"),
        ("start", "スタート"),
        ("go", "ゴー"),
        ("come", "カム"),
        ("minecraft", "マインクラフト"),
        ("discord", "ディスコード"),
        ("game", "ゲーム"),
        ("play", "プレイ"),
        ("player", "プレイヤー"),
        ("server", "サーバー"),
        ("world", "ワールド"),
        ("build", "ビルド"),
        ("craft", "クラフト"),
        ("mine", "マイン"),
    ];
    
    for (english, katakana) in english_replacements.iter() {
        result = regex::Regex::new(&format!(r"(?i)\b{}\b", regex::escape(english)))
            .unwrap()
            .replace_all(&result, *katakana)
            .to_string();
    }
    
    // 残った英語の単語を少しマシにする（アルファベット一文字ずつ読まれるのを防ぐ）
    result = regex::Regex::new(r"\b[a-zA-Z]{2,}\b")
        .unwrap()
        .replace_all(&result, |caps: &regex::Captures| {
            let word = &caps[0];
            // 短い単語はそのまま、長い単語は区切って読みやすくする
            if word.len() <= 4 {
                word.to_string()
            } else {
                format!("{}、{}", &word[..word.len()/2], &word[word.len()/2..])
            }
        })
        .to_string();
    
    result
}

/// メッセージのURLを除去（レガシー用途）
fn remove_url(text: &str) -> String {
    url_regex().replace_all(text, "、").into()
}
