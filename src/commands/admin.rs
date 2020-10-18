use crate::{util::minecraft_server_get, config::Config};
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use serenity::{
    framework::standard::{
        macros::{command, group},
        ArgError, Args, CommandResult,
    },
    model::{
        channel::Message,
        id::{ChannelId, UserId},
    },
    prelude::*,
};
use std::{
    os::unix::process::CommandExt,
    process::Command as Fork,
    str,
    sync::{Mutex, TryLockError},
};

#[group]
#[commands(edit, update, say, whitelist)]
#[required_permissions(ADMINISTRATOR)]
#[prefixes("sudo")]
#[sub_groups(Channels, GreetingChannels, LogChannel)]
struct Admin;

#[group]
#[commands(del, add, list)]
#[prefixes("ch","channel")]
struct Channels;

#[group]
#[commands(greet_channel_set, greet_channel, greet_channel_clear)]
#[prefixes("greet")]
struct GreetingChannels;

#[group]
#[commands(log_channel, log_channel_set)]
#[prefixes("log")]
struct LogChannel;

#[group]
#[commands(server_do, pair, pair_guild_set)]
#[required_permissions(ADMINISTRATOR)]
#[default_command(server_do)]
#[prefixes("mc")]
struct Minecraft;

#[group]
#[commands(daemon_now, daemon_list)]
#[required_permissions(ADMINISTRATOR)]
#[prefixes("daemons", "deamons")]
struct Daemons;

#[command]
#[description("Whitelists a player in the minecraft server")]
#[usage("name")]
#[usage("name uuid")]
#[aliases("wl")]
#[min_args(1)]
pub fn whitelist(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
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
        eprintln!(
            "WHITELIST COMMAND LOG:\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout, stderr
        );
        msg.channel_id
            .say(&ctx, "Whitelist changed and reloaded!")?;
        Ok(())
    } else {
        msg.channel_id.say(&ctx, "Whitelist change failed:")?;
        let mut stdout = stdout;
        stdout += stderr;
        Err(stdout.into())
    }
}

#[command]
#[description("Run a command as op on the server")]
#[usage("command 1 ; command 2 ; ...")]
#[min_args(1)]
fn server_do(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    for command in args.rest().split(";") {
        let output = minecraft_server_get(&[command.trim()])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        msg.channel_id
            .say(&ctx, format!("`{}`: {}{}", command, stdout, stderr))?;
    }
    Ok(())
}

#[command]
#[description("Associate a minecraft username with the discord's username")]
#[usage("minecraft_nickname @mention")]
#[min_args(2)]
fn pair(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let nick = args.single::<String>()?;
    let user = args.single::<UserId>()?;
    let share_map = ctx.data.write();
    share_map
        .get::<crate::daemons::minecraft::Minecraft>()
        .unwrap()
        .write()
        .unwrap()
        .pair(nick, user)?;
    msg.channel_id.say(&ctx, "User paired")?;
    Ok(())
}

#[command]
#[description("Set's this guild as the one to use for the minecraft daemon")]
#[usage("")]
fn pair_guild_set(ctx: &mut Context, msg: &Message) -> CommandResult {
    let share_map = ctx.data.write();
    match msg.guild_id {
        Some(gid) => {
            share_map
                .get::<crate::daemons::minecraft::Minecraft>()
                .unwrap()
                .write()
                .unwrap()
                .set_guild_id(gid)?;
            msg.channel_id.say(&ctx, "Guild id set")?
        }
        None => msg.channel_id.say(&ctx, "Couldn't find guild id")?,
    };
    Ok(())
}

#[command("list")]
#[description("List current daemons")]
#[usage("")]
fn daemon_list(ctx: &mut Context, msg: &Message) -> CommandResult {
    let share_map = ctx.data.read();
    msg.channel_id.say(
        &ctx,
        format!(
            "{:?}",
            share_map
                .get::<crate::daemons::DaemonThread>()
                .unwrap()
                .list
        ),
    )?;
    Ok(())
}

#[command("now")]
#[description("Runs all or one daemon now")]
#[usage("[number]")]
fn daemon_now(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let share_map = ctx.data.read();
    let daemon_t = share_map.get::<crate::daemons::DaemonThread>().unwrap();
    match args.single::<usize>() {
        Ok(u) if u < daemon_t.list.len() => daemon_t.run_one(u)?,
        Ok(_) => return Err("Index out of bounds".into()),
        Err(ArgError::Eos) => daemon_t.run_all()?,
        Err(_) => return Err("Invalid index".into()),
    }
    msg.channel_id.say(&ctx, "Done")?;
    Ok(())
}

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
#[description("Set the channel where the greet will be sent and optionaly which message to show")]
#[usage("#channel_mention [Message]")]
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

#[command("get")]
#[description("Check the current log channel")]
#[usage("")]
pub fn log_channel(ctx: &mut Context, msg: &Message) -> CommandResult {
    match ctx
        .data
        .read()
        .get::<Config>()
        .unwrap()
        .read()?
        .log_channel()
    {
        Some(ch) => msg
            .channel_id
            .say(&ctx, format!("Log channel: {}", ch.mention()))?,
        None => msg.channel_id.say(&ctx, "No log channel")?,
    };
    Ok(())
}

#[command("set")]
#[description("Set the logging channel")]
#[usage("#channel_mention")]
pub fn log_channel_set(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>().ok();
    let share_map = ctx.data.read();
    let mut config = share_map.get::<Config>().unwrap().write()?;
    config.set_log_channel(channel_id)?;
    msg.channel_id.say(
        &ctx,
        if channel_id.is_some() {
            "Log channel set"
        } else {
            "Log channel disabled"
        },
    )?;
    Ok(())
}
