import { memo, useEffect, useRef, useState } from "react";
import { dispatchCommand, executeQuery } from "../services/api";
import { sameSongMetadata } from "../localTrackMatch";
import type { RecommendationSetup } from "../types";
const genres = ["Pop", "Rock", "Hip-Hop", "R&B", "Electronic", "Indie", "Jazz", "Classical", "Country", "Folk", "Metal", "Devotional"];
export const MusicTaste = memo(function MusicTaste({ initial, userId, onSaved }: { initial?: RecommendationSetup; userId: string; onSaved: () => void }) {
  const [artists, setArtists] = useState<string[]>(initial?.artists || []), [selectedGenres, setGenres] = useState<string[]>(initial?.genres || []);
  const [search, setSearch] = useState(""), [suggestions, setSuggestions] = useState<string[]>([]);
  const [loading, setLoading] = useState(!initial), [searching, setSearching] = useState(false), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const [retry, setRetry] = useState(0), [searchRetry, setSearchRetry] = useState(0);
  const [failureKind, setFailureKind] = useState<"load" | "search" | "save" | null>(null);
  const lastSkipped = useRef(false);
  const alive = useRef(true), saving = useRef(false);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  useEffect(() => {
    if (initial) return;
    let active = true; setLoading(true); setError(""); setFailureKind(null);
    void executeQuery({ query: "GetRecommendationSetup" }).then(result => {
      if (!active) return;
      if (result.type !== "RecommendationSetup" || !result.data || result.data.user_id !== userId) throw Error("Music taste is unavailable");
      setArtists(result.data.artists); setGenres(result.data.genres);
    }).catch(() => { if (active) { setFailureKind("load"); setError("Couldn’t load your music taste. Try again."); } }).finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [initial, retry, userId]);
  useEffect(() => {
    let active = true;
    setSuggestions([]); setSearching(false);
    if (search.trim().length < 2) return;
    const timer = setTimeout(() => {
      setSearching(true); setError(""); setFailureKind(null);
      void executeQuery({ query: "SearchOnlineMusic", payload: { query: search.trim(), limit: 25 } }).then(result => {
        if (!active) return;
        const names: string[] = Array.isArray(result.data) ? result.data.flatMap((song: { artist?: string }) => song.artist ? [song.artist] : []) : [];
        setSuggestions(names.filter((name, index) => names.findIndex(other => sameSongMetadata("artist", name, "artist", other)) === index).slice(0, 12));
      }).catch(() => { if (active) { setFailureKind("search"); setError("Artist search is unavailable. Choose genres or try again."); } }).finally(() => { if (active) setSearching(false); });
    }, 250);
    return () => { active = false; clearTimeout(timer); };
  }, [search, searchRetry]);
  const toggle = (value: string, values: string[], set: (v: string[]) => void) => {
    if (values.includes(value)) set(values.filter(v => v !== value));
    else if (values.length < 5) set([...values, value]);
  };
  const save = async (skipped: boolean) => {
    if (saving.current) return;
    saving.current = true; lastSkipped.current = skipped; setBusy(true); setError(""); setFailureKind(null);
    try {
      await dispatchCommand({ command: "SaveRecommendationSetup", payload: { expected_user_id: userId, artists, genres: selectedGenres, skipped } });
      if (alive.current) onSaved();
    } catch { if (alive.current) { setFailureKind("save"); setError("Couldn’t save your music taste. Please try again."); } }
    finally { saving.current = false; if (alive.current) setBusy(false); }
  };
  return <div className="music-taste" aria-busy={busy || loading}>
    <p>Choose artists and genres you enjoy. We’ll use them to find your first songs, then learn as you listen.</p>
    {loading ? <p role="status">Loading your music taste…</p> : <>
      <fieldset disabled={busy || failureKind === "load"}><legend>Artists <span>Choose up to five</span></legend>
        <label htmlFor="taste-artist-search">Find an artist</label>
        <input id="taste-artist-search" value={search} onChange={e => { setSearch(e.target.value); setError(""); }} placeholder="Search artists" autoComplete="off" />
        <div className="taste-choices" aria-label="Selected artists">{artists.map(name => <button type="button" key={name} aria-pressed="true" aria-label={`Remove ${name}`} onClick={() => toggle(name, artists, setArtists)}>{name} ×</button>)}</div>
        {searching && <p role="status">Finding artists…</p>}
        <div className="taste-choices" aria-label="Artist suggestions">{suggestions.filter(n => !artists.some(a => sameSongMetadata("artist", a, "artist", n))).map(name => <button type="button" key={name} disabled={artists.length >= 5} onClick={() => toggle(name, artists, setArtists)}>{name}</button>)}</div>
      </fieldset>
      <fieldset disabled={busy || failureKind === "load"}><legend>Genres <span>Choose up to five</span></legend><div className="taste-choices">{genres.map(genre => <button type="button" key={genre} aria-pressed={selectedGenres.includes(genre)} disabled={selectedGenres.length >= 5 && !selectedGenres.includes(genre)} onClick={() => toggle(genre, selectedGenres, setGenres)}>{genre}</button>)}</div></fieldset>
      <div className="taste-actions"><button className="btn btn-primary" disabled={busy || failureKind === "load" || (!artists.length && !selectedGenres.length)} onClick={() => void save(false)}>{busy ? "Saving…" : "Save music taste"}</button><button className="btn btn-secondary" disabled={busy || failureKind === "load"} onClick={() => void save(true)}>Skip for now</button></div>
    </>}
    {error && <p role="alert">{error} <button onClick={() => { if (failureKind === "load") setRetry(v => v + 1); else if (failureKind === "search") setSearchRetry(v => v + 1); else void save(lastSkipped.current); }} disabled={busy}>Retry</button></p>}
  </div>;
});
export const RecommendationSetupGate = memo(function RecommendationSetupGate({ ready, userId, onSaved }: { ready: boolean; userId: string; onSaved: () => void }) {
  const [setup, setSetup] = useState<RecommendationSetup | null>(null), [retry, setRetry] = useState(0), [error, setError] = useState(false);
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    if (!ready) return;
    let active = true;
    void executeQuery({ query: "GetRecommendationSetup" }).then(result => { if (!active) return; if (result.type !== "RecommendationSetup" || !result.data || result.data.user_id !== userId) throw Error("Music setup unavailable"); setSetup(result.data.should_prompt ? result.data : null); setError(false); }).catch(() => { if (active) setError(true); });
    return () => { active = false; };
  }, [ready, retry, userId]);
  useEffect(() => { if (setup && ready && !dialog.current?.open) dialog.current?.showModal(); }, [setup, ready]);
  if (!ready) return null;
  if (error) return <div className="taste-retry" role="status">Couldn’t load music setup. <button onClick={() => setRetry(v => v + 1)}>Retry</button></div>;
  return setup ? <dialog className="taste-dialog" ref={dialog} aria-labelledby="taste-heading" onCancel={e => { e.preventDefault(); dialog.current?.querySelector<HTMLButtonElement>(".taste-actions .btn-secondary")?.click(); }}>
    <h2 id="taste-heading" tabIndex={-1} autoFocus>Make Kaze yours</h2>
    <MusicTaste userId={userId} initial={setup} onSaved={() => { setSetup(null); onSaved(); }} />
  </dialog> : null;
});
