pub mod config;
pub mod core;
pub mod database;
pub mod library;
pub mod logging;
pub mod playback;

pub use config::AppConfig;
pub use core::{
    AppError, AppResult, Command, CommandResponse, CoreProcessor, Event, EventBus, Query,
    QueryResponse,
};
