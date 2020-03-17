use rand::random;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{
        channel::{Channel, Message, PermissionOverwrite, PermissionOverwriteType},
        id::{ChannelId, RoleId, UserId},
        permissions::Permissions,
    },
    prelude::*,
};
use std::iter::once;

group!({
    name: "cesium",
    options: {
        allowed_roles: ["Cesium", "Sudoer"],
        prefix: "cesium",
    },
    commands: [add, remove],
});

const CESIUM_CATEGORY: &str = "418798551317872660";
const CESIUM_ROLE: &str = "689456522157359104";

#[command]
#[description("Adds a new private room")]
#[usage("[StudentMention...]")]
pub fn add(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.ok_or("Message with no guild id")?;
    args.iter::<UserId>().try_for_each(|x| x.map(|_| ()))?;
    args.restore();
    guild_id.create_channel(&ctx, |channel| {
        channel
            .name(format!("mentor-channel-{}", random::<u8>()))
            .category(CESIUM_CATEGORY.parse::<ChannelId>().unwrap())
            .permissions({
                args.iter::<UserId>()
                    .map(|ur| ur.unwrap())
                    .map(|u| PermissionOverwrite {
                        kind: PermissionOverwriteType::Member(u),
                        allow: Permissions::READ_MESSAGES,
                        deny: Permissions::empty(),
                    })
                    .chain(once(PermissionOverwrite {
                        kind: PermissionOverwriteType::Role(RoleId(guild_id.0)),
                        allow: Permissions::empty(),
                        deny: Permissions::READ_MESSAGES,
                    }))
                    .chain(once(PermissionOverwrite {
                        kind: PermissionOverwriteType::Role(CESIUM_ROLE.parse().unwrap()),
                        allow: Permissions::READ_MESSAGES,
                        deny: Permissions::empty(),
                    }))
            })
    })?;
    msg.channel_id.say(&ctx, "Room created")?;
    Ok(())
}

#[command]
#[description("Removes a new private room")]
#[usage("channel_id")]
pub fn remove(ctx: &mut Context, _: &Message, mut args: Args) -> CommandResult {
    let cesium_category = CESIUM_CATEGORY.parse::<ChannelId>().unwrap();
    args.iter::<ChannelId>()
        .filter(|cr| {
            cr.as_ref()
                .map_err(|_| ())
                .and_then(|c| {
                    c.to_channel(&ctx)
                        .map(|c| {
                            if let Channel::Guild(ch) = c {
                                ch.read()
                                    .category_id
                                    .map(|c| c == cesium_category)
                                    .unwrap_or(false)
                            } else {
                                false
                            }
                        })
                        .map_err(|_| ())
                })
                .unwrap_or(false)
        })
        .try_for_each(|ch| ch.map(|c| c.delete(&ctx)).map(|_| ()))?;
    Ok(())
}
