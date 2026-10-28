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
  activeTrackId?: string | null;
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
              background: "rgba(0, 0, 0, 0.7)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              zIndex: 1000,
            }}
            onClick={() => setWhyModalTrack(null)}
          >
            <div
              style={{
                background: "var(--bg-card, #2a211b)",
                border: "1px solid rgba(232, 216, 201, 0.2)",
                borderRadius: "14px",
                padding: "24px",
                maxWidth: "420px",
                width: "90%",
                boxShadow: "0 20px 40px rgba(0, 0, 0, 0.6)",
              }}
              onClick={(e) => e.stopPropagation()}
            >
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  marginBottom: "14px",
                }}
              >
                <div
                  style={{ display: "flex", alignItems: "center", gap: "8px" }}
                >
                  <Info size={18} color="var(--accent, #f3701e)" />
                  <h4
                    style={{
                      margin: 0,
                      fontSize: "1.1rem",
                      fontWeight: 700,
                      color: "#e8d8c9",
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
                    padding: "4px",
                  }}
                >
                  <X size={18} />
                </button>
              </div>

              <div style={{ marginBottom: "12px" }}>
                <div
                  style={{
                    fontWeight: 650,
                    fontSize: "0.95rem",
                    color: "#e8d8c9",
                  }}
                >
                  {whyModalTrack.title}
                </div>
                <div
                  style={{ fontSize: "0.85rem", color: "var(--text-muted)" }}
                >
                  {whyModalTrack.artist}
                </div>
              </div>

              <div
                style={{
                  background: "rgba(22, 19, 17, 0.6)",
                  border: "1px solid var(--border)",
                  borderRadius: "8px",
                  padding: "12px 14px",
                  fontSize: "0.88rem",
                  color: "var(--accent-light, #ff9b60)",
                  lineHeight: 1.45,
                }}
              >
                <TrackExplanation evidence={whyModalTrack.explanation} />
              </div>

              <div
                style={{
                  marginTop: "18px",
                  display: "flex",
                  justifyContent: "flex-end",
                }}
              >
                <button
                  className="btn btn-secondary"
                  style={{ padding: "6px 16px", borderRadius: "6px" }}
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
