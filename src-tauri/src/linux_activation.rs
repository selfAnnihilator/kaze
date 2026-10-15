//! Keep the launcher's Wayland activation context across the official
//! single-instance plugin, whose callback transports argv/cwd but not env.
use std::os::unix::process::CommandExt;

const TOKEN_ARG: &str = "--kaze-activation-token=";

pub fn token_from_args(args: &[String]) -> Option<String> {
    args.iter()
        .skip(1)
        .find_map(|arg| {
            arg.strip_prefix(TOKEN_ARG)
                .filter(|token| !token.is_empty())
        })
        .map(str::to_owned)
}

pub fn preserve_launcher_token() {
    let Some(token) = std::env::var("XDG_ACTIVATION_TOKEN")
        .ok()
        .filter(|token| !token.is_empty())
    else {
        return;
    };
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args
        .iter()
        .any(|arg| arg.to_string_lossy().starts_with(TOKEN_ARG))
    {
        return;
    }
    let Ok(binary) = std::env::current_exe() else {
        return;
    };
    // One exec, before GTK/Tauri/backend initialization, preserves the PID and
    // makes the token available in the plugin's existing argv payload. The
    // marker prevents another exec; no helper process, lock or IPC is created.
    let error = std::process::Command::new(binary)
        .args(args)
        .arg(format!("{TOKEN_ARG}{token}"))
        .exec();
    eprintln!("Could not preserve Kaze launcher activation context: {error}");
}

pub fn present<R: tauri::Runtime>(
    window: &tauri::WebviewWindow<R>,
    token: Option<&str>,
) -> tauri::Result<()> {
    use gtk::prelude::*;
    let native = window.gtk_window()?;
    native.deiconify();
    native.show();
    if let Some((token, surface)) = token.zip(native.window()) {
        // GTK 3.24's Wayland backend consumes this as xdg_activation_v1 for
        // the mapped surface. Applying it directly to GDK avoids retaining a
        // consumed token in GtkWindow's startup-id property for later opens.
        surface.set_startup_id(token);
    } else {
        native.present();
        if !native.is_active() {
            window.request_user_attention(Some(tauri::UserAttentionType::Informational))?;
        }
    }
    Ok(())
}
