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
                    if let Some(ignored_channels) = &config.ignore_channels {
                        let channel_i64 = channel as i64;
                        if ignored_channels.contains(&channel_i64) {
                            return
                        }
                    }
                    for emote in config.emotes.iter() {
                        if emote == &reaction.emoji.to_string() {
                            if let Ok(message) = reaction.message(&ctx.http).await {
                                if message.author.bot {
                                    return
                                }
                                for rec in message.reactions {
                                    if &rec.reaction_type.to_string() != emote {
                                        return
                                    }
                                    if rec.count < config.quota {
                                        return
                                    }
                                    if let Ok(found) = self.mongo.check_message_on_board(message.id.0 as i64, channel as i64).await {
                                        if found {
                                            return
                                        }
                                    }
                                    let mut content: String = message.content.clone();
                                    for attachment in message.attachments.iter() {
                                        content.push_str(&format!("\n{}", &attachment.url));
                                    }
                                    if let Err(_) = ChannelId(channel.clone()).send_message(&ctx.http, |msg| {
                                        msg
                                            .content(format!("{}\nby <@{}>", content, message.author.id));
                                        msg
                                    }).await {
                                        return
                                    }
                                    if let Err(_) = self.mongo.add_message_to_board(message.id.0 as i64, channel as i64).await {
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