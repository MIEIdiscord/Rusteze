pub mod minecraft;

use serenity::{prelude::TypeMapKey, CacheAndHttp};
use std::{ops::{Deref, DerefMut}, sync::Arc };
use tokio::sync::Mutex;

pub struct DaemonThread(daemons::DaemonThread<CacheAndHttp>);

impl From<Arc<CacheAndHttp>> for DaemonThread {
    fn from(data: Arc<CacheAndHttp>) -> Self {
        Self(data.into())
    }
}

impl TypeMapKey for DaemonThread {
    type Value = Arc<Mutex<DaemonThread>>;
}

impl Deref for DaemonThread {
    type Target = daemons::DaemonThread<CacheAndHttp>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DaemonThread {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
