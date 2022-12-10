use std::collections::HashMap;

use serde_json::Value;
use serenity::{prelude::Context, model::{prelude::{interaction::application_command::ApplicationCommandInteraction, command::CommandOptionType, RoleId, UserId, Member}, permissions}};
use tracing::{warn, error, info};

use crate::{Handler, commands::{structs::CommandError, utils::{messages::{send_message, defer}, guild::guild_id_to_guild}}, mongo::structs::Permissions};

pub async fn user_run(handler: &Handler, ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<(), CommandError> {
    defer(ctx, cmd, false).await?;
    let mut user_id: Option<i64> = None;

    match cmd.data.options[0].options[0].kind {
        CommandOptionType::User => {
            match Value::to_string(&cmd.data.options[0].options[0].value.clone().unwrap()).replace('\"', "").parse::<i64>() {
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
        _ => warn!("Option type {:?} not handled", cmd.data.options[0].options[0].kind)
    }

    let guild = match guild_id_to_guild(ctx, cmd.guild_id.unwrap().0 as i64).await {
        Ok(guild) => guild,
        Err(_) => return Err(CommandError {
            message: format!("Failed to get guild with id {}", cmd.guild_id.unwrap().0),
            command_error: None
        })
    };

    if user_id.unwrap() == guild.owner_id.0 as i64 {
        return send_message(ctx, cmd, format!("<@{}> is the server owner, so has all permissions", cmd.user.id.0)).await;
    }

    let mut member: Option<Member> = ctx.cache.member(cmd.guild_id.unwrap(), UserId(user_id.unwrap() as u64));
    if member.is_none() {
        match ctx.http.get_member(cmd.guild_id.unwrap().0, user_id.unwrap() as u64).await {
            Ok(mbr) => {
                if let Some(permission) = mbr.permissions {
                    if permission.contains(permissions::Permissions::ADMINISTRATOR) {
                        return send_message(ctx, cmd, format!("<@{}> is a server administrator, so has all permissions", cmd.user.id.0)).await;
                    }
                }
                member = Some(mbr);
            },
            Err(err) => {
                error!("Failed to get member with id {}. Failed with error: {}", user_id.unwrap(), err);
                return Err(CommandError {
                    message: format!("Failed to get member with id {}", user_id.unwrap()),
                    command_error: None
                });
            }
        }
    }

    let mut user_permissions: Vec<Permissions> = Vec::new();
    match handler.mongo.get_user(
        user_id.unwrap(),
        cmd.guild_id.unwrap().0 as i64
    ).await {
        Ok(user) => {
            for permission in user.permissions {
                user_permissions.push(permission);
            }
        },
        Err(err) => {
            error!("Failed to get user from database. Failed with error: {}", err);
            return Err(CommandError {
                message: "Failed to get user from database".to_string(),
                command_error: None
            });
        }
    }
    
    let mut user_roles = member.unwrap().roles.clone();
    user_roles.push(RoleId(cmd.guild_id.unwrap().0));
    let mut role_permissions: HashMap<Permissions, i64> = HashMap::new();
    for role in user_roles {
        match handler.mongo.get_role(
            role.0 as i64,
            cmd.guild_id.unwrap().0 as i64
        ).await {
            Ok(role) => {
                for permission in role.permissions {
                    if !user_permissions.contains(&permission) && !role_permissions.contains_key(&permission) {
                        role_permissions.insert(permission, role.id);
                    }
                }
            },
            Err(err) => {
                error!("Failed to get role from database. Failed with error: {}", err);
                return Err(CommandError {
                    message: "Failed to get role from database".to_string(),
                    command_error: None
                });
            }
        }
    }
    // Sort role permissions by value
    role_permissions = role_permissions.into_iter().collect();
    info!("Role permissions: {:?}", role_permissions);

    let mut message_content = format!("<@{}>", user_id.unwrap());
    if user_permissions.is_empty() && role_permissions.is_empty() {
        message_content.push_str(" has no permissions");
    }

    if user_permissions.is_empty() && !role_permissions.is_empty() {
        message_content.push_str(" has no permissions, but inherits these permissions:");
        let mut last_role: &i64 = &0;
        for (permission, role) in role_permissions.iter() {
            if role != last_role {
                last_role = role;
                message_content.push_str(&format!("\n*<@&{}>*:\n", role));
            }
            message_content.push_str(&format!("`{}`\n", permission.to_string()));
        }
    }

    if !user_permissions.is_empty() && role_permissions.is_empty() {
        message_content.push_str(" has the following permissions:\n");
        for permission in user_permissions.iter() {
            message_content.push_str(&format!("`{}`\n", permission.to_string()));
        }
    }

    if !user_permissions.is_empty() && !role_permissions.is_empty() {
        message_content.push_str(" has the following permissions:\n");
        for permission in user_permissions.iter() {
            message_content.push_str(&format!("`{}`\n", permission.to_string()));
        }
        message_content.push_str("\nThese permissions are inherited from their roles:");
        let mut last_role: &i64 = &0;
        for (permission, role) in role_permissions.iter() {
            if role != last_role {
                last_role = role;
                message_content.push_str(&format!("\n\t*<@&{}>*:\n", role));
            }
            message_content.push_str(&format!("\t - `{}`\n", permission.to_string()));
        }
    }

    send_message(ctx, cmd, message_content).await
}

pub async fn role_run(handler: &Handler, ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<(), CommandError> {
    defer(ctx, cmd, false).await?;
    let mut role_id: Option<i64> = None;

    match cmd.data.options[0].options[0].options[0].kind {
        CommandOptionType::Role => {
            match Value::to_string(&cmd.data.options[0].options[0].options[0].value.clone().unwrap()).replace('\"', "").parse::<i64>() {
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
        _ => warn!("Option type {:?} not handled", cmd.data.options[0].options[0].options[0].kind)
    }

    match handler.mongo.get_role(
        role_id.unwrap(),
        cmd.guild_id.unwrap().0 as i64
    ).await {
        Ok(role) => {
            let mut message_content = format!("<@&{}>", role_id.unwrap());
            if role.permissions.is_empty() {
                message_content.push_str(" has no permissions");
            } else {
                message_content.push_str(" has the following permissions:\n");
                for permission in role.permissions.iter() {
                    message_content.push_str(&format!("`{}`\n", permission.to_string()));
                }
            }
            send_message(ctx, cmd, message_content).await
        },
        Err(err) => {
            error!("Failed to get role from database. Failed with error: {}", err);
            Err(CommandError {
                message: "Failed to get role from database".to_string(),
                command_error: None
            })
        }
    }
}