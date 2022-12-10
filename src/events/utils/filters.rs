use regex::Regex;
use tracing::error;

use crate::Handler;

pub async fn filter_message(handler: &Handler, guild_id: i64, message_content: String) -> Option<(String, String)> {
    let guild = match handler.mongo.get_guild(guild_id).await {
        Ok(guild) => guild,
        Err(err) => {
            error!("Failed to get guild {}. Failed with error: {}", guild_id, err);
            return None;
        }
    };

    if let Some(moderation_config) = guild.config.moderation {
        let mut strike_reason: Option<String> = None;

        for word in moderation_config.blacklisted_words {
            if message_content.to_lowercase().contains(&word) {
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
            if regex.is_match(&message_content) {
                strike_reason = Some(format!("Blacklisted regex: \"{}\"", regex));
                break;
            }
        }

        if let Some(strike_reason) = strike_reason {
            Some((strike_reason, moderation_config.default_strike_duration))
        }
        else {
            None
        }
    }
    else {
        None
    }
}