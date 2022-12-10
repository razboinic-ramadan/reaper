use serenity::{prelude::Context, model::prelude::{ChannelId}};
use tracing::error;

use crate::Handler;

impl Handler {
    pub async fn on_message_delete(&self, ctx: &Context, guild_id: i64, channel_id: i64, message_id: i64) {
        match self.redis.get_message(guild_id, channel_id, message_id).await {
            Ok(message) => {
                if let Some(message) = message {
                    let (user_id, message) = message.split_once(':').unwrap();
                    
                    let guild = match self.mongo.get_guild(guild_id).await {
                        Ok(guild) => guild,
                        Err(err) => {
                            error!("Failed to get guild {}. Failed with error: {}", guild_id, err);
                            return;
                        }
                    };

                    match self.redis.delete_message(guild_id, channel_id, message_id).await {
                        Ok(_) => {},
                        Err(err) => {
                            error!("Failed to delete message from Redis. Failed with error: {}", err);
                            return;
                        }
                    }

                    if let Some(logging_config) = guild.config.logging {
                        match ChannelId(logging_config.logging_channel as u64)
                        .send_message(ctx.http.as_ref(), |msg| {
                            msg
                                .content(format!("Message deleted in <#{}> by <@{}>:\n`{}`", channel_id, user_id, message.replace('`', r"\`")))
                                .allowed_mentions(|allowed_mentions| {
                                    allowed_mentions.empty_parse()
                                })
                        }).await {
                            Ok(_) => {},
                            Err(err) => {
                                error!("Failed to send message to logging channel. Failed with error: {}", err);
                                
                            }
                        };
                    }
                }
            }
            Err(err) => {
                error!("Failed to get message. Failed with error: {}", err);
                
            }
        }
    }
}