#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn main() {
    music_player_backend::platform::desktop::preconfigure();
    music_player_backend::run();
}

// Mobile entry is the cdylib `run()`; the bin target is a no-op there.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn main() {}
