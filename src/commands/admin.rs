mod channels;
mod greeting_channels;
mod log_channel;
mod user_groups;

use super::cesium::CESIUM_ROLE;
use channels::*;
use futures::stream::TryStreamExt;
use greeting_channels::*;
use log_channel::*;
use serenity::{
    all::EditMessage,
    framework::standard::{
        Args, CommandResult,
        macros::{command, group},
    },
    model::{channel::Message, id::ChannelId},
    prelude::*,
};
use std::{collections::HashSet, str};
use user_groups::*;

#[group]
#[commands(edit, say, tomada_de_posse)]
#[required_permissions(ADMINISTRATOR)]
#[prefixes("sudo")]
#[sub_groups(Channels, GreetingChannels, LogChannel, UserGroups)]
struct Admin;

#[command]
#[description("Sets the users that are now cesium")]
#[usage("[new line separated list of users]")]
#[min_args(1)]
pub async fn tomada_de_posse(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild_id = msg
        .guild_id
        .ok_or("must be in a guild to use this command")?;
    let users = &args
        .rest()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.trim())
        .collect::<HashSet<&str>>();

    guild_id
        .members_iter(ctx)
        .try_for_each(|m| async move {
            match (
                m.roles.contains(&CESIUM_ROLE),
                users.contains(m.user.name.as_str()),
            ) {
                (true, false) => {
                    m.remove_role(ctx, CESIUM_ROLE).await?;
                    msg.channel_id
                        .say(ctx, format!("❌ Removed from cesium: {}", m.user.name))
                        .await?;
                }
                (false, true) => {
                    m.add_role(ctx, CESIUM_ROLE).await?;
                    msg.channel_id
                        .say(ctx, format!("✅ Added to cesium: {}", m.user.name))
                        .await?;
                }
                (_, _) => {}
            }
            Ok(())
        })
        .await?;
    Ok(())
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
    message
        .edit(&ctx, EditMessage::new().content(args.rest()))
        .await?;
    Ok(())
}
