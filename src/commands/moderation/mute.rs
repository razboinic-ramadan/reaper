use serde_json::Value;
use serenity::{builder::CreateApplicationCommand, prelude::Context, model::prelude::{interaction::application_command::ApplicationCommandInteraction, command::CommandOptionType, UserId}};
use tracing::{error, warn};

use crate::{Handler, commands::{structs::CommandError, utils::{duration::Duration, messages::{send_message, defer}}}, mongo::structs::{Action, ActionType, Permissions}};

impl Handler {
    pub async fn mute(&self, ctx: &Context, guild_id: i64, user_id: i64, reason: String, moderator_id: Option<i64>, duration: Option<Duration>) -> Result<Option<Action>, CommandError> {
        let guild = match self.mongo.get_guild(guild_id).await {
            Ok(guild) => {
                guild
            },
            Err(err) => {
                error!("Failed to get guild with id {}. Failed with error: {}", guild_id, err);
                return Err(CommandError {
                    message: format!("Failed to get guild with id {}", guild_id),
                    command_error: None
                });
            }
        };

        let mod_id = match moderator_id {
            Some(id) => id,
            None => ctx.cache.current_user().id.0 as i64
        };

        if let Some(moderation_config) = guild.config.moderation {
            match ctx.http.add_member_role(guild_id as u64, user_id as u64, moderation_config.mute_role as u64, Some(reason.as_str())).await {
                Ok(_) => {
                    match self.mongo.add_action_to_user(user_id, guild_id, ActionType::Mute, reason, mod_id, duration).await {
                        Ok(action) => {
                            self.log_action(ctx, action.guild_id, &action).await;
                            Ok(Some(action))
                        },
                        Err(err) => {
                            error!("Failed to add action to user with id {}. Failed with error: {}", user_id, err);
                            Err(CommandError {
                                message: format!("Failed to add action to user with id {}", user_id),
                                command_error: None
                            })
                        }
                    }
                },
                Err(err) => {
                    error!("Failed to add mute role to user with id {}. Failed with error: {}", user_id, err);
                    Err(CommandError {
                        message: format!("Failed to add mute role to user with id {}", user_id),
                        command_error: None
                    })
                }
            }
        }
        else {
            warn!("Unable to mute user {} in guild {} because there is no moderation configuration", user_id, guild_id);
            Ok(None)
        }
    }
}

pub async fn run(handler: &Handler, ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<(), CommandError> {
    defer(ctx, cmd, false).await?;
    match handler.has_permission(ctx, cmd.member.as_ref().unwrap(), Permissions::ModerationMute).await {
        Ok(has_permission) => {
            if !has_permission {
                return handler.missing_permissions(ctx, cmd, Permissions::ModerationMute).await
            }
        },
        Err(err) => {
            error!("Failed to check if user has permission to use moderation mute command. Failed with error: {}", err);
            return Err(CommandError {
                message: "Failed to check if user has permission to use moderation mute command".to_string(),
                command_error: None
            });
        }
    }

    let mut user_id: Option<i64> = None;
    let mut reason: Option<String> = None;
    let mut duration: Option<Duration> = None;

    for option in cmd.data.options.iter() {
        match option.kind {
            CommandOptionType::User => {
                match Value::to_string(&option.value.clone().unwrap()).replace('\"', "").parse::<i64>() {
                    Ok(id) => {
                        if id == cmd.user.id.0 as i64 {
                            warn!("User {} in guild {} tried to mute themselves", cmd.user.id.0, cmd.guild_id.unwrap().0);
                            return send_message(ctx, cmd, "You cannot mute yourself".to_string()).await;
                        }
                        user_id = Some(id)
                    },
                    Err(err) => {
                        error!("Failed to parse user ID. This is because: {}", err);
                        return Err(CommandError {
                            message: "Failed to parse user ID".to_string(),
                            command_error: None
                        });
                    }
                }
            },
            CommandOptionType::String => {
                match option.name.as_str() {
                    "reason" => {
                        reason = Some(option.value.as_ref().unwrap().as_str().unwrap().to_string());
                    },
                    "duration" => {
                        duration = Some(Duration::new(option.value.as_ref().unwrap().as_str().unwrap().to_string()));
                    },
                    _ => {}
                }
            },
            _ => warn!("Option type {:?} not handled", option.kind)
        }
    }

    match handler.mute(
        ctx,
        cmd.guild_id.unwrap().0 as i64,
        user_id.unwrap(),
        reason.unwrap(),
        Some(cmd.user.id.0 as i64),
        duration.clone()
    ).await {
        Ok(action) => {
            if let Some(action) = action {
                let mut messaged_user = false;
                let mut user = ctx.cache.user(UserId(action.user_id as u64));
                if user.is_none() {
                    user = match ctx.http.get_user(action.user_id as u64).await {
                        Ok(usr) => {
                            Some(usr)
                        },
                        Err(err) => {
                            error!("Failed to get user with id {}. Failed with error: {}", action.user_id, err);
                            return Err(CommandError {
                                message: format!("Failed to get user with id {}", action.user_id),
                                command_error: None
                            });
                        }
                    }
                }

                let mut dm_content = format!("You have been muted in {} by <@{}>", cmd.guild_id.unwrap().to_partial_guild(&ctx).await.unwrap().name, action.moderator_id);
                if let Some(duration) = duration.as_ref() {
                    dm_content.push_str(&format!(" until <t:{}:F>", duration.to_unix_timestamp()));
                }
                dm_content.push_str(&format!(" for:\n`{}`", action.reason));
                match user.as_ref().unwrap().direct_message(&ctx.http, |message| {
                    message
                        .content(dm_content)
                }).await {
                    Ok(_) => messaged_user = true,
                    Err(err) => {
                        warn!("{} could not be notified. Failed with error: {}", user.as_ref().unwrap().id.0, err);
                    }
                }

                let mut message_content = format!("<@{}> is muted until <t:{}:F> for:\n`{}`", action.user_id, duration.unwrap().to_unix_timestamp(), action.reason);
                if !messaged_user {
                    message_content.push_str(&format!("\n*<@{}> could not be notified*", user.as_ref().unwrap().id.0));
                }
                send_message(ctx, cmd, message_content).await
            }
            else {
                send_message(ctx, cmd, format!("Failed to mute <@{}> because there is no mute role configured", user_id.unwrap())).await
            }
        },
        Err(_) => {
            error!("Failed to mute user {} in guild {}", user_id.unwrap(), cmd.guild_id.unwrap().0);
            Err(CommandError {
                message: format!("Failed to mute user {} in guild {}", user_id.unwrap(), cmd.guild_id.unwrap().0),
                command_error: None
            })
        }
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("mute")
        .dm_permission(false)
        .description("Mute a user for a specified amount of time")
        .create_option(|option| {
            option
                .name("user")
                .description("The user to mute")
                .kind(CommandOptionType::User)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("reason")
                .description("The reason for the mute")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("duration")
                .description("The duration of the mute")
                .kind(CommandOptionType::String)
                .required(true)
        })
}