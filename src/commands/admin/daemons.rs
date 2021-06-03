use itertools::Itertools;
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
                "```\n{}\n```",
                share_map
                    .get::<crate::DaemonManager>()
                    .unwrap()
                    .lock()
                    .await
                    .daemon_names()
                    .format_with("\n", |(i, n), f| f(&format_args!("{}: {}", i, n)))
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
    let daemon_t = share_map.get_mut::<crate::DaemonManager>().unwrap();
    let mut daemon_t = daemon_t.lock().await;
    let e = match args.single::<usize>() {
        Ok(u) => daemon_t.run_one(u).await,
        Err(ArgError::Eos) => {
            daemon_t.run_all().await;
            Ok(())
        }
        Err(e) => return Err(format!("Invalid index: {}", e).into()),
    };
    if let Err(e) = e {
        Err(format!("Could not run daemon with id {}", e).into())
    } else {
        msg.channel_id.say(&ctx, "Done").await?;
        Ok(())
    }
}
