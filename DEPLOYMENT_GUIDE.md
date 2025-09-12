# Discord 読み上げBot (Koe) - 配布版実行ガイド

このガイドでは、Docker を使用してKoe Discord読み上げBotを実行する方法を説明します。

## 必要なファイルのダウンロード

以下のファイルを同じフォルダにダウンロードしてください：

1. **docker-compose.yml** - メインの設定ファイル
2. **config/** フォルダ（以下のファイルを含む）
   - `koe.yaml.template` - Bot設定ファイルのテンプレート
   - `redis.conf` - Redis設定ファイル
   - `voicevox_presets.yaml` - VOICEVOX音声設定
3. **.env.template** - 環境変数テンプレート

## システム要件

### ハードウェア
- **CPU**: 高性能CPU推奨（音声合成処理のため）
- **メモリ**: 2GB以上推奨
- **ストレージ**: 1GB以上の空き容量

### ソフトウェア
- **Docker**: 最新版
- **Docker Compose**: 最新版

## セットアップ手順

### 1. Discord Bot の作成

1. [Discord Developer Portal](https://discord.com/developers/applications) にアクセス
2. 新しいアプリケーションを作成
3. **Application ID (Client ID)** をメモ
4. Bot ページで Token を生成し、**Bot Token** をメモ
5. **Message Content Intent** を有効化
6. 以下のURLでサーバーに招待（`CLIENT_ID`を置き換え）：
   ```
   https://discord.com/api/oauth2/authorize?client_id=CLIENT_ID&permissions=3146752&scope=bot%20applications.commands
   ```

### 2. 環境設定

1. **環境変数ファイルの作成**
   ```bash
   cp .env.template .env
   ```

2. **.env ファイルを編集**
   ```bash
   # Discord設定
   DISCORD_CLIENT_ID=bot Client(devオンにした際のbotのユーザーID) ID
   DISCORD_BOT_TOKEN=Bot Token
   
   # ログレベル（デバッグ時のみ変更）
   RUST_LOG=info
   ```

## 実行コマンド

### Bot起動
```bash
docker compose up -d
```

### ログ確認
```bash
docker compose logs -f
```

### Bot停止
```bash
docker compose down
```

### 完全リセット（Redis データも削除）
```bash
docker compose down --volumes
```

### イメージ更新
```bash
docker compose pull
docker compose up -d
```

## トラブルシューティング

### Bot が起動しない場合

1. **ログを確認**
   ```bash
   docker compose logs app
   ```

2. **よくある原因**
   - Discord Token が間違っている
   - Client ID が間違っている
   - Redis パスワードが一致していない
   - Message Content Intent が無効になっている

### 音声が再生されない場合

1. **VOICEVOX ENGINE の状態確認**
   ```bash
   docker compose logs voicevox
   ```

2. **Bot がボイスチャンネルに参加できる権限があるか確認**

### パフォーマンスが悪い場合

1. システムリソースを確認
2. `docker-compose.yml` でメモリ制限を調整
3. VOICEVOX のプリセット設定を調整（`config/voicevox_presets.yaml`）

## セキュリティについて

- **.env ファイルは Git に含めないでください**
- **Bot Token は絶対に他人と共有しないでください**
- **定期的にパスワードを変更してください**

## 更新について

新しいバージョンがリリースされた場合：

1. Bot を停止
   ```bash
   docker compose down
   ```

2. 新しいイメージを取得
   ```bash
   docker compose pull
   ```

3. Bot を再起動
   ```bash
   docker compose up -d
   ```

**注意**: このBotは日本語テキストの読み上げに特化しています。他の言語での動作は保証されていません。