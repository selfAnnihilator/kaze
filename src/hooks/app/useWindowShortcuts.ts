import { useCallback, useEffect, useMemo, useRef, useState } from "react";

/**
 * Window-level state and shortcuts: connectivity, fullscreen (Tauri window +
 * HTML5 document), F/F11/Escape fullscreen keys and the Space play/pause key.
 */
export function useWindowShortcuts() {
  // Online / Offline Connectivity State
  const [isOnline, setIsOnline] = useState<boolean>(
    typeof navigator !== "undefined" ? navigator.onLine : true,
  );

  useEffect(() => {
    const handleOnline = () => setIsOnline(true);
    const handleOffline = () => setIsOnline(false);
    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);
    return () => {
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
    };
  }, []);

  // Full Screen & Lyrics View State
  const [isFullscreen, setIsFullscreen] = useState(false);

  const handleToggleFullscreen = useCallback(
    async (override?: boolean) => {
      const nextState = typeof override === "boolean" ? override : !isFullscreen;
      setIsFullscreen(nextState);

      // 1. Tauri OS Window Fullscreen (takes entire monitor, hides titlebar/OS panel like YouTube video full screen)
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        const appWindow = getCurrentWindow();
        await appWindow.setFullscreen(nextState);
      } catch (err) {
        console.warn("Tauri window setFullscreen failed:", err);
      }

      // 2. HTML5 document fullscreen standard
      try {
        if (nextState) {
          if (!document.fullscreenElement && document.documentElement.requestFullscreen) {
            await document.documentElement.requestFullscreen();
          }
        } else {
          if (document.fullscreenElement && document.exitFullscreen) {
            await document.exitFullscreen();
          }
        }
      } catch (err) {
        // Ignore fallback errors
      }
    },
    [isFullscreen],
  );

  // App assigns the current play/pause handler; Space reads it at key time.
  const handlePlayPauseRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.key === "Escape" && isFullscreen) || e.key === "F11") {
        e.preventDefault();
        handleToggleFullscreen(e.key === "Escape" ? false : undefined);
        return;
      }

      if (
        e.key.toLowerCase() === "f" &&
        !e.defaultPrevented && !e.repeat && !e.isComposing &&
        !e.ctrlKey && !e.altKey && !e.metaKey
      ) {
        const editing = e.composedPath().some(target =>
          target instanceof Element && Boolean(target.closest(
            "input, textarea, select, [contenteditable]:not([contenteditable='false']), [role='textbox'], [role='searchbox'], [role='combobox'], [role='menu'], [role='listbox']",
          )),
        );
        const dialog = document.querySelector(
          ".modal-overlay, [role='dialog']:not(.fullscreen-player), [aria-modal='true']:not(.fullscreen-player)",
        );
        if (!editing && !dialog) {
          e.preventDefault();
          handleToggleFullscreen();
        }
        return;
      }

      if (
        e.code === "Space" && !e.defaultPrevented && !e.isComposing &&
        !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey
      ) {
        const target = e.target instanceof Element ? e.target : null;
        const editing = target?.closest(
          "input, textarea, select, [contenteditable]:not([contenteditable='false']), [role='textbox'], [role='searchbox'], [role='combobox'], [role='slider'], [role='menu'], [role='listbox']",
        );
        const dialog = document.querySelector(
          ".modal-overlay, [role='dialog']:not(.fullscreen-player), [aria-modal='true']:not(.fullscreen-player)",
        );
        const action = target?.closest("button, a[href], [role='button'], [role='tab'], [role='menuitem']");
        if (editing || dialog || (action && !action.hasAttribute("data-pointer-focus"))) return;

        // A mouse-selected action must not consume the playback shortcut.
        // Tab removes the pointer marker, restoring ordinary Space activation.
        e.preventDefault();
        if (!e.repeat) handlePlayPauseRef.current?.();
      }
    };

    const handlePointerSpace = (event: KeyboardEvent) => {
      if (event.code !== "Space" || !(event.target instanceof Element) ||
          !event.target.closest("[data-pointer-focus]")) return;
      handleKeyDown(event);
      // Run before control-level handlers and native button activation.
      if (event.defaultPrevented) event.stopPropagation();
    };

    const handleFullscreenChange = () => {
      if (!document.fullscreenElement && isFullscreen) {
        handleToggleFullscreen(false);
      }
    };

    window.addEventListener("keydown", handlePointerSpace, true);
    window.addEventListener("keydown", handleKeyDown);
    document.addEventListener("fullscreenchange", handleFullscreenChange);
    return () => {
      window.removeEventListener("keydown", handlePointerSpace, true);
      window.removeEventListener("keydown", handleKeyDown);
      document.removeEventListener("fullscreenchange", handleFullscreenChange);
    };
  }, [isFullscreen, handleToggleFullscreen]);

  return useMemo(
    () => ({ isOnline, isFullscreen, handleToggleFullscreen, handlePlayPauseRef }),
    [isOnline, isFullscreen, handleToggleFullscreen],
  );
}
