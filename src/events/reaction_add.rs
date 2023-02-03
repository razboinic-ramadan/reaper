use serenity::{prelude::Context, model::prelude::{Reaction, ChannelId}};

use crate::Handler;

impl Handler {
    pub async fn reaction_add(&self, ctx: &Context, reaction: &Reaction) {
        if let None = reaction.guild_id {
            return;
        }
        let guild = match self.mongo.get_guild(reaction.guild_id.expect("This has previously been validated to Some").0 as i64).await {
            Ok(guild) => guild,
            Err(_) => return
        };
        match &guild.config.boards {
            Some(boards) => {
                for (channel, config) in boards.iter() {
                    let channel = channel.parse::<u64>().unwrap();
                    for emote in config.emotes.iter() {
                        if emote == &reaction.emoji.to_string() {
                            if let Ok(message) = reaction.message(&ctx.http).await {
                                for rec in message.reactions {
                                    if &rec.reaction_type.to_string() == emote && rec.count < config.quota {
                                        return
                                    }
                                    let mut content: String = message.content.clone();
                                    for attachment in message.attachments.iter() {
                                        content.push_str(&format!("\n{}", &attachment.url));
                                    }
                                    if let Err(_) = ChannelId(channel.clone()).send_message(&ctx.http, |msg| {
                                        msg
                                            .content(format!("`{}` by <@{}>", content, message.author.id));
                                        msg
                                    }).await {
                                        return
                                    }
                                }
                            }
                            else {
                                return
                            }
                            
                        }
                    }
                }
            },
            None => return
        };
        
    }
}