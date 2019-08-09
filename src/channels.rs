use regex::Regex;
use serde::{Deserialize, Serialize};
use serenity::model::id::{ChannelId, GuildId, RoleId};
use serenity::prelude::Context;
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::{BufReader, BufWriter};

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
        let years = &self.courses;
        lazy_static! {
            static ref REGEX: Regex = Regex::new("([0-9]+)(?i)ano([0-9]+)((?i)semestre|sem)").unwrap();
            static ref YEAR_REGEX: Regex = Regex::new("([0-9])+ANO").unwrap();
        };
        if REGEX.is_match(role_name) {
            let splits = REGEX.captures(role_name).unwrap();
            match years.get(&splits[1]) {
                Some(x) => x.get_semester_roles(&splits[2]),
                None => Vec::new(),
            }
        } else if YEAR_REGEX.is_match(role_name) {
            let splits = YEAR_REGEX.captures(role_name).unwrap();
            match years.get(&splits[1]) {
                Some(x) => x.get_year_roles(),
                None => Vec::new(),
            }
        } else {
            years
                .values()
                .flat_map(|x| x.get_role(&role_name.to_uppercase()))
                .collect::<Vec<RoleId>>()
        }
    }

    fn role_exists(&self, role_name: &str) -> bool {
        self.courses.values().any(|x| x.role_exists(role_name))
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
    courses: HashMap<String, Semester>,
}

impl Year {
    fn role_exists(&self, role_name: &str) -> bool {
        self.courses
            .values()
            .any(|x| x.courses.contains_key(role_name))
    }

    fn get_semester_roles(&self, semester: &str) -> Vec<RoleId> {
        match self.courses.get(semester) {
            Some(x) => x.courses.values().map(|z| z.role).collect::<Vec<RoleId>>(),
            None => Vec::new(),
        }
    }

    fn get_year_roles(&self) -> Vec<RoleId> {
        self.courses
            .values()
            .flat_map(|x| x.courses.values().map(|z| z.role))
            .collect::<Vec<RoleId>>()
    }

    fn get_role(&self, role_name: &str) -> Vec<RoleId> {
        self.courses
            .values()
            .filter_map(|x| x.courses.get(role_name))
            .map(|x| x.role)
            .collect::<Vec<RoleId>>()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Semester {
    #[serde(flatten)]
    courses: HashMap<String, Course>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Course {
    role: RoleId,
    channels: Vec<ChannelId>,
}

pub fn read_courses() -> io::Result<MiEI> {
    let file = File::open("config.json")?;
    let reader = BufReader::new(file);

    let u = serde_json::from_reader(reader)?;
    Ok(u)
}
