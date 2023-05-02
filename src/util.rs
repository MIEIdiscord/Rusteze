use serenity::{
    cache::Cache,
    http::{CacheHttp, Http},
    prelude::TypeMapKey,
};
use std::{
    io,
    process::{Command, Output, Stdio},
    sync::Arc,
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

pub type SendSyncError = Box<dyn std::error::Error + Send + Sync>;

pub struct Endpoint {
    http: Arc<Http>,
    cache: Arc<Cache>,
}

impl From<&serenity::CacheAndHttp> for Endpoint {
    fn from(cache_and_http: &serenity::CacheAndHttp) -> Self {
        Self {
            http: cache_and_http.http.clone(),
            cache: cache_and_http.cache.clone(),
        }
    }
}

impl CacheHttp for Endpoint {
    fn http(&self) -> &Http {
        &self.http
    }

    fn cache(&self) -> Option<&Arc<Cache>> {
        Some(&self.cache)
    }
}

impl AsRef<Http> for Endpoint {
    fn as_ref(&self) -> &Http {
        &self.http
    }
}

impl TypeMapKey for Endpoint {
    type Value = Endpoint;
}

#[macro_export]
macro_rules! get {
    ($ctx:ident, $t:ty) => {
        $ctx.data.read().await.get::<$t>().expect(::std::concat!(
            ::std::stringify!($t),
            " was not initialized"
        ))
    };
    (mut $ctx:ident, $t:ty) => {
        $ctx.data
            .write()
            .await
            .get_mut::<$t>()
            .expect(::std::concat!(
                ::std::stringify!($t),
                " was not initialized"
            ))
    };
    ($ctx:ident, $t:ty, $lock:ident) => {
        $ctx.data
            .read()
            .await
            .get::<$t>()
            .expect(::std::concat!(
                ::std::stringify!($t),
                " was not initialized"
            ))
            .$lock()
            .await
    };
    (mut $ctx:ident, $t:ty, $lock:ident) => {
        $ctx.data
            .write()
            .await
            .get_mut::<$t>()
            .expect(::std::concat!(
                ::std::stringify!($t),
                " was not initialized"
            ))
            .$lock()
            .await
    };
    (> $data:ident, $t:ty) => {
        $data.get::<$t>().expect(::std::concat!(
            ::std::stringify!($t),
            " was not initialized"
        ))
    };
    (> $data:ident, $t:ty, $lock:ident) => {
        $data
            .get::<$t>()
            .expect(::std::concat!(
                ::std::stringify!($t),
                " was not initialized"
            ))
            .$lock()
            .await
    };
}
