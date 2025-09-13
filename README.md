# Discord 読み上げBot

読み上げる Discord Bot です。

## 特徴

- [VOICEVOX ENGINE](https://github.com/VOICEVOX/voicevox_engine) を使った流暢な発音
- 日本語テキストチャットの読み上げに特化
- 特定の語句の読み方を設定する辞書機能を搭載
- Slash Commands に対応

## 使い方（コマンド一覧）

[使い方](docs/user_guide.md)をご覧ください。

## インストール

[セットアップガイド](docs/setup_guide.md)をご覧ください。

## Docker での実行

```bash
# 起動
docker compose up -d

# 停止
docker compose down

# ログ確認
docker compose logs -f
```
