import { memo, useCallback, useRef } from "react";
import { Pause, Play, SkipForward } from "lucide-react";
import { PullIndicator } from "./pullRefresh/PullIndicator";
import { usePullToRefresh } from "./pullRefresh/usePullToRefresh";
import { HomeSections } from "./HomeSections";
import { MobileTopBar } from "./MobileTopBar";
import { Sleeve } from "./Sleeve";
import { joinMeta } from "./format";
import type { MobileScreenProps } from "./types";
import { useSleeveSlide } from "./sleeveSlide";
import { useMobileNav } from "./mobileNav";

/**
 * Home (Figma B1 Home — Flow, then B2/B3 sections): Flow poster with previous,
 * current and next sleeves, the Flow controls and Flow Steering.
 */
export const MobileHome = memo(function MobileHome({ c }: MobileScreenProps) {
  const overview = c.homeOverview;
  const active = !!overview?.flow_session?.is_active;
  const playing = active && c.playbackState.is_playing;
  const currentTrack = c.playbackState.current_track;
  const rotation = overview?.recent_rotation ?? [];
  const previous = c.previousTrack;
  const next = c.nextQueuedTrack;

  const handleFlowToggle = useCallback(() => {
    if (active) c.handleToggleFlowPlayback();
    else void c.handleStartFlow(c.flowSteering);
  }, [active, c]);

  const nav = useMobileNav();
  // Pull down refreshes Flow and the sections below it (the phone has no ⟳ buttons).
  const pull = useRef<HTMLDivElement>(null);
  usePullToRefresh(pull, () => Promise.all([c.fetchHomeOverview(), c.handleStartFlow(c.flowSteering)]));
  const slide = useSleeveSlide(currentTrack?.id, previous?.id, next?.id);
  const steeringPercent = Math.round(c.flowSteering * 100);
  const featured = currentTrack ?? (rotation[0] ? {
    id: rotation[0].track_id,
    title: rotation[0].title,
    artist_name: rotation[0].artist,
    album_title: rotation[0].album,
    cover_art_url: rotation[0].cover_art_url,
  } : null);

  return (
    <div className="m-screen__container">
      <MobileTopBar num="01" name="HOME" />

      <div className="m-screen__content">
        <PullIndicator ref={pull} />
        {/* Flow fills the first screen; For You starts below the fold. */}
        <section className="m-flow" aria-label="Flow">
        <div className="m-flow-header">
          <span className="m-kicker">01 // FLOW — YOUR PERSONAL RADIO</span>
          <h1 className="m-display m-flow-header__title">FLOW.</h1>
        </div>

        <div className="m-flow-carousel">
          <button type="button" className="m-flow-carousel__side" onClick={() => void c.handlePreviousTrack()}
            disabled={!previous} aria-label="Previous track">
            <Sleeve key={previous?.id} size="preview" src={previous?.cover_art_url} trackId={previous?.id} title={previous?.title} />
            <span className="m-flow-carousel__side-label">PREV</span>
          </button>
          <div key={featured?.id} className={slide}>
            <Sleeve size="hero" src={c.activePlayingArtwork ?? featured?.cover_art_url} trackId={featured?.id} title={featured?.title}
              onPress={currentTrack ? nav.openDeck : undefined} />
          </div>
          <button type="button" className="m-flow-carousel__side" onClick={() => void c.handleNextTrack()}
            disabled={!next} aria-label="Next track">
            <Sleeve key={next?.id} size="preview" src={next?.cover_art_url} trackId={next?.id} title={next?.title} />
            <span className="m-flow-carousel__side-label">NEXT</span>
          </button>
        </div>

        <div className="m-flow-track-info">
          <h2 className="m-flow-track-info__title">{featured?.title ?? "Start your Flow"}</h2>
          <p className="m-flow-track-info__meta">
            {joinMeta(featured?.artist_name, featured?.album_title).replace(" · ", " — ") || "Kaze picks what plays next"}
          </p>
        </div>

        <div className="m-flow-controls">
          <button type="button" className="m-btn m-btn--grow m-flow-controls__play" onClick={handleFlowToggle}
            disabled={c.flowPendingAction !== null}>
            {playing ? <Pause size={18} strokeWidth={2} /> : <Play size={18} strokeWidth={2} />}
            <span>{playing ? "PAUSE FLOW" : active ? "RESUME FLOW" : "START FLOW"}</span>
          </button>
          <button type="button" className="m-box-btn m-flow-controls__square" onClick={() => void c.handleNextTrack()} aria-label="Skip forward">
            <SkipForward size={20} strokeWidth={2} />
          </button>
        </div>

        <div className="m-flow-steer">
          <div className="m-flow-steer__header">
            <span className="m-kicker m-kicker--ink">FLOW STEERING</span>
            <span className="m-kicker">{steeringPercent}% UNFAMILIAR</span>
          </div>
          <div
            className="m-flow-steer__track"
            role="slider"
            aria-label="Flow steering"
            aria-valuenow={steeringPercent}
            aria-valuemin={0}
            aria-valuemax={100}
            onClick={(e) => {
              const rect = e.currentTarget.getBoundingClientRect();
              c.handleFlowSteeringChange(Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width)));
            }}
          >
            <div className="m-flow-steer__fill" style={{ width: `${steeringPercent}%` }} />
            <div className="m-flow-steer__thumb" style={{ left: `${steeringPercent}%` }} />
          </div>
          <div className="m-flow-steer__footer">
            <span>FAMILIAR</span>
            <span>UNFAMILIAR</span>
          </div>
        </div>
        </section>

        <HomeSections c={c} />
      </div>
    </div>
  );
});
