use crate::util::SendSyncError;
use serde::{Deserialize, Serialize};
use serenity::{
    model::{
        channel::{ChannelType, PermissionOverwrite, PermissionOverwriteType::Role},
        id::{ChannelId, GuildId, RoleId},
        permissions::Permissions,
    },
    prelude::{Context, RwLock, TypeMapKey},
};
use std::{collections::HashMap, fs::File, io, sync::Arc};

const COURSES: &str = "courses.json";

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct MiEI {
    #[serde(flatten)]
    courses: HashMap<String, Year>,
}

impl MiEI {
    fn write_courses(&self) -> Result<(), io::Error> {
        serde_json::to_writer(File::create(COURSES)?, &self)?;
        Ok(())
    }

    pub fn role_by_name<'a>(&'a self, role_name: &'a str) -> Option<RoleId> {
        let role_name_ = role_name.to_uppercase();
        self.courses
            .values()
            .filter_map(move |x| x.get_role(&role_name_))
            .map(|x| x.role)
            .next()
    }

    pub fn wildcard_roles<'a>(
        &'a self,
        wildcard: &str,
    ) -> impl Iterator<Item = (&str, RoleId)> + 'a {
        let upper = wildcard.to_uppercase();
        self.courses
            .values()
            .flat_map(|x| x.all_roles())
            .filter(move |(n, _r)| n.starts_with(&upper))
    }

    pub fn roles_by_year(&self, year: &str) -> Option<impl Iterator<Item = (&str, RoleId)>> {
        self.courses.get(year).map(Year::all_roles)
    }

    pub fn roles_by_year_and_semester(
        &self,
        year: &str,
        semester: &str,
    ) -> Option<impl Iterator<Item = (&str, RoleId)> + '_> {
        self.courses
            .get(year)
            .and_then(|y| y.roles_by_semester(semester))
    }

    pub async fn create_role<'a>(
        &mut self,
        ctx: &Context,
        year: &str,
        semester: &str,
        course: &'a str,
        guild: GuildId,
    ) -> Result<Option<&'a str>, SendSyncError> {
        let upper_course = course.to_uppercase();
        if self.role_exists(&upper_course) {
            Ok(None)
        } else {
            let role = guild
                .create_role(&ctx.http, |z| {
                    z.hoist(false).mentionable(true).name(&upper_course)
                })
                .await
                .unwrap();
            let perms = vec![
                PermissionOverwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::READ_MESSAGES,
                    kind: Role(guild.as_u64().to_owned().into()),
                },
                PermissionOverwrite {
                    allow: Permissions::READ_MESSAGES,
                    deny: Permissions::empty(),
                    kind: Role(role.id),
                },
            ];
            let category = guild
                .create_channel(&ctx, |c| {
                    c.name(&upper_course)
                        .kind(ChannelType::Category)
                        .permissions(perms)
                })
                .await
                .unwrap();
            let duvidas = guild
                .create_channel(&ctx, |c| {
                    c.name(format!("duvidas-{}", &upper_course))
                        .kind(ChannelType::Text)
                        .category(category.id)
                })
                .await
                .unwrap();
            let anexos = guild
                .create_channel(&ctx, |c| {
                    c.name(format!("anexos-{}", &upper_course))
                        .kind(ChannelType::Text)
                        .category(category.id)
                })
                .await
                .unwrap();
            let courses = Course {
                role: role.id,
                channels: vec![category.id, anexos.id, duvidas.id],
            };
            self.add_role(&upper_course, courses, semester, year);
            self.write_courses()?;
            Ok(Some(course))
        }
    }

    fn add_role(&mut self, role_name: &str, course: Course, semester: &str, year: &str) {
        self.courses
            .entry(year.to_string())
            .or_insert(Year {
                courses: HashMap::new(),
            })
            .add_role(role_name, course, semester);
    }

    pub async fn remove_role<'a>(
        &mut self,
        role_name: &'a str,
        ctx: &Context,
        guild: GuildId,
    ) -> serenity::Result<&'a str> {
        if let Some(x) = self
            .courses
            .values_mut()
            .find_map(|x| x.pop_role(role_name))
        {
            x.remove_course(&ctx, guild).await?;
            self.write_courses()?;
            Ok(role_name)
        } else {
            Err(serenity::Error::Other("No such role"))
        }
    }

    fn role_exists(&self, role_name: &str) -> bool {
        self.courses.values().any(|x| x.role_exists(role_name))
    }

    pub fn iter(&self) -> impl Iterator<Item = Channel> {
        self.courses.iter().flat_map(|(year, sems)| {
            sems.courses.iter().flat_map(move |(semester, courses)| {
                courses.courses.keys().map(move |channel| Channel {
                    year,
                    semester,
                    channel,
                })
            })
        })
    }
}

impl TypeMapKey for MiEI {
    type Value = Arc<RwLock<MiEI>>;
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
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

    fn all_roles(&self) -> impl Iterator<Item = (&str, RoleId)> + '_ {
        self.courses
            .values()
            .flat_map(|x| x.courses.iter().map(|(a, z)| (a.as_str(), z.role)))
    }

    fn roles_by_semester(
        &self,
        semester: &str,
    ) -> Option<impl Iterator<Item = (&str, RoleId)> + '_> {
        self.courses
            .get(semester)
            .map(|x| x.courses.iter().map(|(a, c)| (a.as_str(), c.role)))
    }

    fn get_role<'a>(&self, role_name: &'a str) -> Option<&Course> {
        self.courses.values().find_map(|x| x.courses.get(role_name))
    }

    fn add_role(&mut self, role_name: &str, course: Course, semester: &str) {
        self.courses
            .entry(semester.to_string())
            .or_insert(Semester {
                courses: HashMap::new(),
            })
            .courses
            .insert(role_name.to_string(), course);
    }

    fn pop_role(&mut self, role_name: &str) -> Option<Course> {
        self.courses
            .values_mut()
            .find_map(|x| x.courses.remove(&role_name.to_uppercase()))
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
struct Semester {
    #[serde(flatten)]
    courses: HashMap<String, Course>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
struct Course {
    role: RoleId,
    channels: Vec<ChannelId>,
}

impl Course {
    async fn remove_course(&self, ctx: &Context, guild: GuildId) -> serenity::Result<()> {
        for channel in &self.channels {
            channel.delete(&ctx.http).await?;
        }
        guild.delete_role(&ctx.http, self.role).await?;
        Ok(())
    }
}

pub fn read_courses() -> io::Result<MiEI> {
    Ok(serde_json::from_reader(File::open(COURSES)?)?)
}

pub struct Channel<'a> {
    pub channel: &'a str,
    pub semester: &'a str,
    pub year: &'a str,
}
