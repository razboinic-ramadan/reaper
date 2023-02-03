use std::{collections::HashMap, borrow::Borrow, fmt::Display, hash::Hash};
use mongodb::bson::Bson;
use serde::{Deserializer, Deserialize as SerdeDeserialize};
use serde_derive::{Serialize, Deserialize};
use strum_macros::{EnumIter};

pub struct MongoError {
    pub message: String,
    pub mongo_error: Option<mongodb::error::Error>
}

impl Display for MongoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, EnumIter)]
#[serde(rename_all = "camelCase")]
pub enum Permissions {
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "permissions.add")]
    PermissionsAdd,
    #[serde(rename = "permissions.list")]
    PermissionsList,
    #[serde(rename = "permissions.remove")]
    PermissionsRemove,
    #[serde(rename = "permissions.view")]
    PermissionsView,
    #[serde(rename = "moderation.strike")]
    ModerationStrike,
    #[serde(rename = "moderation.search.self")]
    ModerationSearchSelf,
    #[serde(rename = "moderation.search.others")]
    ModerationSearchOthers,
    #[serde(rename = "moderation.search.self.expired")]
    ModerationSearchSelfExpired,
    #[serde(rename = "moderation.search.others.expired")]
    ModerationSearchOthersExpired,
    #[serde(rename = "moderation.search.uuid")]
    ModerationSearchUuid,
    #[serde(rename = "moderation.mute")]
    ModerationMute,
    #[serde(rename = "moderation.unmute")]
    ModerationUnmute,
    #[serde(rename = "moderation.kick")]
    ModerationKick,
    #[serde(rename = "moderation.ban")]
    ModerationBan,
    #[serde(rename = "moderation.unban")]
    ModerationUnban,
    #[serde(rename = "moderation.remove")]
    ModerationRemove,
    #[serde(rename = "moderation.expire")]
    ModerationExpire,
    #[serde(rename = "moderation.duration")]
    ModerationDuration,
    #[serde(rename = "moderation.reason")]
    ModerationReason,
}

impl AsRef<Permissions> for Permissions {
    fn as_ref(&self) -> &Permissions {
        self
    }
}

impl From<Permissions> for Bson {
    fn from(p: Permissions) -> Self {
        Bson::String(p.to_string())
    }
}

impl ToString for Permissions {
    fn to_string(&self) -> String {
        match self {
            Permissions::PermissionsAdd => "permissions.add".to_string(),
            Permissions::PermissionsList => "permissions.list".to_string(),
            Permissions::PermissionsRemove => "permissions.remove".to_string(),
            Permissions::PermissionsView => "permissions.view".to_string(),
            Permissions::ModerationStrike => "moderation.strike".to_string(),
            Permissions::ModerationSearchSelf => "moderation.search.self".to_string(),
            Permissions::ModerationSearchOthers => "moderation.search.others".to_string(),
            Permissions::ModerationSearchSelfExpired => "moderation.search.self.expired".to_string(),
            Permissions::ModerationSearchOthersExpired => "moderation.search.others.expired".to_string(),
            Permissions::ModerationSearchUuid => "moderation.search.uuid".to_string(),
            Permissions::ModerationMute => "moderation.mute".to_string(),
            Permissions::ModerationUnmute => "moderation.unmute".to_string(),
            Permissions::ModerationKick => "moderation.kick".to_string(),
            Permissions::ModerationBan => "moderation.ban".to_string(),
            Permissions::ModerationUnban => "moderation.unban".to_string(),
            Permissions::ModerationRemove => "moderation.remove".to_string(),
            Permissions::ModerationExpire => "moderation.expire".to_string(),
            Permissions::ModerationDuration => "moderation.duration".to_string(),
            Permissions::ModerationReason => "moderation.reason".to_string(),
            _ => "unknown".to_string(),
        }
    }
}

impl From<String> for Permissions {
    fn from(s: String) -> Self {
        match s.as_str() {
            "permissions.add" => Permissions::PermissionsAdd,
            "permissions.list" => Permissions::PermissionsList,
            "permissions.remove" => Permissions::PermissionsRemove,
            "permissions.view" => Permissions::PermissionsView,
            "moderation.strike" => Permissions::ModerationStrike,
            "moderation.search.self" => Permissions::ModerationSearchSelf,
            "moderation.search.others" => Permissions::ModerationSearchOthers,
            "moderation.search.self.expired" => Permissions::ModerationSearchSelfExpired,
            "moderation.search.others.expired" => Permissions::ModerationSearchOthersExpired,
            "moderation.search.uuid" => Permissions::ModerationSearchUuid,
            "moderation.mute" => Permissions::ModerationMute,
            "moderation.unmute" => Permissions::ModerationUnmute,
            "moderation.kick" => Permissions::ModerationKick,
            "moderation.ban" => Permissions::ModerationBan,
            "moderation.unban" => Permissions::ModerationUnban,
            "moderation.remove" => Permissions::ModerationRemove,
            "moderation.expire" => Permissions::ModerationExpire,
            "moderation.duration" => Permissions::ModerationDuration,
            "moderation.reason" => Permissions::ModerationReason,
            _ => Permissions::Unknown
        }
    }
}

impl PartialEq for Permissions {
    fn eq(&self, other: &Permissions) -> bool {
        self.to_string() == other.to_string()
    }
}

impl Hash for Permissions {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_string().hash(state);
    }
}

impl Eq for Permissions {}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i64,
    #[serde(rename = "guildID")]
    pub guild_id: i64,
    pub permissions: Vec<Permissions>
}

impl AsRef<User> for User {
    fn as_ref(&self) -> &User {
        self
    }
}

impl Borrow<User> for mongodb::bson::Document {
    fn borrow(&self) -> &User {
        let user = User {
            id: self.get_i64("id").unwrap(),
            guild_id: self.get_i64("guildID").unwrap(),
            permissions: self.get_array("permissions").unwrap().iter().map(|permission| Permissions::from(permission.as_str().unwrap().to_string())).collect()
        };
        Box::leak(Box::new(user))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    pub id: i64,
    #[serde(rename = "guildID")]
    pub guild_id: i64,
    pub permissions: Vec<Permissions>
}

impl AsRef<Role> for Role {
    fn as_ref(&self) -> &Role {
        self
    }
}

impl Borrow<Role> for mongodb::bson::Document {
    fn borrow(&self) -> &Role {
        let role = Role {
            id: self.get_i64("id").unwrap(),
            guild_id: self.get_i64("guildID").unwrap(),
            permissions: self.get_array("permissions").unwrap().iter().map(|permission| Permissions::from(permission.as_str().unwrap().to_string())).collect()
        };
        Box::leak(Box::new(role))
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfig {
    pub logging_channel: i64
}

impl From<LoggingConfig> for mongodb::bson::Document {
    fn from(logging_config: LoggingConfig) -> Self {
        let mut document = mongodb::bson::Document::new();
        document.insert("loggingChannel", Bson::Int64(logging_config.logging_channel));
        document
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModerationConfig {
    pub mute_role: i64,
    #[serde(deserialize_with = "deserialize_strike_escalations")]
    pub strike_escalations: HashMap<u64, StrikeEscalation>,
    pub blacklisted_words: Vec<String>,
    pub blacklisted_regex: Vec<String>,
    pub default_strike_duration: String,
}

fn deserialize_strike_escalations<'de, D>(deserializer: D) -> Result<HashMap<u64, StrikeEscalation>, D::Error>
    where D: Deserializer<'de>
{
    let mut map: HashMap::<u64, StrikeEscalation> = HashMap::new();
    let mut incoming_map: HashMap::<String, StrikeEscalation> = HashMap::deserialize(deserializer)?;
    for (key, value) in incoming_map.drain() {
        map.insert(key.parse::<u64>().unwrap(), value);
    }
    Ok(map)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BoardConfig {
    pub emotes: Vec<String>,
    pub quota: u64
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuildConfig {
    pub logging: Option<LoggingConfig>,
    pub moderation: Option<ModerationConfig>,
    pub boards: Option<HashMap<String, BoardConfig>>
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Guild {
    pub id: i64,
    pub config: GuildConfig
}

impl AsRef<Guild> for Guild {
    fn as_ref(&self) -> &Guild {
        self
    }
}

impl Borrow<Guild> for mongodb::bson::Document {
    fn borrow(&self) -> &Guild {
        let guild = Guild {
            id: self.get_i64("id").unwrap(),
            config: GuildConfig {
                logging: match self.get_document("config").unwrap().get_document("logging") {
                    Ok(logging) => Some(LoggingConfig {
                        logging_channel: logging.get_i64("loggingChannel").unwrap()
                    }),
                    Err(_) => None
                },
                moderation: match self.get_document("config").unwrap().get_document("moderation") {
                    Ok(moderation) => Some(ModerationConfig {
                        mute_role: moderation.get_i64("muteRole").unwrap(),
                        strike_escalations: moderation.get_document("strikeEscalations").unwrap().iter().map(|(key, value)| (key.parse::<u64>().unwrap(), StrikeEscalation {
                            duration: match value.as_document().unwrap().get_str("duration") {
                                Ok(duration) => Some(duration.to_string()),
                                Err(_) => None
                            },
                            action: match value.as_document().unwrap().get_str("actionType") {
                                Ok(action_type) => ActionType::from(action_type.to_string()),
                                Err(_) => ActionType::Unknown
                            }
                        })).collect(),
                        blacklisted_words: moderation.get_array("blacklistedWords").unwrap().iter().map(|word| word.as_str().unwrap().to_string()).collect(),
                        blacklisted_regex: moderation.get_array("blacklistedRegex").unwrap().iter().map(|regex| regex.as_str().unwrap().to_string()).collect(),
                        default_strike_duration: moderation.get_str("defaultStrikeDuration").unwrap().to_string()
                    }),
                    Err(_) => None
                },
                boards: match self.get_document("config").unwrap().get_document("boards") {
                    Ok(boards) => Some(boards.iter().map(|(key, value)| (key.to_string(), BoardConfig {
                        emotes: value.as_document().unwrap().get_array("emotes").unwrap().iter().map(|emote| emote.as_str().unwrap().to_string()).collect(),
                        quota: value.as_document().unwrap().get_i64("quota").unwrap() as u64
                    })).collect()),
                    Err(_) => None
                }
            }
        };
        Box::leak(Box::new(guild))
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ActionType {
    Unknown,
    Strike,
    Mute,
    Kick,
    Ban
}

impl From<String> for ActionType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "strike" => ActionType::Strike,
            "mute" => ActionType::Mute,
            "kick" => ActionType::Kick,
            "ban" => ActionType::Ban,
            _ => ActionType::Unknown
        }
    }
}

impl ToString for ActionType {
    fn to_string(&self) -> String {
        match self {
            ActionType::Unknown => "unknown".to_string(),
            ActionType::Strike => "strike".to_string(),
            ActionType::Mute => "mute".to_string(),
            ActionType::Kick => "kick".to_string(),
            ActionType::Ban => "ban".to_string()
        }
    }
}

impl PartialEq for ActionType {
    fn eq(&self, other: &ActionType) -> bool {
        self.to_string() == other.to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StrikeEscalation {
    pub action: ActionType,
    pub duration: Option<String>
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    #[serde(rename = "_id")]
    pub uuid: mongodb::bson::oid::ObjectId,
    pub action_type: ActionType,
    #[serde(rename = "guildID")]
    pub guild_id: i64,
    #[serde(rename = "userID")]
    pub user_id: i64,
    #[serde(rename = "moderatorID")]
    pub moderator_id: i64,
    pub reason: String,
    pub active: bool,
    pub expiry: Option<i64>
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BoardMessage {
    #[serde(rename = "messageID")]
    pub message_id: i64,
    #[serde(rename = "channelID")]
    pub channel_id: i64
}

impl Borrow<BoardMessage> for mongodb::bson::Document {
    fn borrow(&self) -> &BoardMessage {
        let board_message = BoardMessage {
            message_id: self.get_i64("messageID").unwrap(),
            channel_id: self.get_i64("channelID").unwrap()
        };
        Box::leak(Box::new(board_message))
    }
}