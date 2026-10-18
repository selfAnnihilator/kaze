import { memo, useEffect } from "react";
import appIcon from "../../src-tauri/icons/128x128@2x.png";
import type { InterfaceStyle } from "../presentation/InterfaceContext";

const INTRO_DURATION_MS = 2000;

/** A mount-only intro; animation never sends updates through the player tree. */
export const StartupScreen = memo(function StartupScreen({
  interfaceStyle,
  onIntroComplete,
}: {
  interfaceStyle?: InterfaceStyle;
  onIntroComplete?: () => void;
}) {
  useEffect(() => {
    if (!interfaceStyle || !onIntroComplete) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      onIntroComplete();
      return;
    }
    const timer = window.setTimeout(onIntroComplete, INTRO_DURATION_MS);
    return () => window.clearTimeout(timer);
  }, [interfaceStyle, onIntroComplete]);

  return (
    <div className="app-startup" data-startup-style={interfaceStyle ?? "pending"} role="status">
      <span className="app-startup-sr">Opening Kaze…</span>
      <div className="app-startup-brand" aria-hidden="true">
        <div className="app-startup-mark">
          {interfaceStyle === "spatial" && <span className="app-startup-ring" />}
          {interfaceStyle === "brutalist" && <span className="app-startup-frame" />}
          <img className="app-startup-icon" src={appIcon} alt="" width={104} height={104} />
        </div>
        <div className="app-startup-name">
          <span>K</span><span>A</span><span>Z</span><span>E</span>
        </div>
        {interfaceStyle === "brutalist" && <div className="app-startup-rail"><span /></div>}
      </div>
    </div>
  );
});
