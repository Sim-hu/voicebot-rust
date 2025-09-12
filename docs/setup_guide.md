# Koe セットアップガイド

この文章では起動するための手順を説明します。

## 0. システム要件

### 0-1. ハードウェア

読み上げ音声の合成は非常に負荷の大きい処理です。Koe を実行するコンピュータの性能が低い場合、テキストチャンネルにメッセージが送信されてからボイスチャンネルで読み上げられるまでの遅延が大きくなります。

音声合成エンジンである VOICEVOX ENGINE では、音声合成処理に CPU または GPU を使用することができます。Bot を快適に使用するには高性能な CPU または GPU と 2GB 以上のメモリを搭載したマシンが必要です。

※起動直後はモデルの初期化処理が行われているため、遅延がより大きくなります。

### 0-2. ソフトウェア

実行には Docker および Docker Compose が必要です。あらかじめインストールしておいてください。なお、動作するには Redis と VOICEVOX ENGINE が必要ですが、これらは Docker Compose を用いて起動するため事前のインストールは不要です。

## 1. Discord Bot の登録

### 1-1. アプリケーションの作成

1. [Discord Developer Portal](https://discord.com/developers/applications) を開き、新しくアプリケーションを作成します。
2. General Information ページに記載されている Application ID (Client ID) を控えておきます。
3. Bot ページに移動し、Add Bot をクリックして Bot を有効にします。
4. Message Content Intent を有効にします。
5. Reset Token をクリックして Token を生成し、控えておきます。

### 1-2. サーバーに Bot を追加

以下の URL にアクセスしてサーバーに Bot を追加します。URL の`CLIENT_ID`は先ほど控えた Application ID に置き換えてください。

```
https://discord.com/api/oauth2/authorize?client_id=CLIENT_ID&permissions=3146752&scope=bot%20applications.commands
```

<details>
  <summary>参考: 使用する権限</summary>
  
  - OAuth2 Scopes
    - `application.commands`
    - `bot`
  - Bot Permissions
    - General Permissions
      - View Channels
    - Voice Permissions
      - Connect
      - Speak
</details>

## 2. 設定ファイルの準備

### 2-1. Redis のパスワード設定

1. `config/redis.conf`をテキストエディタで開きます。
2. `YOUR_STRONG_PASSWORD` を適当なパスワードに変更します。

### 2-2. VOICEVOX ENGINE のプリセット設定（任意）

1. `config/voicevox_presets.yaml`をテキストエディタで開きます。
2. 必要に応じてプリセットを変更します。

### 2-3. 設定

1. `config/koe.yaml`をテキストエディタで開きます。
2. 次の設定を書き換えます。
   - `discord.client_id`: 1-1 で控えた Client ID
   - `discord.bot_token`: 1-1 で控えた Bot Token
   - `voicevox.api_base`: VOICEVOX ENGINE の URL
     - Docker Compose を使用する場合はデフォルトのままで問題ありません。
   - `redis.url`: Redis に接続するための URL
     - 形式は `redis://[<username>][:<password>@]<hostname>[:port][/<db>]` です。
     - Docker Compose を使用する場合は`YOUR_STRONG_PASSWORD`を Redis のパスワードに置き換えるのみで問題ありません。
     - 詳細は https://docs.rs/redis#connection-parameters をご確認ください。

### 2-4. 環境変数の設定（任意）

`docker-compose.yml` から下記の環境変数を設定することができます。いずれも原則として設定する必要はありませんが、デバッグ時に役立ちます。

- `KOE_CONFIG`: 設定ファイルの場所
  - デフォルトでは `/etc/koe.yaml` となっています。
- `RUST_LOG`: ログレベル
  - `koe`に設定すると詳細なログが出力されます。
  - 詳細は https://docs.rs/env_logger#enabling-logging をご確認ください。
- `SENTRY_DSN`: Sentry の DSN
  - 設定するとエラーを Sentry に送信することができます。

## 3. 起動

下記のコマンドで開始・停止等の操作を行うことができます。詳細は https://docs.docker.com/compose/ をご確認ください。

- `docker compose up --detach`
  - bot, Redis, VOICEVOX ENGINE を起動します。
- `docker compose logs`
  - ログを確認します。
- `docker compose down`
  - bot, Redis, VOICEVOX ENGINE を停止します。
- `docker compose down --volumes`
  - bot, Redis, VOICEVOX ENGINE を停止し、Redis に保存されている設定をすべて削除します。
- `docker compose pull`
  - コンテナイメージを更新します。

---

