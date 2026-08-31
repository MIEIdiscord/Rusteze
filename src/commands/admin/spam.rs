use crate::{config::Config, get};
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::channel::Message,
    prelude::*,
};

#[group]
#[commands(spam_get, spam_set)]
#[prefixes("spam")]
struct Spam;

#[command("get")]
#[description("Check whether automatic spam detection is enabled")]
#[usage("")]
pub async fn spam_get(ctx: &Context, msg: &Message) -> CommandResult {
    let enabled = get!(ctx, Config, read).spam_detection();
    msg.channel_id
        .say(
            &ctx,
            if enabled {
                "Spam detection is **enabled**"
            } else {
                "Spam detection is **disabled**"
            },
        )
        .await?;
    Ok(())
}

#[command("set")]
#[description("Enable or disable automatic spam detection")]
#[usage("on|off")]
#[min_args(1)]
pub async fn spam_set(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let arg = args.single::<String>()?;
    let enabled = match arg.to_lowercase().as_str() {
        "on" | "true" | "enable" | "enabled" | "yes" => true,
        "off" | "false" | "disable" | "disabled" | "no" => false,
        _ => return Err("Use `on` or `off`".into()),
    };
    get!(ctx, Config, write).set_spam_detection(enabled)?;
    msg.channel_id
        .say(
            &ctx,
            if enabled {
                "Spam detection enabled"
            } else {
                "Spam detection disabled"
            },
        )
        .await?;
    Ok(())
}
