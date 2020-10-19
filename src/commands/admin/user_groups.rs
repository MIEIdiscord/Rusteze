use crate::config::Config;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{channel::Message, id::RoleId},
    prelude::*,
};

#[group]
#[commands(add, remove)]
#[required_permissions(ADMINISTRATOR)]
#[prefixes("usermod")]
struct UserGroups;

#[command("-i")]
#[description("Set a role as a user group")]
#[usage("RoleMention description")]
#[min_args(2)]
fn add(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let role = args.single::<RoleId>()?;
    let desc = args.rest();
    ctx.data
        .write()
        .get_mut::<Config>()
        .expect("Config not loaded")
        .write()
        .add_user_group(role, desc.to_string())?;
    msg.channel_id.say(&ctx, "Role added")?;
    Ok(())
}

#[command("-r")]
#[description("Unset a role as a user group")]
#[usage("RoleMention")]
#[min_args(1)]
fn remove(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let role = args.single::<RoleId>()?;
    ctx.data
        .write()
        .get_mut::<Config>()
        .expect("Config not loaded")
        .write()
        .remove_user_group(role)?;
    msg.channel_id.say(&ctx, "Role removed")?;
    Ok(())
}
