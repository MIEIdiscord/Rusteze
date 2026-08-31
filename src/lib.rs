#![deny(unused_must_use)]
#![deny(unused_crate_dependencies)]
#![expect(deprecated)] // serenity standard framework is deprecated

use tokio as _;

pub mod channels;
pub mod commands;
pub mod config;
pub mod spam;
mod util;

use crate::config::Config;
use once_cell::sync::Lazy;
use regex::Regex;
use serenity::{
    all::{
        ActivityData, ButtonStyle, Colour, CreateActionRow, CreateButton, CreateEmbed,
        CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage,
        CreateMessage, Interaction,
    },
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

        handle_spam(&ctx, &msg).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let Interaction::Component(component) = interaction else {
            return;
        };

        let Some((action, user_part)) = component.data.custom_id.split_once(':') else {
            return;
        };
        if action != "spam_kick" && action != "spam_ignore" {
            return;
        }
        let Ok(user_id) = user_part.parse::<u64>().map(UserId::new) else {
            return;
        };

        // Only moderators (kick permission) may act on the alert.
        let allowed = component
            .member
            .as_ref()
            .and_then(|m| m.permissions)
            .map(|p| p.kick_members() || p.administrator())
            .unwrap_or(false);
        if !allowed {
            respond_ephemeral(
                &ctx,
                &component,
                "You don't have permission to do that.",
            )
            .await;
            return;
        }

        let Some(guild_id) = component.guild_id else {
            return;
        };

        if action == "spam_ignore" {
            get!(ctx, spam::SpamTracker, write).take_pending(user_id);
            update_alert(
                &ctx,
                &component,
                format!("Dismissed by {}.", component.user.name),
            )
            .await;
            return;
        }

        // action == "spam_kick"
        let messages = get!(ctx, spam::SpamTracker, write)
            .take_pending(user_id)
            .unwrap_or_default();

        let mut deleted = 0usize;
        for (channel, message) in &messages {
            if channel.delete_message(&ctx.http, message).await.is_ok() {
                deleted += 1;
            }
        }

        // Warn the user (in European Portuguese) before kicking, while we still
        // share a guild with them and the DM is likely to go through.
        if let Ok(user) = user_id.to_user(&ctx).await {
            user.direct_message(
                &ctx,
                CreateMessage::new().content(
                    "Olá! Foste expulso(a) do servidor porque detetámos que a tua conta \
                     poderá ter sido comprometida e esteve a enviar spam.\n\n\
                     Por precaução, altera imediatamente a tua palavra-passe do Discord e \
                     ativa a autenticação de dois fatores. Depois de protegeres a tua conta, \
                     podes voltar a juntar-te ao servidor.",
                ),
            )
            .await
            .map_err(|e| log!("Couldn't DM kicked spammer {}: {:?}", user_id.get(), e))
            .ok();
        }

        let status = match guild_id
            .kick_with_reason(
                &ctx.http,
                user_id,
                "Automated spam detection - confirmed by a moderator",
            )
            .await
        {
            Ok(()) => format!(
                "Kicked <@{}> and deleted {} spam message(s). Action by {}.",
                user_id.get(),
                deleted,
                component.user.name
            ),
            Err(e) => {
                log!("Failed to kick spammer {}: {:?}", user_id.get(), e);
                format!(
                    "Deleted {} spam message(s) but failed to kick <@{}>: {}",
                    deleted,
                    user_id.get(),
                    e
                )
            }
        };

        update_alert(&ctx, &component, status).await;
    }
}

/// Detects cross-channel image spam and posts a moderator alert with a
/// confirmation button to the configured log channel.
async fn handle_spam(ctx: &Context, msg: &Message) {
    if msg.author.bot || msg.guild_id.is_none() {
        return;
    }

    let Some(signature) = spam::image_signature(msg) else {
        return;
    };

    let (enabled, log_channel) = {
        let share_map = ctx.data.read().await;
        let config = get!(> share_map, Config, read);
        (config.spam_detection(), config.log_channel())
    };
    if !enabled {
        return;
    }

    let outcome =
        get!(ctx, spam::SpamTracker, write).record(msg.author.id, msg.channel_id, msg.id, signature);

    let spam::SpamOutcome::Detected {
        channels,
        message_count,
    } = outcome
    else {
        return;
    };

    let Some(ch) = log_channel else {
        log!(
            "Spam detected from {} but no log channel is configured",
            msg.author.name
        );
        return;
    };

    let channel_list = channels
        .iter()
        .map(|c| c.mention().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let mut embed = CreateEmbed::new()
        .title("Possible spam detected")
        .description(format!(
            "**User:** {} ({})\n**Same image posted in {} channels** ({} messages) within {} seconds.\n**Channels:** {}",
            msg.author.mention(),
            msg.author.name,
            channels.len(),
            message_count,
            spam::SPAM_WINDOW.as_secs(),
            channel_list,
        ))
        .colour(Colour::from_rgb(220, 20, 20));
    if let Some(url) = spam::first_image_url(msg) {
        embed = embed.thumbnail(url);
    }

    let buttons = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("spam_kick:{}", msg.author.id.get()))
            .label("Kick & delete spam")
            .style(ButtonStyle::Danger),
        CreateButton::new(format!("spam_ignore:{}", msg.author.id.get()))
            .label("Ignore")
            .style(ButtonStyle::Secondary),
    ]);

    ch.send_message(
        ctx,
        CreateMessage::new().embed(embed).components(vec![buttons]),
    )
    .await
    .map_err(|e| log!("Couldn't send spam alert: {:?}", e))
    .ok();
}

/// Replaces the alert's buttons with a status line describing the taken action.
async fn update_alert(
    ctx: &Context,
    component: &serenity::all::ComponentInteraction,
    status: String,
) {
    component
        .create_response(
            ctx,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .content(status)
                    .components(vec![]),
            ),
        )
        .await
        .map_err(|e| log!("Couldn't update spam alert: {:?}", e))
        .ok();
}

/// Sends a private (ephemeral) response to the interacting user.
async fn respond_ephemeral(
    ctx: &Context,
    component: &serenity::all::ComponentInteraction,
    content: &str,
) {
    component
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
        .map_err(|e| log!("Couldn't respond to interaction: {:?}", e))
        .ok();
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
