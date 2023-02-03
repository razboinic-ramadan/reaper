use std::{env, time::{SystemTime, UNIX_EPOCH}};
use tracing::{info, error};
use mongodb::{Client, bson::{doc, to_document}, options::{ClientOptions, FindOneOptions}, Collection};
use serenity::futures::StreamExt;
use crate::{mongo::structs, commands::utils::duration::Duration};

#[derive(Clone)]
pub struct Mongo {
    pub client: Client
}

impl Mongo {
    pub async fn create() -> Result<Self, structs::MongoError> {
        let uri = match env::var("MONGO_URI") {
            Ok(uri) => uri,
            Err(err) => {
                error!("Attempted to obtain MONGO_URI from environment. Failed with error: {}", err);
                return Err(structs::MongoError {
                    message: "Failed to obtain MONGO_URI from environment".to_string(),
                    mongo_error: None
                });
            }
        };

        let mut client_options = match ClientOptions::parse(&uri).await {
            Ok(client_options) => client_options,
            Err(err) => {
                error!("Attempted to parse MONGO_URI. Failed with error: {}", err);
                return Err(structs::MongoError {
                    message: "Failed to parse MONGO_URI".to_string(),
                    mongo_error: Some(err)
                });
            }
        };
        client_options.app_name = Some("Grim-Reaper".to_string());

        match Client::with_options(client_options) {
            Ok(client) => Ok(Self { client }),
            Err(err) => {
                error!("Attempted to create a client. Failed with error: {}", err);
                Err(structs::MongoError {
                    message: "Failed to create a client".to_string(),
                    mongo_error: Some(err)
                })
            }
        }
    }

    pub async fn create_user(&self, user_id: i64, guild_id: i64) -> Result<structs::User, structs::MongoError> {
        let collection: Collection<structs::User> = self.client.database("reaper").collection("users");
        let user = structs::User {
            id: user_id,
            guild_id,
            permissions: vec![]
        };

        match collection.insert_one(to_document(&user).unwrap(), None).await {
            Ok(_) => Ok(user),
            Err(err) => {
                error!("Attempted to create user {} in guild {}. Failed with error: {}", user_id, guild_id, err);
                Err(structs::MongoError {
                    message: "Failed to create a user".to_string(),
                    mongo_error: Some(err)
                })
            }
        }
    }

    pub async fn get_user(&self, user_id: i64, guild_id: i64) -> Result<structs::User, structs::MongoError> {
        let collection: Collection<structs::User> = self.client.database("reaper").collection("users");
        let user = match collection.find_one(doc!{"id": user_id, "guildID": guild_id}, None).await {
            Ok(user) => user,
            Err(err) => {
                error!("Attempted to get user {} for guild {}. Failed with error: {}", user_id, guild_id, err);
                return Err(structs::MongoError {
                    message: "Failed to get a user".to_string(),
                    mongo_error: Some(err)
                });
            }
        };

        match user {
            Some(user) => {
                Ok(user)
            },
            None => {
                info!("User {} does not exist in guild {}. Creating user", user_id, guild_id);
                self.create_user(user_id, guild_id).await
            }
        }
    }

    pub async fn add_permission_to_user(&self, user_id: i64, guild_id: i64, permission: structs::Permissions) -> Result<Vec<structs::Permissions>, structs::MongoError> {
        let user = match self.get_user(user_id, guild_id).await {
            Ok(user) => user,
            Err(err) => {
                error!("Attempted to add permission {} to user {} in guild {}. Failed with error: {}", permission.to_string(), user_id, guild_id, err.message);
                return Err(structs::MongoError {
                    message: "Failed to add permission to user".to_string(),
                    mongo_error: err.mongo_error
                });
            }
        };
        let mut user_permissions = user.permissions.clone();
        user_permissions.push(permission);

        let users: Collection<structs::User> = self.client.database("reaper").collection("users");
        match users.update_one(doc!{"id": user.as_ref().id, "guildID": user.as_ref().guild_id}, doc!{"$set": {"permissions": user_permissions.clone()}}, None).await {
            Ok(_) => Ok(user_permissions),
            Err(err) => {
                error!("Attempted to add permission {} to user {} in guild {}. Failed with error: {}", permission.to_string(), user_id, guild_id, err);
                Err(structs::MongoError {
                    message: "Failed to add permission to user".to_string(),
                    mongo_error: Some(err)
                })
            }
        }
    }

    pub async fn remove_permission_from_user(&self, user_id: i64, guild_id: i64, permission: structs::Permissions) -> Result<Vec<structs::Permissions>, structs::MongoError> {
        let user = match self.get_user(user_id, guild_id).await {
            Ok(user) => user,
            Err(err) => {
                error!("Attempted to remove permission {} from user {} in guild {}. Failed with error: {}", permission.to_string(), user_id, guild_id, err.message);
                return Err(structs::MongoError {
                    message: "Failed to remove permission from user".to_string(),
                    mongo_error: err.mongo_error
                });
            }
        };
        let mut user_permissions = user.permissions.clone();
        user_permissions.retain(|x| x != &permission);

        let users: Collection<structs::User> = self.client.database("reaper").collection("users");
        match users.update_one(doc!{"id": user.as_ref().id, "guildID": user.as_ref().guild_id}, doc!{"$set": {"permissions": user_permissions.clone()}}, None).await {
            Ok(_) => Ok(user_permissions),
            Err(err) => {
                error!("Attempted to remove permission {} from user {} in guild {}. Failed with error: {}", permission.to_string(), user_id, guild_id, err);
                Err(structs::MongoError {
                    message: "Failed to remove permission from user".to_string(),
                    mongo_error: Some(err)
                })
            }
        }
    }

    pub async fn get_actions_for_user(&self, user_id: i64, guild_id: i64) -> Result<Vec<structs::Action>, structs::MongoError> {
        let collection: Collection<structs::Action> = self.client.database("reaper").collection("actions");
        let mut actions = match collection.find(doc!{"userID": user_id, "guildID": guild_id}, None).await {
            Ok(actions) => actions,
            Err(err) => {
                error!("Attempted to get actions for user {} in guild {}. Failed with error: {}", user_id, guild_id, err);
                return Err(structs::MongoError {
                    message: "Failed to get actions for user".to_string(),
                    mongo_error: Some(err)
                });
            }
        };

        let mut actions_vec: Vec<structs::Action> = vec![];
        while let Some(action) = actions.next().await {
            match action {
                Ok(action) => actions_vec.push(action),
                Err(err) => {
                    error!("Attempted to get actions for user {} in guild {}. Failed with error: {}", user_id, guild_id, err);
                    return Err(structs::MongoError {
                        message: "Failed to get actions for user".to_string(),
                        mongo_error: Some(err)
                    });
                }
            }
        }

        Ok(actions_vec)
    }

    pub async fn get_action(&self, action_id: String) -> Result<Option<structs::Action>, structs::MongoError> {
        let collection: Collection<structs::Action> = self.client.database("reaper").collection("actions");
        let uuid = match mongodb::bson::oid::ObjectId::parse_str(&action_id) {
            Ok(oid) => oid,
            Err(err) => {
                error!("Attempted to parse ObjectID {}. Failed with error: {}", &action_id, err);
                return Err(structs::MongoError {
                    message: "Failed to expire action".to_string(),
                    mongo_error: None
                });
            }
        };
        match collection.find_one(doc!{"_id": uuid}, None).await {
            Ok(action) => Ok(action),
            Err(err) => {
                error!("Attempted to get action {}. Failed with error: {}", uuid.to_string(), err);
                Err(structs::MongoError {
                    message: "Failed to get action".to_string(),
                    mongo_error: Some(err)
                })
            }
        }
    }

    pub async fn add_action_to_user(&self, user_id: i64, guild_id: i64, action_type: structs::ActionType, reason: String, moderator_id: i64, expiry: Option<Duration>) -> Result<structs::Action, structs::MongoError> {
        let actions: Collection<structs::Action> = self.client.database("reaper").collection("actions");
        let mut duration: Option<i64> = None;
        if let Some(dur) = expiry {
            if dur.to_unix_timestamp() == 0 {
                duration = None;
            }
            else {
                duration = Some(dur.to_unix_timestamp() as i64);
            }
            
        }
        let action = structs::Action {
            uuid: mongodb::bson::oid::ObjectId::new(),
            action_type,
            guild_id,
            user_id,
            moderator_id,
            reason,
            active: true,
            expiry: duration
        };

        match actions.insert_one(action.clone(), None).await {
            Ok(_) => Ok(action),
            Err(err) => {
                error!("Attempted to add action {} to user {} in guild {}. Failed with error: {}", action_type.to_string(), user_id, guild_id, err);
                Err(structs::MongoError {
                    message: "Failed to add action to user".to_string(),
                    mongo_error: Some(err)
                })
            }
        }
    }

    pub async fn update_action_reason(&self, guild_id: i64, action_id: String, reason: String) -> Result<Option<structs::Action>, structs::MongoError> {
        let actions: Collection<structs::Action> = self.client.database("reaper").collection("actions");
        let uuid = match mongodb::bson::oid::ObjectId::parse_str(&action_id) {
            Ok(oid) => oid,
            Err(err) => {
                error!("Attempted to parse ObjectID {}. Failed with error: {}", &action_id, err);
                return Err(structs::MongoError {
                    message: "Failed to expire action".to_string(),
                    mongo_error: None
                });
            }
        };
        let action = match actions.find_one_and_update(doc!{"_id": uuid, "guildID": guild_id}, doc!{"$set": {"reason": reason}}, None).await {
            Ok(action) => action,
            Err(err) => {
                error!("Attempted to update reason for action {} in guild {}. Failed with error: {}", action_id, guild_id, err);
                return Err(structs::MongoError {
                    message: "Failed to update reason for action".to_string(),
                    mongo_error: Some(err)
                });
            }
        };

        Ok(action)
    }

    pub async fn update_action_duration(&self, guild_id: i64, action_id: String, duration: Duration) -> Result<Option<structs::Action>, structs::MongoError> {
        let actions: Collection<structs::Action> = self.client.database("reaper").collection("actions");
        let uuid = match mongodb::bson::oid::ObjectId::parse_str(&action_id) {
            Ok(oid) => oid,
            Err(err) => {
                error!("Attempted to parse ObjectID {}. Failed with error: {}", &action_id, err);
                return Err(structs::MongoError {
                    message: "Failed to expire action".to_string(),
                    mongo_error: None
                });
            }
        };
        let action = match actions.find_one_and_update(doc!{"_id": uuid, "guildID": guild_id}, doc!{"$set": {"expiry": duration.to_unix_timestamp() as i64}}, None).await {
            Ok(action) => action,
            Err(err) => {
                error!("Attempted to update duration for action {} in guild {}. Failed with error: {}", action_id, guild_id, err);
                return Err(structs::MongoError {
                    message: "Failed to update duration for action".to_string(),
                    mongo_error: Some(err)
                });
            }
        };

        Ok(action)
    }

    pub async fn expire_action(&self, guild_id: i64, action_id: String) -> Result<(), structs::MongoError> {
        let actions: Collection<structs::Action> = self.client.database("reaper").collection("actions");
        let uuid = match mongodb::bson::oid::ObjectId::parse_str(&action_id) {
            Ok(oid) => oid,
            Err(err) => {
                error!("Attempted to parse ObjectID {}. Failed with error: {}", &action_id, err);
                return Err(structs::MongoError {
                    message: "Failed to expire action".to_string(),
                    mongo_error: None
                });
            }
        };
        match actions.update_one(doc!{"guildID": guild_id, "_id": uuid}, doc!{"$set": {"active": false}}, None).await {
            Ok(_) => Ok(()),
            Err(err) => {
                error!("Attempted to expire action {} in guild {}. Failed with error: {}", action_id, guild_id, err);
                Err(structs::MongoError {
                    message: "Failed to expire action".to_string(),
                    mongo_error: Some(err)
                })
            }
        }
    }

    pub async fn remove_action(&self, guild_id: i64, action_id: String) -> Result<(), structs::MongoError> {
        let actions: Collection<structs::Action> = self.client.database("reaper").collection("actions");
        let uuid = match mongodb::bson::oid::ObjectId::parse_str(&action_id) {
            Ok(oid) => oid,
            Err(err) => {
                error!("Attempted to parse ObjectID {}. Failed with error: {}", &action_id, err);
                return Err(structs::MongoError {
                    message: "Failed to expire action".to_string(),
                    mongo_error: None
                });
            }
        };
        match actions.delete_one(doc!{"guildID": guild_id, "_id": uuid}, None).await {
            Ok(_) => Ok(()),
            Err(err) => {
                error!("Attempted to remove action {} in guild {}. Failed with error: {}", action_id, guild_id, err);
                Err(structs::MongoError {
                    message: "Failed to remove action".to_string(),
                    mongo_error: Some(err)
                })
            }
        }
    }

    pub async fn get_recent_mod_action(&self, guild_id: i64, moderator_id: i64) -> Result<Option<structs::Action>, structs::MongoError> {
        let actions: Collection<structs::Action> = self.client.database("reaper").collection("actions");
        match actions.find_one(doc!{"guildID": guild_id, "moderatorID": moderator_id}, Some(
            FindOneOptions::builder().sort(doc!{"_id": -1}).build()
        )).await {
            Ok(action) => {
                Ok(action)
            },
            Err(err) => {
                error!("Attempted to get recent mod action for moderator {} in guild {}. Failed with error: {}", moderator_id, guild_id, err);
                Err(structs::MongoError {
                    message: "Failed to get recent mod action".to_string(),
                    mongo_error: Some(err)
                })
            }
        }
    }

    pub async fn get_expired_actions(&self) -> Result<Vec<structs::Action>, structs::MongoError> {
        let actions: Collection<structs::Action> = self.client.database("reaper").collection("actions");
        let mut actions = match actions.find(doc!{"expiry": {"$lt": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64}, "active": true}, None).await {
            Ok(actions) => {
                actions
            },
            Err(err) => {
                error!("Attempted to get expired actions. Failed with error: {}", err);
                return Err(structs::MongoError {
                    message: "Failed to get expired actions".to_string(),
                    mongo_error: Some(err)
                });
            }
        };

        let mut actions_vec: Vec<structs::Action> = vec![];
        while let Some(action) = actions.next().await {
            match action {
                Ok(action) => {
                    match self.expire_action(action.guild_id, action.uuid.to_string()).await {
                        Ok(_) => {
                            actions_vec.push(action);
                        },
                        Err(err) => {
                            error!("Failed to expire action {} in guild {}. Failed with error: {}", action.uuid, action.guild_id, err);
                        }
                    }
                },
                Err(err) => {
                    error!("Failed to get expired actions. Failed with error: {}", err);
                    return Err(structs::MongoError {
                        message: "Failed to get expired actions".to_string(),
                        mongo_error: Some(err)
                    });
                }
            }
        }
        Ok(actions_vec)
    }

    pub async fn create_role(&self, role_id: i64, guild_id: i64) -> Result<structs::Role, structs::MongoError> {
        let collection: Collection<structs::Role> = self.client.database("reaper").collection("roles");
        let role = structs::Role {
            id: role_id,
            guild_id,
            permissions: vec![]
        };

        match collection.insert_one(to_document(&role).unwrap(), None).await {
            Ok(_) => Ok(role),
            Err(err) => {
                error!("Attempted to create role {} in guild {}. Failed with error: {}", role_id, guild_id, err);
                Err(structs::MongoError {
                    message: "Failed to create a role".to_string(),
                    mongo_error: Some(err)
                })
            }
        }
    }

    pub async fn get_role(&self, role_id: i64, guild_id: i64) -> Result<structs::Role, structs::MongoError> {
        let collection: Collection<structs::Role> = self.client.database("reaper").collection("roles");
        let role = match collection.find_one(doc!{"id": role_id, "guildID": guild_id}, None).await {
            Ok(role) => role,
            Err(err) => {
                error!("Attempted to get role {} for guild {}. Failed with error: {}", role_id, guild_id, err);
                return Err(structs::MongoError {
                    message: "Failed to get a role".to_string(),
                    mongo_error: Some(err)
                });
            }
        };

        match role {
            Some(role) => {
                Ok(role)
            },
            None => {
                info!("Role {} does not exist in guild {}. Creating role", role_id, guild_id);
                self.create_role(role_id, guild_id).await
            }
        }
    }

    pub async fn add_permission_to_role(&self, role_id: i64, guild_id: i64, permission: structs::Permissions) -> Result<Vec<structs::Permissions>, structs::MongoError> {
        let role = match self.get_role(role_id, guild_id).await {
            Ok(role) => role,
            Err(err) => {
                error!("Attempted to add permission {} to role {} in guild {}. Failed with error: {}", permission.to_string(), role_id, guild_id, err.message);
                return Err(structs::MongoError {
                    message: "Failed to add permission to role".to_string(),
                    mongo_error: err.mongo_error
                });
            }
        };
        let mut role_permissions = role.permissions.clone();
        role_permissions.push(permission);

        let roles: Collection<structs::Role> = self.client.database("reaper").collection("roles");
        match roles.update_one(doc!{"id": role.as_ref().id, "guildID": role.as_ref().guild_id}, doc!{"$set": {"permissions": role_permissions.clone()}}, None).await {
            Ok(_) => Ok(role_permissions),
            Err(err) => {
                error!("Attempted to add permission {} to role {} in guild {}. Failed with error: {}", permission.to_string(), role_id, guild_id, err);
                Err(structs::MongoError {
                    message: "Failed to add permission to role".to_string(),
                    mongo_error: Some(err)
                })
            }
        }
    }

    pub async fn remove_permission_from_role(&self, role_id: i64, guild_id: i64, permission: structs::Permissions) -> Result<Vec<structs::Permissions>, structs::MongoError> {
        let role = match self.get_role(role_id, guild_id).await {
            Ok(role) => role,
            Err(err) => {
                error!("Attempted to remove permission {} from role {} in guild {}. Failed with error: {}", permission.to_string(), role_id, guild_id, err.message);
                return Err(structs::MongoError {
                    message: "Failed to remove permission from role".to_string(),
                    mongo_error: err.mongo_error
                });
            }
        };
        let mut role_permissions = role.permissions.clone();
        role_permissions.retain(|x| x != &permission);

        let roles: Collection<structs::Role> = self.client.database("reaper").collection("roles");
        match roles.update_one(doc!{"id": role.as_ref().id, "guildID": role.as_ref().guild_id}, doc!{"$set": {"permissions": role_permissions.clone()}}, None).await {
            Ok(_) => Ok(role_permissions),
            Err(err) => {
                error!("Attempted to remove permission {} from role {} in guild {}. Failed with error: {}", permission.to_string(), role_id, guild_id, err);
                Err(structs::MongoError {
                    message: "Failed to remove permission from role".to_string(),
                    mongo_error: Some(err)
                })
            }
        }
    }

    pub async fn create_guild(&self, guild_id: i64) -> Result<structs::Guild, structs::MongoError> {
        let collection: Collection<structs::Guild> = self.client.database("reaper").collection("guilds");
        let guild = structs::Guild {
            id: guild_id,
            config: structs::GuildConfig {
                logging: None,
                moderation: None,
                boards: None
            }
        };

        match collection.insert_one(to_document(&guild).unwrap(), None).await {
            Ok(_) => {
                Ok(guild)
            },
            Err(err) => {
                error!("Attempted to create guild {}. Failed with error: {}", guild_id, err);
                Err(structs::MongoError {
                    message: "Failed to create a guild".to_string(),
                    mongo_error: Some(err)
                })
            }
        }
    }

    pub async fn get_guild(&self, guild_id: i64) -> Result<structs::Guild, structs::MongoError> {
        let collection: Collection<structs::Guild> = self.client.database("reaper").collection("guilds");
        let guild = match collection.find_one(doc!{"id": guild_id}, None).await {
            Ok(guild) => guild,
            Err(err) => {
                error!("Attempted to get guild {}. Failed with error: {}", guild_id, err);
                return Err(structs::MongoError {
                    message: "Failed to get a guild".to_string(),
                    mongo_error: Some(err)
                });
            }
        };

        match guild {
            Some(guild) => {
                Ok(guild)
            },
            None => {
                info!("Guild {} does not exist. Creating guild", guild_id);
                self.create_guild(guild_id).await
            }
        }
    }

    pub async fn check_message_on_board(&self, message_id: i64, channel_id: i64) -> Result<bool, structs::MongoError> {
        let collection: Collection<structs::BoardMessage> = self.client.database("reaper").collection("boards");
        let message = match collection.find_one(doc!{"messageID": message_id, "channelID": channel_id}, None).await {
            Ok(message) => message,
            Err(err) => {
                error!("Attempted to check if message {} is on a board. Failed with error: {}", message_id, err);
                return Err(structs::MongoError {
                    message: "Failed to check if message is on a board".to_string(),
                    mongo_error: Some(err)
                });
            }
        };
        match message {
            Some(_) => Ok(true),
            None => Ok(false)
        }
    }

    pub async fn add_message_to_board(&self, message_id: i64, channel_id: i64) -> Result<(), structs::MongoError> {
        let collection: Collection<structs::BoardMessage> = self.client.database("reaper").collection("boards");
        let board_message = structs::BoardMessage {
            message_id,
            channel_id
        };
        match collection.insert_one(to_document(&board_message).unwrap(), None).await {
            Ok(_) => Ok(()),
            Err(err) => {
                error!("Attempted to add message {} to board. Failed with error: {}", message_id, err);
                Err(structs::MongoError {
                    message: "Failed to add message to board".to_string(),
                    mongo_error: Some(err)
                })
            }
        }
    }
}