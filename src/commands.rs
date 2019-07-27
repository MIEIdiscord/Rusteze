use crate::channels::{read_courses, MiEI};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::{
        channel::{Channel, Message},
        guild::Role,
        id::RoleId,
    },
    prelude::*,
};

#[command]
pub fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong but in rust!")?;
    Ok(())
}

#[command]
pub fn study(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let roles = read_courses()?;
    let ids = args
        .iter::<String>()
        .filter_map(Result::ok)
        .filter_map(|x| roles.get_role_id(&x))
        .collect::<Vec<RoleId>>();
    msg.member(&ctx.cache)
        .map(|mut x| x.add_roles(&ctx.http, ids.as_slice()));
    Ok(())
}

#[command]
pub fn unstudy(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let roles = read_courses()?;
    let ids = args
        .iter::<String>()
        .filter_map(Result::ok)
        .filter_map(|x| roles.get_role_id(&x))
        .collect::<Vec<RoleId>>();
    msg.member(&ctx.cache)
        .map(|mut x| x.remove_roles(&ctx.http, ids.as_slice()));
    Ok(())
}
