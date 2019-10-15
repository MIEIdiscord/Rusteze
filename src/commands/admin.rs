use crate::config::Config;
use itertools::Itertools;
use once_cell::sync::Lazy;
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
use std::sync::{Mutex, TryLockError};

group!({
    name: "Admin",
    options: {
        required_permissions: [ADMINISTRATOR],
        prefixes: ["sudo"],
    },
    commands: [edit, update, say],
    sub_groups: [CHANNELS, GREETING_CHANNELS],
});

group!({
    name: "Channels",
    options: {
        prefixes: ["ch", "channel"]
    },
    commands: [del, add, list],
});

group!({
    name: "greeting_channels",
    options: {
        prefixes: ["greet"]
    },
    commands: [greet_channel_set, greet_channel_clear, greet_channel]
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
    static UPDATING: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
    let _ = match UPDATING.try_lock() {
        Err(TryLockError::WouldBlock) => return Err("Alreading updating".into()),
        Err(TryLockError::Poisoned(p)) => return Err(p.into()),
        Ok(guard) => guard,
    };
    let check_msg = |mut m: Message| {
        let new_msg = format!("{} :white_check_mark:", m.content);
        m.edit(&ctx, |m| m.content(new_msg))
    };
    let message = msg.channel_id.say(&ctx, "Fetching...")?;
    Fork::new("git").arg("fetch").spawn()?.wait()?;
    check_msg(message)?;

    let message = msg.channel_id.say(&ctx, "Checking remote...")?;
    let status = Fork::new("git")
        .args(&["rev-list", "--count", "master...master@{upstream}"])
        .output()?;
    check_msg(message)?;

    if 0 == String::from_utf8_lossy(&status.stdout)
        .trim()
        .parse::<i32>()?
    {
        return Err("No updates!".into());
    }

    let message = msg.channel_id.say(&ctx, "Pulling from remote...")?;
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
    check_msg(message)?;

    let message = msg.channel_id.say(&ctx, "Compiling...")?;
    let out = &Fork::new("cargo").args(&["build", "--release"]).output()?;
    if !out.status.success() {
        return Err(format!(
            "Build Error!
            ```
            ============= stderr =============
            {}
            ```",
            {
                let s = str::from_utf8(&out.stderr)?;
                &s[s.len() - 1500..]
            }
        )
        .into());
    }
    check_msg(message)?;

    msg.channel_id.say(ctx, "Rebooting...")?;
    std::env::set_var("RUST_BACKTRACE", "1");
    let error = Fork::new("cargo")
        .args(&["run", "--release", "--", "-r", &msg.channel_id.to_string()])
        .exec();
    std::env::remove_var("RUST_BACKTRACE");
    Err(error.into())
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
#[min_args(3)]
pub fn edit(ctx: &mut Context, _msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>()?;
    let msg_id = args.single::<u64>()?;
    let mut message = channel_id.message(&ctx.http, msg_id)?;
    message.edit(&ctx, |c| c.content(args.rest()))?;
    Ok(())
}

#[command("set")]
#[description("Set the channel where the greet will be sent")]
#[usage("#channel_mention")]
#[min_args(1)]
pub fn greet_channel_set(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>()?;
    let greeting = Some(args.rest()).and_then(|m| {
        if m.is_empty() {
            None
        } else {
            Some(m.to_string())
        }
    });
    let share_map = ctx.data.read();
    let mut config = share_map.get::<Config>().unwrap().write()?;
    config.set_greet_channel(channel_id, greeting)?;
    msg.channel_id.say(&ctx, "Greet channel set")?;
    Ok(())
}

#[command("clear")]
#[description("Disable the greeting channel")]
#[usage("")]
pub fn greet_channel_clear(ctx: &mut Context, msg: &Message) -> CommandResult {
    let share_map = ctx.data.read();
    let mut config = share_map.get::<Config>().unwrap().write()?;
    config.remove_greet_channel()?;
    msg.channel_id.say(&ctx, "Greet channel cleared")?;
    Ok(())
}

#[command("get")]
#[description("Check the current greet channel")]
#[usage("")]
pub fn greet_channel(ctx: &mut Context, msg: &Message) -> CommandResult {
    match ctx
        .data
        .read()
        .get::<Config>()
        .unwrap()
        .read()?
        .greet_channel()
    {
        Some(ch) => msg
            .channel_id
            .say(&ctx, format!("Greet channel: {}", ch.mention()))?,
        None => msg.channel_id.say(&ctx, "No greet channel")?,
    };
    Ok(())
}
