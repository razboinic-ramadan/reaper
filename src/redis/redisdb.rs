use std::env;
use tracing::error;
use redis::{Client, AsyncCommands};
use crate::redis::structs;

#[derive(Clone)]
pub struct Redis {
    pub client: Client
}

impl Redis {
    pub async fn create() -> Result<Self, structs::RedisError> {
        let uri = match env::var("REDIS_URI") {
            Ok(uri) => uri,
            Err(err) => {
                error!("Attempted to obtain REDIS_URI from environment. Failed with error: {}", err);
                return Err(structs::RedisError {
                    message: "Failed to obtain REDIS_URI from environment".to_string(),
                    redis_error: None
                });
            }
        };

        match Client::open(uri) {
            Ok(client) => Ok( Redis {client} ),
            Err(err) => {
                error!("Attempted to create a client. Failed with error: {}", err);
                Err(structs::RedisError {
                    message: "Failed to create a client".to_string(),
                    redis_error: Some(err)
                })
            }
        }
    }

    pub async fn get_message(&self, guild_id: i64, channel_id: i64, message_id: i64) -> Result<Option<String>, structs::RedisError> {
        let key = format!("message:{}:{}:{}", guild_id, channel_id, message_id);
        match self.client.get_async_connection().await {
            Ok(mut connection) => {
                match connection.get(key).await {
                    Ok(message) => {
                        Ok(message)
                    },
                    Err(err) => {
                        error!("Failed to get message. Failed with error: {}", err);
                        Err(structs::RedisError {
                            message: "Failed to get message".to_string(),
                            redis_error: Some(err)
                        })
                    }
                }
            },
            Err(err) => {
                error!("Failed to get a connection. Failed with error: {}", err);
                Err(structs::RedisError {
                    message: "Failed to get a connection".to_string(),
                    redis_error: Some(err)
                })
            }
        }
    }

    pub async fn set_message(&self, guild_id: i64, channel_id: i64, message_id: i64, user_id: i64, content: String) -> Result<String, structs::RedisError> {
        let key = format!("message:{}:{}:{}", guild_id, channel_id, message_id);
        match self.client.get_async_connection().await {
            Ok(mut connection) => {
                match connection.set_ex(key.clone(), format!("{}:{}", user_id, content), 603800).await {
                    Ok(message) => Ok(message),
                    Err(err) => {
                        error!("Failed to set message {}. Failed with error: {}", key, err);
                        Err(structs::RedisError {
                            message: format!("Failed to set message {}", key),
                            redis_error: Some(err)
                        })
                    }
                }
            },
            Err(err) => {
                error!("Failed to get a connection. Failed with error: {}", err);
                Err(structs::RedisError {
                    message: "Failed to get a connection".to_string(),
                    redis_error: Some(err)
                })
            }
        }
    }

    pub async fn delete_message(&self, guild_id: i64, channel_id: i64, message_id: i64) -> Result<(), structs::RedisError> {
        let key = format!("message:{}:{}:{}", guild_id, channel_id, message_id);
        match self.client.get_async_connection().await {
            Ok(mut connection) => {
                match connection.del(key.clone()).await {
                    Ok(()) => Ok(()),
                    Err(err) => {
                        error!("Failed to delete message {}. Failed with error: {}", key, err);
                        Err(structs::RedisError {
                            message: format!("Failed to delete message {}", key),
                            redis_error: Some(err)
                        })
                    }
                }
            },
            Err(err) => {
                error!("Failed to get a connection. Failed with error: {}", err);
                Err(structs::RedisError {
                    message: "Failed to get a connection".to_string(),
                    redis_error: Some(err)
                })
            }
        }
    }
}