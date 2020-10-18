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
fn daemon_list(ctx: &mut Context, msg: &Message) -> CommandResult {
    let share_map = ctx.data.read();
    msg.channel_id.say(
        &ctx,
        format!(
            "{:?}",
            share_map
                .get::<crate::daemons::DaemonThread>()
                .unwrap()
                .list
        ),
    )?;
    Ok(())
}

#[command("now")]
#[description("Runs all or one daemon now")]
#[usage("[number]")]
fn daemon_now(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let share_map = ctx.data.read();
    let daemon_t = share_map.get::<crate::daemons::DaemonThread>().unwrap();
    match args.single::<usize>() {
        Ok(u) if u < daemon_t.list.len() => daemon_t.run_one(u)?,
        Ok(_) => return Err("Index out of bounds".into()),
        Err(ArgError::Eos) => daemon_t.run_all()?,
        Err(_) => return Err("Invalid index".into()),
    }
    msg.channel_id.say(&ctx, "Done")?;
    Ok(())
}
