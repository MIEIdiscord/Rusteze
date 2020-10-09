use std::{
    io,
    process::{Command, Output},
};

pub fn minecraft_server_get<I, S>(args: I) -> io::Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut output = Command::new("./server_do.sh")
        .args(args)
        .spawn()?
        .wait_with_output()?;
    let o_len = output.stdout.len();
    output.stdout.truncate(o_len - 5);
    Ok(output)
}
