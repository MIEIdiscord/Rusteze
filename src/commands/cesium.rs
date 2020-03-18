#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use serenity::{
    framework::standard::{
        macros::{check, command, group},
        Args, CheckResult, CommandOptions, CommandResult, Reason,
    },
    http::{CacheHttp, Http},
    model::{
        channel::{ChannelType, Message, PermissionOverwrite, PermissionOverwriteType},
        id::{ChannelId, GuildId, RoleId, UserId},
        permissions::Permissions,
    },
    prelude::*,
};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{self, BufWriter},
    iter::once,
    sync::{Arc, RwLock},
};

group!({
    name: "cesium",
    options: {
        prefix: "cesium",
        checks: [is_mod_or_cesium],
    },
    commands: [add, join, remove],
});

const CESIUM_CATEGORY: ChannelId = ChannelId(418798551317872660);
const CESIUM_ROLE: RoleId = RoleId(418842665061318676);
const MODS_ROLE: RoleId = RoleId(618572138718298132);
const MENTOR_ROLE: RoleId = RoleId(688760837980291120);
const CHANNELS: &str = "cesium_channels.json";

#[check]
#[name = "is_mod_or_cesium"]
pub fn is_mod_or_cesium(
    _: &mut Context,
    msg: &Message,
    _: &mut Args,
    _: &CommandOptions,
) -> CheckResult {
    msg.member
        .as_ref()
        .and_then(|m| {
            if [MENTOR_ROLE, CESIUM_ROLE, MODS_ROLE]
                .iter()
                .any(|r| m.roles.contains(r))
            {
                Some(CheckResult::Success)
            } else {
                None
            }
        })
        .unwrap_or(CheckResult::Failure(Reason::User(
            "You don't have permission to use that command!".to_string(),
        )))
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct ChannelMapping {
    last_number: u32,
    channels: HashMap<ChannelId, ChannelId>,
}

impl ChannelMapping {
    pub fn load() -> io::Result<Self> {
        Ok(serde_json::from_reader(File::open(CHANNELS)?).unwrap_or_default())
    }

    fn write_channels(&self) -> Result<(), io::Error> {
        serde_json::to_writer(
            BufWriter::new(
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(CHANNELS)?,
            ),
            &self,
        )
        .map_err(|j| j.into())
    }

    fn get_channel(&self, channel: &ChannelId) -> Option<&ChannelId> {
        self.channels.get(channel)
    }

    fn create_channel<C, U>(&mut self, guild_id: GuildId, ctx: C, users: U) -> CommandResult
    where
        C: CacheHttp + AsRef<Http>,
        U: Iterator<Item = UserId>,
    {
        let users: Vec<_> = users
            .map(|u| PermissionOverwrite {
                kind: PermissionOverwriteType::Member(u),
                allow: Permissions::READ_MESSAGES | Permissions { bits: 0x00000400 },
                deny: Permissions::empty(),
            })
            .chain(once(PermissionOverwrite {
                kind: PermissionOverwriteType::Role(CESIUM_ROLE),
                allow: Permissions::READ_MESSAGES | Permissions { bits: 0x00000400 },
                deny: Permissions::empty(),
            }))
            .chain(once(PermissionOverwrite {
                kind: PermissionOverwriteType::Role(MODS_ROLE),
                allow: Permissions::READ_MESSAGES | Permissions { bits: 0x00000400 },
                deny: Permissions::empty(),
            }))
            .chain(once(PermissionOverwrite {
                kind: PermissionOverwriteType::Role(MENTOR_ROLE),
                allow: Permissions::READ_MESSAGES | Permissions { bits: 0x00000400 },
                deny: Permissions::empty(),
            }))
            .chain(once(PermissionOverwrite {
                kind: PermissionOverwriteType::Role(RoleId(guild_id.0)),
                allow: Permissions::empty(),
                deny: Permissions::READ_MESSAGES | Permissions { bits: 0x00000400 },
            }))
            .collect();
        let text = guild_id.create_channel(&ctx, |channel| {
            channel
                .name(format!("mentor-channel-{}", self.last_number))
                .kind(ChannelType::Text)
                .category(CESIUM_CATEGORY)
                .permissions(users.iter().map(|p| p.clone()))
        })?;
        let voice = guild_id.create_channel(&ctx, |channel| {
            channel
                .name(format!("mentor-channel-{}", self.last_number))
                .kind(ChannelType::Voice)
                .category(CESIUM_CATEGORY)
                .permissions(users.into_iter())
        })?;
        self.last_number += 1;
        self.channels.insert(text.id, voice.id);
        self.write_channels()?;
        Ok(())
    }

    fn delete_channel<C>(&mut self, channel_id: ChannelId, ctx: C) -> CommandResult
    where
        C: CacheHttp + AsRef<Http> + Copy,
    {
        let voice = channel_id
            .to_channel(ctx)?
            .guild()
            .filter(|ch| ch.read().category_id == Some(CESIUM_CATEGORY))
            .and_then(|_| self.channels.get(&channel_id))
            .ok_or("Invalid channel")?;
        channel_id.delete(ctx)?;
        voice.delete(ctx)?;
        self.write_channels()?;
        Ok(())
    }
}

impl TypeMapKey for ChannelMapping {
    type Value = Arc<RwLock<ChannelMapping>>;
}

#[command]
#[description("Adds a new private room")]
#[usage("[StudentMention...]")]
pub fn add(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.ok_or("Message with no guild id")?;
    let data_lock = ctx.data.write();
    let mut channels = data_lock.get::<ChannelMapping>().unwrap().write().unwrap();
    args.iter::<UserId>().try_for_each(|x| x.map(|_| ()))?;
    args.restore();
    channels.create_channel(guild_id, &ctx, args.iter::<UserId>().map(Result::unwrap))?;
    msg.channel_id.say(&ctx, "Room created")?;
    Ok(())
}

#[command]
#[description("Removes a new private room")]
#[usage("")]
pub fn remove(ctx: &mut Context, msg: &Message) -> CommandResult {
    let data_lock = ctx.data.write();
    let mut channels = data_lock.get::<ChannelMapping>().unwrap().write().unwrap();
    channels.delete_channel(msg.channel_id, &ctx)
}

#[command]
#[description("Adds a student to a private room")]
#[usage("[StudentMention]")]
pub fn join(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read();
    let channels = data_lock.get::<ChannelMapping>().unwrap().read().unwrap();
    let text = msg.channel_id;
    let voice = channels.get_channel(&text).ok_or("Invalid channel")?;
    args.iter::<UserId>().try_for_each(|x| {
        x.map_err(|_| "invalid user id")
            .and_then(|u| {
                text.create_permission(
                    &ctx,
                    &PermissionOverwrite {
                        kind: PermissionOverwriteType::Member(u),
                        allow: Permissions::READ_MESSAGES,
                        deny: Permissions::empty(),
                    },
                )
                .map(|_| u)
                .map_err(|_| "Failed to create permission for text channel")
            })
            .and_then(|u| {
                voice
                    .create_permission(
                        &ctx,
                        &PermissionOverwrite {
                            kind: PermissionOverwriteType::Member(u),
                            allow: Permissions::READ_MESSAGES,
                            deny: Permissions::empty(),
                        },
                    )
                    .map_err(|_| "Failed to create permissio for voice channeln")
            })
    })?;
    msg.channel_id.say(&ctx, "User(s) added")?;
    Ok(())
}
