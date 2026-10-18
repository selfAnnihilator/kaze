import React, { lazy, Suspense } from "react";
import ReactDOM from "react-dom/client";
import { executeQuery } from "./services/api";
import { StartupScreen } from "./components/StartupScreen";
import type { AppSettings } from "./types";
import "./index.css";
import "./kaze-theme.css";

// Explicit ?lab= entries preserve the original accepted studies for reference.
// Normal entry (including old ?design= URLs) always mounts the real App.
// The lab is a separate frontend entry. Mounting it never mounts App's IPC or
// playback subscriptions, and its state cannot reach the production backend.
const SpatialDesignLab = lazy(
  () => import("./components/spatial-design/SpatialDesignLab"),
);
const BrutalistDesignLab = lazy(
  () => import("./components/brutalist-design/BrutalistDesignLab"),
);
const design = new URLSearchParams(window.location.search).get("lab");

// Disable the native right-click context menu (Inspect Element, Reload, etc.)
document.addEventListener("contextmenu", (e) => e.preventDefault());

const rootElement = document.getElementById("root");

if (rootElement) {
  const root = ReactDOM.createRoot(rootElement);
  const opening = <StartupScreen />;
  const render = (content: React.ReactNode) =>
    root.render(<React.StrictMode>{content}</React.StrictMode>);

  if (design === "spatial" || design === "brutalist") {
    render(
      <Suspense fallback={opening}>
        {design === "spatial" ? <SpatialDesignLab /> : <BrutalistDesignLab />}
      </Suspense>,
    );
  } else {
    const openApp = async () => {
      render(opening);
      let cancelled = false;
      const loadSettingsAndIntro = async () => {
        const response = await executeQuery({ query: "GetSettings" });
        // A rejected App chunk can finish before this read. Do not revive its
        // cancelled startup screen after showing the error/retry surface.
        if (cancelled) throw new Error("Startup interrupted");
        if (response.type !== "Settings" || !response.data || typeof response.data !== "object") {
          throw new Error("Startup settings are unavailable");
        }
        const settings = response.data as AppSettings;
        const interfaceStyle = settings.interface_style === "brutalist" ? "brutalist" : "spatial";
        document.documentElement.dataset.interface = interfaceStyle;
        await new Promise<void>((resolve) => {
          render(<StartupScreen interfaceStyle={interfaceStyle} onIntroComplete={resolve} />);
        });
        return settings;
      };
      try {
        // Load code and the existing persisted preference concurrently. Never
        // mount the functional shell in a guessed theme, even for one frame.
        const [{ App }, settings] = await Promise.all([
          import("./App"),
          loadSettingsAndIntro(),
        ]);
        render(<App initialSettings={settings} />);
      } catch (error) {
        cancelled = true;
        console.error("Failed to open Kaze:", error);
        render(
          <div className="app-startup" role="alert">
            <p>Couldn’t open Kaze. Please try again.</p>
            <button className="app-startup-retry" onClick={() => void openApp()}>
              Try again
            </button>
          </div>,
        );
      }
    };
    void openApp();
  }
}
