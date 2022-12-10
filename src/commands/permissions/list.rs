use serenity::{prelude::Context, model::prelude::interaction::application_command::ApplicationCommandInteraction};
use strum::IntoEnumIterator;
use crate::{commands::{structs::CommandError, utils::messages::{send_message, defer}}, mongo::structs::Permissions};

pub async fn run(ctx: &Context, cmd: &ApplicationCommandInteraction) -> Result<(), CommandError> {
    defer(ctx, cmd, true).await?;
    let mut message_content = "The following permissions are available:\n".to_string();
    for permission in Permissions::iter() {
        if permission != Permissions::Unknown {
            message_content.push_str(&format!("`{}`\n", permission.to_string()));
        }
    }
    send_message(ctx, cmd, message_content).await
}