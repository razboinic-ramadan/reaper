use serenity::{model::prelude::{GuildId, PartialGuild, interaction::application_command::ApplicationCommandInteraction}, prelude::Context};

use crate::{commands::structs::CommandError, mongo::structs::{Permissions}, Handler};

use super::messages::send_message;

pub async fn guild_id_to_guild(ctx: &Context, guild_id: i64) -> Result<PartialGuild, CommandError> {
    let guild_id = GuildId(guild_id as u64);
    match guild_id.to_partial_guild(&ctx.http).await {
        Ok(guild) => Ok(guild),
        Err(_) => {
            Err(CommandError {
                message: "Could not get guild".to_string(),
                command_error: None
            })
        }
    }
}

impl Handler {
    pub async fn missing_permissions(&self, ctx: &Context, cmd: &ApplicationCommandInteraction, permission: Permissions) -> Result<(), CommandError> {
        send_message(ctx, cmd, format!("You are missing the `{}` permission to run this!", permission.to_string())).await
    }
}