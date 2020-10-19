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
#[commands(greet_channel_set, greet_channel, greet_channel_clear)]
#[prefixes("greet")]
struct GreetingChannels;

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
    let mut config = share_map.get::<Config>().unwrap().write();
    config.set_greet_channel(channel_id, greeting)?;
    msg.channel_id.say(&ctx, "Greet channel set")?;
    Ok(())
}

#[command("clear")]
#[description("Disable the greeting channel")]
#[usage("")]
pub fn greet_channel_clear(ctx: &mut Context, msg: &Message) -> CommandResult {
    let share_map = ctx.data.read();
    let mut config = share_map.get::<Config>().unwrap().write();
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
        .read()
        .greet_channel()
    {
        Some(ch) => msg
            .channel_id
            .say(&ctx, format!("Greet channel: {}", ch.mention()))?,
        None => msg.channel_id.say(&ctx, "No greet channel")?,
    };
    Ok(())
}
