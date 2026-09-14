pub mod cloud;
pub mod config;
pub mod core;
pub mod database;
pub mod discovery;
pub mod downloads;
pub mod history;
pub mod library;
pub mod logging;
pub mod playback;
pub mod providers;
pub mod ranking;
pub mod recommendations;

pub use config::AppConfig;
pub use core::{
    AppError, AppResult, Command, CommandResponse, CoreProcessor, Event, EventBus, Query,
    QueryResponse,
};
