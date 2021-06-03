pub mod minecraft;

use serenity::{prelude::TypeMapKey, CacheAndHttp};
use std::{ops::{Deref, DerefMut}, sync::Arc };
use tokio::sync::Mutex;

pub struct DaemonManager(daemons::DaemonManager<CacheAndHttp>);

impl DaemonManager {
    pub fn new(data: Arc<CacheAndHttp>) -> Self {
        data.into()
    }
}

impl From<Arc<CacheAndHttp>> for DaemonManager {
    fn from(data: Arc<CacheAndHttp>) -> Self {
        Self(data.into())
    }
}

impl TypeMapKey for DaemonManager {
    type Value = Arc<Mutex<DaemonManager>>;
}

impl Deref for DaemonManager {
    type Target = daemons::DaemonManager<CacheAndHttp>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DaemonManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
