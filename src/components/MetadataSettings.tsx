import { memo, useEffect, useState } from "react";
import { dispatchCommand, executeQuery } from "../services/api";

export const MetadataSettings = memo(function MetadataSettings() {
  const [preferences, setPreferences] = useState<Record<string, boolean> | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    let cancelled = false;
    executeQuery({ query: "GetSettings" }).then(result => {
      if (cancelled) return;
      const metadata = result.data?.metadata;
      setPreferences({ enable_jiosaavn: metadata?.enable_jiosaavn !== false, auto_fill_metadata: metadata?.auto_fill_metadata !== false });
      setError("");
    }).catch(() => { if (!cancelled) setError("Couldn’t load music source settings."); });
    return () => { cancelled = true; };
  }, [revision]);
  const save = async (key: string, enabled: boolean) => {
    setBusy(true); setError(""); setMessage("");
    try {
      await dispatchCommand({ command: "SetMetadataPreference", payload: { key, enabled } });
      setPreferences(current => ({ ...current, [key]: enabled }));
    } catch { setError("Couldn’t save this choice. Try again."); }
    finally { setBusy(false); }
  };
  const fill = async (retry: boolean) => {
    setBusy(true); setError("");
    try {
      await dispatchCommand({ command: "FillLibraryGaps", payload: { retry } });
      setMessage("Filling library gaps in the background. Uncertain matches stay unchanged.");
    } catch { setError("Couldn’t start gap filling. Try again."); }
    finally { setBusy(false); }
  };
  const remove = async () => {
    setBusy(true); setError("");
    try {
      await dispatchCommand({ command: "RemoveProviderEnrichment" });
      setPreferences(current => ({ ...current, auto_fill_metadata: false }));
      setMessage("Provider-filled data removed. Original tags are restored and automatic filling is off.");
    } catch { setError("Couldn’t remove provider-filled data. Try again."); }
    finally { setBusy(false); }
  };
  return <section className="settings-section metadata-settings" aria-labelledby="metadata-settings-heading">
    <h2 id="metadata-settings-heading">Online music &amp; library details</h2>
    <label className="settings-startup-switch"><input type="checkbox" role="switch" disabled={busy || !preferences} checked={preferences?.enable_jiosaavn ?? true} onChange={e => void save("enable_jiosaavn", e.target.checked)} /><span>Use JioSaavn for online music</span></label>
    <label className="settings-startup-switch"><input type="checkbox" role="switch" disabled={busy || !preferences} checked={preferences?.auto_fill_metadata ?? true} onChange={e => void save("auto_fill_metadata", e.target.checked)} /><span>Automatically fill missing artwork and details</span></label>
    <p className="settings-hint">Matches need the same recording and duration. Your audio files and existing tags stay intact.</p>
    <div className="settings-metadata-actions">
      <button type="button" disabled={busy || !preferences} onClick={() => void fill(false)}>Fill missing artwork and details</button>
      <button type="button" disabled={busy || !preferences} onClick={() => void fill(true)}>Retry unmatched songs</button>
      <button type="button" disabled={busy || !preferences} onClick={() => void remove()}>Remove provider-filled data</button>
    </div>
    {busy && <p role="status">Working…</p>}
    {message && <p role="status">{message}</p>}
    {error && <p role="alert">{error} <button type="button" disabled={busy} onClick={() => setRevision(r => r + 1)}>Retry</button></p>}
  </section>;
});
