use serenity::{prelude::Context, model::{prelude::{interaction::{Interaction, InteractionType}, Member, ChannelId}, permissions}};
use tracing::error;
use crate::{Handler, commands, commands::{structs::CommandError, utils::messages::send_message}, mongo::structs::{Permissions, Action, ActionType}};

use super::{utils::{guild::guild_id_to_guild}};

impl Handler {
    pub async fn on_command(&self, ctx: Context, interaction: Interaction) {
        match interaction.kind() {
            InteractionType::ApplicationCommand => {
                let command = interaction.application_command().unwrap();
                let command_result: Result<(), CommandError> = match command.data.name.as_str() {
                    "permissions" => commands::permissions::router::run(&self, &ctx, &command).await,
                    "strike" => commands::moderation::strike::run(&self, &ctx, &command).await,
                    "search" => commands::moderation::search::run(&self, &ctx, &command).await,
                    "mute" => commands::moderation::mute::run(&self, &ctx, &command).await,
                    "unmute" => commands::moderation::unmute::run(&self, &ctx, &command).await,
                    "kick" => commands::moderation::kick::run(&self, &ctx, &command).await,
                    "ban" => commands::moderation::ban::run(&self, &ctx, &command).await,
                    "unban" => commands::moderation::unban::run(&self, &ctx, &command).await,
                    "remove" => commands::moderation::remove::run(&self, &ctx, &command).await,
                    "expire" => commands::moderation::expire::run(&self, &ctx, &command).await,
                    "duration" => commands::moderation::duration::run(&self, &ctx, &command).await,
                    "reason" => commands::moderation::reason::run(&self, &ctx, &command).await,
                    _ => Err(CommandError {
                        message: "Command not found".to_string(),
                        command_error: None
                    })
                };
                match command_result {
                    Ok(_) => (),
                    Err(err) => {
                        error!("Command failed with message: {}", err.message);
                        let mut message_content = format!("Failed to run /{} command with message: {}", command.data.name, err.message);
                        if let Some(command_error) = err.command_error {
                            error!("An error was provided: {}", command_error);
                            message_content.push_str(&format!("\nError: {}", command_error));
                        }
                        if let Err(err) = send_message(&ctx, &command, message_content).await {
                            error!("Failed to send message to user notifying of an error. Failed with error: {}", err);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub async fn has_permission(&self, ctx: &Context, member: &Member, permission: Permissions) -> Result<bool, CommandError> {
        let guild = match guild_id_to_guild(ctx, member.guild_id.0 as i64).await {
            Ok(guild) => guild,
            Err(_) => return Err(CommandError {
                message: format!("Failed to get guild with id {}", member.guild_id.0),
                command_error: None
            })
        };

        if member.user.id == guild.owner_id {
            return Ok(true);
        }

        if let Some(permission) = member.permissions {
            if permission.contains(permissions::Permissions::ADMINISTRATOR) {
                return Ok(true);
            }
        }

        let user = match self.mongo.get_user(member.user.id.0 as i64, member.guild_id.0 as i64).await {
            Ok(user) => user,
            Err(_) => return Err(CommandError {
                message: format!("Failed to get user with id {}", member.user.id.0),
                command_error: None
            })
        };
        if user.permissions.contains(&permission) {
            return Ok(true);
        }

        for role in member.roles.clone() {
            match self.mongo.get_role(role.0 as i64, member.guild_id.0 as i64).await {
                Ok(role) => {
                    if role.permissions.contains(&permission) {
                        return Ok(true);
                    }
                },
                Err(_) => return Err(CommandError {
                    message: format!("Failed to get role with id {}", role.0),
                    command_error: None
                })
            }
        }

        match self.mongo.get_role(member.guild_id.0 as i64, member.guild_id.0 as i64).await {
            Ok(role) => {
                if role.permissions.contains(&permission) {
                    return Ok(true);
                }
            },
            Err(_) => return Err(CommandError {
                message: format!("Failed to get everyone role for guild {}", member.guild_id.0),
                command_error: None
            })
        }

        Ok(false)
    }

    pub async fn log_action(&self, ctx: &Context, guild_id: i64, action: &Action) {
        let guild = match self.mongo.get_guild(guild_id).await {
            Ok(guild) => guild,
            Err(err) => {
                error!("Failed to get guild with id {}. Failed with error: {}", guild_id, err);
                return;
            }
        };

        let mut message_content: String = String::new();
        match action.action_type {
            ActionType::Strike => {
                message_content.push_str(&format!("<@{}> has been issued a strike by <@{}>", action.user_id, action.moderator_id));
                if let Some(expiry) = action.expiry {
                    message_content.push_str(&format!(" until <t:{}:F>", expiry));
                }
                message_content.push_str(&format!(" for `{}`", action.reason));
            },
            ActionType::Mute => {
                message_content.push_str(&format!("<@{}> has been muted by <@{}>", action.user_id, action.moderator_id));
                if let Some(duration) = &action.expiry {
                    message_content.push_str(&format!(" until <t:{}:F>", duration));
                }
                message_content.push_str(&format!(" for `{}`", action.reason));
                if action.expiry.is_none() {
                    message_content.push_str("\n*This mute will not expiry by default, please notify <@179292162037514241>*")
                }
            },
            ActionType::Kick => {
                message_content.push_str(&format!("<@{}> has been kicked by <@{}>", action.user_id, action.moderator_id));
                message_content.push_str(&format!(" for `{}`", action.reason));
            },
            ActionType::Ban => {
                message_content.push_str(&format!("<@{}> has been banned by <@{}>", action.user_id, action.moderator_id));
                if let Some(expiry) = action.expiry {
                    message_content.push_str(&format!(" until <t:{}:F>", expiry));
                }
                message_content.push_str(&format!(" for `{}`", action.reason));
            },
            _ => {}
        }
        message_content.push_str(&format!("\nUUID: `{}`", action.uuid));

        if let Some(logging_config) = guild.config.logging {
            match ChannelId(logging_config.logging_channel as u64).send_message(&ctx.http, |message| {
                message
                    .content(message_content)
                    .allowed_mentions(|allowed_mentions| {
                        allowed_mentions.empty_parse()
                    })
            }).await {
                Ok(_) => (),
                Err(err) => error!("Failed to send message to logging channel. Failed with error: {}", err)
            };
        }
    }
}