[日本語](README_JA.md)

# Discord Text-to-Speech Bot

An open-source Discord TTS bot written in Rust. It reads aloud Japanese text chat messages using [VOICEVOX ENGINE](https://github.com/VOICEVOX/voicevox_engine), providing a lightweight self-hosted alternative to existing TTS bots.

## Features

- Natural Japanese speech synthesis powered by VOICEVOX ENGINE
- Per-server dictionary for custom word pronunciations
- Slash command interface (`/v`, `/s`, `/dict`, `/help`)
- Auto-join voice channels on message
- Hourly time announcements

## Prerequisites

### Hardware

Speech synthesis is CPU/GPU intensive. A reasonably powerful machine with at least 2 GB of RAM is recommended. Latency will be higher on slower hardware, especially right after startup while the VOICEVOX model initializes.

### Software

| Dependency | Purpose | Notes |
|-----------|---------|-------|
| libopus-dev | Opus audio encoding | Required at build time |
| ffmpeg | Audio format conversion | Required at runtime |
| Redis | Persistent storage (dictionaries, settings) | v7+ recommended |
| VOICEVOX ENGINE | Japanese speech synthesis | CPU or GPU version |

## Discord Bot Setup

1. Open the [Discord Developer Portal](https://discord.com/developers/applications) and create a new application.
2. Note the **Application ID** (Client ID).
3. Go to the **Bot** page, click **Add Bot**, and enable **Message Content Intent**.
4. Click **Reset Token** to generate a token — save it securely.
5. Invite the bot to your server with the following URL (replace `CLIENT_ID`):

```
https://discord.com/api/oauth2/authorize?client_id=CLIENT_ID&permissions=3146752&scope=bot%20applications.commands
```

Required permissions: View Channels, Connect, Speak.

## Installation

### Docker Compose (recommended)

```bash
cd deployment

# Create .env from template and fill in your values
cp config/.env.template .env
vi .env

# Start all services
docker compose up -d

# View logs
docker compose logs -f

# Stop
docker compose down
```

### Native Build

```bash
# Install system dependencies (Debian/Ubuntu)
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libopus-dev ffmpeg

# Build
cargo build --release

# The binary is at target/release/bot
# Make sure Redis and VOICEVOX ENGINE are running, then:
export DISCORD_CLIENT_ID="your_client_id"
export DISCORD_BOT_TOKEN="your_bot_token"
export REDIS_URL="redis://localhost:6379"
export VOICEVOX_API_BASE="http://localhost:50021"

./target/release/bot
```

### NixOS

```bash
# Enter development shell
nix develop

# Or build the bot directly
nix build
./result/bin/bot
```

### systemd

A systemd unit file is provided at `deployment/voicebot.service`.

```bash
# Create a dedicated user
sudo useradd --system --user-group voicebot

# Install the binary
sudo cp target/release/bot /usr/local/bin/bot

# Set up environment file
sudo mkdir -p /etc/voicebot
sudo vi /etc/voicebot/env   # Add DISCORD_CLIENT_ID, DISCORD_BOT_TOKEN, etc.

# Install and start the service
sudo cp deployment/voicebot.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now voicebot
```

## Configuration

The bot reads configuration from environment variables. If they are not set, it falls back to a YAML config file (default: `/etc/bot.yaml`, override with `BOT_CONFIG`).

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DISCORD_CLIENT_ID` | Yes | — | Discord application ID |
| `DISCORD_BOT_TOKEN` | Yes | — | Discord bot token |
| `REDIS_URL` | Yes | — | Redis connection URL |
| `VOICEVOX_API_BASE` | No | `http://voicevox:50021` | VOICEVOX ENGINE endpoint |
| `RUST_LOG` | No | `info` | Log level filter ([env_logger](https://docs.rs/env_logger)) |
| `SENTRY_DSN` | No | — | Sentry error tracking DSN |
| `BOT_CONFIG` | No | `/etc/bot.yaml` | Path to YAML config file (fallback) |

## Usage

| Command | Description |
|---------|-------------|
| `/v` | Join / leave the voice channel (toggle) |
| `/s` | Skip the current message being read |
| `/dict add <word> <reading>` | Add a word to the server dictionary |
| `/dict remove <word>` | Remove a word from the dictionary |
| `/dict list` | Show all dictionary entries |
| `/autojoin` | Toggle auto-join for the current channel |
| `/time` | Toggle hourly time announcements |
| `/help` | Show help information |

## Project Structure

```
crates/
├── bot/          # Main bot binary — event handling, commands, message processing
├── bot-audio/    # Audio encoding and FFmpeg integration
├── bot-call/     # Voice connection management
├── bot-config/   # Configuration loading (env vars / YAML)
├── bot-db/       # Redis-backed persistence (dictionaries, settings)
└── bot-speech/   # VOICEVOX API client and speech synthesis
```

## License

MIT — see [LICENSE](LICENSE) for details.
