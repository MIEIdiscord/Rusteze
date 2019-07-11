use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::{
        channel::{Channel, Message},
        guild::{Role},
        id::{RoleId},
    },
    prelude::*,
};
use crate::channels::{read_courses, MiEI}; 
use std::sync::Arc;
#[command]
pub fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong but in rust!")?;
    Ok(())
}

#[command]
pub fn study(ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut p: Vec<&str> = msg.content.split_whitespace().collect();
    p.remove(0);
    let roles = read_courses()?;
    let ids = p.iter()
        .map(|x| roles.get_role_id(x))
        .filter(|x| x == "")
        .map(|x| x.parse::<RoleId>().unwrap())
        .collect::<Vec<RoleId>>();
    msg.member(&ctx.cache)
        .map(|mut x| x.add_roles(&ctx.http, ids.as_slice()));
        Ok(())
}

#[command]
pub fn unstudy(ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut p: Vec<&str> = msg.content.split_whitespace().collect();
    p.remove(0);
    let roles = read_courses()?;
    let ids = p.iter()
        .map(|x| roles.get_role_id(x))
        .filter(|x| x == "")
        .map(|x| x.parse::<RoleId>().unwrap())
        .collect::<Vec<RoleId>>();
    msg.member(&ctx.cache)
        .map(|mut x| x.remove_roles(&ctx.http, ids.as_slice()));
        Ok(())
}

#[command]
pub fn mkcourse(ctx: &mut Context, msg: &Message) -> CommandResult {
    Ok(())
}
