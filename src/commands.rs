use crate::channels::{read_courses};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::{
        channel::{Message},
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
pub fn study(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let roles = read_courses()?;
    let mut names = Vec::new();
    let ids = args
        .raw()
        .map(|x| roles.get_role_id(x))
        .flatten()
        .filter(|(_, b)| {
            msg.author
                .has_role(&ctx, msg.guild_id.unwrap(), b)
                .map(|x| !x)
                .unwrap_or(false)
        })
        .map(|(a,b)| {names.push(a); b})
        .collect::<Vec<RoleId>>();
    msg.member(&ctx.cache)
        .map(|mut x| x.add_roles(&ctx.http, ids.as_slice()))
        .transpose()?;

    msg.channel_id
        .say(&ctx.http, format!("Studying {}", names.join(" ")))?;
    Ok(())
}

#[command]
pub fn unstudy(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let roles = read_courses()?;
    let mut names = Vec::new();
    let ids = args
        .raw()
        .map(|x| roles.get_role_id(x))
        .flatten()
        .map(|(a,b)| {names.push(a); b})
        .collect::<Vec<RoleId>>();
    msg.member(&ctx.cache)
        .map(|mut x| x.remove_roles(&ctx.http, ids.as_slice()))
        .transpose()?;

    msg.channel_id.say(&ctx.http, format!("Stoped Studying: {}", names.join(" ")))?;
    Ok(())
}
