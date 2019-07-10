use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::{
        channel::{Channel, Message},
        guild::{Role},
        id::{RoleId},
    },
    prelude::*,
};
use std::sync::Arc;
#[command]
pub fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong but in rust!")?;
    Ok(())
}

#[command]
pub fn study(ctx: &mut Context, msg: &Message) -> CommandResult {
    let o = &mut msg.guild(&ctx.cache).unwrap();
    let guild = Arc::get_mut(o).unwrap().read();
    let mut p: Vec<&str> = msg.content.split_whitespace().collect();
    p.remove(0);
    msg.member(&ctx.cache)
        .map(|mut z| z.add_roles(&ctx.http, 
                                 p.iter()
                                    .map(|x| guild.role_by_name(x))
                                    .filter_map(|x| x.map(|o| RoleId::from(o)))
                                        .collect::<Vec<RoleId>>()
                                        .as_slice()));
        Ok(())
}

#[command]
pub fn unstudy(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.member(&ctx.cache).map(|mut x| x.remove_roles(&ctx.http, &msg.mention_roles));
    Ok(())
}
