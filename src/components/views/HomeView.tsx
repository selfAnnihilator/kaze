import { BrutalistHome } from "../../presentation/BrutalistHome";
import { SpatialHome } from "../../presentation/SpatialHome";
import type { PlaybackArtworkTrack } from "../../presentation/queuePresentation";
import { useInterfaceStyle } from "../../presentation/InterfaceContext";
import React, { useState, useCallback } from "react";
import { Info, X } from "lucide-react";
import {
  HomeOverview, FlowExplanation, Track } from "../../types";

import { TrackExplanation } from "../TrackExplanation";

interface HomeViewProps {
  onDiscover?: () => void;
  overview: HomeOverview | null;
  isLoading: boolean;
  flowPendingAction?: "start" | "resume" | "pause" | null;
  onRefresh: () => void;
  onStartFlow: (familiarityRatio: number) => void;
  onTogglePlayback: () => void;
  onToggleFlowPlayback: () => void;
  currentTrack?: Track | null;
  currentCover?: string | null;
  previousTrack?: Track;
  nextTrack?: PlaybackArtworkTrack;
  onPreviousTrack: () => void;
  onNextTrack: () => void;
  onPlayTrack: (trackId: string, source?: string) => void;
  onEnqueueTrack?: (trackId: string) => void;
  onOpenAlbum?: (albumId: string) => void;
  loadingTrackId?: string | null;
  isPlaying?: boolean;
  flowSteering: number;
  onFlowSteeringChange: (val: number) => void;
  queuedTrackIds?: Set<string>;
}

export const HomeView: React.FC<HomeViewProps> = React.memo(({
  overview,
  onDiscover,
  isLoading,
  flowPendingAction = null,
  onRefresh,
  onStartFlow,
  onTogglePlayback,
  onToggleFlowPlayback,
  currentTrack,
  currentCover,
  previousTrack,
  nextTrack,
  onPreviousTrack,
  onNextTrack,
  onPlayTrack,
  onEnqueueTrack,
  onOpenAlbum,
  loadingTrackId,
  isPlaying = false,
  flowSteering,
  onFlowSteeringChange,
  queuedTrackIds,
}) => {
  const interfaceStyle = useInterfaceStyle();
  const [whyModalTrack, setWhyModalTrack] = useState<{
    title: string;
    artist: string;
    explanation?: FlowExplanation | null;
  } | null>(null);

  const flowSession = overview?.flow_session;
  const hasPreviousSession = !!flowSession?.is_active;
  const isFlowCurrent = hasPreviousSession && !!flowSession?.last_track_id && flowSession.last_track_id === currentTrack?.id;
  const flowAction = flowPendingAction ? ({ start: "Starting Flow…", resume: "Starting Flow…", pause: "Pausing Flow…" })[flowPendingAction] : isFlowCurrent ? isPlaying
        ? "Pause Flow"
        : "Start Flow"
      : "Start Flow";

  const handleStartOrResume = useCallback(() => {
    if (isFlowCurrent) {
      onToggleFlowPlayback();
    } else {
      onStartFlow(flowSteering);
    }
  }, [isFlowCurrent, onToggleFlowPlayback, onStartFlow, flowSteering]);

  const whyDialog = (whyModalTrack && (
          <div
            className="modal-overlay"
            style={{
              position: "fixed",
              inset: 0,
              background: "var(--shade-70)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              zIndex: 1000,
            }}
            onClick={() => setWhyModalTrack(null)}
          >
            <div
              style={{
                background: "var(--bg-card, var(--bg-card))",
                border: "1px solid var(--text-main-a20)",
                borderRadius: "14px",
                padding: "var(--space-24)",
                maxWidth: "420px",
                width: "90%",
                boxShadow: "0 20px 40px var(--shade-60)",
              }}
              onClick={(e) => e.stopPropagation()}
            >
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  marginBottom: "var(--space-14)",
                }}
              >
                <div
                  style={{ display: "flex", alignItems: "center", gap: "var(--space-8)" }}
                >
                  <Info size={18} color="var(--accent, var(--accent))" />
                  <h4
                    style={{
                      margin: 0,
                      fontSize: "var(--fs-18)",
                      fontWeight: "var(--weight-bold)",
                      color: "var(--text-main)",
                    }}
                  >
                    Why this track?
                  </h4>
                </div>
                <button
                  onClick={() => setWhyModalTrack(null)}
                  style={{
                    background: "none",
                    border: "none",
                    color: "var(--text-dim)",
                    cursor: "pointer",
                    padding: "var(--space-4)",
                  }}
                >
                  <X size={18} />
                </button>
              </div>

              <div style={{ marginBottom: "var(--space-12)" }}>
                <div
                  style={{
                    fontWeight: "var(--weight-demibold)",
                    fontSize: "var(--fs-15)",
                    color: "var(--text-main)",
                  }}
                >
                  {whyModalTrack.title}
                </div>
                <div
                  style={{ fontSize: "var(--fs-14)", color: "var(--text-muted)" }}
                >
                  {whyModalTrack.artist}
                </div>
              </div>

              <div
                style={{
                  background: "var(--palette-sand-900-a60)",
                  border: "1px solid var(--border)",
                  borderRadius: "8px",
                  padding: "var(--space-12) var(--space-14)",
                  fontSize: "var(--fs-14)",
                  color: "var(--accent-light, var(--accent-light))",
                  lineHeight: 1.45,
                }}
              >
                <TrackExplanation evidence={whyModalTrack.explanation} />
              </div>

              <div
                style={{
                  marginTop: "var(--space-18)",
                  display: "flex",
                  justifyContent: "flex-end",
                }}
              >
                <button
                  className="btn btn-secondary"
                  style={{ padding: "var(--space-6) var(--space-16)", borderRadius: "6px" }}
                  onClick={() => setWhyModalTrack(null)}
                >
                  Close
                </button>
              </div>
            </div>
          </div>
        ));
  if (interfaceStyle === "spatial") return <><SpatialHome overview={overview} currentTrack={currentTrack} currentCover={currentCover} previousTrack={previousTrack} nextTrack={nextTrack} onPreviousTrack={onPreviousTrack} onNextTrack={onNextTrack} isLoading={isLoading} isFlowPending={flowPendingAction !== null} isPlaying={isPlaying} flowAction={flowAction} onFlow={handleStartOrResume} flowSteering={flowSteering} onFlowSteeringChange={onFlowSteeringChange} onRefresh={onRefresh} onDiscover={onDiscover} onOpenAlbum={onOpenAlbum} onPlayTrack={onPlayTrack} onTogglePlayback={onTogglePlayback} onEnqueueTrack={onEnqueueTrack} queuedTrackIds={queuedTrackIds} loadingTrackId={loadingTrackId} onExplain={setWhyModalTrack} />{whyDialog}</>;

  return <><BrutalistHome overview={overview} currentTrack={currentTrack} currentCover={currentCover} previousTrack={previousTrack} nextTrack={nextTrack} onPreviousTrack={onPreviousTrack} onNextTrack={onNextTrack} isLoading={isLoading} isFlowPending={flowPendingAction !== null} isPlaying={isPlaying} flowAction={flowAction} onFlow={handleStartOrResume} flowSteering={flowSteering} onFlowSteeringChange={onFlowSteeringChange} onRefresh={onRefresh} onDiscover={onDiscover} onOpenAlbum={onOpenAlbum} onPlayTrack={onPlayTrack} onTogglePlayback={onTogglePlayback} onEnqueueTrack={onEnqueueTrack} queuedTrackIds={queuedTrackIds} loadingTrackId={loadingTrackId} onExplain={setWhyModalTrack} />{whyDialog}</>;
});
