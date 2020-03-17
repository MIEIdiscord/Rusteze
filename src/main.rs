pub mod channels;
mod commands;
pub mod config;

use crate::{
    channels::{read_courses, MiEI},
    commands::{admin::ADMIN_GROUP, cesium::{ChannelMapping, CESIUM_GROUP}, COURSES_GROUP, MISC_GROUP, STUDY_GROUP},
    config::Config,
};
use serenity::{
    framework::standard::{
        help_commands, macros::help, Args, CommandGroup, CommandResult, DispatchError, HelpOptions,
        StandardFramework,
    },
    model::{
        channel::Message,
        gateway::{Activity, Ready},
        guild::Member,
        id::{ChannelId, GuildId, UserId},
        user::{OnlineStatus, User},
    },
    prelude::*,
    utils::Colour,
};
use std::collections::HashSet;
use std::fs;
use std::sync::{Arc, RwLock};

struct UpdateNotify;

impl TypeMapKey for UpdateNotify {
    type Value = Arc<u64>;
}

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, ctx: Context, _ready: Ready) {
        ctx.set_presence(Some(Activity::playing("$man")), OnlineStatus::Online);
        println!("Up and running");
        if let Some(id) = ctx.data.read().get::<UpdateNotify>() {
            ChannelId::from(**id)
                .send_message(&ctx, |m| m.content("Updated successfully!"))
                .expect("Couldn't send update notification");
        }
        ctx.data.write().remove::<UpdateNotify>();
    }

    fn guild_member_addition(&self, ctx: Context, guild_id: GuildId, new_member: Member) {
        let share_map = ctx.data.read();
        let config = share_map.get::<Config>().unwrap().read().unwrap();
        if let (Some(ch), Some(greet_message)) =
            (config.greet_channel(), config.greet_channel_message())
        {
            let user = new_member.user_id();
            ch.send_message(&ctx, |m| {
                m.content(user.mention());
                m.embed(|e| {
                    e.title("Bem-vindo(a) ao servidor de MIEI!");
                    e.description(greet_message);
                    e.thumbnail(guild_id.to_partial_guild(&ctx.http).map(|u|u.icon_url().expect("No Guild Image available")).unwrap());
                    e.colour(Colour::from_rgb(0, 0, 0));
                    e.footer( |f| {
                        f.text("Se tiveres alguma dúvida sobre o bot podes usar o comando $man para saberes o que podes fazer.");
                        f
                    });
                    e
                });
                m
            }).map_err(|e| eprintln!("Couldn't greet new user {}: {:?}", user, e)).ok();
        }
    }

    fn guild_member_removal(
        &self,
        ctx: Context,
        _: GuildId,
        user: User,
        member_data: Option<Member>,
    ) {
        let share_map = ctx.data.read();
        let config = share_map.get::<Config>().unwrap().read().unwrap();
        if let Some(ch) = config.log_channel() {
            let nick = member_data
                .as_ref()
                .and_then(|m| m.nick.as_ref().map(|s| s.as_str()))
                .unwrap_or("None");
            ch.send_message(&ctx, |m| {
                m.embed(|e| {
                    e.title("User left the server")
                        .description(format!(
                            "**Name:**      {}\n**Nickname:** {}",
                            user.name, nick
                        ))
                        .thumbnail(
                            user.avatar_url()
                                .as_ref()
                                .map(|s| s.as_str())
                                .unwrap_or("https://i.imgur.com/lKmW0tc.png"),
                        )
                })
            })
            .map_err(|e| {
                println!(
                    "Couldn't log user {} (nickname {}) leaving the server. Error: {:?}",
                    user.name, nick, e
                )
            })
            .ok();
        }
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
        let roles = read_courses().unwrap_or_default();
        data.insert::<MiEI>(Arc::new(RwLock::new(roles)));
        let config = Config::new().unwrap_or_default();
        data.insert::<Config>(Arc::new(RwLock::new(config)));
        data.insert::<ChannelMapping>(Arc::new(RwLock::new(ChannelMapping::load().unwrap())));
    }
    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("$"))
            .before(|ctx, msg, _message| valid_channel(ctx, msg) || is_admin(ctx, msg))
            .after(|ctx, msg, cmd_name, error| match error {
                Ok(()) => eprintln!("Processed command '{}' for user '{}'", cmd_name, msg.author),
                Err(why) => {
                    let _ = msg.channel_id.say(ctx, &why.0);
                    eprintln!("Command '{}' failed with {:?}", cmd_name, why)
                }
            })
            .on_dispatch_error(|ctx, msg, error| {
                eprintln!(
                    "Command '{}' for user '{}' failed with error '{:?}'",
                    msg.content, msg.author, error
                );
                if let Some(s) = match error {
                    DispatchError::NotEnoughArguments { min: m, given: g } => {
                        Some(format!("Not enough arguments! min: {}, given: {}", m, g))
                    }
                    DispatchError::TooManyArguments { max: m, given: g } => {
                        Some(format!("Too many arguments! max: {}, given: {}", m, g))
                    }
                    _ => None,
                } {
                    msg.channel_id
                        .say(ctx, s)
                        .expect("Couldn't communicate dispatch error");
                }
            })
            .group(&STUDY_GROUP)
            .group(&COURSES_GROUP)
            .group(&ADMIN_GROUP)
            .group(&MISC_GROUP)
            .group(&CESIUM_GROUP)
            .help(&MY_HELP),
    );
    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}

#[help("man")]
#[command_not_found_text("No manual entry for that")]
#[max_levenshtein_distance(5)]
#[lacking_permissions("hide")]
#[strikethrough_commands_tip_in_guild(" ")]
#[strikethrough_commands_tip_in_dm(" ")]
fn my_help(
    context: &mut Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners)
}

fn valid_channel(ctx: &mut Context, msg: &Message) -> bool {
    ctx.data
        .read()
        .get::<Config>()
        .unwrap()
        .read()
        .unwrap()
        .channel_is_allowed(msg.channel_id)
}

fn is_admin(ctx: &mut Context, msg: &Message) -> bool {
    msg.guild_id
        .and_then(|g| g.member(&ctx, &msg.author).ok())
        .and_then(|u| u.permissions(&ctx).ok())
        .map(|p| p.administrator())
        .unwrap_or(false)
}
