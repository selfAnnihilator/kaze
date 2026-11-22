//! Configure WebKit before GTK, Tauri or backend threads start.
use std::ffi::OsString;

const DISABLE_DMABUF: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";

pub fn configure() {
    // Includes hybrid laptops: the primary GPU need not be the NVIDIA device
    // whose EGL libraries WebKit loads. Keep this workaround out of X11 and
    // non-NVIDIA sessions, and respect an explicit launcher/user override.
    let nvidia_loaded = std::path::Path::new("/sys/module/nvidia/version").exists();
    if needs_compatibility_renderer(nvidia_loaded, |name| std::env::var_os(name)) {
        // WebKitGTK 2.54/NVIDIA Wayland can stop delivering frames after
        // scrolling Queue and leaving it, while JS and Rust audio still run.
        // Disabling only the DMA-BUF renderer restores frame delivery; do not
        // disable compositing or GPU acceleration for the whole application.
        std::env::set_var(DISABLE_DMABUF, "1");
    }
}

fn needs_compatibility_renderer(
    nvidia_loaded: bool,
    env: impl Fn(&str) -> Option<OsString>,
) -> bool {
    if !nvidia_loaded || env(DISABLE_DMABUF).is_some() {
        return false;
    }
    if env("GDK_BACKEND")
        .is_some_and(|backend| backend.to_string_lossy().split(',').next() == Some("x11"))
    {
        return false;
    }
    env("WAYLAND_DISPLAY").is_some_and(|display| !display.is_empty())
        || env("XDG_SESSION_TYPE").is_some_and(|session| session == "wayland")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detect(nvidia: bool, values: &[(&str, &str)]) -> bool {
        needs_compatibility_renderer(nvidia, |key| {
            values
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| OsString::from(value))
        })
    }

    #[test]
    fn covers_wayland_and_hybrid_nvidia_sessions() {
        assert!(detect(true, &[("WAYLAND_DISPLAY", "wayland-1")]));
        assert!(detect(true, &[("XDG_SESSION_TYPE", "wayland")]));
        assert!(detect(
            true,
            &[
                ("WAYLAND_DISPLAY", "wayland-1"),
                ("GDK_BACKEND", "wayland,x11")
            ]
        ));
    }

    #[test]
    fn leaves_other_graphics_sessions_unchanged() {
        assert!(!detect(false, &[("WAYLAND_DISPLAY", "wayland-1")]));
        assert!(!detect(true, &[("XDG_SESSION_TYPE", "x11")]));
        assert!(!detect(true, &[("WAYLAND_DISPLAY", "")]));
        assert!(!detect(true, &[]));
        assert!(!detect(
            true,
            &[
                ("WAYLAND_DISPLAY", "wayland-1"),
                ("GDK_BACKEND", "x11,wayland")
            ]
        ));
    }

    #[test]
    fn preserves_explicit_overrides_including_empty_and_zero() {
        for value in ["1", "0", ""] {
            assert!(!detect(
                true,
                &[("WAYLAND_DISPLAY", "wayland-1"), (DISABLE_DMABUF, value)]
            ));
        }
    }
}
