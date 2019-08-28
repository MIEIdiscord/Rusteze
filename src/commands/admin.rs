use serenity::{
    framework::standard::{
        macros::{command, group},
        CommandResult,
    },
    model::channel::Message,
    prelude::*,
};

use std::os::unix::process::CommandExt;
use std::process::Command as Fork;
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};

static UPDATING: AtomicBool = AtomicBool::new(false);

group!({
    name: "Admin",
    options: {
        required_permissions: [ADMINISTRATOR],
    },
    commands: [update],
});

#[command]
#[description("Update the bot")]
pub fn update(ctx: &mut Context, msg: &Message) -> CommandResult {
    if UPDATING.load(Ordering::SeqCst) {
        Err("Alreading updating")?;
    } else {
        UPDATING.store(true, Ordering::SeqCst);
    }
    msg.channel_id.say(&ctx, "Fetching...")?;
    Fork::new("git").arg("fetch").spawn()?.wait()?;

    msg.channel_id.say(&ctx, "Checking remote...")?;
    let status = Fork::new("git")
        .args(&["rev-list", "--count", "master...master@{upstream}"])
        .output()?;
    if let 0 = String::from_utf8_lossy(&status.stdout)
        .trim()
        .parse::<i32>()?
    {
        Err("No updates!".to_string())?;
    }

    msg.channel_id.say(&ctx, "Pulling from remote...")?;
    match &Fork::new("git").arg("pull").output()? {
        out if !out.status.success() => Err(format!(
            "Error pulling!
            ```
            ============= stdout =============
            {}
            ============= stderr =============
            {}
            ```",
            str::from_utf8(&out.stdout)?,
            str::from_utf8(&out.stderr)?
        ))?,
        _ => (),
    }

    msg.channel_id.say(&ctx, "Compiling...")?;
    match &Fork::new("cargo").args(&["build", "--release"]).output()? {
        out if !out.status.success() => Err(format!(
            "Build Error!
            ```
            {}
            ```",
            str::from_utf8(&out.stderr)?
        ))?,
        _ => (),
    }

    msg.channel_id.say(ctx, "Rebooting...")?;
    Err(Fork::new("cargo")
        .args(&["run", "--release", "--", "-r", &msg.channel_id.to_string()])
        .exec())?;
    Ok(())
}
