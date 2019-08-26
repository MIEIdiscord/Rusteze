use crate::channels::read_courses;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{channel::Message, id::RoleId},
    prelude::*,
};

group!({
    name: "study",
    options: {},
    commands: [ping, study, unstudy],
});

group!({
    name: "courses",
    options: {
        required_permissions: [ADMINISTRATOR],
        prefixes: ["courses"],
    },
    commands: [mk],
});

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
        .map(|(a, b)| {
            names.push(a);
            b
        })
        .collect::<Vec<RoleId>>();
    msg.member(&ctx.cache)
        .map(|mut x| x.add_roles(&ctx.http, ids.as_slice()))
        .transpose()?;

    if names.is_empty() {
        msg.channel_id
            .say(&ctx.http, "Não foste adicionado a nenhuma cadeira nova")?;
    } else {
        msg.channel_id
            .say(&ctx.http, format!("Studying {}", names.join(" ")))?;
    }
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
        .map(|(a, b)| {
            names.push(a);
            b
        })
        .collect::<Vec<RoleId>>();
    msg.member(&ctx.cache)
        .map(|mut x| x.remove_roles(&ctx.http, ids.as_slice()))
        .transpose()?;
    if names.is_empty() {
        msg.channel_id
            .say(&ctx.http, "Não foste removido de nenhuma cadeira")?;
    } else {
        msg.channel_id
            .say(&ctx.http, format!("Stoped Studying: {}", names.join(" ")))?;
    }
    Ok(())
}

#[command]
#[min_args(3)]
pub fn mk(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut roles = read_courses()?;
    let year = args.single::<String>();
    let semester = args.single::<String>();
    if let (Ok(y), Ok(s), Some(g)) = (year, semester, msg.guild_id) {
        let new_roles = args
            .raw()
            .skip(2)
            .filter_map(|x| roles.create_role(ctx, &y, &s, x, g))
            .collect::<Vec<&str>>();
        if new_roles.is_empty() {
            msg.channel_id
                .say(&ctx.http, "Não foram criadas novas cadeiras")?;
        } else {
            msg.channel_id.say(
                &ctx.http,
                format!("Cadeiras criadas: {}", new_roles.join(" ")),
            )?;
        }
    }
    Ok(())
}
