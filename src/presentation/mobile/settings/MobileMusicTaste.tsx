import { useEffect, useState } from "react";
import { executeQuery } from "../../../services/api";
import { Search, X } from "lucide-react";
import { TASTE_GENRES, TASTE_LIMIT, useMusicTaste } from "./useMusicTaste";
import type { RecommendationSetup } from "../../../types";

/**
 * Music Taste (Figma A3 "MAKE KAZE YOURS."): artist search with chosen artists
 * as red chips, the genre grid, then Skip / Start Listening. Used for the
 * first-run setup and from Settings.
 */
export function MobileMusicTaste({ userId, initial, onSaved, kicker = "SETUP 02 / 02 // TASTE", doneLabel = "START LISTENING" }: {
  userId: string; initial?: RecommendationSetup; onSaved: () => void; kicker?: string; doneLabel?: string;
}) {
  const t = useMusicTaste(userId, initial, onSaved);
  const blocked = t.loading || t.busy;
  return (
    <div className="m-taste">
      <div className="m-taste__body">
        <span className="m-kicker">{kicker}</span>
        <h1 className="m-display m-taste__title">MAKE KAZE YOURS.</h1>
        <p className="m-taste__intro">Pick up to five artists and the genres you love. Flow and For You start from here.</p>
        <label className="m-field m-field--sm m-taste__search">
          <Search size={18} strokeWidth={2} className="m-field__icon" />
          <input type="search" className="m-field__input" placeholder="Search artists…" value={t.search} disabled={blocked}
            onChange={(e) => t.setSearch(e.target.value)} aria-label="Search artists" autoComplete="off" />
        </label>
        <div className="m-taste__label"><span>ARTISTS</span><span className="m-taste__count">{t.artists.length} / {TASTE_LIMIT}</span></div>
        <div className="m-taste__chips">
          {t.artists.map((name) => (
            <button key={name} type="button" className="m-taste__chip is-on" disabled={blocked} onClick={() => t.toggleArtist(name)} aria-label={`Remove ${name}`}>
              <span>{name.toUpperCase()}</span><X size={12} strokeWidth={2} />
            </button>
          ))}
          {t.suggestions.map((name) => (
            <button key={name} type="button" className="m-taste__chip" disabled={blocked || t.artists.length >= TASTE_LIMIT} onClick={() => t.toggleArtist(name)}>
              <span>{name.toUpperCase()}</span>
            </button>
          ))}
          {!t.artists.length && !t.suggestions.length && <p className="m-set-note">No artists yet. Search above and tap a name to add it.</p>}
        </div>
        <div className="m-taste__label"><span>GENRES</span><span className="m-taste__count">{t.genres.length} SELECTED</span></div>
        <div className="m-taste__grid">
          {TASTE_GENRES.map((genre) => {
            const on = t.genres.includes(genre);
            return (
              <button key={genre} type="button" className={`m-taste__chip m-taste__genre ${on ? "is-on" : ""}`} aria-pressed={on}
                disabled={blocked || (!on && t.genres.length >= TASTE_LIMIT)} onClick={() => t.toggleGenre(genre)}>
                {genre.toUpperCase()}
              </button>
            );
          })}
        </div>
        {t.error && <p className="m-set-note is-error" role="alert">{t.error}</p>}
      </div>
      <div className="m-taste__actions">
        <button type="button" className="m-btn m-btn--outline m-taste__skip" disabled={blocked} onClick={() => void t.save(true)}>SKIP</button>
        <button type="button" className="m-btn m-btn--grow m-taste__go" disabled={blocked || (!t.artists.length && !t.genres.length)} onClick={() => void t.save(false)}>
          {t.busy ? "SAVING…" : doneLabel}
        </button>
      </div>
    </div>
  );
}

/** First-run music taste on the phone: shown full screen when the account still needs it. */
export function MobileTasteGate({ ready, userId, onSaved }: { ready: boolean; userId: string; onSaved: () => void }) {
  const [setup, setSetup] = useState<RecommendationSetup | null>(null);
  useEffect(() => {
    if (!ready) return;
    let active = true;
    executeQuery({ query: "GetRecommendationSetup" }).then((result) => {
      if (!active || result.type !== "RecommendationSetup" || !result.data || result.data.user_id !== userId) return;
      setSetup(result.data.should_prompt ? result.data : null);
    }).catch(() => { /* Settings → Music taste stays available */ });
    return () => { active = false; };
  }, [ready, userId]);
  if (!ready || !setup) return null;
  return (
    <section className="m-sheet" role="dialog" aria-modal="true" aria-label="Music taste">
      <MobileMusicTaste userId={userId} initial={setup} onSaved={() => { setSetup(null); onSaved(); }} />
    </section>
  );
}
