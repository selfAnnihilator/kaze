import React from "react";
import ReactDOM from "react-dom/client";
import { executeQuery } from "./services/api";
import { readStartupSettings } from "./services/startupSettings";
import { StartupScreen } from "./components/StartupScreen";
import { MobileStartup } from "./presentation/mobile/MobileStartup";
import { isMobileDevice } from "./presentation/mobile/platform";
import "./styles/tokens.css";
import "./index.css";
import "./kaze-theme.css";

// Disable the native right-click context menu (Inspect Element, Reload, etc.)
document.addEventListener("contextmenu", (e) => e.preventDefault());

const rootElement = document.getElementById("root");

if (rootElement) {
  const root = ReactDOM.createRoot(rootElement);
  // The phone opens on Figma A1 and keeps it until the library loads; the PC plays its intro.
  const phone = isMobileDevice();
  if (phone) document.documentElement.dataset.interface = "mobile";
  const opening = phone ? <MobileStartup /> : <StartupScreen />;
  const render = (content: React.ReactNode) =>
    root.render(<React.StrictMode>{content}</React.StrictMode>);

  /** The PC's intro in its interface style; resolves when it has played. */
  const playIntro = (interfaceStyle: "spatial" | "brutalist") => new Promise<void>((resolve) => {
    document.documentElement.dataset.interface = interfaceStyle;
    render(<StartupScreen interfaceStyle={interfaceStyle} onIntroComplete={resolve} />);
  });

  const openApp = async () => {
    render(opening);
    let cancelled = false;
    const loadSettingsAndIntro = async () => {
      const settings = await readStartupSettings(executeQuery, phone);
      // A rejected App chunk can finish before this read. Do not revive its
      // cancelled startup screen after showing the error/retry surface.
      if (cancelled) throw new Error("Startup interrupted");
      const interfaceStyle = settings.interface_style === "brutalist" ? "brutalist" : "spatial";
      if (!phone) await playIntro(interfaceStyle);
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
