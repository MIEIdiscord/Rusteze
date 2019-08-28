pub mod channels;
mod commands;

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
        guild::{PartialGuild, Member}
    },
    prelude::*,
    utils::Colour,
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

    fn guild_member_addition(
    &self,
    ctx: Context,
    guild_id: GuildId,
    new_member: Member
    ) {
        new_member
            .user_id()
            .to_user(&ctx)
            .map(|x|
                x.direct_message(&ctx, |m|{
                    m.embed(|e| {
                        e.title("Bem vindo ao servidor de MIEI!");
                        e.description(format!("O nosso objetivo é facilitar a vossa passagem neste curso, \
                        através de um servidor com todas as cadeiras, materiais e conteúdos para \
                        que possam estar sempre a par do que acontece em cada cadeira.
      Temos também uma sala `#geral` onde podemos conversar de uma forma mais informal e um \
      conjunto de `#regras` que devem ser cumpridas e que podem sempre consultar com alguma \
      dúvida que tenham.
      Temos também o nosso bot {} que permite que te juntes às salas das \
      cadeiras com o comando `$study CADEIRA1, CADEIRA2, ...` ou, se preferires, podes-te juntar \
      a todas as cadeiras de um ano com o comando `$study Xano` substituindo o `X` pelo ano que queres.", ctx.cache.read().user.name));
                        e.footer( |f| {
                            f.text("Qualquer dúvida sobre o bot podes usar $man man para saberes o que podes fazer.");
                            f
                        });
                        e.thumbnail(guild_id.to_partial_guild(&ctx.http).map(|u|u.icon_url().expect("No Guild Image available")).unwrap());
                        e.colour(Colour::from_rgb(0, 0, 0));
                        e
                    });
                    m
                })).unwrap().unwrap();
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
