use serenity::{
    cache::Cache,
    http::{CacheHttp, Http},
    prelude::TypeMapKey,
};
use std::sync::Arc;

pub type SendSyncError = Box<dyn std::error::Error + Send + Sync>;

pub struct Endpoint {
    http: Arc<Http>,
    cache: Arc<Cache>,
}

impl From<&(Arc<Cache>, Arc<Http>)> for Endpoint {
    fn from(cache_and_http: &(Arc<Cache>, Arc<Http>)) -> Self {
        Self {
            http: cache_and_http.1.clone(),
            cache: cache_and_http.0.clone(),
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
