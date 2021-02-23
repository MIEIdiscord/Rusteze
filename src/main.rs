#![deny(unused_must_use)]

pub mod channels;
mod commands;
pub mod config;
mod daemons;
pub mod util;

use crate::{
    channels::{read_courses, MiEI},
    commands::{admin::*, cesium::*, misc::*, study::*, usermod::*},
    config::Config,
    daemons::minecraft::Minecraft,
};
use serenity::{
    framework::standard::{
        help_commands,
        macros::{help, hook},
        Args, CommandGroup, CommandResult, DispatchError, HelpOptions, StandardFramework,
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
    client::bridge::gateway::GatewayIntents,
};
use std::{collections::HashSet, fs, sync::Arc};

struct UpdateNotify;

impl TypeMapKey for UpdateNotify {
    type Value = Arc<u64>;
}

#[macro_export]
macro_rules! log {
    ($fmt:expr $(, $param:expr)*$(,)?) => {
        eprintln!(
            concat!("[{}] ", $fmt),
            ::chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            $($param,)*
        )
    }
}

struct Handler;

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _ready: Ready) {
        ctx.set_presence(Some(Activity::playing("$man")), OnlineStatus::Online)
            .await;
        crate::log!("Up and running");
        if let Some(id) = ctx.data.read().await.get::<UpdateNotify>() {
            ChannelId::from(**id)
                .send_message(&ctx, |m| m.content("Rebooted successfully!"))
                .await
                .expect("Couldn't send update notification");
        }
        ctx.data.write().await.remove::<UpdateNotify>();
    }

    async fn guild_member_addition(&self, ctx: Context, guild_id: GuildId, new_member: Member) {
        let share_map = ctx.data.read().await;
        let config = share_map.get::<Config>().unwrap().read().await;
        if let (Some(ch), Some(greet_message)) =
            (config.greet_channel(), config.greet_channel_message())
        {
            let user = new_member.user.id;
            let guild = guild_id.to_partial_guild(&ctx.http).await;
            ch.send_message(&ctx, |m| {
                m.content(user.mention());
                m.embed(|e| {
                    e.title("Bem-vindo(a) ao servidor de MIEI!");
                    e.description(greet_message);
                    e.thumbnail(guild.map(|u|u.icon_url().expect("No Guild Image available")).unwrap());
                    e.colour(Colour::from_rgb(0, 0, 0));
                    e.footer( |f| {
                        f.text("Se tiveres alguma dúvida sobre o bot podes usar o comando $man para saberes o que podes fazer.");
                        f
                    });
                    e
                });
                m
            }).await.map_err(|e| log!("Couldn't greet new user {}: {:?}", user, e)).ok();
        }
    }

    async fn guild_member_removal(
        &self,
        ctx: Context,
        _: GuildId,
        user: User,
        member_data: Option<Member>,
    ) {
        let share_map = ctx.data.read().await;
        let config = share_map.get::<Config>().unwrap().read().await;
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
            .await
            .map_err(|e| {
                log!(
                    "Couldn't log user {} (nickname {}) leaving the server. Error: {:?}",
                    user.name,
                    nick,
                    e
                )
            })
            .ok();
        }
    }
}

#[tokio::main]
async fn main() {
    let token = match fs::read_to_string("auth") {
        Ok(token) => token,
        Err(e) => {
            log!("Could not open auth file");
            log!("Error: {}", e);
            std::process::exit(1);
        }
    };
    let mut client_builder = Client::builder(token)
        .event_handler(Handler)
        .intents(GatewayIntents::all())
        .type_map_insert::<MiEI>(Arc::new(RwLock::new(read_courses().unwrap_or_default())))
        .type_map_insert::<Config>(Arc::new(RwLock::new(Config::new().unwrap_or_default())))
        .type_map_insert::<ChannelMapping>(Arc::new(RwLock::new(
            ChannelMapping::load().unwrap_or_default(),
        )))
        .framework(
            StandardFramework::new()
                .configure(|c| c.prefix("$"))
                .before(before_hook)
                .after(after_hook)
                .on_dispatch_error(dispatch_error_hook)
                .group(&STUDY_GROUP)
                .group(&COURSES_GROUP)
                .group(&ADMIN_GROUP)
                .group(&MISC_GROUP)
                .group(&CESIUM_GROUP)
                .group(&USERMOD_GROUP)
                .help(&MY_HELP),
        );
    if let Some(id) = std::env::args()
        .skip_while(|x| x != "-r")
        .nth(1)
        .and_then(|id| id.parse::<u64>().ok())
    {
        client_builder = client_builder.type_map_insert::<UpdateNotify>(Arc::new(id))
    }
    let mut client = client_builder.await.expect("failed to start client");
    if let Ok(_) = util::minecraft_server_get(&["list"]) {
        log!("Initializing minecraft daemon");
        let mc = Arc::new(RwLock::new(Minecraft::load().unwrap_or_default()));
        let mut data = client.data.write().await;
        data.insert::<Minecraft>(Arc::clone(&mc));
        data.insert::<daemons::DaemonThread>(
            daemons::start_daemon_thread(vec![mc], Arc::clone(&client.cache_and_http)).await,
        );
    }
    if let Err(why) = client.start().await {
        log!("Client error: {:?}", why);
    }
}

#[help("man")]
#[command_not_found_text("No manual entry for that")]
#[max_levenshtein_distance(5)]
#[lacking_permissions("hide")]
#[strikethrough_commands_tip_in_guild(" ")]
#[strikethrough_commands_tip_in_dm(" ")]
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[hook]
async fn before_hook(ctx: &Context, msg: &Message, _: &str) -> bool {
    valid_channel(ctx, msg).await
        || is_mc_cmd(ctx, msg).await
        || is_admin(ctx, msg).await
        || is_cesium_cmd(msg).await
}

#[hook]
async fn after_hook(ctx: &Context, msg: &Message, cmd_name: &str, error: CommandResult) {
    match error {
        Ok(()) => log!(
            "Processed command '{}' for user '{}::{}'",
            cmd_name,
            msg.author.name,
            msg.author,
        ),
        Err(why) => {
            let _ = msg.channel_id.say(ctx, &why);
            log!(
                "Command '{}' for user '{}::{}' failed because {:?}",
                cmd_name,
                msg.author.name,
                msg.author,
                why
            )
        }
    }
}

#[hook]
async fn dispatch_error_hook(ctx: &Context, msg: &Message, error: DispatchError) {
    log!(
        "Command '{}' for user '{}::{}' failed to dispatch because '{:?}'",
        msg.content,
        msg.author.name,
        msg.author,
        error
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
            .await
            .expect("Couldn't communicate dispatch error");
    }
}

async fn valid_channel(ctx: &Context, msg: &Message) -> bool {
    ctx.data
        .read()
        .await
        .get::<Config>()
        .unwrap()
        .read()
        .await
        .channel_is_allowed(msg.channel_id)
}

async fn is_admin(ctx: &Context, msg: &Message) -> bool {
    async fn _f(ctx: &Context, msg: &Message) -> Option<bool> {
        Some(
            msg.guild_id?
                .member(&ctx.http, &msg.author)
                .await
                .ok()?
                .permissions(&ctx)
                .await
                .ok()?
                .administrator(),
        )
    }
    _f(ctx, msg).await.unwrap_or(false)
}

async fn is_cesium_cmd(msg: &Message) -> bool {
    msg.content.split_whitespace().next() == Some("$cesium")
}

async fn is_mc_cmd(ctx: &Context, msg: &Message) -> bool {
    msg.content
        .trim()
        .trim_start_matches('$')
        .starts_with("online")
        && msg
            .channel_id
            .name(&ctx)
            .await
            .map(|name| name == "minecraft")
            .unwrap_or_default()
}
