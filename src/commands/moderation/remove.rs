use serenity::{builder::CreateApplicationCommand, prelude::Context, model::prelude::{interaction::application_command::ApplicationCommandInteraction, command::CommandOptionType, ChannelId}};
use tracing::error;

use crate::{Handler, commands::{structs::CommandError, utils::messages::{send_message, defer}}, mongo::structs::Permissions};

pub async fn run(handler: &Handler, ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<(), CommandError> {
    defer(ctx, cmd, false).await?;
    match handler.has_permission(ctx, cmd.member.as_ref().unwrap(), Permissions::ModerationRemove).await {
        Ok(has_permission) => {
            if !has_permission {
                return handler.missing_permissions(ctx, cmd, Permissions::ModerationRemove).await
            }
        },
        Err(err) => {
            error!("Failed to check if user has permission to use moderation remove command. Failed with error: {}", err);
            return Err(CommandError {
                message: "Failed to check if user has permission to use moderation remove command".to_string(),
                command_error: None
            });
        }
    }

    let uuid = cmd.data.options[0].value.as_ref().unwrap().as_str().unwrap().to_string();
    match handler.mongo.remove_action(cmd.guild_id.unwrap().0 as i64, uuid.clone()).await {
        Ok(_) => {
            let guild = match handler.mongo.get_guild(cmd.guild_id.unwrap().0 as i64).await {
                Ok(guild) => Some(guild),
                Err(err) => {
                    error!("Failed to get guild. Failed with error: {}", err);
                    None
                }
            };
            if let Some(guild) = guild {
                if let Some(logging_config) = guild.config.logging {
                    if let Err(err) = ChannelId(logging_config.logging_channel as u64).send_message(&ctx.http, |message| {
                        message
                            .content(format!("UUID `{}` has been removed by <@{}>", uuid, cmd.user.id.0))
                            .allowed_mentions(|allowed_mentions| {
                                allowed_mentions.empty_parse()
                            })
                    }).await {
                        error!("Failed to send message to logging channel. Failed with error: {}", err);
                    }
                }
            }
            send_message(ctx, cmd, format!("Action with UUID `{}` successfully removed!", uuid)).await
        },
        Err(err) => {
            error!("Failed to remove action. Failed with error: {}", err);
            Err(CommandError {
                message: "Failed to remove action".to_string(),
                command_error: None
            })
        }
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("remove")
        .dm_permission(false)
        .description("Remove a moderation action completely")
        .create_option(|option| {
            option
                .name("uuid")
                .description("The UUID of the action to remove")
                .kind(CommandOptionType::String)
                .required(true)
        })
}