use itertools::Itertools;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{channel::Message, id::ChannelId},
    prelude::*,
};

use std::os::unix::process::CommandExt;
use std::process::Command as Fork;
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::Config;

static UPDATING: AtomicBool = AtomicBool::new(false);

group!({
    name: "Admin",
    options: {
        required_permissions: [ADMINISTRATOR],
        prefixes: ["sudo"],
    },
    commands: [edit, update, say],
    sub_groups: [CHANNELS],
});

group!({
    name: "Channels",
    options: {
        prefixes: ["ch", "channel"]
    },
    commands: [del, add, list],
});

#[command]
#[description("Adds an allowed channel")]
#[usage("#channel_mention")]
#[min_args(1)]
pub fn add(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>()?;
    let share_map = ctx.data.read();
    let mut config = share_map.get::<Config>().unwrap().write().unwrap();
    config.add_allowed_channel(channel_id)?;
    msg.channel_id.say(&ctx, "Channel added")?;
    Ok(())
}

#[command]
#[description("Lists all the allowed channels")]
#[usage("")]
pub fn list(ctx: &mut Context, msg: &Message) -> CommandResult {
    let share_map = ctx.data.read();
    let config = share_map.get::<Config>().unwrap().write().unwrap();
    msg.channel_id.say(
        &ctx,
        format!(
            "Allowed Channels: {}",
            config.allowed_channels().map(|c| c.mention()).format(", ")
        ),
    )?;
    Ok(())
}

#[command]
#[description("Removes an allowed channel")]
#[usage("#channel_mention")]
#[min_args(1)]
pub fn del(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>()?;
    let share_map = ctx.data.read();
    let mut config = share_map.get::<Config>().unwrap().write().unwrap();
    config.remove_allowed_channel(channel_id)?;
    msg.channel_id.say(&ctx, "Channel removed")?;
    Ok(())
}

#[command]
#[description("Update the bot")]
pub fn update(ctx: &mut Context, msg: &Message) -> CommandResult {
    if UPDATING.load(Ordering::SeqCst) {
        Err("Alreading updating")?;
    } else {
        UPDATING.store(true, Ordering::SeqCst);
    }
    msg.channel_id.say(&ctx, "Fetching...")?;
    Fork::new("git").arg("fetch").spawn()?.wait()?;

    msg.channel_id.say(&ctx, "Checking remote...")?;
    let status = Fork::new("git")
        .args(&["rev-list", "--count", "master...master@{upstream}"])
        .output()?;
    if let 0 = String::from_utf8_lossy(&status.stdout)
        .trim()
        .parse::<i32>()?
    {
        Err("No updates!".to_string())?;
    }

    msg.channel_id.say(&ctx, "Pulling from remote...")?;
    match &Fork::new("git").arg("pull").output()? {
        out if !out.status.success() => Err(format!(
            "Error pulling!
            ```
            ============= stdout =============
            {}
            ============= stderr =============
            {}
            ```",
            str::from_utf8(&out.stdout)?,
            str::from_utf8(&out.stderr)?
        ))?,
        _ => (),
    }

    msg.channel_id.say(&ctx, "Compiling...")?;
    match &Fork::new("cargo").args(&["build", "--release"]).output()? {
        out if !out.status.success() => Err(format!(
            "Build Error!
            ```
            {}
            ```",
            str::from_utf8(&out.stderr)?
        ))?,
        _ => (),
    }

    msg.channel_id.say(ctx, "Rebooting...")?;
    Err(Fork::new("cargo")
        .args(&["run", "--release", "--", "-r", &msg.channel_id.to_string()])
        .exec())?;
    Ok(())
}

#[command]
#[description("Make the bot send a message to a specific channel")]
#[usage("#channel_mention message")]
#[min_args(2)]
pub fn say(ctx: &mut Context, _msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>()?;
    channel_id.say(&ctx.http, args.rest())?;
    Ok(())
}

#[command]
#[description("Edit a message sent by the bot")]
#[usage("#channel_mention #message_id message")]
#[min_args(2)]
pub fn edit(ctx: &mut Context, _msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>()?;
    let msg_id = args.single::<u64>()?;
    let mut message = channel_id.message(&ctx.http, msg_id)?;
    message.edit(&ctx, |c| c.content(args.rest()))?;
    Ok(())
}
