use serde::{Deserialize, Serialize};
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

