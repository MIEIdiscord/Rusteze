use crate::{config::Config, get};
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
pub async fn join(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let request = args.rest();
    let role = match role_by_name(ctx, msg.guild_id.ok_or("Not in a server")?, request).await? {
        Some(role) => role,
        None => return Err("No such role".into()),
    };
    if get!(ctx, Config, read).user_group_exists(role) {
        let mut member = msg.member(&ctx).await?;
        if !member.roles.contains(&role) {
            member.add_role(&ctx, role).await?;
            msg.channel_id.say(&ctx, "User group added").await?;
        } else {
            msg.channel_id.say(&ctx, "No user group added").await?;
        }
    } else {
        msg.channel_id
            .say(&ctx, "That role is not a user group")
            .await?;
    }
    Ok(())
}

#[command("-d")]
#[description("Leave a role")]
#[usage("role_name")]
pub async fn leave(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let request = args.rest();
    let role = match role_by_name(ctx, msg.guild_id.ok_or("Not in a server")?, request).await? {
        Some(role) => role,
        None => return Err("No such role".into()),
    };
    if get!(ctx, Config, read).user_group_exists(role) {
        let mut member = msg.member(&ctx).await?;
        if member.roles.contains(&role) {
            member.remove_role(&ctx, role).await?;
            msg.channel_id.say(&ctx, "User group removed").await?;
        } else {
            msg.channel_id.say(&ctx, "No user group removed").await?;
        }
    } else {
        msg.channel_id
            .say(&ctx, "That role is not a user group")
            .await?;
    }
    Ok(())
}

#[command("-l")]
#[description("List user groups")]
pub async fn list(ctx: &Context, msg: &Message) -> CommandResult {
    let map = ctx.data.read().await;
    let config = get!(> map, Config, read);
    let guild = msg
        .guild_id
        .ok_or("Not in a server")?
        .to_partial_guild(&ctx)
        .await?;
    msg.channel_id
        .send_message(&ctx, |m| {
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
        })
        .await?;
    Ok(())
}

pub async fn role_exists(
    ctx: &Context,
    guild_id: GuildId,
    role: RoleId,
) -> Result<bool, serenity::Error> {
    Ok(guild_id
        .to_partial_guild(&ctx)
        .await?
        .roles
        .contains_key(&role))
}

pub async fn role_by_name(
    ctx: &Context,
    guild_id: GuildId,
    role: &str,
) -> Result<Option<RoleId>, serenity::Error> {
    let finder = aho_corasick::AhoCorasickBuilder::new()
        .ascii_case_insensitive(true)
        .build(&[role]);
    Ok(guild_id
        .to_partial_guild(ctx)
        .await?
        .roles
        .values()
        .find(|r| finder.is_match(&r.name))
        .map(|r| r.id))
}
