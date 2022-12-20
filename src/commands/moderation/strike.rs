use serde_json::Value;
use serenity::{prelude::Context, builder::CreateApplicationCommand, model::prelude::{command::CommandOptionType, interaction::application_command::ApplicationCommandInteraction, UserId}};
use tracing::{error, warn};

use crate::{Handler, commands::{utils::{duration::Duration, messages::{send_message, defer}}, structs::CommandError}, mongo::structs::{Action, ActionType, Permissions}};

impl Handler {
    pub async fn strike(&self, ctx: &Context, guild_id: i64, user_id: i64, reason: String, moderator_id: Option<i64>, duration: Option<Duration>) -> Result<(Action, Option<Action>), CommandError> {
        let guild = match self.mongo.get_guild(guild_id).await {
            Ok(guild) => guild,
            Err(err) => {
                error!("Failed to get guild with id {}. Failed with error: {}", guild_id, err);
                return Err(CommandError {
                    message: format!("Failed to get guild with id {}", guild_id),
                    command_error: None
                });
            }
        };
        let mut strike_duration = duration;
        let mut default_strike_string = "".to_string();
        if let Some(mod_config) = &guild.config.moderation {
            default_strike_string = mod_config.default_strike_duration.to_owned();
        }
        if strike_duration.is_none() {
            strike_duration = Some(Duration::new(default_strike_string.to_owned()));
        }
        let mut escalation = None;

        match self.mongo.get_actions_for_user(user_id, guild_id).await {
            Ok(actions) => {
                let mut strikes = 1;
                for action in actions {
                    if action.active && action.action_type == ActionType::Strike {
                        strikes += 1;
                    }
                }
                if let Some(moderation_config) = &guild.config.moderation {
                    if let Some(strike_escalation) = moderation_config.strike_escalations.get(&strikes) {
                        match strike_escalation.action {
                            ActionType::Mute => {
                                let duration = match strike_escalation.duration.as_ref() {
                                    Some(duration) => Duration::new(duration.to_owned()),
                                    None => Duration::new(default_strike_string.to_owned())
                                };
                                match self.mute(ctx, guild_id, user_id, format!("Strike escalation ({})", strikes), None, Some(duration)).await {
                                    Ok(action) => {
                                        if action.is_none() {
                                            warn!("Could not escalate strike (mute) for user {} in guild {}", user_id, guild_id);
                                        }
                                        escalation = action;
                                    },
                                    Err(err) => {
                                        return Err(err);
                                    }
                                }
                            },
                            ActionType::Kick => {
                                match self.kick(ctx, guild_id, user_id, format!("Strike escalation ({})", strikes), None).await {
                                    Ok(action) => {
                                        if action.is_none() {
                                            warn!("Could not escalate strike (kick) for user {} in guild {}", user_id, guild_id);
                                        }
                                        escalation = action;
                                    },
                                    Err(err) => {
                                        return Err(err);
                                    }
                                }
                            },
                            ActionType::Ban => {
                                let duration = match strike_escalation.duration.as_ref() {
                                    Some(duration) => Duration::new(duration.to_owned()),
                                    None => Duration::new("".to_string())
                                };
                                match self.ban(ctx, guild_id, user_id, format!("Strike escalation ({})", strikes), None, Some(duration)).await {
                                    Ok(action) => {
                                        if action.is_none() {
                                            warn!("Could not escalate strike (ban) for user {} in guild {}", user_id, guild_id);
                                        }
                                        escalation = action;
                                    },
                                    Err(err) => {
                                        return Err(err);
                                    }
                                }
                            },
                            _ => {
                                warn!("{:?} is not a valid action type for strike escalation", strike_escalation.action);
                            }
                        }
                    }
                }
            },
            Err(err) => {
                error!("Failed to get actions for user with id {}. Failed with error: {}", user_id, err);
                return Err(CommandError {
                    message: format!("Failed to get actions for user with id {}", user_id),
                    command_error: None
                });
            }
        }

        let mod_id = match moderator_id {
            Some(id) => id,
            None => ctx.cache.current_user().id.0 as i64
        };
        match self.mongo.add_action_to_user(user_id, guild_id, ActionType::Strike, reason, mod_id, strike_duration).await {
            Ok(action) => {
                self.log_action(ctx, action.guild_id, &action).await;
                Ok((action, escalation))
            },
            Err(err) => {
                error!("Failed to add strike to user with id {}. Failed with error: {}", user_id, err);
                Err(CommandError {
                    message: format!("Failed to add strike to user with id {}", user_id),
                    command_error: None
                })
            }
        }
    }
}

pub async fn run(handler: &Handler, ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<(), CommandError> {
    defer(ctx, cmd, false).await?;
    match handler.has_permission(ctx, cmd.member.as_ref().unwrap(), Permissions::ModerationStrike).await {
        Ok(has_permission) => {
            if !has_permission {
                return handler.missing_permissions(ctx, cmd, Permissions::ModerationStrike).await
            }
        },
        Err(err) => {
            error!("Failed to check if user has permission to use moderation strike command. Failed with error: {}", err);
            return Err(CommandError {
                message: "Failed to check if user has permission to use moderation strike command".to_string(),
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
                            warn!("User {} in guild {} tried to strike themselves", cmd.user.id.0, cmd.guild_id.unwrap().0);
                            return send_message(ctx, cmd, "You cannot strike yourself".to_string()).await;
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

    if duration.is_none() {
        let guild = match handler.mongo.get_guild(cmd.guild_id.unwrap().0 as i64).await {
            Ok(guild) => Some(guild),
            Err(err) => {
                warn!("Guild configuration could not be found for guild {}. Failed with error: {}", cmd.guild_id.unwrap().0, err);
                None
            }
        };
        if let Some(guild) = guild {
            if let Some(mod_config) = guild.config.moderation {
                duration = Some(Duration::new(mod_config.default_strike_duration));
            }
        }
    }

    match handler.strike(
        ctx,
        cmd.guild_id.unwrap().0 as i64,
        user_id.unwrap(),
        reason.unwrap(),
        Some(cmd.user.id.0 as i64),
        duration.clone()
    ).await {
        Ok((action, escalation)) => {
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

            let mut dm_content = format!("You have been given a strike in {} by <@{}>", cmd.guild_id.unwrap().to_partial_guild(&ctx).await.unwrap().name, action.moderator_id);
            if let Some(duration) = duration.as_ref() {
                dm_content.push_str(&format!(" until <t:{}:F>", duration.to_unix_timestamp()));
            }
            dm_content.push_str(&format!(" for:\n`{}`", action.reason));
            if let Some(escalation) = escalation {
                dm_content.push_str(&format!("\n\n*You have also been **{}** ", match escalation.action_type {
                    ActionType::Unknown => "`unknown`",
                    ActionType::Strike => "given a strike",
                    ActionType::Mute => "muted",
                    ActionType::Kick => "kicked",
                    ActionType::Ban => "banned"
                }));
                if let Some(duration) = escalation.expiry {
                    dm_content.push_str(&format!("until <t:{}:F> ", duration));
                }
                dm_content.push_str(&format!("because of the amount of strikes you have*"));
            }
            match user.as_ref().unwrap().direct_message(&ctx.http, |message| {
                message
                    .content(dm_content)
            }).await {
                Ok(_) => messaged_user = true,
                Err(err) => {
                    warn!("{} could not be notified. Failed with error: {}", user.as_ref().unwrap().id.0, err);
                }
            }

            let mut message_content = format!("Strike issued to <@{}>", user.as_ref().unwrap().id.0);
            if let Some(duration) = duration.as_ref() {
                message_content.push_str(&format!(" until <t:{}:F>", duration.to_unix_timestamp()))
            }
            message_content.push_str(&format!(" for:\n`{}`", action.reason));
            if !messaged_user {
                message_content.push_str(&format!("\n*<@{}> could not be notified*", user.as_ref().unwrap().id.0));
            }
            send_message(ctx, cmd, message_content).await
        },
        Err(err) => {
            error!("Failed to strike user. Failed with error: {}", err);
            Err(CommandError {
                message: "Failed to strike user".to_string(),
                command_error: None
            })
        }
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("strike")
        .dm_permission(false)
        .description("Strike a user for an incorrect action")
        .create_option(|option| {
            option
                .name("user")
                .description("The user to give a strike to")
                .kind(CommandOptionType::User)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("reason")
                .description("The reason to give this strike to this user")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("duration")
                .description("The duration to strike the user for (default 30 days)")
                .kind(CommandOptionType::String)
                .required(false)
        })
}