mod channels;
mod daemons;
mod greeting_channels;
mod log_channel;
mod minecraft;
mod user_groups;

use self::daemons::*;
use super::cesium::CESIUM_ROLE;
use crate::{
    config::Config,
    delayed_tasks::{Task, TaskSender},
    get,
    util::Endpoint,
};
use channels::*;
use chrono::{DateTime, Duration, Utc};
use futures::{
    future::{self, TryFutureExt},
    stream::{StreamExt, TryStreamExt},
};
use greeting_channels::*;
use log_channel::*;
use minecraft::*;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serenity::{
    framework::standard::{
        Args, CommandResult,
        macros::{command, group},
    },
    model::{
        channel::Message,
        id::{ChannelId, GuildId, RoleId, UserId},
    },
    prelude::*,
};
use std::{any::Any, collections::HashSet, process::Command as Fork, str};
use user_groups::*;

#[group]
#[commands(
    member_count,
    edit,
    say,
    whitelist,
    mute,
    set_mute_role,
    tomada_de_posse
)]
#[required_permissions(ADMINISTRATOR)]
#[prefixes("sudo")]
#[sub_groups(Channels, GreetingChannels, LogChannel, Minecraft, Daemons, UserGroups)]
struct Admin;

#[command]
#[description("Whitelists a player in the minecraft server")]
#[usage("name")]
#[usage("name uuid")]
#[aliases("wl")]
#[min_args(1)]
pub async fn whitelist(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    static UUID: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?x)^
            [A-Za-z0-9]{8}-
            [A-Za-z0-9]{4}-
            [A-Za-z0-9]{4}-
            [A-Za-z0-9]{4}-
            [A-Za-z0-9]{12}
            $",
        )
        .unwrap()
    });
    let mut args = args.raw();
    let name = args.next().expect("Min args 1");
    let fork_args = match args.next() {
        Some(uuid) if UUID.is_match(uuid) => vec![name, uuid],
        Some(_) => return Err("Invalid uuid".into()),
        None => vec![name],
    };
    let output = Fork::new("./whitelist.sh").args(fork_args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        crate::log!(
            "WHITELIST COMMAND LOG:\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout,
            stderr
        );
        msg.channel_id
            .say(&ctx, "Whitelist changed and reloaded!")
            .await?;
        Ok(())
    } else {
        msg.channel_id.say(&ctx, "Whitelist change failed:").await?;
        let mut stdout = stdout;
        stdout += stderr;
        Err(stdout.into())
    }
}

#[command]
#[description("Sets the users that are now cesium")]
#[usage("[new line separated list of users]")]
#[min_args(1)]
pub async fn tomada_de_posse(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild_id = msg
        .guild_id
        .ok_or("must be in a guild to use this command")?;
    let users = &args
        .rest()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.trim())
        .collect::<HashSet<&str>>();

    guild_id
        .members_iter(ctx)
        .try_for_each(|mut m| async move {
            match (
                m.roles.contains(&CESIUM_ROLE),
                users.contains(m.user.name.as_str()),
            ) {
                (true, false) => {
                    m.remove_role(ctx, CESIUM_ROLE).await?;
                    msg.channel_id
                        .say(ctx, format!("❌ Removed from cesium: {}", m.user.name))
                        .await?;
                }
                (false, true) => {
                    m.add_role(ctx, CESIUM_ROLE).await?;
                    msg.channel_id
                        .say(ctx, format!("✅ Added to cesium: {}", m.user.name))
                        .await?;
                }
                (_, _) => {}
            }
            Ok(())
        })
        .await?;
    Ok(())
}

#[command]
#[description("Make the bot send a message to a specific channel")]
#[usage("#channel_mention message")]
#[min_args(2)]
pub async fn say(ctx: &Context, _msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>()?;
    channel_id.say(&ctx.http, args.rest()).await?;
    Ok(())
}

#[command]
#[description("Edit a message sent by the bot")]
#[usage("#channel_mention #message_id message")]
#[min_args(3)]
pub async fn edit(ctx: &Context, _msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>()?;
    let msg_id = args.single::<u64>()?;
    let mut message = channel_id.message(&ctx.http, msg_id).await?;
    message.edit(&ctx, |c| c.content(args.rest())).await?;
    Ok(())
}

#[command]
#[description("Count the number of members with a role")]
#[usage("#role_mention")]
#[min_args(1)]
pub async fn member_count(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let role = &args.single::<RoleId>()?;
    let member_count = msg
        .guild_id
        .ok_or_else(|| String::from("Not in a guild"))?
        .members_iter(&ctx)
        .filter_map(|x| future::ready(x.ok()))
        .filter(|m| future::ready(m.roles.contains(role)))
        .fold(0_usize, |a, _| future::ready(a + 1))
        .await;
    msg.channel_id
        .say(&ctx, format!("Role has {} members", member_count))
        .await?;
    Ok(())
}

#[command]
#[description("Mute a user for 12h or the specified time")]
#[usage("@user [time] [h|hours|m|minutes|s|seconds|d|days] [reason]")]
#[min_args(1)]
pub async fn mute(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    type UnitMapper = (&'static str, fn(t: i64) -> Duration);
    fn pick_unit(s: &str) -> Option<UnitMapper> {
        match s {
            "d" | "days" => Some(("days", Duration::days)),
            "h" | "hours" | "" => Some(("hours", Duration::hours)),
            "m" | "minutes" => Some(("minutes", Duration::minutes)),
            "s" | "seconds" => Some(("seconds", Duration::seconds)),
            _ => None,
        }
    }
    let guild = msg.guild_id.ok_or("Not in a guild")?;
    let user = args.single::<UserId>()?;
    let (muted_hours, unit_str, unit, reason) = match args.single::<String>() {
        Ok(mut a) => match a.parse::<u32>() {
            Ok(muted_hours) => {
                let (unit_str, unit) = match args.single::<String>() {
                    Ok(time_spec) => pick_unit(&time_spec).ok_or("invalid time unit")?,
                    Err(_) => ("h", Duration::hours as _),
                };
                (muted_hours, unit_str, unit, String::from(args.rest()))
            }
            Err(_) => {
                a += " ";
                a += args.rest();
                (12, "h", Duration::hours as _, a)
            }
        },
        Err(_) => (12, "h", Duration::hours as _, String::new()),
    };
    let mut member = guild.member(ctx, user).await?;
    msg.channel_id
        .say(
            ctx,
            format!(
            "User {} will be muted for {} {} with reason \"{}\"\n\n**Reply with yes to proceed.**",
                member.mention(),
                muted_hours,
                unit_str,
                reason
        ),
        )
        .await?;
    let reply = msg
        .channel_id
        .await_reply(ctx)
        .author_id(msg.author.id)
        .timeout(Duration::minutes(10).to_std().unwrap());
    let reply = match reply.await {
        Some(m) => m,
        None => {
            msg.channel_id
                .say(ctx, "No reply found in time, aborting")
                .await?;
            return Ok(());
        }
    };

    if reply.content != "yes" {
        msg.channel_id.say(ctx, "Aborting").await?;
        return Ok(());
    }

    let mute_role = get!(ctx, Config, read)
        .get_mute_role()
        .ok_or("Mute role not set")?;
    member.add_role(ctx, mute_role).await?;

    let unmute_task = Box::new(Unmute {
        when: Utc::now() + unit(muted_hours.into()),
        guild_id: member.guild_id,
        user_id: member.user.id,
        role_id: mute_role,
    });
    if get!(ctx, TaskSender).send(unmute_task).await.is_err() {
        msg.channel_id
            .say(&ctx, "Failed to set unmute timeout.")
            .await?;
    }
    member
        .user
        .dm(&ctx, |m| {
            m.content(format!(
                "You've been muted for {} {}. {}",
                muted_hours, unit_str, reason
            ))
        })
        .await?;
    msg.channel_id.say(&ctx, "muted.").await?;
    Ok(())
}

#[command]
#[description("Sets the mute role")]
#[usage("@role")]
#[min_args(1)]
pub async fn set_mute_role(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let role = args.single::<RoleId>()?;
    get!(ctx, Config, write).set_mute_role(role)?;
    msg.channel_id.say(ctx, "Mute role set").await?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Unmute {
    when: DateTime<Utc>,
    guild_id: GuildId,
    user_id: UserId,
    role_id: RoleId,
}

#[serenity::async_trait]
#[typetag::serde]
impl Task for Unmute {
    fn when(&self) -> DateTime<Utc> {
        self.when
    }

    async fn call(&mut self, user_data: &mut TypeMap) -> Result<(), Box<dyn std::error::Error>> {
        crate::log!("Unmuting {}", self.user_id);
        let uid = self.user_id;
        if let Some(http) = user_data.get::<Endpoint>() {
            self.guild_id
                .member(http, self.user_id)
                .and_then(|mut m| async move { m.remove_role(http, self.role_id).await })
                .await?;
            crate::log!("Umuted {}", uid);
        }
        Ok(())
    }

    fn is_diferent(&self, other: &dyn Any) -> bool {
        if let Some(unmute) = other.downcast_ref::<Self>() {
            unmute.user_id != self.user_id
        } else {
            true
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
