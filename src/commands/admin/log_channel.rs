use crate::config::Config;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{channel::Message, id::ChannelId},
    prelude::*,
};

#[group]
#[commands(log_channel, log_channel_set)]
#[prefixes("log")]
struct LogChannel;

#[command("get")]
#[description("Check the current log channel")]
#[usage("")]
pub async fn log_channel(ctx: &Context, msg: &Message) -> CommandResult {
    match ctx
        .data
        .read()
        .await
        .get::<Config>()
        .unwrap()
        .read()
        .await
        .log_channel()
    {
        Some(ch) => {
            msg.channel_id
                .say(&ctx, format!("Log channel: {}", ch.mention()))
                .await?
        }
        None => msg.channel_id.say(&ctx, "No log channel").await?,
    };
    Ok(())
}

#[command("set")]
#[description("Set the logging channel")]
#[usage("#channel_mention")]
pub async fn log_channel_set(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>().ok();
    let share_map = ctx.data.read().await;
    let mut config = share_map.get::<Config>().unwrap().write().await;
    config.set_log_channel(channel_id)?;
    msg.channel_id
        .say(
            &ctx,
            if channel_id.is_some() {
                "Log channel set"
            } else {
                "Log channel disabled"
            },
        )
        .await?;
    Ok(())
}
