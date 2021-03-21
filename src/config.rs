use crate::util::SendSyncError as Error;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use serenity::{
    model::id::{ChannelId, RoleId},
    prelude::{RwLock, TypeMapKey},
};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    sync::Arc,
};

#[serde_as]
#[derive(Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    allowed_channels: HashSet<ChannelId>,
    #[serde(default)]
    greet_channel: Option<ChannelId>,
    #[serde(default)]
    greet_message: Option<String>,
    #[serde(default)]
    log_channel: Option<ChannelId>,
    #[serde(default)]
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    user_groups: HashMap<RoleId, String>,
    #[serde(default)]
    mute_role: Option<RoleId>,
}

const CONFIG: &str = "config.json";

impl Config {
    fn serialize(&self) -> Result<(), Error> {
        serde_json::to_writer(File::create(CONFIG)?, self).map_err(|e| e.into())
    }

    pub fn new() -> Result<Self, Error> {
        serde_json::from_reader(File::open(CONFIG)?).map_err(|e| e.into())
    }

    pub fn add_allowed_channel(&mut self, ch: ChannelId) -> Result<(), Error> {
        self.allowed_channels.insert(ch);
        Config::serialize(self)
    }

    pub fn channel_is_allowed(&self, ch: ChannelId) -> bool {
        self.allowed_channels.contains(&ch)
    }

    pub fn allowed_channels(&self) -> impl Iterator<Item = &ChannelId> {
        self.allowed_channels.iter()
    }

    pub fn remove_allowed_channel(&mut self, ch: ChannelId) -> Result<(), Error> {
        self.allowed_channels.remove(&ch);
        Config::serialize(self)
    }

    pub fn greet_channel(&self) -> Option<ChannelId> {
        self.greet_channel
    }

    pub fn set_greet_channel(
        &mut self,
        greet_channel: ChannelId,
        msg: Option<String>,
    ) -> Result<(), Error> {
        if let Some(msg) = msg.or_else(|| self.greet_message.take()) {
            self.greet_message = Some(msg);
            self.greet_channel = Some(greet_channel);
            Config::serialize(self)
        } else {
            Err("Provide a greeting for the channel".into())
        }
    }

    pub fn remove_greet_channel(&mut self) -> Result<(), Error> {
        self.greet_channel = None;
        Config::serialize(self)
    }

    pub fn greet_channel_message(&self) -> Option<&str> {
        self.greet_message.as_ref().map(|s| s.as_str())
    }

    pub fn set_log_channel(&mut self, ch: Option<ChannelId>) -> Result<(), Error> {
        self.log_channel = ch;
        Config::serialize(self)
    }

    pub fn log_channel(&self) -> Option<ChannelId> {
        self.log_channel
    }

    pub fn add_user_group(&mut self, ch: RoleId, desc: String) -> Result<(), Error> {
        self.user_groups.insert(ch, desc);
        Config::serialize(self)
    }

    pub fn user_group_exists(&self, ch: RoleId) -> bool {
        self.user_groups.contains_key(&ch)
    }

    pub fn user_groups(&self) -> impl Iterator<Item = (&RoleId, &str)> {
        self.user_groups.iter().map(|(r, s)| (r, s.as_str()))
    }

    pub fn remove_user_group(&mut self, ch: RoleId) -> Result<(), Error> {
        self.user_groups.remove(&ch);
        Config::serialize(self)
    }

    pub fn get_mute_role(&self) -> Option<RoleId> {
        self.mute_role
    }

    pub fn set_mute_role(&mut self, rl: RoleId) -> Result<(), Error> {
        self.mute_role = Some(rl);
        Config::serialize(self)
    }
}

impl TypeMapKey for Config {
    type Value = Arc<RwLock<Config>>;
}
