pub mod config;
pub mod core;
pub mod database;
pub mod history;
pub mod library;
pub mod logging;
pub mod playback;
pub mod ranking;
pub mod recommendations;

pub use config::AppConfig;
pub use core::{
    AppError, AppResult, Command, CommandResponse, CoreProcessor, Event, EventBus, Query,
    QueryResponse,
};
