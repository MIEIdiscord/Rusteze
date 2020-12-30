use crate::util::minecraft_server_get;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{channel::Message, id::UserId},
    prelude::*,
};

#[group]
#[commands(server_do, pair, pair_guild_set)]
#[required_permissions(ADMINISTRATOR)]
#[default_command(server_do)]
#[prefixes("mc")]
struct Minecraft;

#[command]
#[description("Run a command as op on the server")]
#[usage("command 1 ; command 2 ; ...")]
#[min_args(1)]
async fn server_do(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    for command in args.rest().split(";") {
        let output = minecraft_server_get(&[command.trim()])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        msg.channel_id
            .say(&ctx, format!("`{}`: {}{}", command, stdout, stderr))
            .await?;
    }
    Ok(())
}

#[command]
#[description("Associate a minecraft username with the discord's username")]
#[usage("minecraft_nickname @mention")]
#[min_args(2)]
async fn pair(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let nick = args.single::<String>()?;
    let user = args.single::<UserId>()?;
    let share_map = ctx.data.write().await;
    share_map
        .get::<crate::daemons::minecraft::Minecraft>()
        .unwrap()
        .write()
        .await
        .pair(nick, user)?;
    msg.channel_id.say(&ctx, "User paired").await?;
    Ok(())
}

#[command]
#[description("Set's this guild as the one to use for the minecraft daemon")]
#[usage("")]
async fn pair_guild_set(ctx: &Context, msg: &Message) -> CommandResult {
    let share_map = ctx.data.write().await;
    match msg.guild_id {
        Some(gid) => {
            share_map
                .get::<crate::daemons::minecraft::Minecraft>()
                .unwrap()
                .write()
                .await
                .set_guild_id(gid)?;
            msg.channel_id.say(&ctx, "Guild id set").await?
        }
        None => msg.channel_id.say(&ctx, "Couldn't find guild id").await?,
    };
    Ok(())
}
