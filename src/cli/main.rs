mod client;
mod commands;
mod identity;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pan-cli", about = "PAN — Physical Anchor Network CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage local identity (create, show)
    Identity {
        #[command(subcommand)]
        action: IdentityCmd,
    },
    /// Manage PAN nodes
    Node {
        #[command(subcommand)]
        action: NodeCmd,
    },
    /// Record presence at a PAN node
    Presence {
        /// PAN node ID
        #[arg(long)]
        node: String,
    },
    /// Create a presence/work event
    Event {
        #[command(subcommand)]
        action: EventCmd,
    },
    /// Confirm an existing event
    Confirm {
        /// Event ID to confirm
        #[arg(long)]
        event: String,
        /// Optional description
        #[arg(long)]
        content: Option<String>,
        /// Optional comma-separated tags
        #[arg(long)]
        tags: Option<String>,
    },
    /// Query event history
    History {
        #[command(subcommand)]
        action: HistoryCmd,
    },
}

#[derive(Subcommand)]
enum IdentityCmd {
    /// Generate a keypair, register with server, and save identity locally
    Create {
        /// Phone number (non-interactive; omit to be prompted)
        #[arg(long)]
        phone: Option<String>,
        /// Server URL (non-interactive; omit to be prompted)
        #[arg(long)]
        server: Option<String>,
    },
    /// Print actor_id, pubkey, and server URL (secret key is never printed)
    Show,
}

#[derive(Subcommand)]
enum NodeCmd {
    /// Place a new PAN node at the given coordinates
    Place {
        #[arg(long, allow_hyphen_values = true)]
        lat: f64,
        #[arg(long, allow_hyphen_values = true)]
        lon: f64,
        /// Radius in miles (default: 1.0)
        #[arg(long, default_value = "1.0")]
        radius: f64,
        /// Node type: fixed or ephemeral (default: fixed)
        #[arg(long, default_value = "fixed")]
        r#type: String,
    },
}

#[derive(Subcommand)]
enum EventCmd {
    /// Create a new presence/work event (PresenceRecorded)
    Create {
        #[arg(long)]
        content: String,
        /// Comma-separated tags, e.g. "plumbing,home_repair"
        #[arg(long)]
        tags: Option<String>,
        /// Entity ID (actor_id or pan_id); defaults to own actor_id
        #[arg(long)]
        entity: Option<String>,
    },
}

#[derive(Subcommand)]
enum HistoryCmd {
    /// Show event history for an actor
    Actor {
        /// Actor ID (defaults to own identity)
        #[arg(long)]
        id: Option<String>,
    },
    /// Show event history for a node
    Node {
        #[arg(long)]
        id: String,
        /// Start timestamp in millisecond epoch
        #[arg(long)]
        from: Option<i64>,
        /// End timestamp in millisecond epoch
        #[arg(long)]
        to: Option<i64>,
        /// Filter by event type (e.g. presence_recorded)
        #[arg(long, name = "type")]
        event_type: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli.command).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run(command: Command) -> Result<()> {
    match command {
        Command::Identity { action } => match action {
            IdentityCmd::Create { phone, server } => {
                identity::create(phone.as_deref(), server.as_deref()).await
            }
            IdentityCmd::Show => identity::show(),
        },

        Command::Node { action } => match action {
            NodeCmd::Place { lat, lon, radius, r#type } => {
                commands::node::place(lat, lon, radius, &r#type).await
            }
        },

        Command::Presence { node } => commands::presence::record(&node).await,

        Command::Event { action } => match action {
            EventCmd::Create { content, tags, entity } => {
                commands::event::create(&content, tags.as_deref(), entity.as_deref()).await
            }
        },

        Command::Confirm { event, content, tags } => {
            commands::confirm::confirm(&event, content.as_deref(), tags.as_deref()).await
        }

        Command::History { action } => match action {
            HistoryCmd::Actor { id } => commands::history::actor(id.as_deref()).await,
            HistoryCmd::Node { id, from, to, event_type } => {
                commands::history::node(&id, from, to, event_type.as_deref()).await
            }
        },
    }
}
