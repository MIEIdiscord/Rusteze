#[allow(unused_imports)]
use serenity::{
    framework::standard::{
        help_commands,
        macros::{command, group, help},
        Args, CommandGroup, CommandResult, HelpOptions, StandardFramework,
    },
    model::{
        channel::{Channel, Message},
        gateway::Ready,
        id::{GuildId, UserId},
        voice::VoiceState,
        permissions::Permissions,
    },
    prelude::*,
};
#[macro_use] extern crate lazy_static;

pub mod channels;
mod commands;
const TOKEN: &str = "";
use crate::commands::{PING_COMMAND, STUDY_COMMAND, UNSTUDY_COMMAND, MK_COMMAND};

group!({
    name: "pingpong",
    options: {},
    commands: [ping, study, unstudy],
});

group!({
    name: "courses",
    options: {
        required_permissions: [ADMINISTRATOR],
        prefixes: ["courses"],
    },
    commands: [mk],
});

struct Handler;

impl EventHandler for Handler {}

fn main() {
    let mut client = Client::new(TOKEN, Handler).expect("Error creating client");
    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("%"))
            .group(&PINGPONG_GROUP)
            .group(&COURSES_GROUP),
    );
    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}
