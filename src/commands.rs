pub mod admin;
pub mod cesium;

use crate::channels::MiEI;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{channel::Message, id::RoleId},
    prelude::*,
    utils::Colour,
};
use std::{process::Command as Fork, collections::BTreeMap};

group!({
    name: "study",
    options: {},
    commands: [study, unstudy],
});

group!({
    name: "Misc",
    options: {},
    commands: [online, ping, info, material],
});

#[command]
#[description("Teste de conectividade entre o Bot e os servidores do Discord.")]
pub fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong but in <:rust:530449316607688724>!")?;
    Ok(())
}

#[command]
#[description(
    "Informação relativa à linguagem de programação utilizada para desenvolvimento do Bot."
)]
pub fn info(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Powered by JAVA8™")?;
    Ok(())
}

#[command]
#[description("Apresenta o link para o material de apoio do curso.")]
#[usage("")]
pub fn material(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(
        &ctx.http,
        "**Este é o link para o material do curso** -> http://bit.ly/materialmiei",
    )?;
    Ok(())
}

#[command]
#[description("Shows the list of online players")]
#[usage("")]
pub fn online(ctx: &mut Context, msg: &Message) -> CommandResult {
    let output = Fork::new("./server_do.sh").args(&["list"]).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    msg.channel_id.say(&ctx, format!("{}{}", stdout, stderr))?;
    Ok(())
}

#[command]
#[description("Permite a alguém juntar-se às salas das cadeiras.")]
#[usage("[CADEIRA|ANO|ANOSEMESTRE, ...]")]
#[example("Algebra PI")]
#[example("1ano")]
#[example("2ano1sem")]
pub fn study(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let trash = ctx.data.read();
    let roles = trash.get::<MiEI>().unwrap().read().unwrap();
    let mut names = Vec::new();
    let ids = args
        .raw()
        .map(|x| roles.get_role_id(x))
        .flatten()
        .filter(|(_, b)| {
            msg.author
                .has_role(&ctx, msg.guild_id.unwrap(), b)
                .map(|x| !x)
                .unwrap_or(false)
        })
        .map(|(a, b)| {
            names.push(a);
            b
        })
        .collect::<Vec<RoleId>>();
    msg.member(&ctx.cache)
        .map(|mut x| x.add_roles(&ctx.http, ids.as_slice()))
        .transpose()?;

    if names.is_empty() {
        msg.channel_id
            .say(&ctx.http, "Não foste adicionado(a) a nenhuma cadeira nova.")?;
    } else {
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
    let roles = trash.get::<MiEI>().unwrap().read().unwrap();
    let mut names = Vec::new();
    let ids = args
        .raw()
        .map(|x| roles.get_role_id(x))
        .flatten()
        .map(|(a, b)| {
            names.push(a);
            b
        })
        .collect::<Vec<RoleId>>();
    msg.member(&ctx.cache)
        .map(|mut x| x.remove_roles(&ctx.http, ids.as_slice()))
        .transpose()?;
    if names.is_empty() {
        msg.channel_id
            .say(&ctx.http, "Não foste removido(a) de nenhuma cadeira.")?;
    } else {
        msg.channel_id
            .say(&ctx.http, format!("Stopped studying: {}", names.join(" ")))?;
    }
    Ok(())
}

group!({
    name: "courses",
    options: {
        prefixes: ["courses"],
    },
    commands: [mk, rm, list],
});

#[command]
#[description("Cria salas das cadeiras especificadas, associadas ao ano especificado.")]
#[usage("ano semestre [CADEIRA, ...]")]
#[min_args(3)]
#[required_permissions(ADMINISTRATOR)]
pub fn mk(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let trash = ctx.data.write();
    let mut roles = trash.get::<MiEI>().unwrap().write().unwrap();
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
    let mut roles = trash.get::<MiEI>().unwrap().write().unwrap();
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
    let roles = trash.get::<MiEI>().unwrap().read().unwrap();

    msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| {
            e.title("Informação sobre as cadeiras disponíveis");
            e.description(
                "`$study CADEIRA` junta-te às salas das cadeiras.
`$study Xano` junta-te a todas as cadeiras de um ano.",
            );
            e.fields(
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
            );
            e.colour(Colour::from_rgb(0, 0, 0));
            e
        });
        m
    })?;

    Ok(())
}
