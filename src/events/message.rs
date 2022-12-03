use serenity::{prelude::Context, model::prelude::Message};
use tracing::{error, warn};

use crate::{Handler, commands::utils::duration::Duration};

use super::utils::filters::filter_message;

impl Handler {
    pub async fn on_message(&self, ctx: &Context, new_message: &Message) {
        if let None = new_message.guild_id {
            return;
        }
        if new_message.author.bot {
            return;
        }

        let guild_id = new_message.guild_id.unwrap().0 as i64;

        let mut content: String = new_message.content.clone();
        for attachment in new_message.attachments.iter() {
            content.push_str(&format!("\n{}", &attachment.url));
        }

        match self.redis.set_message(
            guild_id.clone(),
            new_message.channel_id.0 as i64,
            new_message.id.0 as i64,
            new_message.author.id.0 as i64,
            content.clone()
        ).await {
            Ok(_) => {},
            Err(err) => {
                error!("Failed to set message in Redis. Failed with error: {}", err);
            }
        }

        let filter_result = filter_message(&self, guild_id, content).await;

        if let Some(filter_result) = filter_result {
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
                dm_content.push_str(&format!(" until <t:{}:F>", Duration::new(filter_result.1.clone()).to_unix_timestamp()));
                dm_content.push_str(&format!(" for:\n{}", filter_result.0));
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
                filter_result.0,
                None,
                Some(Duration::new(filter_result.1))
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