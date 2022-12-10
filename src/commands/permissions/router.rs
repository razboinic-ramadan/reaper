use serenity::{builder::CreateApplicationCommand, model::prelude::{command::CommandOptionType, interaction::application_command::ApplicationCommandInteraction}, prelude::Context};

use crate::{Handler, commands::structs::CommandError, commands::permissions, mongo::structs::Permissions};

pub async fn run(handler: &Handler, ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<(), CommandError> {
    match cmd.data.options[0].name.as_str() {
        "add" => {
            match handler.has_permission(ctx, cmd.member.as_ref().unwrap(), Permissions::PermissionsAdd).await {
                Ok(has_permission) => {
                    if has_permission {
                        permissions::add::user_run(handler, ctx, cmd).await
                    }
                    else {
                        handler.missing_permissions(ctx, cmd, Permissions::PermissionsAdd).await
                    }
                },
                Err(err) => {
                    Err(CommandError{
                        message: format!("Permissions could not be successfully checked. Failed with error: {}", err),
                        command_error: None
                    })
                }
            }
            
        },
        "list" => {
            match handler.has_permission(ctx, cmd.member.as_ref().unwrap(), Permissions::PermissionsList).await {
                Ok(has_permission) => {
                    if has_permission {
                        permissions::list::run(ctx, cmd).await
                    }
                    else {
                        handler.missing_permissions(ctx, cmd, Permissions::PermissionsList).await
                    }
                },
                Err(err) => {
                    Err(CommandError{
                        message: format!("Permissions could not be successfully checked. Failed with error: {}", err),
                        command_error: None
                    })
                }
            }
        },
        "remove" => {
            match handler.has_permission(ctx, cmd.member.as_ref().unwrap(), Permissions::PermissionsRemove).await {
                Ok(has_permission) => {
                    if has_permission {
                        permissions::remove::user_run(handler, ctx, cmd).await
                    }
                    else {
                        handler.missing_permissions(ctx, cmd, Permissions::PermissionsRemove).await
                    }
                },
                Err(err) => {
                    Err(CommandError{
                        message: format!("Permissions could not be successfully checked. Failed with error: {}", err),
                        command_error: None
                    })
                }
            }
        },
        "view" => {
            match handler.has_permission(ctx, cmd.member.as_ref().unwrap(), Permissions::PermissionsView).await {
                Ok(has_permission) => {
                    if has_permission {
                        permissions::view::user_run(handler, ctx, cmd).await
                    }
                    else {
                        handler.missing_permissions(ctx, cmd, Permissions::PermissionsView).await
                    }
                },
                Err(err) => {
                    Err(CommandError{
                        message: format!("Permissions could not be successfully checked. Failed with error: {}", err),
                        command_error: None
                    })
                }
            }
        },
        "role" => {
            match cmd.data.options[0].options[0].name.as_str() {
                "add" => {
                    match handler.has_permission(ctx, cmd.member.as_ref().unwrap(), Permissions::PermissionsAdd).await {
                        Ok(has_permission) => {
                            if has_permission {
                                permissions::add::role_run(handler, ctx, cmd).await
                            }
                            else {
                                handler.missing_permissions(ctx, cmd, Permissions::PermissionsAdd).await
                            }
                        },
                        Err(err) => {
                            Err(CommandError{
                                message: format!("Permissions could not be successfully checked. Failed with error: {}", err),
                                command_error: None
                            })
                        }
                    }
                },
                "remove" => {
                    match handler.has_permission(ctx, cmd.member.as_ref().unwrap(), Permissions::PermissionsRemove).await {
                        Ok(has_permission) => {
                            if has_permission {
                                permissions::remove::role_run(handler, ctx, cmd).await
                            }
                            else {
                                handler.missing_permissions(ctx, cmd, Permissions::PermissionsRemove).await
                            }
                        },
                        Err(err) => {
                            Err(CommandError{
                                message: format!("Permissions could not be successfully checked. Failed with error: {}", err),
                                command_error: None
                            })
                        }
                    }
                },
                "view" => {
                    match handler.has_permission(ctx, cmd.member.as_ref().unwrap(), Permissions::PermissionsView).await {
                        Ok(has_permission) => {
                            if has_permission {
                                permissions::view::role_run(handler, ctx, cmd).await
                            }
                            else {
                                handler.missing_permissions(ctx, cmd, Permissions::PermissionsView).await
                            }
                        },
                        Err(err) => {
                            Err(CommandError{
                                message: format!("Permissions could not be successfully checked. Failed with error: {}", err),
                                command_error: None
                            })
                        }
                    }
                },
                _ => Err(CommandError {
                    message: "Command not found".to_string(),
                    command_error: None
                })
            }
        },
        _ => Err(CommandError {
            message: "Command not found".to_string(),
            command_error: None
        })
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("permissions")
        .dm_permission(false)
        .description("View and modify permissions for users and roles")
        .create_option(|option| {
            option
                .name("add")
                .description("Add a Reaper permission to a user")
                .kind(CommandOptionType::SubCommand)
                .create_sub_option(|option| {
                    option
                        .name("user")
                        .description("The user to add the Reaper permission to")
                        .kind(CommandOptionType::User)
                        .required(true)
                })
                .create_sub_option(|option| {
                    option
                        .name("permission")
                        .description("The Reaper permission to add to the user")
                        .kind(CommandOptionType::String)
                        .required(true)
                }) 
        })
        .create_option(|option| {
            option
                .name("list")
                .description("List all available Reaper permissions")
                .kind(CommandOptionType::SubCommand)
        })
        .create_option(|option| {
            option
                .name("remove")
                .description("Remove a Reaper permission from a user")
                .kind(CommandOptionType::SubCommand)
                .create_sub_option(|option| {
                    option
                        .name("user")
                        .description("The user to remove the Reaper permission from")
                        .kind(CommandOptionType::User)
                        .required(true)
                })
                .create_sub_option(|option| {
                    option
                        .name("permission")
                        .description("The Reaper permission to remove from the user")
                        .kind(CommandOptionType::String)
                        .required(true)
                })
        })
        .create_option(|option| {
            option
                .name("view")
                .description("View the Reaper permissions for a user")
                .kind(CommandOptionType::SubCommand)
                .create_sub_option(|option| {
                    option
                        .name("user")
                        .description("The user to view the Reaper permissions for")
                        .kind(CommandOptionType::User)
                        .required(true)
                })
        })
        .create_option(|option| {
            option
                .name("role")
                .description("View and modify permissions for roles")
                .kind(CommandOptionType::SubCommandGroup)
                .create_sub_option(|option| {
                    option
                        .name("add")
                        .description("Add a Reaper permission to a role")
                        .kind(CommandOptionType::SubCommand)
                        .create_sub_option(|option| {
                            option
                                .name("role")
                                .description("The role to add the Reaper permission to")
                                .kind(CommandOptionType::Role)
                                .required(true)
                        })
                        .create_sub_option(|option| {
                            option
                                .name("permission")
                                .description("The Reaper permission to add to the role")
                                .kind(CommandOptionType::String)
                                .required(true)
                        })
                })
                .create_sub_option(|option| {
                    option
                        .name("remove")
                        .description("Remove a Reaper permission from a role")
                        .kind(CommandOptionType::SubCommand)
                        .create_sub_option(|option| {
                            option
                                .name("role")
                                .description("The role to remove the Reaper permission from")
                                .kind(CommandOptionType::Role)
                                .required(true)
                        })
                        .create_sub_option(|option| {
                            option
                                .name("permission")
                                .description("The Reaper permission to remove from the role")
                                .kind(CommandOptionType::String)
                                .required(true)
                        })
                })
                .create_sub_option(|option| {
                    option
                        .name("view")
                        .description("View the Reaper permissions for a role")
                        .kind(CommandOptionType::SubCommand)
                        .create_sub_option(|option| {
                            option
                                .name("role")
                                .description("The role to view the Reaper permissions for")
                                .kind(CommandOptionType::Role)
                                .required(true)
                        })
                })
        })
}