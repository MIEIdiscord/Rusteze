//! In-memory spam detection.
//!
//! A spammer typically posts the same image into several channels within a few
//! seconds. This module keeps a short-lived, per-user history of image posts and
//! flags a user when the *same* image shows up across enough distinct channels
//! inside a small time window. Flagged users' messages are remembered so a
//! moderator can delete them (and kick the user) with a single button press.

use serenity::{
    model::{
        channel::Message,
        id::{ChannelId, MessageId, UserId},
    },
    prelude::{RwLock, TypeMapKey},
};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

/// Time window within which repeated cross-channel posts count as spam.
pub const SPAM_WINDOW: Duration = Duration::from_secs(60);

/// Number of distinct channels the same image must appear in to be flagged.
pub const SPAM_CHANNEL_THRESHOLD: usize = 3;

/// How long a flagged user is kept around waiting for a moderator decision
/// before their tracking state is garbage collected.
pub const PENDING_TTL: Duration = Duration::from_secs(60 * 60);

struct RecentPost {
    channel: ChannelId,
    message: MessageId,
    signature: String,
    when: Instant,
}

struct Pending {
    messages: Vec<(ChannelId, MessageId)>,
    when: Instant,
}

/// Tracks recent image posts and users currently flagged as spammers.
#[derive(Default)]
pub struct SpamTracker {
    recent: HashMap<UserId, Vec<RecentPost>>,
    pending: HashMap<UserId, Pending>,
}

/// Result of recording a new image message.
pub enum SpamOutcome {
    /// Nothing suspicious, carry on.
    Ignore,
    /// Newly detected spam. Carries info used to build the moderator alert.
    Detected {
        channels: Vec<ChannelId>,
        message_count: usize,
    },
    /// User was already flagged; the message was stored for later deletion but
    /// no new alert should be sent.
    AlreadyFlagged,
}

impl SpamTracker {
    /// Records an image message and decides whether it constitutes spam.
    pub fn record(
        &mut self,
        user: UserId,
        channel: ChannelId,
        message: MessageId,
        signature: String,
    ) -> SpamOutcome {
        let now = Instant::now();
        self.cleanup(now);

        // Already flagged: keep collecting messages so they can be purged too,
        // but don't spam the moderators with another alert.
        if let Some(pending) = self.pending.get_mut(&user) {
            pending.messages.push((channel, message));
            pending.when = now;
            return SpamOutcome::AlreadyFlagged;
        }

        let posts = self.recent.entry(user).or_default();
        posts.retain(|p| now.duration_since(p.when) <= SPAM_WINDOW);
        posts.push(RecentPost {
            channel,
            message,
            signature: signature.clone(),
            when: now,
        });

        let matching: Vec<&RecentPost> =
            posts.iter().filter(|p| p.signature == signature).collect();

        let mut channels: Vec<ChannelId> = matching.iter().map(|p| p.channel).collect();
        channels.sort();
        channels.dedup();

        if channels.len() >= SPAM_CHANNEL_THRESHOLD {
            let messages: Vec<(ChannelId, MessageId)> =
                matching.iter().map(|p| (p.channel, p.message)).collect();
            let message_count = messages.len();
            self.pending.insert(user, Pending { messages, when: now });
            self.recent.remove(&user);
            SpamOutcome::Detected {
                channels,
                message_count,
            }
        } else {
            SpamOutcome::Ignore
        }
    }

    /// Removes and returns the flagged messages for a user, if any.
    pub fn take_pending(&mut self, user: UserId) -> Option<Vec<(ChannelId, MessageId)>> {
        self.pending.remove(&user).map(|p| p.messages)
    }

    fn cleanup(&mut self, now: Instant) {
        self.recent.retain(|_, posts| {
            posts.retain(|p| now.duration_since(p.when) <= SPAM_WINDOW);
            !posts.is_empty()
        });
        self.pending
            .retain(|_, p| now.duration_since(p.when) <= PENDING_TTL);
    }
}

impl TypeMapKey for SpamTracker {
    type Value = Arc<RwLock<SpamTracker>>;
}

/// Returns `true` if the attachment looks like an image.
fn is_image(attachment: &serenity::model::channel::Attachment) -> bool {
    attachment
        .content_type
        .as_deref()
        .is_some_and(|ct| ct.starts_with("image/"))
        || attachment.height.is_some()
}

/// Builds a stable signature for the images in a message, or `None` if the
/// message carries no images. Two identical images uploaded by a spammer share
/// the same filename and byte size, which is enough to correlate them without
/// downloading the file.
pub fn image_signature(msg: &Message) -> Option<String> {
    let mut parts: Vec<String> = msg
        .attachments
        .iter()
        .filter(|a| is_image(a))
        .map(|a| format!("{}:{}", a.filename, a.size))
        .collect();
    if parts.is_empty() {
        return None;
    }
    parts.sort();
    Some(parts.join("|"))
}

/// URL of the first image attachment, for showing a preview in the alert.
pub fn first_image_url(msg: &Message) -> Option<String> {
    msg.attachments
        .iter()
        .find(|a| is_image(a))
        .map(|a| a.url.clone())
}
