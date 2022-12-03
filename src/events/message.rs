use regex::Regex;
use serenity::{prelude::Context, model::prelude::Message};
use tracing::{error, warn};

use crate::{Handler, commands::utils::duration::Duration};

impl Handler {
    pub async fn on_message(&self, ctx: &Context, new_message: &Message) {
        if let None = new_message.guild_id {
            return;
        }
        if new_message.author.bot {
            return;
        }

        let mut content: String = new_message.content.clone();
        for attachment in new_message.attachments.iter() {
            content.push_str(&format!("\n{}", &attachment.url));
        }

        match self.redis.set_message(
            new_message.guild_id.unwrap().0 as i64,
            new_message.channel_id.0 as i64,
            new_message.id.0 as i64,
            new_message.author.id.0 as i64,
            content
        ).await {
            Ok(_) => {},
            Err(err) => {
                error!("Failed to set message in Redis. Failed with error: {}", err);
            }
        }

        let guild_id = new_message.guild_id.unwrap().0 as i64;
        let guild = match self.mongo.get_guild(guild_id).await {
            Ok(guild) => guild,
            Err(err) => {
                error!("Failed to get guild {}. Failed with error: {}", guild_id, err);
                return;
            }
        };

        if let Some(moderation_config) = guild.config.moderation {
            let mut strike_reason: Option<String> = None;

            for word in moderation_config.blacklisted_words {
                if new_message.content.to_lowercase().contains(&word) {
                    strike_reason = Some(format!("Blacklisted word: \"{}\"", word));
                    break;
                }
                
            }

            for regex in moderation_config.blacklisted_regex {
                let regex = match Regex::new(&regex) {
                    Ok(regex) => regex,
                    Err(err) => {
                        error!("Failed to compile regex `{}`. Failed with error: {}", regex, err);
                        continue;
                    }
                };
                if regex.is_match(&new_message.content) {
                    strike_reason = Some(format!("Blacklisted regex: \"{}\"", regex));
                    break;
                }
            }

            if let Some(reason) = strike_reason {
                let mut user = ctx.cache.user(new_message.author.id.0);
                if let None = user {
                    user = match ctx.http.get_user(new_message.author.id.0).await {
                        Ok(user) => Some(user),
                        Err(err) => {
                            error!("Failed to get user {}. Failed with error: {}", new_message.author.id.0, err);
                            return;
                        }
                    } 
                }
                if let Some(user) = user {
                    let mut dm_content = format!("You have been given a strike in {} by <@{}>", new_message.guild_id.unwrap().to_partial_guild(&ctx).await.unwrap().name, ctx.cache.current_user_id().0);
                    dm_content.push_str(&format!(" until <t:{}:F>", Duration::new(moderation_config.default_strike_duration.clone()).to_unix_timestamp()));
                    dm_content.push_str(&format!(" for:\n{}", reason));
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
                
                match self.strike(
                    &ctx,
                    new_message.guild_id.unwrap().0 as i64,
                    new_message.author.id.0 as i64,
                    reason,
                    None,
                    Some(Duration::new(moderation_config.default_strike_duration))
                ).await {
                    Ok(_) => {
                        match ctx.http.delete_message(new_message.channel_id.0, new_message.id.0).await {
                            Ok(_) => {},
                            Err(err) => error!("Failed to delete message. Failed with error: {}", err)
                        }
                    },
                    Err(err) => {
                        error!("Failed to strike user {} in guild {}. Failed with error: {}", new_message.author.id.0, guild_id, err);
                        return;
                    }
                }
            }
        }
    }
}