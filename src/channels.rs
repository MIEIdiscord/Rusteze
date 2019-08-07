use serde::{Deserialize, Serialize};
use serenity::model::id::{GuildId, RoleId};
use serenity::prelude::Context;
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::{BufWriter, BufReader};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct MiEI {
    courses: HashMap<String, Year>,
}

impl MiEI {
    #[allow(dead_code)]
    fn write_courses(&self) -> Result<(), io::Error> {
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open("config.json")?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &self)?;
        Ok(())
    }

    pub fn get_role_id(&self, role_name: &str) -> Vec<RoleId> {
        self.courses
            .values()
            .filter_map(|x| x.courses.get(role_name))
            .map(|x| x.role.parse::<RoleId>().unwrap())
            .collect::<Vec<RoleId>>()
    }

    fn role_exists(&self, role_name: &str) -> bool {
        self.courses
            .values()
            .any(|x| x.courses
                 .contains_key(role_name))
   }

    pub fn create_role(&self, ctx: Context, guild: GuildId, roles: Vec<String>) -> Vec<String> {
        let new_roles = roles
            .iter()
            .filter(|x| self.role_exists(x))
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
        let created_roles = new_roles
            .iter()
            .map(|x| guild.create_role(&ctx.http, |z| z.hoist(false).mentionable(true).name(x)));
        new_roles
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Year {
    #[serde(flatten)]
    courses: HashMap<String, Course>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Course {
    role: String,
    channels: Vec<String>,
}

pub fn read_courses() -> io::Result<MiEI> {
    let file = File::open("config.json")?;
    let reader = BufReader::new(file);

    let u = serde_json::from_reader(reader)?;

    Ok(u)
}
