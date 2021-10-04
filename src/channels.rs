use crate::util::SendSyncError;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serenity::{
    model::{
        channel::{
            Channel as SerenityChannel, ChannelType, PermissionOverwrite,
            PermissionOverwriteType::Role,
        },
        id::{ChannelId, GuildId, RoleId},
        permissions::Permissions,
    },
    prelude::{Context, RwLock, TypeMapKey},
};
use std::{collections::HashMap, fs::File, io, sync::Arc};

const COURSES: &str = "courses.json";
const DEPRECATED_CATEGORY: ChannelId = ChannelId(618553779192856577);

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct MiEI {
    #[serde(flatten)]
    courses: HashMap<String, Year>,
    #[serde(default)]
    deprecated_courses: Vec<Course>,
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

    fn role_color(year: &str) -> u64 {
        match year {
            "1" => 0x843da4,
            "2" => 0x2ecc71,
            "3" => 0x498db,
            "4" => 0xf1c40f,
            "5" => 0x1e1e1e,
            _ => 0xffffff,
        }
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
                    z.hoist(false)
                        .mentionable(true)
                        .name(&upper_course)
                        .colour(MiEI::role_color(year))
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
            x.remove(&ctx, guild).await?;
            self.write_courses()?;
            Ok(role_name)
        } else {
            Err(serenity::Error::Other("No such role"))
        }
    }

    pub async fn move_course(
        &mut self,
        course: &str,
        new_year: &str,
        new_semester: &str,
        new_name: Option<&str>,
        ctx: &Context,
        guild: GuildId,
    ) -> anyhow::Result<String> {
        let upper_new_name = new_name.map(|n| n.to_uppercase());
        if let Some(n) = upper_new_name.as_ref().filter(|r| self.role_exists(r)) {
            Err(anyhow!("Course already exists: {}", n))
        } else if let Some(old_course) = self.courses.values_mut().find_map(|x| x.pop_role(course))
        {
            guild
                .edit_role(&ctx.http, old_course.role, |r| {
                    r.colour(MiEI::role_color(new_year))
                })
                .await?;
            if let Some(n) = upper_new_name {
                old_course.rename(&n, &ctx, guild).await?;
                self.add_role(&n, old_course, new_semester, new_year);
            } else {
                self.add_role(&course.to_uppercase(), old_course, new_semester, new_year);
            }
            self.write_courses()?;
            Ok(new_name.unwrap_or(course).to_string())
        } else {
            Err(anyhow!("No such course: {}", course))
        }
    }

    pub async fn rename_course(
        &mut self,
        course: &str,
        new_name: &str,
        ctx: &Context,
        guild: GuildId,
    ) -> anyhow::Result<String> {
        if let Some((year, semester)) = self.get_year_semester_names(course) {
            self.move_course(course, &year, &semester, Some(new_name), ctx, guild)
                .await
        } else {
            Err(anyhow!("No such course: {}", course))
        }
    }

    pub async fn deprecate_course(
        &mut self,
        course: &str,
        ctx: &Context,
        guild: GuildId,
    ) -> anyhow::Result<String> {
        if let Some(mut c) = self.courses.values_mut().find_map(|x| x.pop_role(course)) {
            c.deprecate(ctx, guild).await?;
            self.deprecated_courses.push(c);
            self.write_courses()?;
            Ok(course.to_string())
        } else {
            Err(anyhow!("No such course: {}", course))
        }
    }

    fn get_year_semester_names(&self, role_name: &str) -> Option<(String, String)> {
        let upper_role_name = role_name.to_uppercase();
        self.courses.iter().find_map(|(key, x)| {
            if let Some(sem) = x.get_semester_name(&upper_role_name) {
                Some((key.clone(), sem))
            } else {
                None
            }
        })
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

    fn get_semester_name(&self, role_name: &str) -> Option<String> {
        self.courses.iter().find_map(|(key, x)| {
            if x.courses.get(role_name).is_some() {
                Some(key.clone())
            } else {
                None
            }
        })
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
    async fn remove(&self, ctx: &Context, guild: GuildId) -> serenity::Result<()> {
        for channel in &self.channels {
            channel.delete(&ctx.http).await?;
        }
        guild.delete_role(&ctx.http, self.role).await?;
        Ok(())
    }

    async fn rename(&self, new_name: &str, ctx: &Context, guild: GuildId) -> serenity::Result<()> {
        for channel in &self.channels {
            match channel.to_channel(&ctx.http).await? {
                SerenityChannel::Guild(mut channel) => {
                    let prefix_index = channel.name.find("-").unwrap_or(channel.name.len());
                    let prefix = &channel.name[..prefix_index].to_string();
                    channel
                        .edit(&ctx.http, |c| c.name(format!("{}-{}", prefix, new_name)))
                        .await?;
                }
                SerenityChannel::Category(mut channel) => {
                    channel.edit(&ctx.http, |c| c.name(new_name)).await?;
                }
                _ => {}
            }
        }
        guild
            .edit_role(&ctx.http, self.role, |r| r.name(new_name))
            .await?;

        Ok(())
    }

    async fn deprecate(&mut self, ctx: &Context, guild: GuildId) -> anyhow::Result<()> {
        let mut role = guild
            .roles(&ctx.http)
            .await?
            .get(&self.role)
            .ok_or(anyhow!("No such role"))?
            .clone();
        let new_role = guild
            .create_role(&ctx.http, |r| {
                r.name(&role.name)
                    .colour(MiEI::role_color(""))
                    .hoist(role.hoist)
                    .mentionable(false)
                    .permissions(role.permissions)
            })
            .await?;
        role.delete(&ctx.http).await?;
        self.role = new_role.id;

        for channel in &mut self.channels {
            match channel.to_channel(&ctx.http).await? {
                SerenityChannel::Guild(mut channel) => {
                    channel
                        .id
                        .say(
                            &ctx.http,
                            "*está cadeira já não está entre nós, descansa em paz cadeira, \
                                    a tua memória será para sempre preservada \
                                    ||num datacenter qualquer do discord||*",
                        )
                        .await?;
                    channel
                        .create_permission(
                            &ctx.http,
                            &PermissionOverwrite {
                                allow: Permissions::READ_MESSAGES,
                                deny: Permissions::SEND_MESSAGES,
                                kind: Role(self.role),
                            },
                        )
                        .await?;
                    channel
                        .edit(&ctx.http, |c| c.category(DEPRECATED_CATEGORY))
                        .await?;
                }
                SerenityChannel::Category(category) => {
                    category.delete(&ctx.http).await?;
                    *channel = DEPRECATED_CATEGORY;
                }
                _ => {}
            }
        }
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
