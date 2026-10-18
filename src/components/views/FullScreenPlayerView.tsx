import React from "react";
import { Minimize2 } from "lucide-react";
import { LyricsView } from "./LyricsView";
import { NowPlayingBar, type NowPlayingBarProps } from "../NowPlayingBar";
import { PlaybackStage } from "../../presentation/PlaybackStage";
import type { Track } from "../../types";
import {
  BrutalistDeck,
  type BrutalistDeckExtras,
} from "../../presentation/BrutalistDeck";
import { BrutalistPoster } from "../../presentation/BrutalistPoster";

interface FullScreenPlayerViewProps extends NowPlayingBarProps {
  isLyricsActive: boolean;
  onToggleLyrics: () => void;
  onToggleFullscreen: () => void;
  previousTrack?: Track;
  nextTrack?: Pick<Track, "id" | "title" | "artist_name" | "cover_art_url">;
  brutalist?: BrutalistDeckExtras & {
    albumId?: string;
    onOpenAlbum: (id: string) => void;
  };
}

export const FullScreenPlayerView: React.FC<FullScreenPlayerViewProps> =
  React.memo((props) => {
    const {
      playbackState,
      currentTrack,
      coverArtUrl,
      previousTrack,
      nextTrack,
      isLoading,
      isLyricsActive,
      onSeek,
      onToggleFullscreen,
      originName,
    } = props;
    return (
      <div
        className={`fullscreen-player${props.brutalist ? " press-fullscreen" : ""}`}
        role="dialog"
        aria-modal="true"
        aria-label="Full screen player"
      >
        <header className="fullscreen-header">
          <span>{originName ? `From: ${originName}` : "Now Playing"}</span>
          <button
            className="player-icon-btn"
            title="Exit full screen (F / Esc)"
            aria-label="Exit full screen"
            aria-keyshortcuts="f Escape"
            onClick={onToggleFullscreen}
          >
            <Minimize2 size={22} />
          </button>
        </header>
        <div className="fullscreen-main">
          {isLyricsActive ? (
            <LyricsView
              trackId={currentTrack?.id}
              artist={currentTrack?.artist_name || ""}
              title={currentTrack?.title || ""}
              durationSecs={
                playbackState.duration_secs || currentTrack?.duration_secs || 0
              }
              onSeek={onSeek}
              isFullScreen
            />
          ) : props.brutalist ? (
            <BrutalistPoster
              track={currentTrack}
              cover={coverArtUrl}
              index={playbackState.current_queue_index}
              playing={playbackState.is_playing}
              loading={isLoading}
              entries={props.brutalist.entries}
              tracks={props.brutalist.tracks}
              onPlayIndex={props.brutalist.onPlayIndex}
              albumId={props.brutalist.albumId}
              onOpenAlbum={(id) => {
                onToggleFullscreen();
                props.brutalist?.onOpenAlbum(id);
              }}
            />
          ) : (
            <PlaybackStage
              track={currentTrack}
              cover={coverArtUrl}
              previous={previousTrack}
              next={nextTrack}
              index={playbackState.current_queue_index}
              playing={playbackState.is_playing}
              loading={isLoading}
              onPrevious={props.onPrevious}
              onNext={props.onNext}
            />
          )}
        </div>
        {props.brutalist ? (
          <BrutalistDeck
            {...props}
            {...props.brutalist}
            deckId="press-fullscreen-deck"
            isFullscreen
            onNavigate={(view) => {
              onToggleFullscreen();
              props.brutalist?.onNavigate(view);
            }}
          />
        ) : (
          <NowPlayingBar {...props} isFullscreen />
        )}
      </div>
    );
  });
