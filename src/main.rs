pub mod channels;
mod commands;

use serenity::{
    framework::standard::StandardFramework,
    model::{gateway::Ready, id::ChannelId},
    prelude::*,
};
use std::fs;
use std::sync::Arc;

use crate::commands::{COURSES_GROUP, STUDY_GROUP, admin::ADMIN_GROUP};

struct UpdateNotify;

impl TypeMapKey for UpdateNotify {
    type Value = Arc<u64>;
}

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, ctx: Context, _ready: Ready) {
        println!("Up and running");
        if let Some(id) = ctx.data.read().get::<UpdateNotify>() {
            ChannelId::from(**id)
                .send_message(&ctx, |m| m.content("Updated successfully!"))
                .expect("Couldn't send update notification");
        }
        ctx.data.write().remove::<UpdateNotify>();
    }
}

fn main() {
    let token = fs::read_to_string("auth").expect("No auth file");
    let mut client = Client::new(token, Handler).expect("Error creating client");
    {
        let mut data = client.data.write();
        if let Some(id) = std::env::args()
            .skip_while(|x| x != "-r")
            .nth(1)
            .and_then(|id| id.parse::<u64>().ok())
        {
            data.insert::<UpdateNotify>(Arc::new(id));
        }
    }
    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("$"))
            .group(&STUDY_GROUP)
            .group(&COURSES_GROUP)
            .group(&ADMIN_GROUP),
    );
    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}
