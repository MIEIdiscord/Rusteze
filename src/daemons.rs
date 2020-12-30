pub mod minecraft;
use futures::stream::{self, StreamExt};
use serenity::{prelude::RwLock, CacheAndHttp};
use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{
    mpsc::{self, error::TryRecvError, Sender},
    Notify,
};

#[serenity::async_trait]
pub trait Daemon {
    async fn run(&self, http: &CacheAndHttp) -> Result<(), Box<dyn Error>>;
    fn interval(&self) -> Duration;
    fn name(&self) -> String;
}

type DynDaemon = dyn Daemon + Send + Sync;

#[derive(Debug)]
pub enum DaemonThreadMsg {
    RunAll,
    RunOne(usize),
}

pub struct DaemonThread {
    notify: Arc<Notify>,
    channel: Sender<DaemonThreadMsg>,
    pub list: Vec<String>,
}

impl DaemonThread {
    pub async fn run_one(
        &mut self,
        u: usize,
    ) -> Result<(), mpsc::error::SendError<DaemonThreadMsg>> {
        self.channel.send(DaemonThreadMsg::RunOne(u)).await?;
        self.notify.notify();
        Ok(())
    }

    pub async fn run_all(&mut self) -> Result<(), mpsc::error::SendError<DaemonThreadMsg>> {
        self.channel.send(DaemonThreadMsg::RunAll).await?;
        self.notify.notify();
        Ok(())
    }
}

impl serenity::prelude::TypeMapKey for DaemonThread {
    type Value = DaemonThread;
}

pub async fn start_daemon_thread(
    daemons: Vec<Arc<RwLock<dyn Daemon + Send + Sync + 'static>>>,
    http: Arc<CacheAndHttp>,
) -> DaemonThread {
    async fn run(d: &DynDaemon, ch_http: &CacheAndHttp) {
        let _ = d
            .run(ch_http)
            .await
            .map_err(|e| crate::log!("Deamon '{}' failed: {:?}", d.name(), e));
    }
    let list = stream::iter(&daemons)
        .then(|d| async move { d.read().await.name().clone() })
        .collect::<Vec<String>>()
        .await;
    let (sx, mut rx) = mpsc::channel(512);
    let mut daemons = daemons
        .into_iter()
        .map(|d| (Instant::now(), d))
        .collect::<Vec<_>>();

    let mut next_global_run = None;
    let notify = Arc::new(Notify::new());
    let wait_to_be = Arc::clone(&notify);
    tokio::spawn(async move {
        loop {
            match rx.try_recv() {
                Ok(DaemonThreadMsg::RunAll) => {
                    stream::iter(&daemons)
                        .for_each(|(_, d)| {
                            let http = &*http;
                            async move {
                                run(&*d.read().await, http).await;
                            }
                        })
                        .await;
                }
                Ok(DaemonThreadMsg::RunOne(i)) => {
                    if let Some((_, d)) = daemons.get(i) {
                        run(&*d.read().await, &*http).await;
                    }
                }
                Err(TryRecvError::Empty) => {
                    let mut smallest_next_instant = None;
                    let now = Instant::now();
                    for (next_run, daemon) in &mut daemons {
                        if now >= *next_run {
                            let d = daemon.read().await;
                            run(&*d, &*http).await;
                            *next_run = now + d.interval();
                        }
                        if smallest_next_instant.map(|s| *next_run < s).unwrap_or(true) {
                            smallest_next_instant = Some(*next_run)
                        }
                    }
                    match smallest_next_instant {
                        Some(s) => next_global_run = Some(s),
                        None => break crate::log!("Deamon thread terminating"),
                    }
                }
                Err(_) => break crate::log!("Deamon thread terminating"),
            }
            let now = Instant::now();
            match next_global_run {
                Some(s) => {
                    let _ = tokio::time::timeout(s - now, wait_to_be.notified()).await;
                }
                None => break crate::log!("Deamon thread terminating"),
            };
        }
    });
    DaemonThread {
        notify,
        channel: sx,
        list,
    }
}
