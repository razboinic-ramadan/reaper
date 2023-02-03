use serenity::{prelude::{EventHandler, Context}, model::prelude::{Ready, Activity, command::Command, interaction::Interaction, Message, ChannelId, MessageId, GuildId, MessageUpdateEvent, Reaction}};
use tracing::{info, error};
use crate::{Handler, commands, events::expiry::expire_actions};

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        self.on_message(&ctx, &msg).await;
    }

    async fn message_delete(&self, ctx: Context, channel_id: ChannelId, deleted_message_id: MessageId, guild_id: Option<GuildId>) {
        if let Some(guild_id) = guild_id {
            self.on_message_delete(&ctx, guild_id.0 as i64, channel_id.0 as i64, deleted_message_id.0 as i64).await;
        }
    }

    async fn message_update(&self, ctx: Context, _old_if_available: Option<Message>, _new: Option<Message>, event: MessageUpdateEvent) {
        self.on_message_edit(&ctx, event).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        self.on_command(ctx, interaction).await;
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        self.reaction_add(&ctx, &reaction).await;
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        tokio::spawn(expire_actions(ctx.to_owned(), self.to_owned()));

        ctx.set_activity(Activity::playing("with users' emotions")).await;
        let commands = Command::set_global_application_commands(&ctx.http, |commands| {
            commands
                .create_application_command(|command| {commands::permissions::router::register(command)})
                .create_application_command(|command| {commands::moderation::strike::register(command)})
                .create_application_command(|command| {commands::moderation::search::register(command)})
                .create_application_command(|command| {commands::moderation::mute::register(command)})
                .create_application_command(|command| {commands::moderation::unmute::register(command)})
                .create_application_command(|command| {commands::moderation::kick::register(command)})
                .create_application_command(|command| {commands::moderation::ban::register(command)})
                .create_application_command(|command| {commands::moderation::unban::register(command)})
                .create_application_command(|command| {commands::moderation::remove::register(command)})
                .create_application_command(|command| {commands::moderation::expire::register(command)})
                .create_application_command(|command| {commands::moderation::duration::register(command)})
                .create_application_command(|command| {commands::moderation::reason::register(command)})
        }).await;
        match commands {
            Ok(commands) => {
                info!("Command registration complete");
                let mut comamnd_names = "Successfully registered commands: ".to_string();
                for command in commands.iter() {
                    comamnd_names.push_str(&command.name);
                    comamnd_names.push_str(", ");
                }
                comamnd_names.pop();
                comamnd_names.pop();
                info!("{}", comamnd_names);
            },
            Err(err) => error!("Could not successfully register all commands. Failed with error: {}", err)
        }
    }
}