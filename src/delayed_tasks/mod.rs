use chrono::{DateTime, Utc};
use futures::future::FutureExt;
use serenity::{
    async_trait,
    prelude::{TypeMap, TypeMapKey},
};
use std::{any::Any, error::Error, fs::File, sync::Arc};
use tokio::{
    self,
    sync::{
        mpsc::{channel, error::SendError, Receiver, Sender},
        Mutex, Notify,
    },
    time::timeout,
};

impl TypeMapKey for TaskSender {
    type Value = Self;
}

const TASKS_PATH: &str = "tasks.json";

#[typetag::serde(tag = "type")]
#[async_trait]
pub trait Task: Send + Sync + Any + 'static {
    fn when(&self) -> DateTime<Utc>;
    async fn call(&mut self, user_data: &mut TypeMap) -> Result<(), Box<dyn Error>>;
    fn is_diferent(&self, _: &dyn Any) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any;
}

pub struct TaskSender {
    channel: Sender<Box<dyn Task>>,
    notify: Arc<Notify>,
}

impl TaskSender {
    fn new(channel: Sender<Box<dyn Task>>, notify: Arc<Notify>) -> Self {
        Self { channel, notify }
    }

    pub async fn send(&self, task: Box<dyn Task>) -> Result<(), SendError<Box<dyn Task>>> {
        self.channel.send(task).await?;
        crate::log!("Sent a new task, notifying");
        self.notify.notify_one();
        Ok(())
    }
}

pub struct DelayedTasks {
    channel: Receiver<Box<dyn Task>>,
    user_data: Arc<Mutex<TypeMap>>,
    tasks: Vec<Box<dyn Task>>,
    notify: Arc<Notify>,
}

impl DelayedTasks {
    fn new(channel: Receiver<Box<dyn Task>>, t: TypeMap) -> std::io::Result<Self> {
        let tasks = File::open(TASKS_PATH)
            .map_err(|e| crate::log!("Failed to open tasks file for {:?} using empty vec", e))
            .and_then(|f| {
                serde_json::from_reader(f)
                    .map_err(|e| crate::log!("Error parsing {}: {}", TASKS_PATH, e))
            })
            .unwrap_or_else(|_| Vec::new());
        Ok(Self {
            channel,
            user_data: Arc::new(Mutex::new(t)),
            tasks,
            notify: Arc::new(Notify::new()),
        })
    }

    fn receive(&mut self) -> bool {
        loop {
            match self.channel.recv().now_or_never() {
                Some(Some(task)) => {
                    self.tasks.retain(|x| x.is_diferent(task.as_any()));
                    self.tasks.push(task)
                },
                None => break true,
                Some(None) => break false,
            }
        }
    }

    fn run_tasks(&mut self) {
        let mut i = 0;
        while i < self.tasks.len() {
            if self.tasks[i].when() < Utc::now() {
                let mut t = self.tasks.remove(i);
                let data = Arc::clone(&self.user_data);
                tokio::spawn(async move {
                    let _ = t
                        .call(&mut *data.lock().await)
                        .await
                        .map_err(|e| crate::log!("{}", e));
                });
            } else {
                i += 1;
            }
        }
        self.serialize();
    }

    fn serialize(&self) {
        File::create(TASKS_PATH)
            .and_then(|d| serde_json::to_writer_pretty(d, &self.tasks).map_err(|e| e.into()))
            .map_err(|e| crate::log!("{}", e))
            .ok();
    }

    async fn run(mut self) {
        while self.receive() || !self.tasks.is_empty() {
            self.run_tasks();
            match self.tasks.iter().map(|t| t.when()).min() {
                None => self.notify.notified().await,
                Some(smallest_timeout) => {
                    match smallest_timeout.signed_duration_since(Utc::now()).to_std() {
                        Ok(t) => {
                            crate::log!("Sleeping for {:?}", t);
                            let x = timeout(t, self.notify.notified()).await;
                            crate::log!(
                                "Woke because {}!",
                                if x.is_ok() {
                                    "was woken up"
                                } else {
                                    "it's time to run a task"
                                }
                            );
                        }
                        Err(e) => crate::log!("Scheduling to the past? {:?}", e),
                    }
                }
            }
        }
        crate::log!("DelayedTasks terminating");
    }
}

pub fn start(t: TypeMap) -> std::io::Result<TaskSender> {
    let (sender, receiver) = channel(5);
    let cron = DelayedTasks::new(receiver, t)?;
    let notify = Arc::clone(&cron.notify);
    tokio::spawn(async move { cron.run().await });
    Ok(TaskSender::new(sender, notify))
}
