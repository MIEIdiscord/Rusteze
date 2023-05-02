pub mod minecraft;

use serenity::{prelude::TypeMapKey, CacheAndHttp};
use std::sync::Arc;
use tokio::sync::Mutex;

daemons::monomorphise!(CacheAndHttp);

pub struct DaemonManagerKey;

impl TypeMapKey for DaemonManagerKey {
    type Value = Arc<Mutex<DaemonManager>>;
}
