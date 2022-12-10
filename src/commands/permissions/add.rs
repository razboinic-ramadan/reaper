use serde_json::Value;
use serenity::{prelude::Context, model::prelude::{interaction::application_command::ApplicationCommandInteraction, command::CommandOptionType}};
use tracing::{error, warn};

use crate::{Handler, commands::{structs::CommandError, utils::messages::{send_message, defer}}, mongo::structs::Permissions};

pub async fn user_run(handler: &Handler, ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<(), CommandError> {
    defer(ctx, cmd, false).await?;
    let mut user_id: Option<i64> = None;
    let mut permission: Option<Permissions> = None;

    for option in cmd.data.options[0].options.iter() {
        match option.kind {
            CommandOptionType::User => {
                match Value::to_string(&option.value.clone().unwrap()).replace('\"', "").parse::<i64>() {
                    Ok(id) => user_id = Some(id),
                    Err(err) => {
                        error!("Failed to get an integer from the User value. Failed with error: {}", err);
                        return Err(CommandError {
                            message: "Failed to get an integer from the User value".to_string(),
                            command_error: None
                        });
                    }
                };
            },
            CommandOptionType::String => {
                match option.value.as_ref().unwrap().as_str() {
                    Some(perm) => {
                        match Permissions::from(perm.to_string()) {
                            Permissions::Unknown => {
                                warn!("Permission {} is not a valid permission and could not be applied", perm);
                                return send_message(ctx, cmd, format!("`{}` is not a valid permission and could not be applied", perm)).await;
                            }
                            _ => permission = Some(Permissions::from(perm.to_string())),
                        }
                    },
                    None => {
                        error!("Failed to get a string from the String value");
                        return Err(CommandError {
                            message: "Failed to get a string from the String value".to_string(),
                            command_error: None
                        });
                    }
                }
            },
            _ => warn!("Option type {:?} not handled", option.kind)
        }
    }

    match handler.mongo.get_user(
        user_id.unwrap(),
        cmd.guild_id.unwrap().0 as i64,
    ).await {
        Ok(user) => {
            if user.permissions.contains(&permission.unwrap()) {
                warn!("User {} already has permission {}", user_id.unwrap(), permission.unwrap().to_string());
                return send_message(ctx, cmd, format!("<@{}> already has `{}`", user_id.unwrap(), permission.unwrap().to_string())).await;
            }
        },
        Err(err) => {
            error!("Failed to get user from database: {}", err);
            return Err(CommandError {
                message: "Failed to get user from database".to_string(),
                command_error: None
            });
        }
    }

    match handler.mongo.add_permission_to_user(
        user_id.unwrap(),
        cmd.guild_id.unwrap().0 as i64,
        permission.unwrap()
    ).await {
        Ok(_) => {
            send_message(ctx, cmd, format!("Successfully added `{}` to <@{}>", permission.unwrap().to_string(), user_id.unwrap())).await
        },
        Err(err) => {
            error!("Failed to add permission to user: {}", err);
            Err(CommandError {
                message: "Failed to add permission to user".to_string(),
                command_error: None
            })
        }
    }
}

pub async fn role_run(handler: &Handler, ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<(), CommandError> {
    defer(ctx, cmd, false).await?;
    let mut role_id: Option<i64> = None;
    let mut permission: Option<Permissions> = None;

    for option in cmd.data.options[0].options[0].options.iter() {
        match option.kind {
            CommandOptionType::Role => {
                match Value::to_string(&option.value.clone().unwrap()).replace('\"', "").parse::<i64>() {
                    Ok(id) => role_id = Some(id),
                    Err(err) => {
                        error!("Failed to get an integer from the Role value. Failed with error: {}", err);
                        return Err(CommandError {
                            message: "Failed to get an integer from the Role value".to_string(),
                            command_error: None
                        });
                    }
                };
            },
            CommandOptionType::String => {
                match option.value.as_ref().unwrap().as_str() {
                    Some(perm) => {
                        match Permissions::from(perm.to_string()) {
                            Permissions::Unknown => {
                                warn!("Permission {} is not a valid permission and could not be applied", perm);
                                return send_message(ctx, cmd, format!("`{}` is not a valid permission and could not be applied", perm)).await;
                            }
                            _ => permission = Some(Permissions::from(perm.to_string())),
                        }
                    },
                    None => {
                        error!("Failed to get a string from the String value");
                        return Err(CommandError {
                            message: "Failed to get a string from the String value".to_string(),
                            command_error: None
                        });
                    }
                }
            },
            _ => warn!("Option type {:?} not handled", option.kind)
        }
    }

    match handler.mongo.get_role(
        role_id.unwrap(),
        cmd.guild_id.unwrap().0 as i64,
    ).await {
        Ok(role) => {
            if role.permissions.contains(&permission.unwrap()) {
                warn!("Role {} already has permission {}", role_id.unwrap(), permission.unwrap().to_string());
                return send_message(ctx, cmd, format!("<@&{}> already has `{}`", role_id.unwrap(), permission.unwrap().to_string())).await;
            }
        },
        Err(err) => {
            error!("Failed to get role from database: {}", err);
            return Err(CommandError {
                message: "Failed to get role from database".to_string(),
                command_error: None
            });
        }
    }

    match handler.mongo.add_permission_to_role(
        role_id.unwrap(),
        cmd.guild_id.unwrap().0 as i64,
        permission.unwrap()
    ).await {
        Ok(_) => {
            send_message(ctx, cmd, format!("Successfully added `{}` to <@&{}>", permission.unwrap().to_string(), role_id.unwrap())).await
        },
        Err(err) => {
            error!("Failed to add permission to role: {}", err);
            Err(CommandError {
                message: "Failed to add permission to role".to_string(),
                command_error: None
            })
        }
    }
}