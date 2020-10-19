use crate::config::Config;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{
        channel::Message,
        id::{GuildId, RoleId},
    },
    prelude::*,
};

#[group]
#[prefixes("usermod")]
#[commands(join, leave, list)]
struct UserMod;

#[command("-a")]
#[description("Join a role")]
#[usage("role_name")]
#[min_args(1)]
pub fn join(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let request = args.rest();
    let role = match role_by_name(ctx, msg.guild_id.ok_or("Not in a server")?, request)? {
        Some(role) => role,
        None => return Err("No such role".into()),
    };
    if ctx
        .data
        .read()
        .get::<Config>()
        .expect("Config not loaded")
        .read()
        .user_group_exists(role)
    {
        match msg
            .member(&ctx)
            .filter(|m| !m.roles.contains(&role))
            .map(|mut m| m.add_role(&ctx, role))
            .transpose()?
        {
            Some(_) => msg.channel_id.say(&ctx, "User group added")?,
            None => msg.channel_id.say(&ctx, "No user group added")?,
        };
    } else {
        msg.channel_id.say(&ctx, "That role is not a user group")?;
    }
    Ok(())
}

#[command("-d")]
#[description("Leave a role")]
#[usage("role_name")]
pub fn leave(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let request = args.rest();
    let role = match role_by_name(ctx, msg.guild_id.ok_or("Not in a server")?, request)? {
        Some(role) => role,
        None => return Err("No such role".into()),
    };
    if ctx
        .data
        .read()
        .get::<Config>()
        .expect("Config not loaded")
        .read()
        .user_group_exists(role)
    {
        match msg
            .member(&ctx)
            .filter(|m| m.roles.contains(&role))
            .map(|mut m| m.remove_role(&ctx, role))
            .transpose()?
        {
            Some(_) => msg.channel_id.say(&ctx, "User group removed")?,
            None => msg.channel_id.say(&ctx, "No user group removed")?,
        };
    } else {
        msg.channel_id.say(&ctx, "That role is not a user group")?;
    }
    Ok(())
}

#[command("-l")]
#[description("List user groups")]
pub fn list(ctx: &mut Context, msg: &Message) -> CommandResult {
    let map = ctx.data.read();
    let config = map.get::<Config>().expect("Config not loaded").read();
    let guild = msg
        .guild_id
        .ok_or("Not in a server")?
        .to_partial_guild(&ctx)?;
    msg.channel_id.send_message(&ctx, |m| {
        m.embed(|e| {
            e.title("User groups")
                .description(
                    "`$usermod -a Role` adiciona te a um user group
`$usermod -d Role` remove te de um user group",
                )
                .fields(
                    config
                        .user_groups()
                        .filter_map(|(r, d)| guild.roles.get(&r).map(|r| (&r.name, d)))
                        .map(|(r, d)| (r, d, true)),
                )
        })
    })?;
    Ok(())
}

pub fn role_exists(
    ctx: &Context,
    guild_id: GuildId,
    role: RoleId,
) -> Result<bool, serenity::Error> {
    Ok(guild_id.to_partial_guild(&ctx)?.roles.contains_key(&role))
}

pub fn role_by_name(
    ctx: &Context,
    guild_id: GuildId,
    role: &str,
) -> Result<Option<RoleId>, serenity::Error> {
    Ok(guild_id
        .to_partial_guild(ctx)?
        .roles
        .values()
        .filter(|r| r.name.eq_ignore_ascii_case(role))
        .map(|r| r.id)
        .next())
}
