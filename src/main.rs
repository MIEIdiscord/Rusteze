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
        permissions::Permissions,
        voice::VoiceState,
    },
    prelude::*,
};
#[macro_use]
extern crate lazy_static;

pub mod channels;
mod commands;
const TOKEN: &str = "";
use crate::commands::{COURSES_GROUP, STUDY_GROUP};

struct Handler;

impl EventHandler for Handler {}

fn main() {
    let mut client = Client::new(TOKEN, Handler).expect("Error creating client");
    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("%"))
            .group(&STUDY_GROUP)
            .group(&COURSES_GROUP),
    );
    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}
