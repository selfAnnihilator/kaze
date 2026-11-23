//! Platform split: `shared` is common to all targets, `desktop` holds the
//! desktop-only shell integrations.

pub mod media_bridge;
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub mod saf_path;
pub mod shared;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod desktop;

#[cfg(target_os = "android")]
pub mod android;
