use serde_json::Value;
use serenity::{builder::CreateApplicationCommand, prelude::Context, model::prelude::{interaction::application_command::ApplicationCommandInteraction, RoleId, command::CommandOptionType, ChannelId}};
use tracing::{error, warn};

use crate::{Handler, commands::{structs::CommandError, utils::messages::{send_message, defer}}, mongo::structs::{ActionType, Permissions}};

impl Handler {
    pub async fn unmute(&self, ctx: &Context, guild_id: i64, user_id: i64, moderator_id: Option<i64>) -> Result<bool, CommandError> {
        let mod_id = match moderator_id {
            Some(id) => id,
            None => ctx.cache.current_user().id.0 as i64
        };

        let guild = match self.mongo.get_guild(guild_id).await {
            Ok(guild) => guild,
            Err(err) => {
                error!("Failed to get guild. Failed with error: {}", err);
                return Err(CommandError {
                    message: "Failed to get guild".to_string(),
                    command_error: None
                });
            }
        };

        let mut member = ctx.cache.member(guild_id as u64, user_id as u64);
        if member.is_none() {
            match ctx.http.get_member(guild_id as u64, user_id as u64).await {
                Ok(mbr) => {
                    member = Some(mbr)
                },
                Err(err) => {
                    if err.to_string() == "Unknown Member" || err.to_string() == "Unknown User" {
                        let mut action_id = None;
                        match self.mongo.get_actions_for_user(user_id, guild_id).await {
                            Ok(actions) => {
                                for action in actions {
                                    if action.action_type == ActionType::Mute && action.active {
                                        action_id = Some(action.uuid.to_string());
                                    }
                                }
                            },
                            Err(err) => {
                                error!("Failed to get actions for user. Failed with error: {}", err);
                                return Err(CommandError {
                                    message: "Failed to get actions for user".to_string(),
                                    command_error: None
                                });
                            }
                        };
                        if let Some(action_id) = action_id {
                            match self.mongo.expire_action(guild_id, action_id).await {
                                Ok(_) => {
                                    return Ok(true);
                                },
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
                    error!("Failed to get member. Failed with error: {}", err);
                    return Err(CommandError {
                        message: "Failed to get member".to_string(),
                        command_error: None
                    });
                }
            }
        }

        if let Some(moderation_config) = guild.config.moderation {
            if member.unwrap().roles.contains(&RoleId(moderation_config.mute_role as u64)) {
                match ctx.http.remove_member_role(guild_id as u64, user_id as u64, moderation_config.mute_role as u64, Some(format!("Unmuted by <@{}>", mod_id).as_str())).await {
                    Ok(_) => {
                        match self.mongo.get_actions_for_user(user_id, guild_id).await {
                            Ok(actions) => {
                                for action in actions {
                                    if action.action_type == ActionType::Mute && action.active {
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
                        error!("Failed to unmute user. Failed with error: {}", err);
                        Err(CommandError {
                            message: "Failed to unmute user".to_string(),
                            command_error: None
                        })
                    }
                }
            } else {
                Ok(false)
            }
        }
        else {
            Ok(false)
        }
    }
}

pub async fn run(handler: &Handler, ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<(), CommandError> {
    defer(ctx, cmd, false).await?;
    match handler.has_permission(ctx, cmd.member.as_ref().unwrap(), Permissions::ModerationUnmute).await {
        Ok(has_permission) => {
            if !has_permission {
                return handler.missing_permissions(ctx, cmd, Permissions::ModerationUnmute).await
            }
        },
        Err(err) => {
            error!("Failed to check if user has permission to use moderation unmute command. Failed with error: {}", err);
            return Err(CommandError {
                message: "Failed to check if user has permission to use moderation unmute command".to_string(),
                command_error: None
            });
        }
    }

    let user_id = match Value::to_string(&cmd.data.options[0].value.clone().unwrap()).replace('\"', "").parse::<i64>() {
        Ok(id) => {
            if id == cmd.user.id.0 as i64 {
                warn!("User {} in guild {} tried to unmute themselves", cmd.user.id.0, cmd.guild_id.unwrap().0);
                return send_message(ctx, cmd, "You cannot unmute yourself".to_string()).await;
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

    match handler.unmute(
        ctx,
        cmd.guild_id.unwrap().0 as i64,
        user_id as i64,
        Some(cmd.user.id.0 as i64)
    ).await {
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
                            .content(format!("<@{}> has been unmuted by <@{}>", user_id, cmd.user.id.0))
                            .allowed_mentions(|allowed_mentions| {
                                allowed_mentions.empty_parse()
                            })
                    }).await {
                        error!("Failed to send message to logging channel. Failed with error: {}", err);
                    }
                }
            }
            send_message(ctx, cmd, format!("Unmuted <@{}>", user_id)).await
        },
        Err(err) => {
            error!("Failed to unmute user. Failed with error: {}", err);
            Err(CommandError {
                message: "Failed to unmute user".to_string(),
                command_error: None
            })
        }
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("unmute")
        .dm_permission(false)
        .description("Unmutes a user")
        .create_option(|option|
            option
                .name("user")
                .description("The user to unmute")
                .kind(CommandOptionType::User)
                .required(true)
        )
}