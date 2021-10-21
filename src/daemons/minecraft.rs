use crate::util::minecraft_server_get;
use daemons::{ControlFlow, Daemon};
use serde::{Deserialize, Serialize};
use serenity::{
    model::id::{GuildId, UserId},
    prelude::{Mutex, TypeMapKey},
    CacheAndHttp,
};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    fs::File,
    process::Command as Fork,
    sync::Arc,
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
    type Value = Arc<Mutex<Minecraft>>;
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

#[serenity::async_trait]
impl Daemon<true> for Minecraft {
    type Data = CacheAndHttp;
    async fn name(&self) -> String {
        String::from("Minecraft colours")
    }

    async fn interval(&self) -> Duration {
        Duration::from_secs(60 * 30)
    }

    async fn run(&mut self, cache_http: &Self::Data) -> ControlFlow {
        let guild_id = match self.guild_id {
            Some(g) => g,
            None => return ControlFlow::CONTINUE,
        };
        let output = match minecraft_server_get(&["list"]) {
            Ok(o) => o,
            Err(e) => {
                crate::log!("Failed to get from the minecraft server: {}", e);
                return ControlFlow::CONTINUE;
            }
        };
        if output.status.success() {
            let online_list = match std::str::from_utf8(&output.stdout) {
                Ok(o) => o,
                Err(e) => {
                    crate::log!(
                        "Failed to parse to string minecraft_server_get output: {}",
                        e
                    );
                    return ControlFlow::CONTINUE;
                }
            };
            let index = match online_list.find(':') {
                Some(i) => i,
                None => {
                    crate::log!("Invalid online list: {}", online_list);
                    return ControlFlow::CONTINUE;
                }
            };
            let (_, list) = online_list.split_at(index + ':'.len_utf8());
            for name in list.split(',').map(str::trim).filter(|x| !x.is_empty()) {
                match self.names.get(name) {
                    Some(uuid) => {
                        let member = match guild_id.member(cache_http, uuid).await {
                            Ok(m) => m,
                            Err(e) => {
                                crate::log!("Can't find member {}: {}", uuid, e);
                                return ControlFlow::CONTINUE;
                            }
                        };
                        let guild = match guild_id.to_partial_guild(&cache_http.http).await {
                            Ok(g) => g,
                            Err(e) => {
                                crate::log!("Can't find partial guild from id {}: {}", guild_id, e);
                                return ControlFlow::CONTINUE;
                            }
                        };
                        let c = member
                            .roles
                            .iter()
                            .filter_map(|r| guild.roles.get(r))
                            .map(|r| r.colour.tuple())
                            .filter_map(Color::from_rgb)
                            .min();
                        let status = match c {
                            Some(c) => Fork::new("./server_do.sh")
                                .args(&[format!("team join {} {}", c, name)])
                                .spawn()
                                .and_then(|mut c| c.wait()),
                            None => Fork::new("./server_do.sh")
                                .args(&[format!("team leave {}", name)])
                                .spawn()
                                .and_then(|mut c| c.wait()),
                        };
                        if let Err(status) = status {
                            crate::log!("Failed to execute server_do: {}", status);
                        }
                    }
                    None => crate::log!("[Minecraft daemon]: '{}' not stored", name),
                }
            }
        }
        ControlFlow::CONTINUE
    }
}
