use crate::channels::MiEI;
use futures::{
    future,
    stream::{self, StreamExt},
};
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{
        channel::Message,
        id::{GuildId, RoleId},
        user::User,
    },
    prelude::*,
    utils::Colour,
};
use std::collections::BTreeMap;

#[group]
#[commands(study, unstudy)]
struct Study;

#[command]
#[description("Permite a alguém juntar-se às salas das cadeiras.")]
#[usage("[CADEIRA|ANO|ANOSEMESTRE, ...]")]
#[example("Algebra PI")]
#[example("1ano")]
#[example("2ano1sem")]
pub async fn study(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let trash = ctx.data.read().await;
    let roles = trash.get::<MiEI>().unwrap().read().await;
    let (ids, names) = parse_study_args(
        args.rest(),
        &*roles,
        &msg.author,
        &ctx,
        msg.guild_id.ok_or("Guild id not found")?,
        true,
    )
    .await;
    if names.is_empty() {
        msg.channel_id
            .say(&ctx.http, "Não foste adicionado(a) a nenhuma cadeira nova.")
            .await?;
    } else {
        msg.member(&ctx)
            .await?
            .add_roles(&ctx.http, ids.as_slice())
            .await?;
        msg.channel_id
            .say(&ctx.http, format!("Studying {}", names.join(" ")))
            .await?;
    }
    Ok(())
}

#[command]
#[description("Permite a alguém sair das salas das cadeiras.")]
#[usage("[CADEIRA|ANO|ANOSEMESTRE, ...]")]
#[example("Algebra PI")]
#[example("1ano")]
#[example("2ano1sem")]
pub async fn unstudy(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let trash = ctx.data.read().await;
    let roles = trash.get::<MiEI>().unwrap().read().await;
    let (ids, names) = parse_study_args(
        args.rest(),
        &*roles,
        &msg.author,
        &ctx,
        msg.guild_id.ok_or("Guild id not found")?,
        false,
    )
    .await;
    if names.is_empty() {
        msg.channel_id
            .say(&ctx.http, "Não foste removido(a) de nenhuma cadeira.")
            .await?;
    } else {
        msg.member(&ctx)
            .await?
            .remove_roles(&ctx.http, ids.as_slice())
            .await?;
        msg.channel_id
            .say(&ctx.http, format!("Stopped studying: {}", names.join(" ")))
            .await?;
    }
    Ok(())
}

async fn parse_study_args<'args, 'miei: 'args>(
    args: &'args str,
    roles: &'miei MiEI,
    user: &'_ User,
    ctx: &Context,
    guild_id: GuildId,
    filter: bool,
) -> (Vec<RoleId>, Vec<&'args str>) {
    static REGEX: Lazy<Regex> = Lazy::new(|| {
        RegexBuilder::new(concat!(
            r"(",
            r"(?P<year>\d+) *ano(( *(?P<sem>\d+)( *sem(estre)?)?)| |$)|",
            r"(?P<wildcard>[^ *]+)\*|",
            r"(?P<course>\S+)",
            r")*"
        ))
        .case_insensitive(true)
        .build()
        .unwrap()
    });
    let mut names = Vec::new();
    let mut ids = Vec::new();
    let mut push = |(n, r)| {
        ids.push(r);
        names.push(n);
    };
    async fn not_has_role(
        ctx: &Context,
        guild_id: GuildId,
        r: RoleId,
        user: &User,
        filter: bool,
    ) -> bool {
        !filter || !user.has_role(ctx, guild_id, r).await.unwrap_or(true)
    }
    let captures = REGEX.captures_iter(args).collect::<Vec<_>>();
    for c in captures {
        if let Some(wild) = c.name("wildcard") {
            stream::iter(roles.wildcard_roles(wild.as_str()))
                .filter(|r| not_has_role(ctx, guild_id, r.1, user, filter))
                .for_each(|x| future::ready(push(x)))
                .await;
        } else if let Some(course) = c.name("course") {
            stream::iter(
                roles
                    .role_by_name(course.as_str())
                    .map(|r| (course.as_str(), r)),
            )
            .filter(|r| not_has_role(ctx, guild_id, r.1, user, filter))
            .for_each(|x| future::ready(push(x)))
            .await;
        } else {
            if let Some(year) = c.name("year") {
                match c.name("sem") {
                    Some(sem) => {
                        if let Some(rs) =
                            roles.roles_by_year_and_semester(year.as_str(), sem.as_str())
                        {
                            stream::iter(rs)
                                .filter(|r| not_has_role(ctx, guild_id, r.1, user, filter))
                                .for_each(|x| future::ready(push(x)))
                                .await;
                        }
                    }
                    None => {
                        if let Some(rs) = roles.roles_by_year(year.as_str()) {
                            stream::iter(rs)
                                .filter(|r| not_has_role(ctx, guild_id, r.1, user, filter))
                                .for_each(|x| future::ready(push(x)))
                                .await;
                        }
                    }
                }
            }
        }
    }
    (ids, names)
}

#[group]
#[prefixes("courses")]
#[commands(mk, rm, list)]
struct Courses;

#[command]
#[description("Cria salas das cadeiras especificadas, associadas ao ano especificado.")]
#[usage("ano semestre [CADEIRA, ...]")]
#[min_args(3)]
#[required_permissions(ADMINISTRATOR)]
pub async fn mk(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let trash = ctx.data.write().await;
    let mut roles = trash.get::<MiEI>().unwrap().write().await;
    let mut args = args.raw();
    let year = args.next();
    let semester = args.next();
    if let (Some(y), Some(s), Some(g)) = (year, semester, msg.guild_id) {
        let mut new_roles = Vec::new();
        for course in args {
            if let Some(c) = roles.create_role(ctx, &y, &s, course, g).await? {
                new_roles.push(c);
            }
        }
        if new_roles.is_empty() {
            msg.channel_id
                .say(&ctx.http, "Não foram criadas novas cadeiras.")
                .await?;
        } else {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Cadeiras criadas: {}", new_roles.join(" ")),
                )
                .await?;
        }
    }
    Ok(())
}

#[command]
#[description("Remove salas das cadeiras especificadas.")]
#[usage("[CADEIRA, ...]")]
#[required_permissions(ADMINISTRATOR)]
pub async fn rm(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let trash = ctx.data.write().await;
    let mut roles = trash.get::<MiEI>().unwrap().write().await;
    if let Some(guild) = msg.guild_id {
        let mut rm_roles = Vec::new();
        for course in args.raw() {
            if let Ok(c) = roles.remove_role(course, &ctx, guild).await {
                rm_roles.push(c);
            }
        }
        if rm_roles.is_empty() {
            msg.channel_id
                .say(&ctx.http, "Não foram removidas cadeiras.")
                .await?;
        } else {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Cadeiras removidas: {}", rm_roles.join(" ")),
                )
                .await?;
        }
    }
    Ok(())
}

#[command]
#[description("Lista as cadeiras disponíveis.")]
#[usage("")]
pub async fn list(ctx: &Context, msg: &Message) -> CommandResult {
    let trash = ctx.data.read().await;
    let roles = trash.get::<MiEI>().unwrap().read().await;

    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title("Informação sobre as cadeiras disponíveis")
                    .description(
                        "`$study CADEIRA` junta-te às salas das cadeiras.
`$study Xano` junta-te a todas as cadeiras de um ano.",
                    )
                    .fields(
                        roles
                            .iter()
                            .fold(BTreeMap::new(), |mut acc, c| {
                                let s = acc
                                    .entry(format!("{}ano{}semestre", c.year, c.semester))
                                    .or_insert_with(String::new);
                                s.push_str(c.channel);
                                s.push_str("\n");
                                acc
                            })
                            .iter()
                            .map(|(k, v)| (k, v, true)),
                    )
                    .colour(Colour::from_rgb(0, 0, 0))
            });
            m
        })
        .await?;

    Ok(())
}
