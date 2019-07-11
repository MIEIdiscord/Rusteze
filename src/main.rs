use serenity::{
    framework::standard::{
        help_commands, macros::{group, command, help}, Args, CommandGroup, CommandResult, HelpOptions,
        StandardFramework,
    },
    model::{
        channel::{Channel, Message},
        gateway::Ready,
        id::{GuildId, UserId},
        voice::VoiceState,
    },
    prelude::*,
};
mod commands;
pub mod channels;
use crate::commands::{
    PING_COMMAND, STUDY_COMMAND, UNSTUDY_COMMAND, MKCOURSE_COMMAND,
};

group!({
    name: "pingpong",
    options: {},
    commands: [ping, study, unstudy, mkcourse],
});

struct Handler;

impl EventHandler for Handler {}

fn main() {
    let mut client = Client::new(TOKEN, Handler).expect("Error creating client");
    client.with_framework(StandardFramework::new()
                          .configure(|c| c.prefix("!"))
                          .group(&PINGPONG_GROUP));
    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}
