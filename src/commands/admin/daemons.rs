use serenity::{
    framework::standard::{
        macros::{command, group},
        ArgError, Args, CommandResult,
    },
    model::channel::Message,
    prelude::*,
};

#[group]
#[commands(daemon_now, daemon_list)]
#[required_permissions(ADMINISTRATOR)]
#[prefixes("daemons", "deamons")]
struct Daemons;

#[command("list")]
#[description("List current daemons")]
#[usage("")]
async fn daemon_list(ctx: &Context, msg: &Message) -> CommandResult {
    let share_map = ctx.data.read().await;
    msg.channel_id
        .say(
            &ctx,
            format!(
                "{:?}",
                share_map
                    .get::<crate::daemons::DaemonThread>()
                    .unwrap()
                    .list
            ),
        )
        .await?;
    Ok(())
}

#[command("now")]
#[description("Runs all or one daemon now")]
#[usage("[number]")]
async fn daemon_now(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut share_map = ctx.data.write().await;
    let daemon_t = share_map.get_mut::<crate::daemons::DaemonThread>().unwrap();
    match args.single::<usize>() {
        Ok(u) if u < daemon_t.list.len() => daemon_t.run_one(u).await?,
        Ok(_) => return Err("Index out of bounds".into()),
        Err(ArgError::Eos) => daemon_t.run_all().await?,
        Err(_) => return Err("Invalid index".into()),
    }
    msg.channel_id.say(&ctx, "Done").await?;
    Ok(())
}
