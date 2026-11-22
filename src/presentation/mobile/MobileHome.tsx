import { memo } from "react";
import { MobileButton, EmptyNote, SectionHeader } from "./primitives";
import { TrackRow } from "./TrackRow";
import { joinMeta } from "./format";
import type { MobileScreenProps } from "./types";

/** Home: start or pause Flow, then the recent, rediscovered and recommended tracks. */
export const MobileHome = memo(function MobileHome({ c }: MobileScreenProps) {
  const overview = c.homeOverview;
  const session = overview?.flow_session;
  const active = !!session?.is_active;
  const onFlow = () => void (active ? c.handleToggleFlowPlayback() : c.handleStartFlow(c.flowSteering));
  const pending = c.flowPendingAction !== null;
  return (
    <div className="m-screen__body">
      <section className="m-flow" aria-label="Flow">
        <p className="m-flow__label">{active ? session?.flow_label : "Flow"}</p>
        <h1 className="m-flow__title">{active ? "In the flow" : "Play what fits"}</h1>
        <label className="m-flow__steer">
          <span className="m-field__label">Familiar ↔ New</span>
          <input type="range" min={0} max={1} step={0.05} value={c.flowSteering}
            onChange={(e) => c.handleFlowSteeringChange(Number(e.target.value))} />
        </label>
        <MobileButton disabled={pending} onClick={onFlow}>{active ? "Pause Flow" : "Start Flow"}</MobileButton>
      </section>
      <SectionHeader title="Recent rotation" />
      {overview?.recent_rotation.length ? overview.recent_rotation.map((item) => (
        <TrackRow key={`${item.track_id}:${item.started_at}`} title={item.title}
          subtitle={joinMeta(item.artist, item.album)} durationSecs={item.duration_secs}
          src={item.cover_art_url} trackId={item.track_id} onPress={() => void c.handlePlayTrack(item.track_id)} />
      )) : <EmptyNote>{c.isHomeLoading ? "Loading…" : "Nothing played yet."}</EmptyNote>}
      <SectionHeader title="For you" />
      {overview?.for_you.length ? overview.for_you.map((item) => (
        <TrackRow key={item.track_id} title={item.title} subtitle={joinMeta(item.artist_name, item.album_title)}
          durationSecs={item.duration_secs} trackId={item.track_id}
          onPress={() => void c.handlePlayTrack(item.track_id)} />
      )) : <EmptyNote>Recommendations appear as you listen.</EmptyNote>}
    </div>
  );
});
