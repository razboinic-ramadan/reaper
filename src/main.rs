use std::env;
use mongo::mongodb::Mongo;
use crate::redis::redisdb::Redis;
use serenity::{prelude::GatewayIntents, Client, framework::StandardFramework};
use tracing::{error, info};

mod commands;
mod events;
mod mongo;
mod redis;

#[derive(Clone)]
pub struct Handler {
    pub mongo: Mongo,
    pub redis: Redis,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    
    let token = match env::var("DISCORD_TOKEN") {
        Ok(token) => token,
        Err(err) => {
            error!("Attempted to obtain DISCORD_TOKEN from environment. Failed with error: {}", err);
            return;
        }
    };
    let intents = GatewayIntents::non_privileged() | GatewayIntents::GUILD_MEMBERS | GatewayIntents::MESSAGE_CONTENT;

    let mongo = match mongo::mongodb::Mongo::create().await {
        Ok(mongo) => mongo,
        Err(_) => {
            return;
        }
    };
    info!("Successfully connected to MongoDB");

    let redis = match redis::redisdb::Redis::create().await {
        Ok(redis) => redis,
        Err(_) => {
            return;
        }
    };
    info!("Successfully connected to Redis");

    let mut client = match Client::builder(&token, intents)
        .event_handler(Handler {mongo, redis})
        .framework(StandardFramework::new())
        .await {
        Ok(client) => client,
        Err(err) => {
            error!("Attempted to create a client. Failed with error: {}", err);
            return;
        }
    };

    if let Err(err) = client.start().await {
        error!("Attempted to start a client. Failed with error: {}", err);
    }
}