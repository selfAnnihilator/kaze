use music_player_backend::config::AppConfig;
use music_player_backend::core::command::Command;
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::database::init_db_pool;
use music_player_backend::logging::init_logging;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    info!("Starting Intelligent Local Music Player Backend (Headless / Core Daemon)");

    let config = AppConfig::default_with_dirs();
    info!(database = ?config.database_path, "Loaded application configuration");

    let pool = init_db_pool(&config.database_path).await?;
    let processor = CoreProcessor::new(pool, config);

    info!("CoreProcessor initialized and ready for commands");

    // Execute sample command
    let response = processor
        .dispatch_command(Command::SetVolume { volume: 0.75 })
        .await?;
    info!(?response, "Volume command executed successfully");

    Ok(())
}
