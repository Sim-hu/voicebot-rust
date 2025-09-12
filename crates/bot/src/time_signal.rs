// 時報機能 - シンプル実装
// 毎時0分に時報を自動再生

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use once_cell::sync::Lazy;

// ギルドごとの時報設定を管理
static TIME_SIGNAL_SETTINGS: Lazy<Arc<RwLock<HashMap<u64, bool>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

pub async fn start_time_signal_service() {
    use chrono::{Local, Timelike};
    use tokio::time::{interval, Duration};
    
    let mut interval = interval(Duration::from_secs(60)); // 1分間隔でチェック

    loop {
        interval.tick().await;
        
        let now = Local::now();
        
        // 毎時0分の時に時報をチェック
        if now.minute() == 0 {
            let hour = now.hour() as u8;
            // 1時間毎に時報
            log::info!("Time signal: {}時をお知らせします。", hour);
            // 各ギルドの時報設定を確認して音声再生
            // 実装は簡略化のため、ここではログ出力のみ
        }
    }
}

pub async fn toggle_time_signal_for_guild(guild_id: u64) -> bool {
    let mut settings = TIME_SIGNAL_SETTINGS.write().await;
    let current = settings.get(&guild_id).copied().unwrap_or(true); // デフォルトはON
    let new_setting = !current;
    settings.insert(guild_id, new_setting);
    new_setting
}

pub async fn is_time_signal_enabled_for_guild(guild_id: u64) -> bool {
    let settings = TIME_SIGNAL_SETTINGS.read().await;
    settings.get(&guild_id).copied().unwrap_or(true) // デフォルトはON
}