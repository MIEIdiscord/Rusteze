use serenity::{
    framework::standard::{
        help_commands, macros::help, Args, CommandGroup, CommandResult, HelpOptions,
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

const TOKEN: &str = "TOKEN HERE";

struct Handler;

impl EventHandler for Handler {}

fn main() {
    let mut client = Client::new(TOKEN, Handler).expect("Error creating client");
    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}
