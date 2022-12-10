use serenity::{prelude::Context, model::prelude::interaction::{application_command::ApplicationCommandInteraction, InteractionResponseType}};

use crate::commands::structs::CommandError;

pub async fn defer(ctx: &Context, interaction: &ApplicationCommandInteraction, ephemeral: bool) -> Result<(), CommandError> {
    match interaction.create_interaction_response(ctx.http.clone(), |response| {
        response
            .kind(InteractionResponseType::DeferredChannelMessageWithSource)
            .interaction_response_data(|message| {
                message
                    .ephemeral(ephemeral)
                    .allowed_mentions(|allowed_mentions| {
                        allowed_mentions.empty_parse()
                    })
            })
    }).await {
        Ok(_) => Ok(()),
        Err(err) => Err(CommandError {
            message: "Failed to defer command".to_string(),
            command_error: Some(err)
        })
    }
}

pub async fn send_message(ctx: &Context, cmd: &ApplicationCommandInteraction, content: String) -> Result<(), CommandError> {
    match cmd.edit_original_interaction_response(&ctx.http, |response| {
        response
            .content(content)
    }).await {
        Ok(_) => Ok(()),
        Err(err) => {
            Err(CommandError {
                message: "Could not send message".to_string(),
                command_error: Some(err)
            })
        }
    }
}