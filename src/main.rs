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
use std::fs;
#[macro_use]
extern crate lazy_static;

pub mod channels;
mod commands;
use crate::commands::{COURSES_GROUP, STUDY_GROUP};

struct Handler;

impl EventHandler for Handler {}

fn main() {
    let token = fs::read_to_string("auth").expect("No auth file");
    let mut client = Client::new(token, Handler).expect("Error creating client");
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
