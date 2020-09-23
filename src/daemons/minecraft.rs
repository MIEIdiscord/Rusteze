use super::Daemon;
use crate::util::minecraft_server_get;
use serde::{Deserialize, Serialize};
use serenity::{
    http::Http,
    model::id::{GuildId, UserId},
    prelude::TypeMapKey,
};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    fs::File,
    process::Command as Fork,
    sync::{Arc, RwLock},
    time::Duration,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Minecraft {
    guild_id: Option<GuildId>,
    names: HashMap<String, UserId>,
}

impl Minecraft {
    const PATH: &'static str = "minecraft.json";

    pub fn load() -> Result<Self, std::io::Error> {
        Ok(serde_json::from_reader(File::open(Minecraft::PATH)?)?)
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        Ok(serde_json::to_writer(File::create(Minecraft::PATH)?, self)?)
    }

    pub fn pair(&mut self, name: String, user: UserId) -> Result<(), std::io::Error> {
        self.names.insert(name, user);
        self.save()
    }

    pub fn set_guild_id(&mut self, gid: GuildId) -> Result<(), std::io::Error> {
        self.guild_id = Some(gid);
        self.save()
    }
}

impl TypeMapKey for Minecraft {
    type Value = Arc<RwLock<Minecraft>>;
}

#[derive(Clone, Copy, PartialOrd, PartialEq, Eq, Ord)]
enum Color {
    Red,
    Cyan,
    Orange,
    Black,
    Yellow,
    Blue,
    Green,
    Purple,
}

impl Color {
    fn from_rgb(rgb: (u8, u8, u8)) -> Option<Self> {
        match rgb {
            (0xff, 0x4c, 0x4c) => Some(Self::Red),
            (0x00, 0xf4, 0xff) => Some(Self::Cyan),
            (0xff, 0x7a, 0x00) => Some(Self::Orange),
            (0xf1, 0xc4, 0x0f) => Some(Self::Yellow),
            (0x34, 0x98, 0xdb) => Some(Self::Blue),
            (0x2e, 0xcc, 0x71) => Some(Self::Green),
            (0x84, 0x3d, 0xa4) => Some(Self::Purple),
            (0x01, 0x01, 0x01) => Some(Self::Black),
            _ => None,
        }
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::Red => "sudoers",
            Self::Cyan => "cool_kids",
            Self::Orange => "cesium",
            Self::Yellow => "4ano",
            Self::Blue => "3ano",
            Self::Green => "2ano",
            Self::Purple => "1ano",
            Self::Black => "blacklist",
        };
        f.write_str(s)
    }
}

impl Daemon for Minecraft {
    fn name(&self) -> String {
        String::from("Minecraft colours")
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(60 * 30)
    }

    fn run(&self, http: &Http) -> Result<(), Box<dyn std::error::Error>> {
        let guild_id = match self.guild_id {
            Some(g) => g,
            None => return Ok(())
        };
        let output = minecraft_server_get(&["list"])?;
        if output.status.success() {
            let online_list = std::str::from_utf8(&output.stdout)?;
            let index = online_list
                .find(':')
                .ok_or_else(|| format!("Invalid online list: {}", online_list))?;
            let (_, list) = online_list.split_at(index + ':'.len_utf8());
            for name in list.split(',').map(str::trim) {
                match self.names.get(name) {
                    Some(uuid) => {
                        let member = guild_id.member(http, uuid)?;
                        let guild = guild_id.to_partial_guild(http)?;
                        let c = member
                            .roles
                            .iter()
                            .filter_map(|r| guild.roles.get(r))
                            .map(|r| r.colour.tuple())
                            .filter_map(Color::from_rgb)
                            .min();
                        match c {
                            Some(c) => {
                                Fork::new("./server_do.sh")
                                    .args(&[format!("team join {} {}", c, name)])
                                    .spawn()?;
                            }
                            None => {
                                Fork::new("./server_do.sh")
                                    .args(&[format!("team leave {}", name)])
                                    .spawn()?;
                            }
                        }
                    }
                    None => eprintln!("[Minecraft daemon]: '{}' not stored", name),
                }
            }
        }

        Ok(())
    }
}
