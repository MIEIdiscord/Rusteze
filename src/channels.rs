use serde::{Deserialize, Serialize};
use serenity::model::id::GuildId;
use serenity::prelude::Context;
use std::io;
use std::io::Write;
use std::fs;
use std::collections::HashMap;
use std::fs::OpenOptions;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct MiEI {
    courses: HashMap<String, Year>,
}

impl MiEI {
    fn write_courses(&self) -> Result<(), io::Error> {
        let mut file = OpenOptions::new().write(true).truncate(true).open("config.json")?;
        let str = serde_json::to_string(&self)?;
        file.write_all(str.as_bytes())?;
        file.sync_all()?;
        Ok(())
    }

    pub fn get_role_id(&self, role_name: &str) -> String {
        let role_id = &self.courses.values()
            .map(|x| x.courses.get(role_name))
            .filter(|x| x.is_some())
            .take(1)
            .collect::<Vec<Option<&Course>>>()
            .pop()
            .unwrap_or(None)
            .map(|x| x.role.to_string())
            .unwrap_or(String::from(""));
        role_id.to_string()
    }

    fn role_exists(&self, role_name: &String) -> bool {
        self.courses.values()
            .map(|x| x.courses.get(role_name))
            .any(|x| x.is_some())
    }

    pub fn create_role(&self, ctx: Context, guild: GuildId, roles: Vec<String>) -> Vec<String> {
        let new_roles = roles.iter().filter(|x| self.role_exists(x))
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
        let created_roles = new_roles.iter().map(|x| guild.create_role(&ctx.http, |z| z.hoist(false)
                                                               .mentionable(true)
                                                               .name(x)));

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
    let str = fs::read_to_string("config.json")?;

    let db = serde_json::from_str::<MiEI>(&str).unwrap();
    Ok(db)
}
