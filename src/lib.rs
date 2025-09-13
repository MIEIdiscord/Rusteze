#![deny(unused_must_use)]
#![expect(deprecated)] // serenity standard framework is deprecated

pub mod channels;
pub mod commands;
pub mod config;
pub mod daemons;
pub mod delayed_tasks;
pub mod util;

pub use self::daemons::{DaemonManager, DaemonManagerKey};
use crate::config::Config;
use once_cell::sync::Lazy;
use regex::Regex;
use serenity::{
    all::{ActivityData, Colour, CreateEmbed, CreateEmbedFooter, CreateMessage},
    framework::standard::{
        Args, CommandGroup, CommandResult, DispatchError, HelpOptions, help_commands,
        macros::{help, hook},
    },
    model::{
        channel::Message,
        gateway::Ready,
        guild::Member,
        id::{ChannelId, GuildId, UserId},
        user::{OnlineStatus, User},
    },
    prelude::*,
};
use std::{collections::HashSet, sync::Arc};

pub struct UpdateNotify;

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

pub struct Handler;

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _ready: Ready) {
        ctx.set_presence(Some(ActivityData::playing("$man")), OnlineStatus::Online);
        crate::log!("Up and running");
        if let Some(id) = ctx.data.write().await.remove::<UpdateNotify>() {
            ChannelId::from(*id)
                .send_message(&ctx, CreateMessage::new().content("Rebooted successfully!"))
                .await
                .expect("Couldn't send update notification");
        }
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        let share_map = ctx.data.read().await;
        let config = get!(> share_map, Config, read);
        if let (Some(ch), Some(greet_message)) =
            (config.greet_channel(), config.greet_channel_message())
        {
            let user = new_member.user.id;
            let guild = new_member.guild_id.to_partial_guild(&ctx).await;
            ch.send_message(&ctx, CreateMessage::new()
                .content(format!("{}", user.mention()))
                .embed(CreateEmbed::new()
                    .title("Bem-vindo(a) ao servidor de MIEI!")
                    .description(greet_message)
                    .thumbnail(guild.map(|u|u.icon_url().expect("No Guild Image available")).unwrap())
                    .colour(Colour::from_rgb(0, 0, 0))
                    .footer(CreateEmbedFooter::new("Se tiveres alguma dúvida sobre o bot podes usar o comando $man para saberes o que podes fazer."))
                )
            ).await.map_err(|e| log!("Couldn't greet new user {}: {:?}", user, e)).ok();
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
        let config = get!(> share_map, Config, read);
        if let Some(ch) = config.log_channel() {
            let (nick, avatar) = member_data
                .as_ref()
                .map(|m| (m.nick.as_deref().unwrap_or("None"), m.face()))
                .unwrap_or_else(|| ("None", user.face()));
            ch.send_message(
                &ctx,
                CreateMessage::new().embed(
                    CreateEmbed::new()
                        .title("User left the server")
                        .description(format!(
                            "**Name:**      {}\n**Nickname:** {}",
                            user.name, nick
                        ))
                        .thumbnail(avatar),
                ),
            )
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

    async fn message(&self, ctx: Context, msg: Message) {
        static INVITE: Lazy<Regex> = Lazy::new(|| {
            Regex::new("(https?://)?(www.)?discord.(gg|li|me)/[[:alnum:]]{2,32}").unwrap()
        });

        if INVITE.is_match(&msg.content) {
            let link = INVITE.find(&msg.content).unwrap().as_str();
            let guild = msg.guild(&ctx.cache).unwrap().clone();
            let invites = guild.invites(&ctx).await;
            if invites
                .unwrap_or_default()
                .iter()
                .map(|i| i.url())
                .all(|i| i != link)
            {
                msg.delete(&ctx).await.unwrap();

                msg.author
                    .direct_message(
                        &ctx,
                        CreateMessage::new().content("Bad person. No share inviterinos!"),
                    )
                    .await
                    .unwrap();

                let share_map = ctx.data.read().await;
                let config = get!(> share_map, Config, read);

                if let Some(ch) = config.log_channel() {
                    let channel_name = match msg.channel(&ctx).await.unwrap().guild() {
                        Some(guild_channel) => guild_channel.name,
                        None => "in DM".to_owned(),
                    };

                    ch.send_message(
                        &ctx,
                        CreateMessage::new().embed(
                            CreateEmbed::new()
                                .title("User sent a external server invite")
                                .description(format!(
                                    "**Name:**   {}\n**Channel** {}\n**Link:**   {}",
                                    msg.author.name, channel_name, link
                                ))
                                .thumbnail(
                                    msg.author
                                        .avatar_url()
                                        .as_deref()
                                        .unwrap_or("https://i.imgur.com/lKmW0tc.png"),
                                ),
                        ),
                    )
                    .await
                    .map_err(|e| {
                        log!(
                            "Couldn't log user {} sending a discord invite (link: {}). Error: {:?}",
                            msg.author.name,
                            link,
                            e
                        )
                    })
                    .ok();
                }
            }
        }
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
pub async fn before_hook(ctx: &Context, msg: &Message, _: &str) -> bool {
    valid_channel(ctx, msg).await || is_admin(ctx, msg).await || is_cesium_cmd(msg).await
}

#[hook]
pub async fn after_hook(ctx: &Context, msg: &Message, cmd_name: &str, error: CommandResult) {
    match error {
        Ok(()) => log!(
            "Processed command '{}' for user '{}::{}'",
            cmd_name,
            msg.author.name,
            msg.author,
        ),
        Err(why) => {
            let _ = msg.channel_id.say(ctx, why.to_string()).await;
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
pub async fn dispatch_error_hook(
    ctx: &Context,
    msg: &Message,
    error: DispatchError,
    _command_name: &str,
) {
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

pub async fn valid_channel(ctx: &Context, msg: &Message) -> bool {
    get!(ctx, Config, read).channel_is_allowed(msg.channel_id)
}

pub async fn is_admin(ctx: &Context, msg: &Message) -> bool {
    async fn _f(ctx: &Context, msg: &Message) -> Option<bool> {
        Some(
            msg.guild_id?
                .member(&ctx.http, &msg.author)
                .await
                .ok()?
                .permissions(ctx)
                .ok()?
                .administrator(),
        )
    }
    _f(ctx, msg).await.unwrap_or(false)
}

pub async fn is_cesium_cmd(msg: &Message) -> bool {
    msg.content.split_whitespace().next() == Some("$cesium")
}
