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
mod channels;
use commands::{
    PING_COMMAND, STUDY_COMMAND, UNSTUDY_COMMAND,
};
use channels::{
    readCourses,
};

group!({
    name: "pingpong",
    options: {},
    commands: [ping, study, unstudy],
});

struct Handler;

impl EventHandler for Handler {}

fn main() {
    readCourses();
    let mut client = Client::new(TOKEN, Handler).expect("Error creating client");
    client.with_framework(StandardFramework::new()
                          .configure(|c| c.prefix("!"))
                          .group(&PINGPONG_GROUP));
    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}
