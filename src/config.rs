use serde::{Deserialize, Serialize};
use serenity::{model::id::ChannelId, prelude::TypeMapKey};

use std::collections::HashSet;
use std::error;
use std::fs::File;
use std::sync::{Arc, RwLock};

#[derive(Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    allowed_channels: HashSet<ChannelId>,
    #[serde(default)]
    greet_channel: Option<ChannelId>,
}

const CONFIG: &str = "config.json";

impl Config {
    pub fn new() -> Result<Self, Box<dyn error::Error>> {
        serde_json::from_reader(File::open(CONFIG)?).map_err(|e| e.into())
    }

    pub fn add_allowed_channel(&mut self, ch: ChannelId) -> Result<(), Box<dyn error::Error>> {
        self.allowed_channels.insert(ch);
        Config::serialize(self)
    }

    pub fn channel_is_allowed(&self, ch: ChannelId) -> bool {
        self.allowed_channels.contains(&ch)
    }

    pub fn allowed_channels(&self) -> impl Iterator<Item = &ChannelId> {
        self.allowed_channels.iter()
    }

    fn serialize(&self) -> Result<(), Box<dyn error::Error>> {
        serde_json::to_writer(File::create(CONFIG)?, self).map_err(|e| e.into())
    }

    pub fn remove_allowed_channel(&mut self, ch: ChannelId) -> Result<(), Box<dyn error::Error>> {
        self.allowed_channels.remove(&ch);
        Config::serialize(self)
    }

    pub fn greet_channel(&self) -> Option<ChannelId> {
        self.greet_channel
    }

    pub fn set_greet_channel(&mut self, greet_channel: ChannelId) -> Result<(), Box<dyn error::Error>> {
        self.greet_channel = Some(greet_channel);
        Config::serialize(self)
    }

    pub fn remove_greet_channel(&mut self) -> Result<(), Box<dyn error::Error>> {
        self.greet_channel = None;
        Config::serialize(self)
    }
}

impl TypeMapKey for Config {
    type Value = Arc<RwLock<Config>>;
}
