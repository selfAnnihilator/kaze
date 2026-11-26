import { useEffect, useRef, useState } from "react";
import { executeQuery } from "../../../services/api";
import { TASTE_GENRES, TASTE_LANGUAGES, useMusicTaste } from "./useMusicTaste";
import { TasteChoices } from "./TasteChoices";
import { TasteArtists } from "./TasteArtists";
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
  const [chooseArtists, setChooseArtists] = useState(false);
  const top = useRef<HTMLDivElement>(null);
  useEffect(() => { top.current?.closest(".m-sheet, .m-screen__content")?.scrollTo(0, 0); }, [chooseArtists]);
  const blocked = t.loading || t.busy;
  const hasChoices = t.languages.length + t.artists.length + t.genres.length > 0;
  return (
    <div className="m-taste" ref={top}>
      <div className="m-taste__body">
        <span className="m-kicker">{kicker}</span>
        <h1 className="m-display m-taste__title">{chooseArtists ? "PICK YOUR ARTISTS." : "MAKE KAZE YOURS."}</h1>
        {chooseArtists ? <TasteArtists languages={t.languages} selected={t.artists} disabled={blocked} onToggle={t.toggleArtist} /> : <>
        <p className="m-taste__intro">Choose your song languages and any genres. Recommendations learn from what you listen to.</p>
        <TasteChoices label="LANGUAGES" options={TASTE_LANGUAGES} selected={t.languages} disabled={blocked} onToggle={t.toggleLanguage} />
        <TasteChoices label="GENRES" options={TASTE_GENRES} selected={t.genres} disabled={blocked} onToggle={t.toggleGenre} />
        </>}
        {t.error && <p className="m-set-note is-error" role="alert">{t.error}</p>}
      </div>
      <div className="m-taste__actions">
        {chooseArtists ? <>
          <button type="button" className="m-btn m-btn--outline" disabled={blocked} onClick={() => setChooseArtists(false)}>BACK</button>
          <button type="button" className="m-btn m-btn--grow m-taste__go" disabled={blocked} onClick={() => void t.save(!hasChoices)}>
            {t.busy ? "SAVING…" : doneLabel}
          </button>
        </> : <>
          <button type="button" className="m-btn m-btn--outline m-taste__skip" disabled={blocked} onClick={() => void t.save(true)}>SKIP</button>
          <button type="button" className="m-btn m-btn--grow m-taste__go" disabled={blocked} onClick={() => setChooseArtists(true)}>NEXT</button>
        </>}
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
