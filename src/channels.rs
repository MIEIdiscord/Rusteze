use serde::{Deserialize, Serialize};
use std::io;
use std::fs;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::BufWriter;
use serenity::model::id::RoleId;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct MiEI {
    courses: HashMap<String, Year>,
}

impl MiEI {
    fn write_courses(&self) -> Result<(), io::Error> {
        let file = OpenOptions::new().write(true).truncate(true).open("config.json")?;
        let mut writer = BufWriter::new(&file);
        serde_json::to_writer(writer, &self)?;
		Ok(())
    }

    fn get_role_id(&self, role_name: &str) -> Result<String, io::Error> {
        let role_id = &self.courses.values()
            .map(|x| x.courses.get(role_name))
            .filter(|x| x.is_some())
            .take(1)
            .collect::<Vec<Option<&Course>>>()
            .pop()
            .unwrap_or(None)
            .map(|x| x.role.to_string())
            .unwrap_or(String::from(""));
        Ok(role_id.to_string())
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


pub fn readCourses() -> io::Result<MiEI> {
    let str = fs::read_to_string("config.json")?;

    let db = serde_json::from_str::<MiEI>(&str).unwrap();
    db.get_role_id("TFB");
    Ok(db)
}

