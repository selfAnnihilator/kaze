//! Desktop lifetime only: hiding never destroys the webview or touches playback.
use music_player_backend::core::processor::CoreProcessor;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{AppHandle, Manager, Runtime};

#[derive(Default)]
pub struct DesktopLifecycle {
    started: AtomicBool,
    complete: AtomicBool,
    cleanup: tokio::sync::Mutex<()>,
    tasks: Mutex<Vec<tauri::async_runtime::JoinHandle<()>>>,
}

impl DesktopLifecycle {
    pub fn begin(&self) -> bool {
        !self.started.swap(true, Ordering::AcqRel)
    }

    pub fn is_complete(&self) -> bool {
        self.complete.load(Ordering::Acquire)
    }

    pub fn is_quitting(&self) -> bool {
        self.started.load(Ordering::Acquire)
    }

    pub fn spawn(&self, future: impl std::future::Future<Output = ()> + Send + 'static) {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        if self.started.load(Ordering::Acquire) {
            return;
        }
        tasks.retain(|task| !task.inner().is_finished());
        tasks.push(tauri::async_runtime::spawn(future));
    }

    pub async fn shutdown(&self, app: &AppHandle) {
        self.begin();
        let _cleanup = self.cleanup.lock().await;
        if self.is_complete() {
            return;
        }
        if let Some(processor) = app.try_state::<Arc<CoreProcessor>>() {
            processor.begin_quit();
        }
        let tasks = std::mem::take(&mut *self.tasks.lock().unwrap_or_else(|e| e.into_inner()));
        for task in &tasks {
            task.abort();
        }
        for task in tasks {
            let _ = task.await;
        }
        #[cfg(target_os = "linux")]
        if let Some(service) = app.try_state::<music_player_backend::mpris::MprisServiceHandle>() {
            service.shutdown().await;
        }
        if let Some(processor) = app.try_state::<Arc<CoreProcessor>>() {
            if let Err(error) = processor.shutdown().await {
                tracing::error!(%error, "Playback shutdown reported an error");
            }
        }
        self.complete.store(true, Ordering::Release);
    }
}

pub fn request_quit<R: Runtime>(app: &AppHandle<R>) {
    if let Some(processor) = app.try_state::<Arc<CoreProcessor>>() {
        if !processor.begin_quit() {
            return;
        }
    }
    app.exit(0);
}

#[cfg(target_os = "linux")]
pub(crate) async fn tray_host_available() -> bool {
    // One query per close request, not a background poll. Bare Wayland sessions
    // may have no tray host even when libappindicator created an icon resource.
    let available = async {
        let conn = zbus::Connection::session().await?;
        let watcher = zbus::Proxy::new(
            &conn,
            "org.kde.StatusNotifierWatcher",
            "/StatusNotifierWatcher",
            "org.kde.StatusNotifierWatcher",
        )
        .await?;
        watcher
            .get_property::<bool>("IsStatusNotifierHostRegistered")
            .await
    };
    matches!(
        tokio::time::timeout(std::time::Duration::from_secs(2), available).await,
        Ok(Ok(true))
    )
}

#[cfg(not(target_os = "linux"))]
pub(crate) async fn tray_host_available() -> bool {
    true
}

pub fn close_main_window<R: Runtime>(window: &tauri::Window<R>) {
    let app = window.app_handle();
    let Some(processor) = app.try_state::<Arc<CoreProcessor>>() else {
        request_quit(app);
        return;
    };
    if processor.is_quitting() {
        return;
    }
    if processor.close_button_behavior() == "quit_app"
        || app.tray_by_id(crate::tray::TRAY_ID).is_none()
    {
        request_quit(app);
        return;
    }
    let window = window.clone();
    let processor = processor.inner().clone();
    app.state::<DesktopLifecycle>().spawn(async move {
        let host = tray_host_available().await;
        if processor.is_quitting() {
            return;
        }
        if !host || processor.close_button_behavior() == "quit_app" {
            tracing::info!(host, "Closing Kaze through the clean quit path");
            request_quit(window.app_handle());
        } else if let Err(error) = window.hide() {
            tracing::warn!(%error, "Could not hide Kaze; keeping the existing window");
        }
    });
}
