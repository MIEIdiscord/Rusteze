use crate::config::Config;
use itertools::Itertools;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{channel::Message, id::ChannelId},
    prelude::*,
};

#[group]
#[commands(del, add, list)]
#[prefixes("ch", "channel")]
struct Channels;

#[command]
#[description("Adds an allowed channel")]
#[usage("#channel_mention")]
#[min_args(1)]
pub fn add(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>()?;
    let share_map = ctx.data.read();
    let mut config = share_map.get::<Config>().unwrap().write();
    config.add_allowed_channel(channel_id)?;
    msg.channel_id.say(&ctx, "Channel added")?;
    Ok(())
}

#[command]
#[description("Lists all the allowed channels")]
#[usage("")]
pub fn list(ctx: &mut Context, msg: &Message) -> CommandResult {
    let share_map = ctx.data.read();
    let config = share_map.get::<Config>().unwrap().write();
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
    let mut config = share_map.get::<Config>().unwrap().write();
    config.remove_allowed_channel(channel_id)?;
    msg.channel_id.say(&ctx, "Channel removed")?;
    Ok(())
}
