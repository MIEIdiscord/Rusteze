mod channels;
mod daemons;
mod greeting_channels;
mod log_channel;
mod minecraft;
mod user_groups;

use channels::*;
use daemons::*;
use futures::{future, stream::StreamExt};
use greeting_channels::*;
use log_channel::*;
use minecraft::*;
use once_cell::sync::Lazy;
use regex::Regex;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{
        channel::Message,
        id::{ChannelId, RoleId},
    },
    prelude::*,
};
use std::{os::unix::process::CommandExt, process::Command as Fork, str, time::Instant};
use user_groups::*;

#[group]
#[commands(member_count, edit, update, reboot, say, whitelist)]
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
#[description("Update the bot")]
pub async fn update(ctx: &Context, msg: &Message) -> CommandResult {
    static UPDATING: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
    let _ = match UPDATING.try_lock() {
        Ok(guard) => guard,
        Err(_) => return Err("Alreading updating".into()),
    };
    let check_msg = |mut m: Message| async move {
        let new_msg = format!("{} :white_check_mark:", m.content);
        m.edit(&ctx, |m| m.content(new_msg)).await
    };
    let message = msg.channel_id.say(&ctx, "Fetching...").await?;
    Fork::new("git").arg("fetch").spawn()?.wait()?;
    check_msg(message).await?;

    let message = msg.channel_id.say(&ctx, "Checking remote...").await?;
    let status = Fork::new("git")
        .args(&["rev-list", "--count", "master...master@{upstream}"])
        .output()?;
    check_msg(message).await?;

    if 0 == String::from_utf8_lossy(&status.stdout)
        .trim()
        .parse::<i32>()?
    {
        return Err("No updates!".into());
    }

    let message = msg.channel_id.say(&ctx, "Pulling from remote...").await?;
    let out = &Fork::new("git").arg("pull").output()?;
    if !out.status.success() {
        return Err(format!(
            "Error pulling!
```
============= stdout =============
{}
============= stderr =============
{}
```",
            str::from_utf8(&out.stdout)?,
            str::from_utf8(&out.stderr)?
        )
        .into());
    }
    check_msg(message).await?;

    let message = msg.channel_id.say(&ctx, "Compiling...").await?;
    let start = Instant::now();
    let out = &Fork::new("cargo")
        .args(if cfg!(debug_assertions) {
            &["build", "--quiet"][..]
        } else {
            &["build", "--quiet", "--release"][..]
        })
        .output()?;
    if !out.status.success() {
        return Err(format!(
            "Build Error!
```
============= stderr =============
{}
```",
            {
                let s = str::from_utf8(&out.stderr)?;
                &s[s.len().saturating_sub(1500)..]
            }
        )
        .into());
    }
    check_msg(message).await?;
    let elapsed = start.elapsed();
    msg.channel_id
        .say(
            &ctx,
            format!(
                "Compiled in {}m{}s",
                elapsed.as_secs() / 60,
                elapsed.as_secs() % 60
            ),
        )
        .await?;

    reboot_bot(ctx, msg.channel_id).await
}

#[command]
#[description("Reboot the bot")]
#[usage("")]
pub async fn reboot(ctx: &Context, msg: &Message) -> CommandResult {
    reboot_bot(ctx, msg.channel_id).await
}

async fn reboot_bot(ctx: &Context, ch_id: ChannelId) -> CommandResult {
    ch_id.say(ctx, "Rebooting...").await?;
    std::env::set_var("RUST_BACKTRACE", "1");
    let error = Fork::new("cargo")
        .args(&[
            "run",
            if cfg!(debug_assertions) {
                ""
            } else {
                "--release"
            },
            "--",
            "-r",
            &ch_id.to_string(),
        ])
        .exec();
    std::env::remove_var("RUST_BACKTRACE");
    Err(error.into())
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
