//! Discord Bot API channel adapter.
//!
//! Uses the Discord Gateway to receive messages and send responses.

use anyhow::Result;
use tracing::{info, warn, error, debug};
use tokio::sync::mpsc;

/// Configuration for the Discord channel.
#[derive(Debug, Clone)]
pub struct DiscordConfig {
    pub bot_token: String,
    /// User IDs allowed to interact with the bot.
    pub allow_from: Vec<u64>,
}

/// Inbound message from Discord.
#[derive(Debug, Clone)]
pub struct InboundMessage {
    pub channel_id: u64,
    pub user_id: u64,
    pub text: String,
    pub message_id: u64,
}

/// Outbound message to Discord.
#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub channel_id: u64,
    pub text: String,
    pub reply_to: Option<u64>,
}

/// Dummy Discord adapter for now (to avoid dependency issues).
/// This will be implemented with serenity when the dependency is added.
pub struct DiscordAdapter;

impl DiscordAdapter {
    /// Start the Discord adapter. Returns channels for inbound/outbound messages.
    pub fn start(
        _config: DiscordConfig,
    ) -> (mpsc::UnboundedReceiver<InboundMessage>, mpsc::UnboundedSender<OutboundMessage>) {
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let (outbound_tx, _outbound_rx) = mpsc::unbounded_channel::<OutboundMessage>();

        // TODO: Implement Discord integration with serenity
        // For now, just spawn a task that logs a warning
        tokio::spawn(async move {
            warn!("Discord adapter is not yet implemented - add serenity dependency");
            // Keep the channel open but don't send any messages
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        });

        info!("Discord adapter started (stub implementation)");

        (inbound_rx, outbound_tx)
    }
}

// TODO: Full Discord implementation with serenity
// This would involve:
// 1. Add serenity dependency to Cargo.toml
// 2. Implement Discord message handling
// 3. Handle Discord Gateway events
// 4. Send responses back to Discord channels
// 
// Example structure:
// 
// use serenity::{
//     client::{Client, EventHandler},
//     model::{channel::Message, gateway::Ready},
//     prelude::*,
// };
//
// struct Handler {
//     inbound_tx: mpsc::UnboundedSender<InboundMessage>,
//     allow_from: Vec<u64>,
// }
//
// #[serenity::async_trait]
// impl EventHandler for Handler {
//     async fn message(&self, ctx: Context, msg: Message) {
//         // Filter by allow_from
//         // Convert to InboundMessage and send
//         // Handle bot mentions, DMs, etc.
//     }
// }