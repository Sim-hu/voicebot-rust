[English](README.md)

# Discord 読み上げBot

Rust で書かれたオープンソースの Discord 読み上げ Bot です。[VOICEVOX ENGINE](https://github.com/VOICEVOX/voicevox_engine) を使って日本語テキストチャットを読み上げます。既存の TTS Bot に代わる、軽量なセルフホスト向けソリューションを目指しています。

## 特徴

- VOICEVOX ENGINE による自然な日本語音声合成
- サーバーごとの辞書機能で語句の読み方をカスタマイズ
- スラッシュコマンド対応 (`/v`, `/s`, `/dict`, `/help`)
- ボイスチャンネルへの自動参加
- 時報機能

## 前提条件

### ハードウェア

音声合成は CPU/GPU に高い負荷がかかる処理です。2 GB 以上のメモリを搭載した十分な性能のマシンを推奨します。起動直後は VOICEVOX モデルの初期化のため、遅延がより大きくなります。

### ソフトウェア

| 依存関係 | 用途 | 備考 |
|---------|------|------|
| libopus-dev | Opus 音声エンコーディング | ビルド時に必要 |
| ffmpeg | 音声フォーマット変換 | 実行時に必要 |
| Redis | 永続ストレージ（辞書、設定） | v7+ 推奨 |
| VOICEVOX ENGINE | 日本語音声合成 | CPU 版または GPU 版 |

## Discord Bot の登録

1. [Discord Developer Portal](https://discord.com/developers/applications) を開き、新しくアプリケーションを作成します。
2. **Application ID**（Client ID）を控えます。
3. **Bot** ページに移動し、**Add Bot** をクリックして **Message Content Intent** を有効にします。
4. **Reset Token** をクリックして Token を生成し、安全に保管します。
5. 以下の URL にアクセスしてサーバーに Bot を追加します（`CLIENT_ID` を置き換えてください）:

```
https://discord.com/api/oauth2/authorize?client_id=CLIENT_ID&permissions=3146752&scope=bot%20applications.commands
```

必要な権限: View Channels, Connect, Speak

## インストール

### Docker Compose（推奨）

```bash
cd deployment

# テンプレートから .env を作成し、値を設定
cp config/.env.template .env
vi .env

# 全サービスを起動
docker compose up -d

# ログ確認
docker compose logs -f

# 停止
docker compose down
```

### ネイティブビルド

```bash
# システム依存パッケージのインストール（Debian/Ubuntu）
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libopus-dev ffmpeg

# ビルド
cargo build --release

# バイナリは target/release/bot に生成されます
# Redis と VOICEVOX ENGINE が起動していることを確認してから:
export DISCORD_CLIENT_ID="your_client_id"
export DISCORD_BOT_TOKEN="your_bot_token"
export REDIS_URL="redis://localhost:6379"
export VOICEVOX_API_BASE="http://localhost:50021"

./target/release/bot
```

### NixOS

```bash
# 開発シェルに入る
nix develop

# または直接ビルド
nix build
./result/bin/bot
```

### systemd

systemd ユニットファイルが `deployment/voicebot.service` に用意されています。

```bash
# 専用ユーザーを作成
sudo useradd --system --user-group voicebot

# バイナリをインストール
sudo cp target/release/bot /usr/local/bin/bot

# 環境変数ファイルを設定
sudo mkdir -p /etc/voicebot
sudo vi /etc/voicebot/env   # DISCORD_CLIENT_ID, DISCORD_BOT_TOKEN 等を追加

# サービスをインストールして起動
sudo cp deployment/voicebot.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now voicebot
```

## 設定

Bot は環境変数から設定を読み込みます。環境変数が設定されていない場合、YAML 設定ファイル（デフォルト: `/etc/bot.yaml`、`BOT_CONFIG` で変更可能）にフォールバックします。

| 変数 | 必須 | デフォルト | 説明 |
|------|------|-----------|------|
| `DISCORD_CLIENT_ID` | はい | — | Discord アプリケーション ID |
| `DISCORD_BOT_TOKEN` | はい | — | Discord Bot トークン |
| `REDIS_URL` | はい | — | Redis 接続 URL |
| `VOICEVOX_API_BASE` | いいえ | `http://voicevox:50021` | VOICEVOX ENGINE エンドポイント |
| `RUST_LOG` | いいえ | `info` | ログレベルフィルター（[env_logger](https://docs.rs/env_logger)） |
| `SENTRY_DSN` | いいえ | — | Sentry エラートラッキング DSN |
| `BOT_CONFIG` | いいえ | `/etc/bot.yaml` | YAML 設定ファイルのパス（フォールバック） |

## 使い方

| コマンド | 説明 |
|---------|------|
| `/v` | ボイスチャンネルへの入退出（トグル） |
| `/s` | 読み上げ中のメッセージをスキップ |
| `/dict add <語句> <読み方>` | サーバー辞書に語句を追加 |
| `/dict remove <語句>` | 辞書から語句を削除 |
| `/dict list` | 辞書の全エントリを表示 |
| `/autojoin` | 現在のチャンネルの自動参加を切り替え |
| `/time` | 時報のオン/オフを切り替え |
| `/help` | ヘルプを表示 |

## プロジェクト構成

```
crates/
├── bot/          # メインバイナリ — イベント処理、コマンド、メッセージ処理
├── bot-audio/    # 音声エンコーディングと FFmpeg 連携
├── bot-call/     # ボイス接続管理
├── bot-config/   # 設定読み込み（環境変数 / YAML）
├── bot-db/       # Redis ベースの永続化（辞書、設定）
└── bot-speech/   # VOICEVOX API クライアントと音声合成
```

## ライセンス

MIT — 詳細は [LICENSE](LICENSE) をご覧ください。
