use crate::channels::MiEI;
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
pub fn study(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let trash = ctx.data.read();
    let roles = trash.get::<MiEI>().unwrap().read();
    let (ids, names) = parse_study_args(
        args.rest(),
        &*roles,
        &msg.author,
        &ctx,
        msg.guild_id.ok_or("Guild id not found")?,
        true,
    );
    if names.is_empty() {
        msg.channel_id
            .say(&ctx.http, "Não foste adicionado(a) a nenhuma cadeira nova.")?;
    } else {
        msg.member(&ctx.cache)
            .map(|mut x| x.add_roles(&ctx.http, ids.as_slice()))
            .transpose()?;
        msg.channel_id
            .say(&ctx.http, format!("Studying {}", names.join(" ")))?;
    }
    Ok(())
}

#[command]
#[description("Permite a alguém sair das salas das cadeiras.")]
#[usage("[CADEIRA|ANO|ANOSEMESTRE, ...]")]
#[example("Algebra PI")]
#[example("1ano")]
#[example("2ano1sem")]
pub fn unstudy(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let trash = ctx.data.read();
    let roles = trash.get::<MiEI>().unwrap().read();
    let (ids, names) = parse_study_args(
        args.rest(),
        &*roles,
        &msg.author,
        &ctx,
        msg.guild_id.ok_or("Guild id not found")?,
        false,
    );
    if names.is_empty() {
        msg.channel_id
            .say(&ctx.http, "Não foste removido(a) de nenhuma cadeira.")?;
    } else {
        msg.member(&ctx.cache)
            .map(|mut x| x.remove_roles(&ctx.http, ids.as_slice()))
            .transpose()?;
        msg.channel_id
            .say(&ctx.http, format!("Stopped studying: {}", names.join(" ")))?;
    }
    Ok(())
}

fn parse_study_args<'args, 'miei: 'args>(
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
    let not_has_role = |(_, r): &_| !filter || !user.has_role(ctx, guild_id, r).unwrap_or(true);
    for c in REGEX.captures_iter(args) {
        match c.name("course") {
            Some(course) => roles
                .role_by_name(course.as_str())
                .map(|r| (course.as_str(), r))
                .filter(not_has_role)
                .map(&mut push),
            None => c.name("year").and_then(|year| match c.name("sem") {
                Some(sem) => roles
                    .roles_by_year_and_semester(year.as_str(), sem.as_str())
                    .map(|rs| rs.filter(not_has_role).for_each(&mut push)),
                None => roles
                    .roles_by_year(year.as_str())
                    .map(|rs| rs.filter(not_has_role).for_each(&mut push)),
            }),
        };
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
pub fn mk(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let trash = ctx.data.write();
    let mut roles = trash.get::<MiEI>().unwrap().write();
    let mut iter = args.raw();
    let year = iter.next();
    let semester = iter.next();
    if let (Some(y), Some(s), Some(g)) = (year, semester, msg.guild_id) {
        let new_roles = iter
            .filter_map(|x| roles.create_role(ctx, &y, &s, x, g).transpose())
            .collect::<Result<Vec<&str>, Box<dyn std::error::Error>>>()?;
        if new_roles.is_empty() {
            msg.channel_id
                .say(&ctx.http, "Não foram criadas novas cadeiras.")?;
        } else {
            msg.channel_id.say(
                &ctx.http,
                format!("Cadeiras criadas: {}", new_roles.join(" ")),
            )?;
        }
    }
    Ok(())
}

#[command]
#[description("Remove salas das cadeiras especificadas.")]
#[usage("[CADEIRA, ...]")]
#[required_permissions(ADMINISTRATOR)]
pub fn rm(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let trash = ctx.data.write();
    let mut roles = trash.get::<MiEI>().unwrap().write();
    if let Some(guild) = msg.guild_id {
        let rm_roles = args
            .raw()
            .filter_map(|x| roles.remove_role(x, &ctx, guild).ok())
            .collect::<Vec<&str>>();
        if rm_roles.is_empty() {
            msg.channel_id
                .say(&ctx.http, "Não foram removidas cadeiras.")?;
        } else {
            msg.channel_id.say(
                &ctx.http,
                format!("Cadeiras removidas: {}", rm_roles.join(" ")),
            )?;
        }
    }
    Ok(())
}

#[command]
#[description("Lista as cadeiras disponíveis.")]
#[usage("")]
pub fn list(ctx: &mut Context, msg: &Message) -> CommandResult {
    let trash = ctx.data.read();
    let roles = trash.get::<MiEI>().unwrap().read();

    msg.channel_id.send_message(&ctx.http, |m| {
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
    })?;

    Ok(())
}
