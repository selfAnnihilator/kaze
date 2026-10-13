import React, { useState, useCallback, useEffect } from "react";
import {
  Play,
  Pause,
  Sparkles,
  RefreshCw,
  Disc,
  Info,
  Sliders,
  ListPlus,
  Check,
  Music2,
  X,
} from "lucide-react";
import { executeQuery } from "../../services/api";
import {
  HomeOverview, FlowExplanation, Track,
} from "../../types";

import { TrackExplanation } from "../TrackExplanation";
import { presentFlowExplanation } from "../../flowExplanation";
import { isCurrentPlaybackTrack } from "../../localTrackMatch";

interface HomeViewProps {
  overview: HomeOverview | null;
  isLoading: boolean;
  onRefresh: () => void;
  onStartFlow: (familiarityRatio: number) => void;
  onTogglePlayback: () => void;
  onToggleFlowPlayback: () => void;
  currentTrack?: Track | null;
  onStopFlow: () => void;
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
  isLoading,
  onRefresh,
  onStartFlow,
  onTogglePlayback,
  onToggleFlowPlayback,
  currentTrack,
  onStopFlow,
  onPlayTrack,
  onEnqueueTrack,
  onOpenAlbum,
  loadingTrackId,
  activeTrackId,
  isPlaying = false,
  flowSteering,
  onFlowSteeringChange,
  queuedTrackIds,
}) => {
  const [whyModalTrack, setWhyModalTrack] = useState<{
    title: string;
    artist: string;
    explanation?: FlowExplanation | null;
  } | null>(null);

  const flowSession = overview?.flow_session;
  const hasPreviousSession = !!flowSession?.is_active;
  const isFlowCurrent = hasPreviousSession && !!flowSession?.last_track_id && flowSession.last_track_id === currentTrack?.id;
  const flowAction = isFlowCurrent ? (isPlaying ? "Pause Flow" : "Resume Flow") : "Start Flow";

  // Steering label derivation
  const getSteeringLabel = (val: number) => {
    if (val >= 0.75) return "Familiar · Proven favorites & high affinity";
    if (val <= 0.25) return "Rediscover · Deep cuts & forgotten library gems";
    return "Balanced blend · Familiar favorites & rediscovery";
  };

  const handleStartOrResume = useCallback(() => {
    if (isFlowCurrent) {
      onToggleFlowPlayback();
    } else {
      onStartFlow(flowSteering);
    }
  }, [isFlowCurrent, onToggleFlowPlayback, onStartFlow, flowSteering]);

  return (
    <div
      className="view-container home-view"
      style={{
        padding: "32px 40px 100px 40px",
        maxWidth: "1400px",
        margin: "0 auto",
        color: "var(--text-main, #e8d8c9)",
      }}
    >
      {/* Header bar */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-end",
          marginBottom: "28px",
        }}
      >
        <div>
          <span
            style={{
              fontSize: "0.75rem",
              letterSpacing: "0.18em",
              textTransform: "uppercase",
              color: "var(--accent, #f3701e)",
              fontWeight: 700,
            }}
          >
            Kaze Home
          </span>
          <h1
            style={{
              fontSize: "2rem",
              fontWeight: 700,
              margin: "4px 0 0 0",
              color: "#e8d8c9",
              letterSpacing: "-0.02em",
            }}
          >
            What should I listen to right now?
          </h1>
          <p
            style={{
              margin: "6px 0 0 0",
              fontSize: "0.9rem",
              color: "var(--text-muted, #b6aa9d)",
            }}
          >
            Your music, always moving.
          </p>
        </div>

        <button
          onClick={onRefresh}
          className="btn btn-secondary"
          disabled={isLoading}
          style={{
            display: "flex",
            alignItems: "center",
            gap: "8px",
            fontSize: "0.85rem",
            padding: "8px 16px",
            borderRadius: "8px",
            background: "rgba(42, 33, 27, 0.7)",
            border: "1px solid var(--border, rgba(232, 216, 201, 0.15))",
            color: "var(--text-main, #e8d8c9)",
            cursor: "pointer",
          }}
          title="Refresh Home recommendations"
        >
          <RefreshCw size={15} className={isLoading ? "spin-animation" : ""} />
          <span>Refresh</span>
        </button>
      </div>

      {/* Primary Card: FLOW */}
      <div
        style={{
          background: "linear-gradient(135deg, #2a211b 0%, #1d1815 100%)",
          border: "1px solid rgba(243, 112, 30, 0.28)",
          borderRadius: "16px",
          padding: "32px",
          marginBottom: "40px",
          position: "relative",
          boxShadow: "0 16px 36px rgba(0, 0, 0, 0.4)",
        }}
      >
        <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "space-between", gap: "24px", alignItems: "flex-start" }}>
          <div style={{ maxWidth: "620px" }}>
            <div style={{ display: "inline-flex", alignItems: "center", gap: "6px", background: "rgba(243, 112, 30, 0.15)", padding: "4px 10px", borderRadius: "20px", marginBottom: "14px" }}>
              <Sparkles size={14} color="var(--accent, #f3701e)" />
              <span style={{ fontSize: "0.75rem", fontWeight: 700, letterSpacing: "0.12em", textTransform: "uppercase", color: "var(--accent-light, #ff9b60)" }}>
                {flowSession?.is_active ? "Flow Active" : "Adaptive Queue"}
              </span>
            </div>

            <h2 style={{ fontSize: "1.75rem", fontWeight: 700, margin: "0 0 8px 0", color: "#e8d8c9" }}>
              {hasPreviousSession ? "Continue your Flow" : "Start your Flow"}
            </h2>

            <p style={{ fontSize: "0.95rem", color: "var(--text-muted, #b6aa9d)", margin: "0 0 16px 0", lineHeight: 1.5 }}>
              {flowSession?.flow_label || "Let Kaze build a continuously adapting queue from your library based on your listening behavior."}
            </p>

            {flowSession?.last_track_title && (
              <div style={{ fontSize: "0.85rem", color: "var(--text-dim, #948779)", marginBottom: "20px" }}>
                <span>Last in Flow: </span>
                <strong style={{ color: "#e8d8c9" }}>{flowSession.last_track_title}</strong>
                {flowSession.last_track_artist && <span> · {flowSession.last_track_artist}</span>}
              </div>
            )}

            <div style={{ display: "flex", alignItems: "center", gap: "16px" }}>
              <button
                onClick={handleStartOrResume}
                disabled={isLoading}
                className="btn btn-primary"
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: "10px",
                  padding: "12px 28px",
                  fontSize: "1rem",
                  fontWeight: 650,
                  borderRadius: "10px",
                  background: "var(--accent, #f3701e)",
                  color: "#1a1714",
                  border: "none",
                  cursor: "pointer",
                  boxShadow: "0 4px 16px rgba(243, 112, 30, 0.35)",
                  transition: "transform 0.15s ease",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.transform = "scale(1.02)")}
                onMouseLeave={(e) => (e.currentTarget.style.transform = "scale(1)")}
              >
                {isFlowCurrent && isPlaying ? <Pause size={18} fill="#1a1714" /> : <Play size={18} fill="#1a1714" />}
                <span>{flowAction}</span>
              </button>
              {flowSession?.is_active && <button className="btn btn-secondary" onClick={onStopFlow}>Stop Flow</button>}
            </div>
          </div>

          {/* Steering Control */}
          <div
            style={{
              background: "rgba(22, 19, 17, 0.7)",
              border: "1px solid var(--border, rgba(232, 216, 201, 0.12))",
              borderRadius: "12px",
              padding: "20px 24px",
              minWidth: "280px",
              maxWidth: "360px",
              flex: "1 1 300px",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "12px" }}>
              <Sliders size={16} color="var(--accent, #f3701e)" />
              <span style={{ fontSize: "0.85rem", fontWeight: 700, color: "#e8d8c9" }}>
                Flow Steering
              </span>
            </div>

            <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.75rem", color: "var(--text-dim, #948779)", marginBottom: "6px" }}>
              <span style={{ fontWeight: flowSteering <= 0.3 ? 700 : 400, color: flowSteering <= 0.3 ? "var(--accent-secondary, #8b7cf6)" : undefined }}>
                Rediscover
              </span>
              <span style={{ fontWeight: flowSteering >= 0.7 ? 700 : 400, color: flowSteering >= 0.7 ? "var(--accent, #f3701e)" : undefined }}>
                Familiar
              </span>
            </div>

            <input
              type="range"
              min={0}
              max={1}
              step={0.05}
              value={flowSteering}
              onChange={(e) => onFlowSteeringChange(parseFloat(e.target.value))}
              style={{
                width: "100%",
                accentColor: "var(--accent, #f3701e)",
                cursor: "pointer",
                marginBottom: "8px",
              }}
            />

            <div style={{ fontSize: "0.8rem", color: "var(--text-muted, #b6aa9d)", lineHeight: 1.4, minHeight: "36px" }}>
              {getSteeringLabel(flowSteering)}
            </div>

            <div style={{ marginTop: "10px", fontSize: "0.72rem", color: "var(--text-dim, #948779)", borderTop: "1px solid rgba(232,216,201,0.08)", paddingTop: "8px" }}>
              Changes guide the next generated batch.
            </div>
          </div>
        </div>
      </div>

      {/* 2-column or grid layout for secondary sections */}
      <div style={{ display: "flex", flexDirection: "column", gap: "40px" }}>

        {/* RECENT ROTATION */}
        <section>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "16px" }}>
            <div>
              <h3 style={{ fontSize: "1.25rem", fontWeight: 700, margin: 0, color: "#e8d8c9" }}>
                Recent Rotation
              </h3>
              <p style={{ fontSize: "0.82rem", color: "var(--text-dim, #948779)", margin: "2px 0 0 0" }}>
                Tracks and artists you have listened to recently
              </p>
            </div>
          </div>

          {(!overview?.recent_rotation || overview.recent_rotation.length === 0) ? (
            <div style={{ padding: "24px", background: "rgba(42, 33, 27, 0.4)", borderRadius: "12px", border: "1px dashed var(--border)", color: "var(--text-dim)" }}>
              No recent listening history yet. Start your Flow or play tracks from your library to build rotation.
            </div>
          ) : (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))",
                gap: "16px",
              }}
            >
              {overview.recent_rotation.map((track) => {
                const isCurrent = activeTrackId === track.track_id;
                const isCurrentPlaying = isCurrent && isPlaying;
                const isLoadingTrack = loadingTrackId === track.track_id;
                const isQueued = queuedTrackIds?.has(track.track_id);

                return (
                  <div
                    key={track.track_id}
                    style={{
                      background: isCurrent ? "rgba(243, 112, 30, 0.12)" : "rgba(42, 33, 27, 0.5)",
                      border: isCurrent ? "1px solid rgba(243, 112, 30, 0.4)" : "1px solid var(--border, rgba(232, 216, 201, 0.1))",
                      borderRadius: "12px",
                      padding: "12px",
                      transition: "transform 0.15s ease, opacity 0.15s ease",
                      position: "relative",
                      cursor: "pointer",
                    }}
                    onClick={() => onPlayTrack(track.track_id, "recent_rotation")}
                  >
                    <div
                      style={{
                        width: "100%",
                        aspectRatio: "1/1",
                        borderRadius: "8px",
                        background: "rgba(23, 21, 19, 0.8)",
                        marginBottom: "10px",
                        position: "relative",
                        overflow: "hidden",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                      }}
                    >
                      {track.cover_art_url ? (
                        <HomeCover
                          trackId={track.track_id}
                          alt=""
                          style={{ width: "100%", height: "100%", objectFit: "cover" }}
                          loading="lazy"
                        />
                      ) : (
                        <Music2 size={32} color="var(--text-dim, #948779)" />
                      )}

                      {/* Play button overlay */}
                      <button
                        style={{
                          position: "absolute",
                          bottom: "8px",
                          right: "8px",
                          width: "36px",
                          height: "36px",
                          borderRadius: "50%",
                          background: "var(--accent, #f3701e)",
                          border: "none",
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "center",
                          cursor: "pointer",
                          boxShadow: "0 4px 10px rgba(0,0,0,0.5)",
                        }}
                        onClick={(e) => {
                          e.stopPropagation();
                          onPlayTrack(track.track_id, "recent_rotation");
                        }}
                      >
                        {isLoadingTrack ? (
                          <RefreshCw size={16} className="spin-animation" color="#1a1714" />
                        ) : isCurrentPlaying ? (
                          <Pause size={16} color="#1a1714" fill="#1a1714" />
                        ) : (
                          <Play size={16} color="#1a1714" fill="#1a1714" style={{ marginLeft: "2px" }} />
                        )}
                      </button>
                    </div>

                    <div style={{ fontWeight: 600, fontSize: "0.9rem", color: "#e8d8c9", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                      {track.title}
                    </div>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: "2px" }}>
                      <div style={{ fontSize: "0.8rem", color: "var(--text-dim, #948779)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                        {track.artist}
                      </div>
                      {onEnqueueTrack && (
                        <button
                          style={{
                            background: "none",
                            border: "none",
                            color: isQueued ? "var(--accent, #f3701e)" : "var(--text-dim, #948779)",
                            padding: "4px",
                            cursor: "pointer",
                          }}
                          title={isQueued ? "In queue" : "Enqueue track"}
                          onClick={(e) => {
                            e.stopPropagation();
                            onEnqueueTrack(track.track_id);
                          }}
                        >
                          {isQueued ? <Check size={14} /> : <ListPlus size={14} />}
                        </button>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </section>

        {/* REDISCOVER */}
        <section>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "16px" }}>
            <div>
              <h3 style={{ fontSize: "1.25rem", fontWeight: 700, margin: 0, color: "#e8d8c9" }}>
                Rediscover
              </h3>
              <p style={{ fontSize: "0.82rem", color: "var(--text-dim, #948779)", margin: "2px 0 0 0" }}>
                Music you haven't heard in a while
              </p>
            </div>
          </div>

          {(!overview?.rediscover || overview.rediscover.length === 0) ? (
            <div style={{ padding: "24px", background: "rgba(42, 33, 27, 0.4)", borderRadius: "12px", border: "1px dashed var(--border)", color: "var(--text-dim)" }}>
              No tracks to rediscover yet. As you build your listening library, forgotten favorites will surface here automatically.
            </div>
          ) : (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))",
                gap: "14px",
              }}
            >
              {overview.rediscover.map((track) => {
                const isCurrent = activeTrackId === track.track_id;
                const isCurrentPlaying = isCurrent && isPlaying;
                const isLoadingTrack = loadingTrackId === track.track_id;
                const isQueued = queuedTrackIds?.has(track.track_id);

                return (
                  <div
                    key={track.track_id}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "12px",
                      background: isCurrent ? "rgba(243, 112, 30, 0.12)" : "rgba(42, 33, 27, 0.45)",
                      border: isCurrent ? "1px solid rgba(243, 112, 30, 0.4)" : "1px solid var(--border, rgba(232, 216, 201, 0.1))",
                      borderRadius: "10px",
                      padding: "10px 14px",
                      cursor: "pointer",
                      transition: "opacity 0.15s ease",
                    }}
                    onClick={() => onPlayTrack(track.track_id, "rediscover")}
                  >
                    <div
                      style={{
                        width: "44px",
                        height: "44px",
                        borderRadius: "6px",
                        background: "rgba(23, 21, 19, 0.8)",
                        overflow: "hidden",
                        flexShrink: 0,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                      }}
                    >
                      {track.cover_art_url ? (
                        <HomeCover trackId={track.track_id} alt="" style={{ width: "100%", height: "100%", objectFit: "cover" }} />
                      ) : (
                        <Music2 size={20} color="var(--text-dim, #948779)" />
                      )}
                    </div>

                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ fontWeight: 600, fontSize: "0.88rem", color: "#e8d8c9", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                        {track.title}
                      </div>
                      <div style={{ fontSize: "0.78rem", color: "var(--text-dim, #948779)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                        {track.artist}
                      </div>
                      <div style={{ fontSize: "0.72rem", color: "var(--accent-light, #ff9b60)", marginTop: "2px" }}>
                        {presentFlowExplanation(track.explanation).title}
                      </div>
                    </div>

                    <div style={{ display: "flex", alignItems: "center", gap: "6px" }} onClick={(e) => e.stopPropagation()}>
                      <button
                        style={{
                          background: "none",
                          border: "none",
                          color: "var(--text-muted)",
                          padding: "6px",
                          cursor: "pointer",
                        }}
                        title="Why this track?"
                        onClick={() => setWhyModalTrack({ title: track.title, artist: track.artist, explanation: track.explanation })}
                      >
                        <Info size={15} />
                      </button>

                      {onEnqueueTrack && (
                        <button
                          style={{
                            background: isQueued ? "rgba(139, 124, 246, 0.18)" : "none",
                            border: "none",
                            color: isQueued ? "var(--accent-secondary, #8b7cf6)" : "var(--text-dim)",
                            padding: "6px",
                            cursor: "pointer",
                          }}
                          title={isQueued ? "Already in queue" : "Add to queue"}
                          onClick={() => onEnqueueTrack(track.track_id)}
                        >
                          {isQueued ? <Check size={16} /> : <ListPlus size={16} />}
                        </button>
                      )}

                      <button
                        style={{
                          width: "30px",
                          height: "30px",
                          borderRadius: "50%",
                          background: "rgba(243, 112, 30, 0.2)",
                          border: "1px solid rgba(243, 112, 30, 0.4)",
                          color: "var(--accent)",
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "center",
                          cursor: "pointer",
                        }}
                        onClick={() => onPlayTrack(track.track_id, "rediscover")}
                      >
                        {isLoadingTrack ? (
                          <RefreshCw size={13} className="spin-animation" />
                        ) : isCurrentPlaying ? (
                          <Pause size={13} />
                        ) : (
                          <Play size={13} style={{ marginLeft: "1px" }} />
                        )}
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </section>

        {/* FOR YOU & LIBRARY MOMENT & YOUR WEEK */}
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(320px, 1fr))", gap: "24px" }}>

          {/* FOR YOU */}
          <div style={{ background: "rgba(42, 33, 27, 0.4)", border: "1px solid var(--border)", borderRadius: "14px", padding: "20px" }}>
            <h3 style={{ fontSize: "1.1rem", fontWeight: 700, margin: "0 0 4px 0", color: "#e8d8c9" }}>
              For You
            </h3>
            <p style={{ fontSize: "0.8rem", color: "var(--text-dim, #948779)", margin: "0 0 14px 0" }}>
              Library picks informed by your listening
            </p>

            {(!overview?.for_you || overview.for_you.length === 0) ? (
              <div style={{ fontSize: "0.85rem", color: "var(--text-dim)", padding: "12px 0" }}>
                No recommendations yet. Start playing tracks to establish your musical taste profile.
              </div>
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                {overview.for_you.slice(0, 5).map((rec) => {
                  const isCurrent = isCurrentPlaybackTrack(rec, currentTrack);
                  const primaryReason = presentFlowExplanation(rec.explanation).title;
                  const playLabel = isCurrent ? (isPlaying ? "Pause" : "Resume") : "Play";
                  const playRecommendation = () => isCurrent ? onTogglePlayback() : onPlayTrack(rec.track_id, "for_you");

                  return (
                    <div
                      key={rec.track_id}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        padding: "8px 10px",
                        borderRadius: "8px",
                        background: isCurrent ? "rgba(243, 112, 30, 0.15)" : "rgba(23, 21, 19, 0.4)",
                        cursor: "pointer",
                      }}
                      onClick={playRecommendation}
                    >
                      <div style={{ minWidth: 0, flex: 1, marginRight: "8px" }}>
                        <div style={{ fontSize: "0.86rem", fontWeight: 600, color: "#e8d8c9", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                          {rec.title}
                        </div>
                        <div style={{ fontSize: "0.76rem", color: "var(--text-dim)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                          {rec.artist_name || "Unknown Artist"}
                        </div>
                      </div>

                      <div style={{ display: "flex", alignItems: "center", gap: "6px" }} onClick={(e) => e.stopPropagation()}>
                        <button
                          style={{ background: "none", border: "none", color: "var(--text-dim)", padding: "4px", cursor: "pointer" }}
                          title={`Why: ${primaryReason}`}
                          onClick={() => setWhyModalTrack({ title: rec.title, artist: rec.artist_name || "Unknown Artist", explanation: rec.explanation })}
                        >
                          <Info size={14} />
                        </button>
                        <button
                          style={{
                            width: "28px",
                            height: "28px",
                            borderRadius: "50%",
                            background: "rgba(243, 112, 30, 0.2)",
                            border: "none",
                            color: "var(--accent)",
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                            cursor: "pointer",
                          }}
                          onClick={playRecommendation}
                          title={`${playLabel} ${rec.title}`}
                          aria-label={`${playLabel} ${rec.title}`}
                        >
                          {isCurrent && isPlaying ? <Pause size={12} /> : <Play size={12} style={{ marginLeft: "1px" }} />}
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          {/* LIBRARY MOMENT */}
          <div style={{ background: "rgba(42, 33, 27, 0.4)", border: "1px solid var(--border)", borderRadius: "14px", padding: "20px", display: "flex", flexDirection: "column" }}>
            <h3 style={{ fontSize: "1.1rem", fontWeight: 700, margin: "0 0 4px 0", color: "#e8d8c9" }}>
              Library Moment
            </h3>
            <p style={{ fontSize: "0.8rem", color: "var(--text-dim, #948779)", margin: "0 0 16px 0" }}>
              Something from your own collection worth revisiting
            </p>

            {overview?.library_moment ? (
              <div style={{ flex: 1, display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
                <div style={{ display: "flex", gap: "16px", alignItems: "center" }}>
                  <div
                    style={{
                      width: "80px",
                      height: "80px",
                      borderRadius: "8px",
                      background: "rgba(23, 21, 19, 0.8)",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      flexShrink: 0,
                    }}
                  >
                    <Disc size={40} color="var(--accent, #f3701e)" />
                  </div>
                  <div>
                    <div style={{ fontWeight: 700, fontSize: "1.05rem", color: "#e8d8c9" }}>
                      {overview.library_moment.album_title}
                    </div>
                    <div style={{ fontSize: "0.88rem", color: "var(--text-muted, #b6aa9d)", marginTop: "2px" }}>
                      {overview.library_moment.artist_name}
                    </div>
                    <div style={{ fontSize: "0.78rem", color: "var(--text-dim)", marginTop: "4px" }}>
                      {overview.library_moment.total_tracks > 0 ? `${overview.library_moment.total_tracks} tracks` : "Album"}
                      {overview.library_moment.release_year ? ` · ${overview.library_moment.release_year}` : ""}
                    </div>
                  </div>
                </div>

                <div style={{ display: "flex", gap: "10px", marginTop: "20px" }}>
                  {onOpenAlbum && (
                    <button
                      className="btn btn-secondary"
                      style={{
                        fontSize: "0.85rem",
                        padding: "8px 16px",
                        borderRadius: "8px",
                        background: "rgba(255,255,255,0.06)",
                        color: "var(--text-main)",
                        border: "1px solid var(--border)",
                        cursor: "pointer",
                      }}
                      onClick={() => onOpenAlbum(overview.library_moment!.album_id)}
                    >
                      View Album
                    </button>
                  )}
                </div>
              </div>
            ) : (
              <div style={{ fontSize: "0.85rem", color: "var(--text-dim)", padding: "12px 0" }}>
                Add music folders to your library to unlock album spotlights.
              </div>
            )}
          </div>

          {/* YOUR WEEK */}
          <div style={{ background: "rgba(42, 33, 27, 0.4)", border: "1px solid var(--border)", borderRadius: "14px", padding: "20px" }}>
            <h3 style={{ fontSize: "1.1rem", fontWeight: 700, margin: "0 0 4px 0", color: "#e8d8c9" }}>
              Your Week
            </h3>
            <p style={{ fontSize: "0.8rem", color: "var(--text-dim, #948779)", margin: "0 0 16px 0" }}>
              Listening summary for the past 7 days
            </p>

            <div
              style={{
                display: "grid",
                gridTemplateColumns: "1fr 1fr",
                gap: "14px",
              }}
            >
              <div style={{ background: "rgba(23, 21, 19, 0.5)", borderRadius: "10px", padding: "14px", border: "1px solid var(--border)" }}>
                <div style={{ fontSize: "0.75rem", textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--text-dim)" }}>
                  Time Listened
                </div>
                <div style={{ fontSize: "1.4rem", fontWeight: 700, color: "var(--accent, #f3701e)", marginTop: "4px" }}>
                  {overview?.your_week?.formatted_time || "0m"}
                </div>
              </div>

              <div style={{ background: "rgba(23, 21, 19, 0.5)", borderRadius: "10px", padding: "14px", border: "1px solid var(--border)" }}>
                <div style={{ fontSize: "0.75rem", textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--text-dim)" }}>
                  Tracks Played
                </div>
                <div style={{ fontSize: "1.4rem", fontWeight: 700, color: "#e8d8c9", marginTop: "4px" }}>
                  {overview?.your_week?.track_count || 0}
                </div>
              </div>

              <div style={{ background: "rgba(23, 21, 19, 0.5)", borderRadius: "10px", padding: "14px", border: "1px solid var(--border)", gridColumn: "span 2" }}>
                <div style={{ fontSize: "0.75rem", textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--text-dim)" }}>
                  Artists Explored
                </div>
                <div style={{ fontSize: "1.4rem", fontWeight: 700, color: "var(--accent-secondary, #8b7cf6)", marginTop: "4px" }}>
                  {overview?.your_week?.artist_count || 0}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* "Why This Track?" Lightweight Modal */}
      {whyModalTrack && (
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
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "14px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <Info size={18} color="var(--accent, #f3701e)" />
                <h4 style={{ margin: 0, fontSize: "1.1rem", fontWeight: 700, color: "#e8d8c9" }}>
                  Why this track?
                </h4>
              </div>
              <button
                onClick={() => setWhyModalTrack(null)}
                style={{ background: "none", border: "none", color: "var(--text-dim)", cursor: "pointer", padding: "4px" }}
              >
                <X size={18} />
              </button>
            </div>

            <div style={{ marginBottom: "12px" }}>
              <div style={{ fontWeight: 650, fontSize: "0.95rem", color: "#e8d8c9" }}>
                {whyModalTrack.title}
              </div>
              <div style={{ fontSize: "0.85rem", color: "var(--text-muted)" }}>
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

            <div style={{ marginTop: "18px", display: "flex", justifyContent: "flex-end" }}>
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
      )}
    </div>
  );
});

const HomeCover = React.memo(({ trackId, ...props }: React.ImgHTMLAttributes<HTMLImageElement> & { trackId: string }) => {
  const [src, setSrc] = useState<string>();
  useEffect(() => {
    let active = true;
    executeQuery({ query: "GetTrackCoverArt", payload: { track_id: trackId } })
      .then(res => { if (active && typeof res.data === "string") setSrc(res.data); })
      .catch(console.error);
    return () => { active = false; };
  }, [trackId]);
  return src ? <img {...props} src={src} /> : <Music2 size={24} />;
});
