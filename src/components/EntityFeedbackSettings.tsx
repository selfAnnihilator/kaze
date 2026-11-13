import { memo, useEffect, useState } from "react";
import { dispatchCommand, executeQuery } from "../services/api";
import type { EntityFeedbackRow } from "../types";

/** Artists and genres hidden or turned down from recommendations, with undo. */
export const EntityFeedbackSettings = memo(function EntityFeedbackSettings() {
  const [rows, setRows] = useState<EntityFeedbackRow[] | null>(null);
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    let cancelled = false;
    executeQuery({ query: "GetEntityFeedback" }).then(result => {
      if (cancelled) return;
      setRows(Array.isArray(result.data) ? result.data : []);
      setError("");
    }).catch(() => { if (!cancelled) setError("Couldn’t load hidden artists and genres."); });
    return () => { cancelled = true; };
  }, [revision]);
  const remove = async (row: EntityFeedbackRow) => {
    const id = `${row.kind}:${row.key}`;
    setBusy(id); setError("");
    try {
      await dispatchCommand({ command: "ClearEntityFeedback", payload: { kind: row.kind, key: row.key } });
      setRows(current => current?.filter(item => item.kind !== row.kind || item.key !== row.key) ?? null);
    } catch { setError("Couldn’t update this preference. Try again."); }
    finally { setBusy(""); }
  };
  return <section className="settings-section entity-feedback-settings" aria-labelledby="entity-feedback-heading">
    <h2 id="entity-feedback-heading">Hidden from recommendations</h2>
    {rows && !rows.length && <p className="settings-hint">Nothing hidden. Use the thumbs-down button on a song to hide an artist or see less of a style.</p>}
    {!!rows?.length && <ul className="entity-feedback-list">
      {rows.map(row => {
        const id = `${row.kind}:${row.key}`;
        const what = row.level === "block" ? "Not recommended" : "Showing less";
        return <li key={id}>
          <span>{row.display} <small>{row.kind === "artist" ? "Artist" : "Genre"} · {what}</small></span>
          <button type="button" disabled={!!busy} aria-label={`Remove ${row.display} from hidden list`} onClick={() => void remove(row)}>Remove</button>
        </li>;
      })}
    </ul>}
    {error && <p role="alert">{error} <button type="button" disabled={!!busy} onClick={() => setRevision(r => r + 1)}>Retry</button></p>}
  </section>;
});
