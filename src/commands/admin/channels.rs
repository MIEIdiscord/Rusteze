//! Channels where the bot will respond to commands

use crate::{config::Config, get};
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
pub struct Channels;

#[command]
#[description("Adds an allowed channel")]
#[usage("#channel_mention")]
#[min_args(1)]
pub async fn add(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>()?;
    get!(ctx, Config, write).add_allowed_channel(channel_id)?;
    msg.channel_id.say(&ctx, "Channel added").await?;
    Ok(())
}

#[command]
#[description("Lists all the allowed channels")]
#[usage("")]
pub async fn list(ctx: &Context, msg: &Message) -> CommandResult {
    let share_map = ctx.data.read().await;
    let config = share_map.get::<Config>().unwrap().write().await;
    msg.channel_id
        .say(
            &ctx,
            format!(
                "Allowed Channels: {}",
                config.allowed_channels().map(|c| c.mention()).format(", ")
            ),
        )
        .await?;
    Ok(())
}

#[command]
#[description("Removes an allowed channel")]
#[usage("#channel_mention")]
#[min_args(1)]
pub async fn del(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>()?;
    get!(ctx, Config, write).remove_allowed_channel(channel_id)?;
    msg.channel_id.say(&ctx, "Channel removed").await?;
    Ok(())
}
