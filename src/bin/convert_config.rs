use rusteze::config::Config;
use std::fs::File;
use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    let c = serde_json::from_reader::<_, Config>(File::open("config.json")?)?;
    let c = toml::to_string(&c).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    File::create("config.toml")?.write_all(c.as_bytes())?;
    let mut s = String::new();
    File::open("config.toml")?.read_to_string(&mut s)?;
    let c = toml::from_str::<Config>(&s).unwrap();
    serde_json::to_writer_pretty(File::create("config2.json")?, &c)?;
    Ok(())
}
