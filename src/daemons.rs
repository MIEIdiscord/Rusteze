pub mod minecraft;

use serenity::{
    all::{Cache, Http},
    prelude::TypeMapKey,
};
use std::sync::Arc;
use tokio::sync::Mutex;

daemons::monomorphise!((Arc<Cache>, Arc<Http>));

pub struct DaemonManagerKey;

impl TypeMapKey for DaemonManagerKey {
    type Value = Arc<Mutex<DaemonManager>>;
}
