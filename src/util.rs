use std::{
    io,
    process::{Command, Output, Stdio},
};

pub fn minecraft_server_get<I, S>(args: I) -> io::Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut output = Command::new("./server_do.sh")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()?;
    let o_len = output.stdout.len();
    output.stdout.truncate(o_len.saturating_sub(5));
    if output.status.success() {
        Ok(output)
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr),
        ))
    }
}
