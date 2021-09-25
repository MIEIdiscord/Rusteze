pub mod minecraft;

use serenity::{prelude::TypeMapKey, CacheAndHttp};
use std::sync::Arc;
use tokio::sync::Mutex;

daemons::monomorphise!(CacheAndHttp);

impl TypeMapKey for DaemonManager {
    type Value = Arc<Mutex<DaemonManager>>;
}
