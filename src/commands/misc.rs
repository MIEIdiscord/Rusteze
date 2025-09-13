use serenity::{
    framework::standard::{
        CommandResult,
        macros::{command, group},
    },
    model::channel::Message,
    prelude::*,
};

#[group]
#[commands(ping, info, material)]
struct Misc;

#[command]
#[description("Teste de conectividade entre o Bot e os servidores do Discord.")]
pub async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id
        .say(&ctx.http, "Pong but in <:rust:530449316607688724>!")
        .await?;
    Ok(())
}

#[command]
#[description(
    "Informação relativa à linguagem de programação utilizada para desenvolvimento do Bot."
)]
pub async fn info(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Powered by Java8™").await?;
    Ok(())
}

#[command]
#[description("Apresenta o link para o material de apoio do curso.")]
#[usage("")]
pub async fn material(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id
        .say(
            &ctx.http,
            "**Este é o link para o material do curso** -> http://bit.ly/materialmiei",
        )
        .await?;
    Ok(())
}
