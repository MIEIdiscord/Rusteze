
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serenity::{
    framework::standard::{
        macros::{check, command, group},
        Args, CheckResult, CommandOptions, CommandResult, Reason,
    },
    http::{CacheHttp, Http},
    model::{
        channel::{ChannelType, Message, PermissionOverwrite, PermissionOverwriteType},
        id::{ChannelId, GuildId, RoleId, UserId},
        misc::Mentionable,
        permissions::Permissions,
    },
    prelude::*,
};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{self, BufWriter},
    iter::once,
    sync::{Arc, RwLock},
};

#[group]
#[prefixes("usermod")]
struct UserMod;

#[command("-a")]
#[description("Join a role")]
#[usage("[role_name, ...]")]
pub fn join(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    Ok(())
}

#[command("-d")]
#[description("Leave a role")]
#[usage("[role_name, ...]")]
pub fn leave(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    Ok(())
}

#[command("-l")]
#[description("Leave a role")]
#[usage("[role_name, ...]")]
pub fn list(ctx: &mut Context, msg: &Message) -> CommandResult {
    Ok(())
}
