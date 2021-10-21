use crate::get;
use futures::future::TryFutureExt;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serenity::{
    framework::standard::{
        macros::{check, command, group},
        ArgError, Args, CommandOptions, CommandResult, Reason,
    },
    http::{CacheHttp, Http},
    model::{
        channel::{ChannelType, Message, PermissionOverwrite, PermissionOverwriteType},
        id::{ChannelId, GuildId, RoleId, UserId},
        misc::Mentionable,
        permissions::Permissions,
    },
    prelude::*,
};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{self, BufWriter},
    iter::once,
    sync::Arc,
};

#[group]
#[commands(add, join, remove)]
#[checks(is_mod_or_cesium)]
#[prefixes("cesium")]
struct Cesium;

const CESIUM_CATEGORY: ChannelId = ChannelId(418798551317872660);
pub const CESIUM_ROLE: RoleId = RoleId(418842665061318676);
const MODS_ROLE: RoleId = RoleId(618572138718298132);
const MENTOR_ROLE: RoleId = RoleId(688760837980291120);
const CHANNELS: &str = "cesium_channels.json";

#[check]
#[name = "is_mod_or_cesium"]
pub async fn is_mod_or_cesium(
    ctx: &Context,
    msg: &Message,
    _: &mut Args,
    _: &CommandOptions,
) -> Result<(), Reason> {
    let (m, gid) = match (&msg.member, msg.guild_id) {
        (Some(m), Some(g)) => (m, g),
        _ => return Err(Reason::User("Not in a guild".to_string())),
    };
    if [MENTOR_ROLE, CESIUM_ROLE, MODS_ROLE]
        .iter()
        .any(|r| m.roles.contains(r))
    {
        Ok(())
    } else if gid
        .member(&ctx, msg.author.id)
        .and_then(|u| async move { u.permissions(&ctx).await })
        .and_then(|p| async move { Ok(p.administrator()) })
        .await
        .unwrap_or(false)
    {
        Ok(())
    } else {
        Err(Reason::User(
            "You don't have permission to use that command!".to_string(),
        ))
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct ChannelMapping {
    last_number: u32,
    channels: HashMap<ChannelId, ChannelId>,
}

impl ChannelMapping {
    pub fn load() -> io::Result<Self> {
        Ok(serde_json::from_reader(File::open(CHANNELS)?)?)
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

    async fn create_channel<C, U>(&mut self, guild_id: GuildId, ctx: C, users: U) -> CommandResult
    where
        C: CacheHttp + AsRef<Http> + Copy,
        U: Iterator<Item = UserId>,
    {
        let user_ids = users.collect::<Vec<_>>();
        let users: Vec<_> = user_ids
            .iter()
            .map(|&u| PermissionOverwrite {
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
        let text = guild_id
            .create_channel(ctx, |channel| {
                channel
                    .name(format!("mentor-channel-{}", self.last_number))
                    .kind(ChannelType::Text)
                    .category(CESIUM_CATEGORY)
                    .permissions(users.iter().map(|p| p.clone()))
            })
            .await?;
        let voice = guild_id
            .create_channel(ctx, |channel| {
                channel
                    .name(format!("mentor-channel-{}", self.last_number))
                    .kind(ChannelType::Voice)
                    .category(CESIUM_CATEGORY)
                    .permissions(users.into_iter())
            })
            .await?;
        text.say(
            &ctx,
            format!(
                "Este canal e temporário e será apagado no fim das sessões.\n
Se quiserem guardar alguma coisa que aqui seja escrita façam-no o mais cedo possível.\n
Bem vindos aos vosso canto privado! {}",
                user_ids
                    .iter()
                    .format_with(" ", |u, f| f(&format_args!("{}", u.mention())))
            ),
        )
        .await?;
        self.last_number += 1;
        self.channels.insert(text.id, voice.id);
        self.write_channels()?;
        Ok(())
    }

    async fn delete_channel<C>(&mut self, channel_id: ChannelId, ctx: C) -> CommandResult
    where
        C: CacheHttp + AsRef<Http> + Copy,
    {
        let voice = channel_id
            .to_channel(ctx)
            .await?
            .guild()
            .filter(|ch| ch.category_id == Some(CESIUM_CATEGORY))
            .and_then(|_| self.channels.get(&channel_id))
            .ok_or("Invalid channel")?;
        channel_id.delete(ctx).await?;
        voice.delete(ctx).await?;
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
#[min_args(1)]
pub async fn add(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.ok_or("Message with no guild id")?;
    args.iter::<UserId>().try_for_each(|x| x.map(|_| ()))?;
    args.restore();
    get!(ctx, ChannelMapping, write)
        .create_channel(guild_id, &ctx, args.iter::<UserId>().map(Result::unwrap))
        .await?;
    msg.channel_id.say(&ctx, "Room created").await?;
    Ok(())
}

#[command]
#[description("Removes a new private room")]
#[usage("")]
pub async fn remove(ctx: &Context, msg: &Message) -> CommandResult {
    get!(ctx, ChannelMapping, write)
        .delete_channel(msg.channel_id, &ctx)
        .await
}

#[command]
#[description("Adds a student to a private room, the room is where the command is called or passed as a second parameter")]
#[usage("StudentMention [channel_mention]")]
#[min_args(1)]
pub async fn join(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let channels = get!(> data_lock, ChannelMapping, read);
    let user = args.single::<UserId>()?;
    let text = match args.single::<ChannelId>() {
        Ok(t) => t,
        Err(ArgError::Eos) => msg.channel_id,
        Err(e) => return Err(e.into()),
    };
    let voice = channels.get_channel(&text).ok_or(
        "Invalid channel, use this command in a #mentor-channel-* channel \
or mention the channel as a second parameter",
    )?;
    text.create_permission(
        &ctx,
        &PermissionOverwrite {
            kind: PermissionOverwriteType::Member(user),
            allow: Permissions::READ_MESSAGES,
            deny: Permissions::empty(),
        },
    )
    .await?;
    voice
        .create_permission(
            &ctx,
            &PermissionOverwrite {
                kind: PermissionOverwriteType::Member(user),
                allow: Permissions::READ_MESSAGES,
                deny: Permissions::empty(),
            },
        )
        .await?;
    msg.channel_id.say(&ctx, "User(s) added").await?;
    Ok(())
}
