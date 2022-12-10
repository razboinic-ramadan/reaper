use serde_json::Value;
use serenity::{builder::CreateApplicationCommand, prelude::Context, model::prelude::{interaction::application_command::ApplicationCommandInteraction, command::CommandOptionType, ChannelId}};
use tracing::{error, warn};

use crate::{Handler, commands::{structs::CommandError, utils::messages::{send_message, defer}}, mongo::structs::{Permissions, ActionType}};

impl Handler {
    pub async fn unban(&self, ctx: &Context, guild_id: i64, user_id: i64, moderator_id: Option<i64>) -> Result<bool, CommandError> {
        let mod_id = match moderator_id {
            Some(id) => id,
            None => ctx.cache.current_user().id.0 as i64
        };

        match ctx.http.remove_ban(guild_id as u64, user_id as u64, Some(format!("Unbanned by <@{}>", mod_id).as_str())).await {
            Ok(_) => {
                match self.mongo.get_actions_for_user(user_id, guild_id).await {
                    Ok(actions) => {
                        for action in actions {
                            if action.action_type == ActionType::Ban {
                                match self.mongo.expire_action(guild_id, action.uuid.to_string().clone()).await {
                                    Ok(_) => return Ok(true),
                                    Err(err) => {
                                        error!("Failed to expire action. Failed with error: {}", err);
                                        return Err(CommandError {
                                            message: "Failed to expire action".to_string(),
                                            command_error: None
                                        });
                                    }
                                }
                            }
                        }
                        Ok(false)
                    },
                    Err(err) => {
                        error!("Failed to get actions for user. Failed with error: {}", err);
                        Err(CommandError {
                            message: "Failed to get actions for user".to_string(),
                            command_error: None
                        })
                    }
                }
            },
            Err(err) => {
                error!("Failed to unban user. Failed with error: {}", err);
                Err(CommandError {
                    message: "Failed to unban user".to_string(),
                    command_error: None
                })
            }
        }
    }
}

pub async fn run(handler: &Handler, ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<(), CommandError> {
    defer(ctx, cmd, false).await?;
    match handler.has_permission(ctx, cmd.member.as_ref().unwrap(), Permissions::ModerationUnban).await {
        Ok(has_permission) => {
            if !has_permission {
                return handler.missing_permissions(ctx, cmd, Permissions::ModerationUnban).await
            }
        },
        Err(err) => {
            error!("Failed to check if user has permission to use moderation unban command. Failed with error: {}", err);
            return Err(CommandError {
                message: "Failed to check if user has permission to use moderation unban command".to_string(),
                command_error: None
            });
        }
    }

    let user_id = match Value::to_string(&cmd.data.options[0].value.clone().unwrap()).replace('\"', "").parse::<i64>() {
        Ok(id) => {
            if id == cmd.user.id.0 as i64 {
                warn!("User {} in guild {} tried to unban themselves", cmd.user.id.0, cmd.guild_id.unwrap().0);
                return send_message(ctx, cmd, "You cannot unban yourself".to_string()).await;
            }
            id as u64
        },
        Err(err) => {
            error!("Failed to parse user ID. This is because: {}", err);
            return Err(CommandError {
                message: "Failed to parse user ID".to_string(),
                command_error: None
            });
        }
    };
    
    match handler.unban(
        ctx,
        cmd.guild_id.unwrap().0 as i64,
        user_id as i64,
        Some(cmd.user.id.0 as i64)
    ).await {
        Ok(unbanned) => {
            if unbanned {
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
                                .content(format!("<@{}> has been unbanned by <@{}>", user_id, cmd.user.id.0))
                                .allowed_mentions(|allowed_mentions| {
                                    allowed_mentions.empty_parse()
                                })
                        }).await {
                            error!("Failed to send message to logging channel. Failed with error: {}", err);
                        }
                    }
                }
                send_message(ctx, cmd, format!("Unbanned <@{}>", user_id)).await
            } else {
                send_message(ctx, cmd, format!("Failed to unban <@{}>", user_id)).await
            }
        },
        Err(err) => {
            error!("Failed to unban user. Failed with error: {}", err);
            Err(CommandError {
                message: "Failed to unban user".to_string(),
                command_error: None
            })
        }
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("unban")
        .dm_permission(false)
        .description("Unban a user")
        .create_option(|option| {
            option
                .name("user")
                .description("The user to unban")
                .kind(CommandOptionType::User)
                .required(true)
        })
}