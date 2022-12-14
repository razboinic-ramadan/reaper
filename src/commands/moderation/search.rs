use std::time::Duration;

use serde_json::Value;
use serenity::{builder::CreateApplicationCommand, prelude::Context, model::prelude::{interaction::{application_command::ApplicationCommandInteraction, InteractionResponseType}, command::CommandOptionType, component::ButtonStyle}, futures::StreamExt};
use tracing::{error, warn};

use crate::{Handler, commands::{structs::CommandError, utils::messages::{defer}}, mongo::structs::{Permissions, ActionType}};

pub async fn run(handler: &Handler, ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<(), CommandError> {
    match cmd.data.options[0].name.as_str() {
        "user" => {
            let mut user_id: i64 = cmd.user.id.0 as i64;
            let mut expired = false;

            for option in cmd.data.options[0].options.iter() {
                match option.kind {
                    CommandOptionType::User => {
                        match Value::to_string(&option.value.clone().unwrap()).replace("\"", "").parse::<i64>() {
                            Ok(id) => {
                                user_id = id;
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
                    CommandOptionType::Boolean => {
                        expired = option.value.as_ref().unwrap().as_bool().unwrap();
                    },
                    _ => warn!("Option type {:?} not handled", option.kind)
                }
            }

            let permission;
            if user_id == cmd.user.id.0 as i64 {
                if let Err(err) = defer(&ctx, &cmd, true).await {
                    return Err(err)
                }
                if expired {
                    permission = Permissions::ModerationSearchSelfExpired;
                }
                else {
                    permission = Permissions::ModerationSearchSelf;
                }
            }
            else {
                if let Err(err) = defer(&ctx, &cmd, false).await {
                    return Err(err)
                }
                if expired {
                    permission = Permissions::ModerationSearchOthersExpired;
                }
                else {
                    permission = Permissions::ModerationSearchOthers;
                }
            }

            match handler.has_permission(&ctx, &cmd.member.as_ref().unwrap(), permission).await {
                Ok(has_permission) => {
                    if !has_permission {
                        return handler.missing_permissions(&ctx, &cmd, permission).await
                    }
                },
                Err(err) => {
                    error!("Failed to check if user has permission to use moderation search command. Failed with error: {}", err);
                    return Err(CommandError {
                        message: "Failed to check if user has permission to use moderation search command".to_string(),
                        command_error: None
                    });
                }
            }

            let user = match ctx.http.get_user(user_id as u64).await {
                Ok(user) => user,
                Err(err) => {
                    error!("Failed to get user. Failed with error: {}", err);
                    return Err(CommandError {
                        message: "Failed to get user".to_string(),
                        command_error: None
                    });
                }
            };

            let mut actions = match handler.mongo.get_actions_for_user(user_id, cmd.guild_id.unwrap().0 as i64).await {
                Ok(actions) => actions,
                Err(err) => {
                    error!("Failed to get actions for user. Failed with error: {}", err);
                    return Err(CommandError {
                        message: "Failed to get actions for user".to_string(),
                        command_error: None
                    });
                }
            };
            actions.retain(|action| {
                if expired {
                    true
                }
                else {
                    action.active
                }
            });
            if actions.len() == 0 {
                if let Err(err) = cmd.edit_original_interaction_response(&ctx.http, |message| {
                    message
                        .embed(|embed| {
                            embed
                                .title(format!("{}#{:0>4}'s history", user.name, user.discriminator))
                                .description(format!("<@{}>\nNo actions found", user.id.0))
                        })
                }).await {
                    error!("Failed to edit original interaction response. Failed with error: {}", err);
                    return Err(CommandError {
                        message: "Failed to edit original interaction response".to_string(),
                        command_error: None
                    });
                }
                return Ok(())
            }
            else {
                let field_title;
                match actions[0].active {
                    true => field_title = match actions[0].action_type {
                        ActionType::Strike => "Strike",
                        ActionType::Mute => "Mute",
                        ActionType::Kick => "Kick",
                        ActionType::Ban => "Ban",
                        ActionType::Unknown => "Unknown"
                    }.to_string(),
                    false => field_title = format!("{} (Expired)", match actions[0].action_type {
                        ActionType::Strike => "Strike",
                        ActionType::Mute => "Mute",
                        ActionType::Kick => "Kick",
                        ActionType::Ban => "Ban",
                        ActionType::Unknown => "Unknown"
                    })
                }
                let mut field_description = format!("{}\n\n*Issued by:* <@{}>\n*Issued at:* <t:{}:F>\n", actions[0].reason, actions[0].moderator_id, actions[0].uuid.timestamp().timestamp_millis() / 1000);
                if let Some(duration) = actions[0].expiry {
                    field_description.push_str(&format!("*Expires:* <t:{}:F>\n", duration));
                }
                field_description.push_str(&format!("*UUID:* `{}`", actions[0].uuid.to_string()));
                if let Err(err) = cmd.edit_original_interaction_response(&ctx.http, |message| {
                    message
                        .embed(|embed| {
                            embed
                                .title(format!("{}#{}'s history", user.name, user.discriminator))
                                .description(format!("<@{}> - 1/{} actions", user.id, actions.len()))
                                .field(field_title, field_description, false)
                        })
                        .components(|components| {
                            components
                                .create_action_row(|action_row| {
                                    action_row
                                        .create_button(|button| {
                                            button
                                                .custom_id("previous")
                                                .style(ButtonStyle::Primary)
                                                .label("Previous")
                                                .disabled(true)
                                        })
                                        .create_button(|button| {
                                            button
                                                .custom_id("next")
                                                .style(ButtonStyle::Primary)
                                                .label("Next")
                                                .disabled(actions.len() == 1)
                                        })
                                })
                                .create_action_row(|row| {
                                    row
                                    .create_select_menu(|menu| {
                                        menu
                                            .custom_id("action")
                                            .placeholder("Action")
                                            .options(|options| {
                                                let mut options = options;
                                                for i in 1..actions.len() + 1 {
                                                    options = options.create_option(|option| {
                                                        let mut label = format!("Action {} - {} ({}", i, actions[i - 1].reason, match actions[i - 1].action_type {
                                                            ActionType::Strike => "Strike",
                                                            ActionType::Mute => "Mute",
                                                            ActionType::Kick => "Kick",
                                                            ActionType::Ban => "Ban",
                                                            ActionType::Unknown => "Unknown"
                                                        });
                                                        match actions[i - 1].active {
                                                            true => {},
                                                            false => label.push_str(" - Expired")
                                                        }
                                                        label.push(')');
                                                        option
                                                            .label(label)
                                                            .value(format!("{}", i))
                                                    });
                                                }
                                                options
                                            })
                                    })
                                })
                        })
                }).await {
                    error!("Failed to edit original interaction response. Failed with error: {}", err);
                    return Err(CommandError {
                        message: "Failed to edit original interaction response".to_string(),
                        command_error: None
                    });
                }
            }
            let mut page = 0;
            let mut interaction_stream = match cmd.get_interaction_response(&ctx.http).await {
                Ok(interaction) => interaction.await_component_interactions(&ctx).timeout(Duration::from_secs(60 * 5)).build(),
                Err(err) => {
                    error!("Failed to get interaction response. Failed with error: {}", err);
                    return Err(CommandError {
                        message: "Failed to get interaction response".to_string(),
                        command_error: None
                    });
                }
            };
            
            while let Some(interaction) = interaction_stream.next().await {
                if interaction.user.id != cmd.user.id {
                    if let Err(err) = interaction.create_followup_message(&ctx.http, |message| {
                        message
                            .content("You can't use this button, since you didn't run this command")
                            .ephemeral(true)
                    }).await {
                        error!("Failed to create followup message. Failed with error: {}", err);
                        return Err(CommandError {
                            message: "Failed to create followup message".to_string(),
                            command_error: None
                        });
                    }
                }
                match interaction.create_interaction_response(&ctx.http, |message| {
                    message
                        .kind(InteractionResponseType::DeferredUpdateMessage)
                }).await {
                    Ok(_) => {},
                    Err(err) => {
                        error!("Failed to create interaction response. Failed with error: {}", err);
                        return Err(CommandError {
                            message: "Failed to create interaction response".to_string(),
                            command_error: None
                        });
                    }
                };
                match interaction.data.custom_id.as_str() {
                    "next" => {
                        if page + 1 < actions.len() {
                            page += 1;
                        }
                    },
                    "previous" => {
                        if page > 0 {
                            page -= 1;
                        }
                    }
                    "action" => {
                        if let Some(value) = interaction.data.values.get(0) {
                            if let Ok(value) = value.parse::<usize>() {
                                if value > 0 && value <= actions.len() {
                                    page = value - 1;
                                }
                            }
                        }
                    }
                    _ => {}
                }
                let field_title;
                match actions[page].active {
                    true => field_title = match actions[page].action_type {
                        ActionType::Strike => "Strike",
                        ActionType::Mute => "Mute",
                        ActionType::Kick => "Kick",
                        ActionType::Ban => "Ban",
                        ActionType::Unknown => "Unknown"
                    }.to_string(),
                    false => field_title = format!("{} (Expired)", match actions[page].action_type {
                        ActionType::Strike => "Strike",
                        ActionType::Mute => "Mute",
                        ActionType::Kick => "Kick",
                        ActionType::Ban => "Ban",
                        ActionType::Unknown => "Unknown"
                    })
                }
                let mut field_description = format!("{}\n\n*Issued by:* <@{}>\n*Issued at:* <t:{}:F>\n", actions[page].reason, actions[page].moderator_id, actions[page].uuid.timestamp().timestamp_millis() / 1000);
                if let Some(duration) = actions[page].expiry {
                    field_description.push_str(&format!("*Expires:* <t:{}:F>\n", duration));
                }
                field_description.push_str(&format!("*UUID:* `{}`", actions[page].uuid.to_string()));
                if let Err(err) = cmd.edit_original_interaction_response(&ctx.http, |message| {
                    message
                        .embed(|embed| {
                            embed
                                .title(format!("{}#{:0>4}'s history", user.name, user.discriminator))
                                .description(format!("<@{}> - {}/{} actions", user.id, page + 1, actions.len()))
                                .field(field_title, field_description, false)
                        })
                        .components(|components| {
                            components
                                .create_action_row(|action_row| {
                                    action_row
                                        .create_button(|button| {
                                            button
                                                .custom_id("previous")
                                                .style(ButtonStyle::Primary)
                                                .label("Previous")
                                                .disabled(page == 0)
                                        })
                                        .create_button(|button| {
                                            button
                                                .custom_id("next")
                                                .style(ButtonStyle::Primary)
                                                .label("Next")
                                                .disabled(page + 1 == actions.len())
                                        })
                                })
                                .create_action_row(|row| {
                                    row
                                    .create_select_menu(|menu| {
                                        menu
                                            .custom_id("action")
                                            .placeholder("Action")
                                            .options(|options| {
                                                let mut options = options;
                                                for i in 1..actions.len() + 1 {
                                                    options = options.create_option(|option| {
                                                        let mut label = format!("Action {} - {} ({}", i, actions[i - 1].reason, match actions[i - 1].action_type {
                                                            ActionType::Strike => "Strike",
                                                            ActionType::Mute => "Mute",
                                                            ActionType::Kick => "Kick",
                                                            ActionType::Ban => "Ban",
                                                            ActionType::Unknown => "Unknown"
                                                        });
                                                        match actions[i - 1].active {
                                                            true => {},
                                                            false => label.push_str(" - Expired")
                                                        }
                                                        label.push(')');
                                                        option
                                                            .label(label)
                                                            .value(format!("{}", i))
                                                    });
                                                }
                                                options
                                            })
                                    })
                                })
                        })
                }).await {
                    error!("Failed to edit original interaction response. Failed with error: {}", err);
                    return Err(CommandError {
                        message: "Failed to edit original interaction response".to_string(),
                        command_error: None
                    });
                }
            }
            match cmd.delete_original_interaction_response(&ctx.http).await {
                Ok(_) => return Ok(()),
                Err(err) => {
                    error!("Failed to delete original interaction response. Failed with error: {}", err);
                    return Err(CommandError {
                        message: "Failed to delete original interaction response".to_string(),
                        command_error: None
                    });
                }
            }
        },
        "action" => {
            if let Err(err) = defer(&ctx, &cmd, true).await {
                return Err(err)
            }
            match handler.has_permission(ctx, cmd.member.as_ref().unwrap(), Permissions::ModerationSearchUuid).await {
                Ok(has_permission) => {
                    if !has_permission {
                        return handler.missing_permissions(ctx, cmd, Permissions::ModerationSearchUuid).await
                    }
                },
                Err(err) => {
                    error!("Failed to check if user has permission to use moderation search command. Failed with error: {}", err);
                    return Err(CommandError {
                        message: "Failed to check if user has permission to use moderation search command".to_string(),
                        command_error: None
                    });
                }
            }

            let uuid = cmd.data.options[0].options[0].value.as_ref().unwrap().as_str().unwrap().to_string();

            match handler.mongo.get_action(
                uuid.clone()
            ).await {
                Ok(action) => {
                    match action {
                        Some(action) => {
                            let field_title;
                            match action.active {
                                true => field_title = match action.action_type {
                                    ActionType::Strike => "Strike",
                                    ActionType::Mute => "Mute",
                                    ActionType::Kick => "Kick",
                                    ActionType::Ban => "Ban",
                                    ActionType::Unknown => "Unknown"
                                }.to_string(),
                                false => field_title = format!("{} (Expired)", match action.action_type {
                                    ActionType::Strike => "Strike",
                                    ActionType::Mute => "Mute",
                                    ActionType::Kick => "Kick",
                                    ActionType::Ban => "Ban",
                                    ActionType::Unknown => "Unknown"
                                })
                            };
                            let mut field_description = format!("{}\n\n*Issued to:* <@{}>\n*Issued by:* <@{}>\n*Issued at:* <t:{}:F>\n", action.reason, action.user_id, action.moderator_id, action.uuid.timestamp().timestamp_millis() / 1000);
                            if let Some(duration) = action.expiry {
                                field_description.push_str(&format!("*Expires:* <t:{}:F>\n", duration));
                            }
                            match cmd.edit_original_interaction_response(&ctx.http, |message| {
                                message
                                    .embed(|embed| {
                                        embed
                                            .title(format!("UUID `{}`", uuid))
                                            .field(field_title, field_description, false)
                                    })
                            }).await {
                                Ok(_) => return Ok(()),
                                Err(err) => {
                                    error!("Failed to edit original interaction response. Failed with error: {}", err);
                                    return Err(CommandError {
                                        message: "Failed to edit original interaction response".to_string(),
                                        command_error: None
                                    });
                                }
                            };
                        },
                        None => {
                            match cmd.edit_original_interaction_response(&ctx.http, |message| {
                                message
                                    .content(format!("Action with UUID `{}` not found", uuid))
                            }).await {
                                Ok(_) => return Ok(()),
                                Err(err) => {
                                    error!("Failed to edit original interaction response. Failed with error: {}", err);
                                    return Err(CommandError {
                                        message: "Failed to edit original interaction response".to_string(),
                                        command_error: None
                                    });
                                }
                            };
                        }
                    }
                },
                Err(err) => {
                    error!("Failed to get action from database. Failed with error: {}", err);
                    return Err(CommandError {
                        message: "Failed to get action from database".to_string(),
                        command_error: None
                    });
                }
            }
        },
        _ => {
            return Err(CommandError {
                message: "Command not found".to_string(),
                command_error: None
            });
        }
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("search")
        .dm_permission(false)
        .description("Searches for moderation history")
        .create_option(|option| {
            option
                .name("user")
                .description("Search for a user's history")
                .kind(CommandOptionType::SubCommand)
                .create_sub_option(|option| {
                    option
                        .name("user")
                        .description("The user to search for")
                        .kind(CommandOptionType::User)
                        .required(false)
                })
                .create_sub_option(|option| {
                    option
                        .name("expired")
                        .description("Whether to include expired actions")
                        .kind(CommandOptionType::Boolean)
                        .required(false)
                })
        })
        .create_option(|option| {
            option
                .name("action")
                .description("Search for an action")
                .kind(CommandOptionType::SubCommand)
                .create_sub_option(|option| {
                    option
                        .name("uuid")
                        .description("The UUID of the action")
                        .kind(CommandOptionType::String)
                        .required(true)
                })
        })
}