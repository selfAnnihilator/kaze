/// Niri supplies a socket even when a custom session has a different name.
/// Other desktops keep Tauri's native close/minimize/maximize controls.
pub fn is_niri_session(env: impl Fn(&str) -> Option<String>) -> bool {
    if env("NIRI_SOCKET").is_some_and(|socket| !socket.trim().is_empty()) {
        return true;
    }

    [
        "XDG_CURRENT_DESKTOP",
        "XDG_SESSION_DESKTOP",
        "DESKTOP_SESSION",
    ]
    .iter()
    .filter_map(|name| env(name))
    .any(|desktop| {
        desktop
            .split(':')
            .any(|name| name.trim().eq_ignore_ascii_case("niri"))
    })
}

#[cfg(test)]
mod tests {
    use super::is_niri_session;

    fn detect(values: &[(&str, &str)]) -> bool {
        is_niri_session(|key| {
            values
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| value.to_string())
        })
    }

    #[test]
    fn detects_niri_and_custom_sessions() {
        assert!(detect(&[("XDG_CURRENT_DESKTOP", "niri")]));
        assert!(detect(&[("XDG_CURRENT_DESKTOP", "Custom:Niri")]));
        assert!(detect(&[("XDG_SESSION_DESKTOP", "niri")]));
        assert!(detect(&[("DESKTOP_SESSION", "niri")]));
        assert!(detect(&[
            ("DESKTOP_SESSION", "zanken"),
            ("NIRI_SOCKET", "/run/user/1000/niri.sock")
        ]));
    }

    #[test]
    fn preserves_controls_for_other_or_unknown_desktops() {
        assert!(!detect(&[]));
        assert!(!detect(&[("XDG_CURRENT_DESKTOP", "GNOME")]));
        assert!(!detect(&[("XDG_CURRENT_DESKTOP", "KDE")]));
        assert!(!detect(&[("XDG_CURRENT_DESKTOP", "sway")]));
        assert!(!detect(&[
            ("DESKTOP_SESSION", "not-niri"),
            ("NIRI_SOCKET", " ")
        ]));
    }
}
