use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

const TOKEN: &str = "TOKEN HERE";

struct Handler;

impl EventHandler for Handler {
    fn message(&self, ctx: Context, msg: Message) {
        if !msg.author.bot {
            if let Err(why) = msg.channel_id.say(ctx, "Hello!") {
                println!("Message Error: {:?}", why);
            }
        }
    }

    fn ready(&self, _: Context, _: Ready) {
        println!("I'm ready boy!");
    }
}

fn main() {
    let mut client = Client::new(TOKEN, Handler).expect("Error creating client");
    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}
