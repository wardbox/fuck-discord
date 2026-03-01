use relay_server::{auth, db, handlers, state};

use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "relay", about = "Self-hosted encrypted chat server")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Port to listen on
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Database path
    #[arg(short, long, default_value = "relay.db")]
    database: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Create an invite code
    Invite {
        /// Maximum number of uses (omit for unlimited)
        #[arg(short, long)]
        max_uses: Option<i64>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "relay_server=debug,tower_http=debug".into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    // Initialize database
    let pool = db::create_pool(&cli.database)?;
    db::run_migrations(&pool)?;

    match cli.command {
        Some(Commands::Invite { max_uses }) => {
            let conn = pool.get()?;
            // For CLI invite creation, we need the first user (server owner)
            // or we create a system invite
            let code = auth::invite::create_invite_code(&conn, "system", max_uses, None)?;
            println!("Invite code: {code}");
            println!("Share this with someone to let them register.");
            return Ok(());
        }
        None => {}
    }

    // Create app state
    let uploads_dir = std::path::PathBuf::from("uploads");
    let app_state = state::AppState::new(pool, uploads_dir);

    // Seed default channels
    {
        let conn = app_state.db.get()?;
        db::channels::seed_defaults(&conn)?;
    }

    // Build router
    let app = handlers::router(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    tracing::info!("Relay server listening on {}", addr);
    tracing::info!("Open http://localhost:{} in your browser", cli.port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
