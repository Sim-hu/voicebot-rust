use crate::app_state;
use crate::component_interaction;
use crate::message;
use crate::time_signal;
use crate::voice_state;
use once_cell::sync::OnceCell;
use serenity::{
    async_trait,
    client::{Context as SerenityContext, EventHandler},
    model::{
        application::interaction::Interaction,
        channel::Message,
        gateway::{Activity, Ready},
        voice::VoiceState,
    },
};

static COMMANDS_INITIALIZED: OnceCell<()> = OnceCell::new();

#[derive(Default)]
pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: SerenityContext, ready: Ready) {
        println!("Bot is ready! Logged in as {}", ready.user.name);

        let activity = Activity::playing("!vか/vでVCに参加");
        ctx.set_activity(activity).await;

        if COMMANDS_INITIALIZED.set(()).is_ok() {
            if let Err(e) = crate::command::setup::setup_commands(&ctx).await {
                eprintln!("Failed to setup slash commands: {}", e);
            }
        }

        for guild in &ready.guilds {
            if let Err(e) = guild
                .id
                .set_application_commands(&ctx.http, |commands| commands)
                .await
            {
                eprintln!("Failed to clear guild commands ({}): {}", guild.id, e);
            }
        }

        time_signal::spawn_service(ctx.clone());
    }

    async fn message(&self, ctx: SerenityContext, msg: Message) {
        if let Err(e) = message::handler::handle(&ctx, msg).await {
            eprintln!("Error handling message: {}", e);
        }
    }

    async fn interaction_create(&self, ctx: SerenityContext, interaction: Interaction) {
        let state = match app_state::get(&ctx).await {
            Ok(state) => state,
            Err(_) => return,
        };

        match interaction {
            Interaction::ApplicationCommand(command) => {
                if let Err(e) = crate::command::handler::handle(&ctx, &command, &state).await {
                    eprintln!("Error handling application command: {}", e);
                }
            }
            Interaction::Autocomplete(autocomplete) => {
                if let Err(e) =
                    crate::command::handler::handle_autocomplete(&ctx, &autocomplete, &state).await
                {
                    eprintln!("Error handling autocomplete interaction: {}", e);
                }
            }
            Interaction::MessageComponent(component) => {
                if let Err(e) = component_interaction::handler::handle(&ctx, &component).await {
                    eprintln!("Error handling component interaction: {}", e);
                }
            }
            _ => {}
        }
    }

    async fn voice_state_update(
        &self,
        ctx: SerenityContext,
        _old: Option<VoiceState>,
        new: VoiceState,
    ) {
        if let Err(e) = voice_state::handler::handle_update(&ctx, new.guild_id).await {
            eprintln!("Error handling voice state update: {}", e);
        }
    }
}
