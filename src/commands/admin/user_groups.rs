use crate::{commands::usermod::*, config::Config};
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{channel::Message, id::RoleId},
    prelude::*,
};

#[group]
#[commands(add, remove, list, join, leave)]
#[required_permissions(ADMINISTRATOR)]
#[prefixes("usermod")]
struct UserGroups;

#[command("-i")]
#[description("Set a role as a user group")]
#[usage("RoleMention description")]
#[min_args(2)]
async fn add(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let role = args.single::<RoleId>()?;
    if !role_exists(ctx, msg.guild_id.ok_or("Not in a guild")?, role).await? {
        return Err("Role doesn't exist".into());
    }
    let desc = args.rest();
    ctx.data
        .write()
        .await
        .get_mut::<Config>()
        .expect("Config not loaded")
        .write()
        .await
        .add_user_group(role, desc.to_string())?;
    msg.channel_id.say(&ctx, "Role added").await?;
    Ok(())
}

#[command("-r")]
#[description("Unset a role as a user group")]
#[usage("RoleMention")]
#[min_args(1)]
async fn remove(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let role = args.single::<RoleId>()?;
    if !role_exists(ctx, msg.guild_id.ok_or("Not in a guild")?, role).await? {
        return Err("Role doesn't exist".into());
    }
    ctx.data
        .write()
        .await
        .get_mut::<Config>()
        .expect("Config not loaded")
        .write()
        .await
        .remove_user_group(role)?;
    msg.channel_id.say(&ctx, "Role removed").await?;
    Ok(())
}
