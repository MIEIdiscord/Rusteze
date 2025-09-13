#![deny(unused_must_use)]
#![expect(deprecated)] // standard framework is deprecated

use rusteze::{
    channels::{MiEI, read_courses},
    commands::{admin::*, cesium::*, misc::*, study::*, usermod::*},
    config::Config,
    daemons::minecraft::Minecraft,
    delayed_tasks::TaskSender,
    util::Endpoint,
    *,
};
use serenity::{all::standard::Configuration, framework::standard::StandardFramework, prelude::*};
use std::{fs, sync::Arc};

#[tokio::main]
async fn main() {
    let token = match fs::read_to_string("auth") {
        Ok(token) => token,
        Err(e) => {
            log!("Could not open auth file");
            log!("Error: {}", e);
            std::process::exit(1);
        }
    };
    let mut client_builder = Client::builder(token, GatewayIntents::all())
        .event_handler(Handler)
        .type_map_insert::<MiEI>(Arc::new(RwLock::new(read_courses().unwrap_or_default())))
        .type_map_insert::<Config>(Arc::new(RwLock::new(Config::new().unwrap_or_default())))
        .type_map_insert::<ChannelMapping>(Arc::new(RwLock::new(
            ChannelMapping::load().unwrap_or_default(),
        )))
        .framework({
            let framework = StandardFramework::new();
            framework.configure(Configuration::new().prefix("$"));
            framework
                .before(before_hook)
                .after(after_hook)
                .on_dispatch_error(dispatch_error_hook)
                .group(&STUDY_GROUP)
                .group(&COURSES_GROUP)
                .group(&ADMIN_GROUP)
                .group(&MISC_GROUP)
                .group(&CESIUM_GROUP)
                .group(&USERMOD_GROUP)
                .help(&MY_HELP)
        });
    if let Some(id) = std::env::args()
        .skip_while(|x| x != "-r")
        .nth(1)
        .and_then(|id| id.parse::<u64>().ok())
    {
        client_builder = client_builder.type_map_insert::<UpdateNotify>(Arc::new(id))
    }
    let mut client = client_builder.await.expect("failed to start client");
    if util::minecraft_server_get(["list"]).is_ok() {
        log!("Initializing minecraft daemon");
        let mc = Arc::new(Mutex::new(Minecraft::load().unwrap_or_default()));
        let mut data = client.data.write().await;
        data.insert::<Minecraft>(Arc::clone(&mc));
        let mut dt = DaemonManager::spawn(Arc::new((client.cache.clone(), client.http.clone())));
        dt.add_daemon(mc).await;
        data.insert::<DaemonManagerKey>(Arc::new(Mutex::new(dt)));
    }
    {
        let mut tasks_data = TypeMap::new();
        tasks_data.insert::<Endpoint>(Endpoint::from(&(client.cache.clone(), client.http.clone())));
        client.data.write().await.insert::<TaskSender>(
            delayed_tasks::start(tasks_data).expect("Couldn't start delayed tasks"),
        );
    }
    if let Err(why) = client.start().await {
        log!("Client error: {:?}", why);
    }
}
