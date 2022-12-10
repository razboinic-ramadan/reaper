use serenity::{builder::CreateApplicationCommand, prelude::Context, model::prelude::{interaction::application_command::ApplicationCommandInteraction, command::CommandOptionType, ChannelId}};
use tracing::error;

use crate::{Handler, commands::{structs::CommandError, utils::{duration::Duration, messages::{send_message, defer}}}, mongo::structs::Permissions};

pub async fn run(handler: &Handler, ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<(), CommandError> {
    defer(ctx, cmd, true).await?;
    match handler.has_permission(ctx, cmd.member.as_ref().unwrap(), Permissions::ModerationDuration).await {
        Ok(has_permission) => {
            if !has_permission {
                return handler.missing_permissions(ctx, cmd, Permissions::ModerationDuration).await
            }
        },
        Err(err) => {
            error!("Failed to check if user has permission to use moderation duration command. Failed with error: {}", err);
            return Err(CommandError {
                message: "Failed to check if user has permission to use moderation duration command".to_string(),
                command_error: None
            });
        }
    }

    let mut duration: Option<Duration> = None;
    let mut uuid: Option<String> = None;

    for option in cmd.data.options.iter() {
        match option.name.as_str() {
            "duration" => {
                duration = Some(Duration::new(option.value.as_ref().unwrap().as_str().unwrap().to_string()));
            },
            "uuid" => {
                uuid = Some(option.value.as_ref().unwrap().as_str().unwrap().to_string());
            },
            _ => {}
        }
    }

    if uuid.is_none() {
        uuid = match handler.mongo.get_recent_mod_action(cmd.guild_id.unwrap().0 as i64, cmd.user.id.0 as i64).await {
            Ok(action) => {
                if let Some(action) = action {
                    Some(action.uuid.to_string())
                }
                else {
                    return send_message(ctx, cmd, "Since you have no recent actions, you will need to specify a UUID".to_string()).await;
                }
            },
            Err(err) => {
                error!("Failed to get recent mod action. Failed with error: {}", err);
                return Err(CommandError {
                    message: "Failed to get recent mod action".to_string(),
                    command_error: None
                });
            }
        }
    }

    match handler.mongo.update_action_duration(cmd.guild_id.unwrap().0 as i64, uuid.unwrap(), duration.unwrap()).await {
        Ok(action) => {
            if let Some(action) = action {
                let guild = match handler.mongo.get_guild(action.guild_id).await {
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
                                .content(format!("UUID `{}` duration (for <@{}>) has been updated to <t:{}:F> by <@{}>", action.uuid, action.user_id, action.expiry.unwrap(), cmd.user.id.0))
                                .allowed_mentions(|allowed_mentions| {
                                    allowed_mentions.empty_parse()
                                })
                        }).await {
                            error!("Failed to send message to logging channel. Failed with error: {}", err);
                        }
                    }
                }
                send_message(ctx, cmd, format!("Updated action with UUID `{}` to have a duration of <t:{}:F>", action.uuid, action.expiry.unwrap())).await
            }
            else {
                send_message(ctx, cmd, "The action with this ID does not exist".to_string()).await
            }
        },
        Err(err) => {
            error!("Failed to update action duration. Failed with error: {}", err);
            Err(CommandError {
                message: "Failed to update action duration".to_string(),
                command_error: None
            })
        }
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("duration")
        .dm_permission(false)
        .description("Update an actions duration")
        .create_option(|option| {
            option
                .name("duration")
                .description("The new duration of the action")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("uuid")
                .description("The UUID of the action to update")
                .kind(CommandOptionType::String)
                .required(false)
        })
}