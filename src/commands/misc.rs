use crate::util::minecraft_server_get;
use serenity::{
    framework::standard::{
        macros::{command, group},
        CommandResult,
    },
    model::channel::Message,
    prelude::*,
};

#[group]
#[commands(online, ping, info, material)]
struct Misc;

#[command]
#[description("Teste de conectividade entre o Bot e os servidores do Discord.")]
pub fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.channel_id
        .say(&ctx.http, "Pong but in <:rust:530449316607688724>!")?;
    Ok(())
}

#[command]
#[description(
    "Informação relativa à linguagem de programação utilizada para desenvolvimento do Bot."
)]
pub fn info(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Powered by JAVA8™")?;
    Ok(())
}

#[command]
#[description("Apresenta o link para o material de apoio do curso.")]
#[usage("")]
pub fn material(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(
        &ctx.http,
        "**Este é o link para o material do curso** -> http://bit.ly/materialmiei",
    )?;
    Ok(())
}

#[command]
#[description("Shows the list of online players")]
#[usage("")]
pub fn online(ctx: &mut Context, msg: &Message) -> CommandResult {
    let output = minecraft_server_get(&["list"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    msg.channel_id
        .say(&ctx, format!("{}\n{}", stdout, stderr))?;
    Ok(())
}
