import React, { useState, useEffect } from "react";
import { Minimize2, Music } from "lucide-react";
import { OnlinePlayingTrack, PlaybackState, Track } from "../../types";
import { LyricsView } from "./LyricsView";
import { NowPlayingBar } from "../NowPlayingBar";
import { executeQuery } from "../../services/api";

interface FullScreenPlayerViewProps {
  playbackState: PlaybackState;
  currentTrack?: Track;
  onlineTrack?: OnlinePlayingTrack | null;
  coverArtUrl?: string | null;
  isInPlaylist?: boolean;
  onOpenAddToPlaylist?: (track: {
    id: string;
    title: string;
    artist?: string;
    album?: string;
    cover_art_url?: string;
  }) => void;
  isLyricsActive: boolean;
  onToggleLyrics: () => void;
  onToggleFullscreen: () => void;
  onPlayPause: () => void;
  onNext: () => void;
  onPrevious: () => void;
  onSeek: (seconds: number) => void;
  onVolumeChange: (volume: number) => void;
  onToggleMute: () => void;
  onToggleRepeat: () => void;
  onToggleShuffle: () => void;
  onLike: (trackId: string) => void;
  onDislike: (trackId: string) => void;
  onRemoveFeedback: (trackId: string) => void;
  onDownloadOnlineTrack?: (artist: string, title: string) => void;
  onOpenOrigin?: () => void;
  originName?: string;
}

export const FullScreenPlayerView: React.FC<FullScreenPlayerViewProps> = ({
  playbackState,
  currentTrack,
  onlineTrack,
  coverArtUrl,
  isInPlaylist = false,
  onOpenAddToPlaylist,
  isLyricsActive,
  onToggleLyrics,
  onToggleFullscreen,
  onPlayPause,
  onNext,
  onPrevious,
  onSeek,
  onVolumeChange,
  onToggleMute,
  onToggleRepeat,
  onToggleShuffle,
  onLike,
  onDislike,
  onRemoveFeedback,
  onDownloadOnlineTrack,
  onOpenOrigin,
  originName,
}) => {
  const isOnline = !!onlineTrack && !playbackState.is_playing;
  const activeTitle = isOnline ? onlineTrack.title : currentTrack?.title || "No Track Selected";
  const activeArtist = isOnline
    ? onlineTrack.artist
    : currentTrack?.artist_name || (currentTrack ? "Unknown Artist" : "");

  const [internalCover, setInternalCover] = useState<string | null>(null);

  // Proactively pull local thumbnail if not already present in props
  useEffect(() => {
    if (isOnline || coverArtUrl || currentTrack?.cover_art_url || !currentTrack?.id) {
      return;
    }
    let isMounted = true;
    executeQuery({
      query: "GetTrackCoverArt",
      payload: { track_id: currentTrack.id },
    })
      .then((res: any) => {
        if (isMounted && res?.data) {
          setInternalCover(res.data);
        }
      })
      .catch(() => {});
    return () => {
      isMounted = false;
    };
  }, [isOnline, coverArtUrl, currentTrack?.cover_art_url, currentTrack?.id]);

  const activeArtworkUrl = isOnline
    ? onlineTrack.cover_art_url
    : coverArtUrl || currentTrack?.cover_art_url || internalCover || undefined;
  const duration = isOnline
    ? onlineTrack.duration
    : playbackState.duration_secs || currentTrack?.duration_secs || 0;
  const position = isOnline ? onlineTrack.currentTime : playbackState.position_secs || 0;

  return (
    <div
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        zIndex: 9999,
        backgroundColor: "#181410",
        backgroundImage:
          "radial-gradient(ellipse at top center, rgba(120, 80, 40, 0.28) 0%, rgba(20, 16, 12, 0.95) 75%, #0f0d0b 100%)",
        display: "flex",
        flexDirection: "column",
        justifyContent: "space-between",
        color: "#ffffff",
        overflow: "hidden",
        userSelect: "none",
      }}
    >
      {/* Top Header */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "24px 32px",
          zIndex: 10,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
          {isLyricsActive ? (
            <span
              style={{
                fontSize: "1.1rem",
                fontWeight: 700,
                letterSpacing: "-0.01em",
                color: "rgba(255, 255, 255, 0.95)",
              }}
            >
              {activeTitle}
            </span>
          ) : originName ? (
            <span
              style={{
                fontSize: "0.82rem",
                fontWeight: 600,
                padding: "3px 10px",
                borderRadius: "9999px",
                backgroundColor: "rgba(255, 255, 255, 0.12)",
                color: "rgba(255, 255, 255, 0.8)",
                letterSpacing: "0.02em",
              }}
            >
              From: {originName}
            </span>
          ) : (
            <span
              style={{
                fontSize: "0.85rem",
                fontWeight: 600,
                textTransform: "uppercase",
                letterSpacing: "0.08em",
                color: "rgba(255, 255, 255, 0.4)",
              }}
            >
              Now Playing
            </span>
          )}
          {isLyricsActive && originName && (
            <span
              style={{
                fontSize: "0.8rem",
                padding: "2px 8px",
                borderRadius: "6px",
                backgroundColor: "rgba(255, 255, 255, 0.1)",
                color: "rgba(255, 255, 255, 0.6)",
              }}
            >
              From: {originName}
            </span>
          )}
        </div>

        <button
          type="button"
          onClick={onToggleFullscreen}
          className="player-icon-btn"
          title="Exit full screen (Esc)"
          style={{
            color: "rgba(255, 255, 255, 0.7)",
            backgroundColor: "rgba(255, 255, 255, 0.08)",
            padding: "8px",
            borderRadius: "50%",
            border: "none",
            cursor: "pointer",
            transition: "all 0.15s ease",
          }}
        >
          <Minimize2 size={20} />
        </button>
      </div>

      {/* Middle Content: Large Album Art OR Lyrics View */}
      <div
        style={{
          flex: 1,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          minHeight: 0,
          position: "relative",
        }}
      >
        {isLyricsActive ? (
          <div style={{ width: "100%", height: "100%", position: "relative" }}>
            <LyricsView
              trackId={isOnline ? undefined : currentTrack?.id}
              artist={activeArtist}
              title={activeTitle}
              durationSecs={duration}
              currentTime={position}
              onSeek={onSeek}
              isFullScreen={true}
            />
          </div>
        ) : (
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              gap: "20px",
              maxWidth: "600px",
              width: "100%",
              padding: "0 24px",
            }}
          >
            <div
              style={{
                width: "min(380px, 50vh)",
                height: "min(380px, 50vh)",
                borderRadius: "16px",
                overflow: "hidden",
                boxShadow: "0 20px 50px rgba(0, 0, 0, 0.75)",
                backgroundColor: "rgba(255, 255, 255, 0.06)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              {activeArtworkUrl ? (
                <img
                  src={activeArtworkUrl}
                  alt={activeTitle}
                  style={{
                    width: "100%",
                    height: "100%",
                    objectFit: "cover",
                    display: "block",
                  }}
                />
              ) : (
                <Music size={72} color="rgba(255, 255, 255, 0.3)" />
              )}
            </div>

            {/* Song Title and Artist Name placed prominently at the bottom of the thumbnail image */}
            <div
              style={{
                textAlign: "center",
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                gap: "6px",
                width: "100%",
              }}
            >
              <h2
                style={{
                  fontSize: "1.75rem",
                  fontWeight: 700,
                  letterSpacing: "-0.02em",
                  color: "#ffffff",
                  margin: 0,
                  maxWidth: "100%",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                  textShadow: "0 2px 12px rgba(0,0,0,0.6)",
                }}
                title={activeTitle}
              >
                {activeTitle}
              </h2>
              {activeArtist && (
                <p
                  style={{
                    fontSize: "1.1rem",
                    fontWeight: 500,
                    color: "rgba(255, 255, 255, 0.7)",
                    margin: 0,
                    maxWidth: "100%",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                  title={activeArtist}
                >
                  {activeArtist}
                </p>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Bottom Status Bar / Player Bar (matching Image 1) */}
      <div
        style={{
          borderTop: "1px solid rgba(255, 255, 255, 0.08)",
          backgroundColor: "rgba(15, 12, 10, 0.95)",
          backdropFilter: "blur(20px)",
        }}
      >
        <NowPlayingBar
          playbackState={playbackState}
          currentTrack={currentTrack}
          onlineTrack={onlineTrack}
          coverArtUrl={activeArtworkUrl}
          isInPlaylist={isInPlaylist}
          onOpenAddToPlaylist={onOpenAddToPlaylist}
          onPlayPause={onPlayPause}
          onNext={onNext}
          onPrevious={onPrevious}
          onSeek={onSeek}
          onVolumeChange={onVolumeChange}
          onToggleMute={onToggleMute}
          onToggleRepeat={onToggleRepeat}
          onToggleShuffle={onToggleShuffle}
          onLike={onLike}
          onDislike={onDislike}
          onRemoveFeedback={onRemoveFeedback}
          onDownloadOnlineTrack={onDownloadOnlineTrack}
          onOpenOrigin={onOpenOrigin}
          originName={originName}
          isLyricsActive={isLyricsActive}
          onToggleLyrics={onToggleLyrics}
          isFullscreen={true}
          onToggleFullscreen={onToggleFullscreen}
        />
      </div>
    </div>
  );
};
