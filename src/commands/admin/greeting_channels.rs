use crate::{config::Config, get};
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
pub async fn greet_channel_set(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>()?;
    let greeting = Some(args.rest()).and_then(|m| {
        if m.is_empty() {
            None
        } else {
            Some(m.to_string())
        }
    });
    get!(ctx, Config, write).set_greet_channel(channel_id, greeting)?;
    msg.channel_id.say(&ctx, "Greet channel set").await?;
    Ok(())
}

#[command("clear")]
#[description("Disable the greeting channel")]
#[usage("")]
pub async fn greet_channel_clear(ctx: &Context, msg: &Message) -> CommandResult {
    get!(ctx, Config, write).remove_greet_channel()?;
    msg.channel_id.say(&ctx, "Greet channel cleared").await?;
    Ok(())
}

#[command("get")]
#[description("Check the current greet channel")]
#[usage("")]
pub async fn greet_channel(ctx: &Context, msg: &Message) -> CommandResult {
    match get!(ctx, Config, read).greet_channel() {
        Some(ch) => {
            msg.channel_id
                .say(&ctx, format!("Greet channel: {}", ch.mention()))
                .await?
        }
        None => msg.channel_id.say(&ctx, "No greet channel").await?,
    };
    Ok(())
}
