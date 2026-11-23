import { useEffect, useState } from "react";
import "./mobile-startup.css";

/** Longest the startup screen stays if the library never answers (the app then shows what it has). */
const GIVE_UP_MS = 8000;

/** Startup (Figma A1): logo between red rules, KAZE, "Opening Kaze…" and a loading bar. */
export function MobileStartup({ ready = false }: { ready?: boolean }) {
  const [gaveUp, setGaveUp] = useState(false);
  useEffect(() => {
    const timer = setTimeout(() => setGaveUp(true), GIVE_UP_MS);
    return () => clearTimeout(timer);
  }, []);
  if (ready || gaveUp) return null;
  return (
    <section className="m-startup" aria-label="Opening Kaze" aria-busy="true">
      <div className="m-startup__center">
        <div className="m-startup__logo">
          <span className="m-startup__rule" />
          <span className="m-startup__frame"><img src="/kaze-icon.png" alt="" /></span>
          <span className="m-startup__rule" />
        </div>
        <h1 className="m-startup__name">KAZE</h1>
        <span className="m-startup__line" />
        <span className="m-startup__status">OPENING KAZE…</span>
      </div>
      <div className="m-startup__foot">
        <span className="m-startup__track"><span className="m-startup__fill" /></span>
        <span className="m-startup__label">LOADING LIBRARY</span>
      </div>
    </section>
  );
}
