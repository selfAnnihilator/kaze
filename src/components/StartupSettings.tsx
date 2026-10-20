import { memo, useEffect, useRef, useState } from "react";
import { getAutostart, setAutostart } from "../services/desktopStartup";

export const StartupSettings = memo(function StartupSettings() {
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [attempt, setAttempt] = useState(0);
  const pending = useRef(false);
  const mounted = useRef(false);

  useEffect(() => {
    mounted.current = true;
    let cancelled = false;
    setLoading(true);
    setError(null);
    getAutostart().then(value => {
      if (!cancelled) setEnabled(value);
    }).catch(() => {
      if (!cancelled) setError("Couldn't read login startup. Try again in the desktop app.");
    }).finally(() => {
      if (!cancelled) setLoading(false);
    });
    return () => { cancelled = true; mounted.current = false; };
  }, [attempt]);

  const save = async (value: boolean) => {
    if (pending.current || enabled === null || value === enabled) return;
    pending.current = true;
    setSaving(true);
    setError(null);
    try {
      const confirmed = await setAutostart(value);
      if (mounted.current) {
        setEnabled(confirmed);
        if (confirmed !== value) setError("Login startup didn't change. Try again.");
      }
    } catch {
      if (mounted.current) setError("Couldn't change login startup. Try again.");
    } finally {
      pending.current = false;
      if (mounted.current) setSaving(false);
    }
  };

  return (
    <section className="settings-section settings-startup" aria-labelledby="settings-startup-heading">
      <h2 id="settings-startup-heading">Startup</h2>
      <label className="settings-startup-switch">
        <input
          type="checkbox"
          role="switch"
          checked={enabled === true}
          disabled={loading || saving || enabled === null}
          aria-describedby="settings-startup-hint"
          onChange={event => void save(event.target.checked)}
        />
        <span>Start in tray on login</span>
      </label>
      <p className="settings-hint" id="settings-startup-hint">
        Launch Kaze in the system tray when you sign in to your computer.
        If the tray is unavailable, the window opens instead.
      </p>
      {loading && <p role="status">Checking login startup…</p>}
      {saving && <p role="status">Saving login startup…</p>}
      {error && <p role="alert">{error}</p>}
      {!loading && enabled === null && <button type="button" onClick={() => setAttempt(value => value + 1)}>Retry</button>}
    </section>
  );
});
