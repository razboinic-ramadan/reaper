use serenity::{prelude::Context, model::prelude::{ChannelId, MessageUpdateEvent}};
use tracing::{error, warn};

use crate::{Handler, commands::utils::duration::Duration, mongo::structs::ActionType};

use super::utils::filters::filter_message;

impl Handler {
    pub async fn on_message_edit(&self, ctx: &Context, event: MessageUpdateEvent) {
        if event.guild_id.is_none() {
            return;
        }
        if event.content.is_none() {
            return;
        }
        if event.author.as_ref().is_none() {
            return;
        }
        if event.author.as_ref().unwrap().bot {
            return;
        }

        let guild_id = event.guild_id.unwrap().0 as i64;

        match self.redis.get_message(
            guild_id,
            event.channel_id.0 as i64,
            event.id.0 as i64
        ).await {
            Ok(message) => {
                match message {
                    Some(message) => {
                        let (user_id, message) = message.split_once(':').unwrap();
                        
                        let mut content: String = event.content.clone().unwrap();
                        if let Some(attachments) = event.attachments.as_ref() {
                            for attachment in attachments.iter() {
                                content.push_str(&format!("\n{}", &attachment.url));
                            }
                        }

                        let filter_result = filter_message(self, guild_id, event.content.unwrap().clone()).await;

                        match self.redis.set_message(
                            event.guild_id.unwrap().0 as i64,
                            event.channel_id.0 as i64,
                            event.id.0 as i64,
                            event.author.as_ref().unwrap().id.0 as i64,
                            content.clone()
                        ).await {
                            Ok(_) => {},
                            Err(err) => {
                                error!("Failed to set message in Redis. Failed with error: {}", err);
                            }
                        }

                        if let Some(filter_result) = filter_result.clone() {
                            let mut user = ctx.cache.user(event.author.as_ref().unwrap().id.0);
                            if user.is_none() {
                                user = match ctx.http.get_user(event.author.as_ref().unwrap().id.0).await {
                                    Ok(user) => Some(user),
                                    Err(err) => {
                                        error!("Failed to get user {}. Failed with error: {}", event.author.as_ref().unwrap().id.0, err);
                                        return;
                                    }
                                } 
                            }

                            let mut escalation = None;
                            match self.strike(
                                ctx,
                                event.guild_id.unwrap().0 as i64,
                                event.author.as_ref().unwrap().id.0 as i64,
                                filter_result.0.clone(),
                                None,
                                Some(Duration::new(filter_result.1.clone()))
                            ).await {
                                Ok((_, escalation_action)) => {
                                    escalation = escalation_action;
                                    match ctx.http.delete_message(event.channel_id.0, event.id.0).await {
                                        Ok(_) => {},
                                        Err(err) => error!("Failed to delete message. Failed with error: {}", err)
                                    }
                                },
                                Err(err) => {
                                    error!("Failed to strike user {} in guild {}. Failed with error: {}", event.author.as_ref().unwrap().id.0, guild_id, err);
                                    return;
                                }
                            }

                            if let Some(user) = user {
                                let mut dm_content = format!("You have been given a strike in {} by <@{}>", event.guild_id.unwrap().to_partial_guild(&ctx).await.unwrap().name, ctx.cache.current_user_id().0);
                                dm_content.push_str(&format!(" until <t:{}:F>", Duration::new(filter_result.1).to_unix_timestamp()));
                                dm_content.push_str(&format!(" for:\n{}", filter_result.0));
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
                                match user.direct_message(&ctx.http, |message| {
                                    message
                                        .content(dm_content)
                                }).await {
                                    Ok(_) => {},
                                    Err(err) => {
                                        warn!("{} could not be notified. Failed with error: {}", user.id.0, err);
                                    }
                                }
                            }
                        }

                        let guild = match self.mongo.get_guild(event.guild_id.unwrap().0 as i64).await {
                            Ok(guild) => guild,
                            Err(err) => {
                                error!("Failed to get guild {}. Failed with error: {}", event.guild_id.unwrap().0 as i64, err);
                                return;
                            }
                        };

                        if let Some(logging_config) = guild.config.logging {
                            let mut content = format!("Message edited in <#{}> by <@{}>:\n**Old:**\n`{}`\n**New:**\n`{}`", event.channel_id.0 as i64, user_id, message.replace('`', r"\`"), content.replace('`', r"\`"));
                            if filter_result.is_some() {
                                content.push_str("\nThis message violated the guild filter and was deleted.");
                            }
                            match ChannelId(logging_config.logging_channel as u64)
                                .send_message(ctx.http.as_ref(), |msg| {
                                    msg
                                        .content(content)
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
                    },
                    None => {
                        warn!("Message not found in Redis. This should not happen.");
                    }
                }
            },
            Err(err) => {
                error!("Failed to get message. Failed with error: {}", err);
                
            }
        }
    }
}