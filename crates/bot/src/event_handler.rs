use crate::error::report_error;
use crate::{command, voice_state};
use crate::{component_interaction, message};
use anyhow::Context as _;
use log::info;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{
        application::interaction::Interaction,
        channel::Message,
        gateway::{Activity, Ready},
        guild::Guild,
        voice::VoiceState,
    },
};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);

        ctx.set_activity(Activity::playing("!vでvcに参加"))
            .await;

        // Clear and recreate guild commands
        for guild in &ready.guilds {
            // First clear existing commands
            if let Err(err) = command::setup::clear_guild_commands(&ctx, guild.id)
                .await
                .context("Failed to clear guild application commands")
            {
                report_error(err);
            }
            
            // Then set up new commands
            if let Err(err) = command::setup::setup_guild_commands(&ctx, guild.id)
                .await
                .context("Failed to set guild application commands")
            {
                report_error(err);
            }
        }
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: bool) {
        if let Err(err) = command::setup::setup_guild_commands(&ctx, guild.id)
            .await
            .context("Failed to set guild application commands")
        {
            report_error(err);
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(command) => {
                if let Err(err) = command::handler::handle(&ctx, &command)
                    .await
                    .context("Failed to respond to slash command")
                {
                    report_error(err);
                }
            }
            Interaction::MessageComponent(component_interaction) => {
                if let Err(err) =
                    component_interaction::handler::handle(&ctx, &component_interaction)
                        .await
                        .context("Failed to respond to message components interaction")
                {
                    report_error(err);
                }
            }
            _ => {}
        };
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if let Err(err) = message::handler::handle(&ctx, msg)
            .await
            .context("Failed to handle message")
        {
            report_error(err);
        }
    }

    async fn voice_state_update(
        &self,
        ctx: Context,
        _old_voice_state: Option<VoiceState>,
        new_voice_state: VoiceState,
    ) {
        if let Err(err) = voice_state::handler::handle_update(&ctx, new_voice_state.guild_id)
            .await
            .context("Failed to handle voice state update")
        {
            report_error(err);
        }
    }
}
