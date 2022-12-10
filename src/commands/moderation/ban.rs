use serde_json::Value;
use serenity::{builder::CreateApplicationCommand, prelude::Context, model::prelude::{interaction::application_command::ApplicationCommandInteraction, command::CommandOptionType, UserId}};
use tracing::{error, warn};

use crate::{Handler, commands::{structs::CommandError, utils::{duration::Duration, messages::{send_message, defer}}}, mongo::structs::{Action, ActionType, Permissions}};

impl Handler {
    pub async fn ban(&self, ctx: &Context, guild_id: i64, user_id: i64, reason: String, moderator_id: Option<i64>, duration: Option<Duration>) -> Result<Option<Action>, CommandError> {
        let mod_id = match moderator_id {
            Some(id) => id,
            None => ctx.cache.current_user().id.0 as i64
        };

        match ctx.http.ban_user(guild_id as u64, user_id as u64, 0, reason.as_str()).await {
            Ok(_) => {
                match self.mongo.add_action_to_user(user_id, guild_id, ActionType::Ban, reason, mod_id, duration).await {
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
                error!("Failed to ban member. Failed with error: {}", err);
                Err(CommandError {
                    message: "Failed to ban member. Please try again later.".to_string(),
                    command_error: None
                })
            }
        }
    }
}

pub async fn run(handler: &Handler, ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<(), CommandError> {
    defer(ctx, cmd, false).await?;

    match handler.has_permission(ctx, cmd.member.as_ref().unwrap(), Permissions::ModerationBan).await {
        Ok(has_permission) => {
            if !has_permission {
                return handler.missing_permissions(ctx, cmd, Permissions::ModerationBan).await
            }
        },
        Err(err) => {
            error!("Failed to check if user has permission to use moderation ban command. Failed with error: {}", err);
            return Err(CommandError {
                message: "Failed to check if user has permission to use moderation ban command".to_string(),
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
                            warn!("User {} in guild {} tried to ban themselves", cmd.user.id.0, cmd.guild_id.unwrap().0);
                            return send_message(ctx, cmd, "You cannot ban yourself".to_string()).await;
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

    let mut messaged_user = false;
    let mut user = ctx.cache.user(UserId(user_id.unwrap() as u64));
    if user.is_none() {
        user = match ctx.http.get_user(user_id.unwrap() as u64).await {
            Ok(usr) => {
                Some(usr)
            },
            Err(err) => {
                error!("Failed to get user with id {}. Failed with error: {}", user_id.unwrap(), err);
                return Err(CommandError {
                    message: format!("Failed to get user with id {}", user_id.unwrap()),
                    command_error: None
                });
            }
        }
    }

    let mut dm_content = format!("You have been banned from {} by <@{}>", cmd.guild_id.unwrap().to_partial_guild(&ctx).await.unwrap().name, cmd.user.id.0);
    if let Some(duration) = duration.as_ref() {
        dm_content.push_str(&format!(" until <t:{}:F>", duration.to_unix_timestamp()));
    }
    dm_content.push_str(&format!(" for:\n`{}`", reason.as_ref().unwrap()));
    match user.as_ref().unwrap().direct_message(&ctx.http, |message| {
        message
            .content(dm_content)
    }).await {
        Ok(_) => messaged_user = true,
        Err(err) => {
            warn!("{} could not be notified. Failed with error: {}", user.as_ref().unwrap().id.0, err);
        }
    }

    match handler.ban(
        ctx,
        cmd.guild_id.unwrap().0 as i64,
        user_id.unwrap(),
        reason.unwrap(),
        Some(cmd.user.id.0 as i64),
        duration.clone()
    ).await {
        Ok(action) => {
            if let Some(action) = action {
                let mut message_content = format!("Banned <@{}>", user.as_ref().unwrap().id.0);
                if let Some(duration) = duration.as_ref() {
                    message_content.push_str(&format!(" until <t:{}:F>", duration.to_unix_timestamp()))
                }
                message_content.push_str(&format!(" for:\n`{}`", action.reason));
                if !messaged_user {
                    message_content.push_str(&format!("\n*<@{}> could not be notified*", user.as_ref().unwrap().id.0));
                }
                send_message(ctx, cmd, message_content).await
            }
            else {
                send_message(ctx, cmd, format!("Failed to ban <@{}>", user.as_ref().unwrap().id.0)).await
            }
            
        },
        Err(err) => {
            error!("Failed to ban user with id {}. Failed with error: {}", user_id.unwrap(), err);
            Err(CommandError {
                message: format!("Failed to ban user with id {}", user_id.unwrap()),
                command_error: None
            })
        }
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("ban")
        .dm_permission(false)
        .description("Ban a user from the guild, with an optional duration")
        .create_option(|option| {
            option
                .name("user")
                .description("The user to ban")
                .kind(CommandOptionType::User)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("reason")
                .description("The reason for the ban")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("duration")
                .description("The duration of the ban")
                .kind(CommandOptionType::String)
                .required(false)
        })
}