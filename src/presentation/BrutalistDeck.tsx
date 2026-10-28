import { DownloadButton } from "../components/DownloadButton";
import { NotInterestedButton } from "../components/NotInterestedButton";
import { memo } from "react";
import {
  Bookmark,
  BookmarkCheck,
  Heart,
  ListMusic,
  Maximize2,
  Minimize2,
  Mic2,
  Pause,
  Play,
  RefreshCw,
  Repeat,
  Repeat1,
  Settings,
  Shuffle,
  SkipBack,
  SkipForward,
  Sparkles,
  Volume2,
  VolumeX,
  ArrowUpRight,
  Wind,
} from "lucide-react";
import type { NowPlayingBarProps } from "../components/NowPlayingBar";
import { NowPlayingProgressBar } from "../components/NowPlayingBar";
import type { Track, UserProfile } from "../types";
import type { ViewType } from "../components/Sidebar";
import type { QueueEntry } from "./queuePresentation";
import { ProfileNavigation } from "./ProfileNavigation";
import { Artwork } from "./Artwork";
import { OverflowTitle } from "./OverflowTitle";

export type BrutalistDeckExtras = {
  entries: QueueEntry[];
  tracks: Track[];
  onPlayIndex: (index: number) => void;
  currentUser: UserProfile | null;
  view: ViewType;
  onNavigate: (view: ViewType) => void;
  flowActive: boolean;
  onToggleFlow: () => void;
  deckId?: string;
};

/** A different transport composition using exactly the same Rust actions. */
export const BrutalistDeck = memo(function BrutalistDeck(
  p: NowPlayingBarProps & BrutalistDeckExtras,
) {
  const { playbackState: state, currentTrack: track } = p;
  const liked = track?.manual_like === 1 || p.isOnlineLiked;
  const artist = track?.artist_name?.trim() || (track ? "Unknown Artist" : "Select a track to start playback");
  const online =
    track?.format === "online" || track?.file_path?.startsWith("online://");
  const toggleLike = () => {
    if (!track) return;
    if (online && p.onLikeOnline)
      p.onLikeOnline({
        id: track.id,
        title: track.title,
        artist: track.artist_name || "Unknown Artist",
        album: track.album_title,
        cover_art_url: track.cover_art_url,
        preview_url: track.preview_url,
        duration_secs: track.duration_secs,
      });
    else if (liked) p.onRemoveFeedback(track.id);
    else p.onLike(track.id);
  };
  return (
    <aside
      className="press-deck"
      aria-label="Playback controls"
      id={p.deckId || "press-deck"}
    >
      <div className="press-deck-controls">
        <div className="press-deck-label">
          <span>THE DECK</span>
          <span>
            {track
              ? String((state.current_queue_index ?? 0) + 1).padStart(2, "0")
              : "00"}{" "}
            /{" "}
            {String(
              (state.current_queue_index ?? -1) + 1 + p.entries.length,
            ).padStart(2, "0")}
          </span>
        </div>
        <div className="press-deck-track" aria-live="polite">
          <span className="press-micro">
            {p.isLoading
              ? "LOADING / BUFFERING"
              : state.is_playing
                ? "NOW PLAYING"
                : track
                  ? "PAUSED"
                  : "SELECT A RECORD"}
          </span>
          <div className="press-deck-identity">
            <button
              className="press-deck-cover"
              aria-label="Open Now Playing"
              title="Open Now Playing"
              onClick={p.onOpenNowPlaying}
            >
              <Artwork
                src={p.coverArtUrl || track?.cover_art_url}
                trackId={track?.id}
                alt={track?.title}
              />
            </button>
            <div>
              <h2>
                <button
                  onClick={p.onOpenOrigin}
                  disabled={!p.onOpenOrigin}
                  title={
                    p.originName ? `Playing from: ${p.originName}` : undefined
                  }
                >
                  <OverflowTitle text={track?.title || "No Track Selected"} />
                </button>
              </h2>
              <p title={artist}>{artist}</p>
            </div>
          </div>
        </div>
        <button
          className="press-play-main play-pause-btn"
          aria-label={
            p.isLoading ? "Loading song" : state.is_playing ? "Pause" : "Play"
          }
          title={
            p.isLoading ? "Loading song" : state.is_playing ? "Pause" : "Play"
          }
          disabled={p.isLoading}
          onClick={p.onPlayPause}
        >
          {p.isLoading ? (
            <RefreshCw size={42} className="spin-animation" />
          ) : state.is_playing ? (
            <Pause size={42} fill="currentColor" />
          ) : (
            <Play size={42} fill="currentColor" />
          )}
          <span>
            {p.isLoading ? "WAIT" : state.is_playing ? "PAUSE" : "PLAY"}
          </span>
        </button>
        <div className="press-skips">
          <button
            title="Previous Track"
            aria-label="Previous Track"
            onClick={p.onPrevious}
          >
            <SkipBack size={21} />
            PREV
          </button>
          <button title="Next Track" aria-label="Next Track" onClick={p.onNext}>
            NEXT
            <SkipForward size={21} />
          </button>
        </div>
        <NowPlayingProgressBar
          layout="deck"
          duration={state.duration_secs || track?.duration_secs || 0}
          canSeek={state.can_seek !== false && !p.isLoading}
          onSeek={p.onSeek}
        />
        <div className="press-options-row">
          <div className="press-deck-options">
            <button
              className="press-icon"
              title="Toggle Shuffle"
              aria-label="Toggle Shuffle"
              aria-pressed={state.is_shuffled}
              onClick={p.onToggleShuffle}
            >
              <Shuffle size={20} />
            </button>
            <button
              className="press-icon"
              title={
                state.repeat_mode === "one"
                  ? "Turn off song loop"
                  : "Loop current song"
              }
              aria-label="Loop current song"
              aria-pressed={state.repeat_mode === "one"}
              onClick={p.onToggleRepeat}
            >
              {state.repeat_mode === "one" ? (
                <Repeat1 size={20} />
              ) : (
                <Repeat size={20} />
              )}
            </button>
            <button
              className="press-icon"
              title={liked ? "Unlike track" : "Like track"}
              aria-label={liked ? "Unlike track" : "Like track"}
              aria-pressed={!!liked}
              disabled={!track}
              onClick={toggleLike}
            >
              <Heart size={20} fill={liked ? "currentColor" : "none"} />
            </button>
          </div>
          <div className="press-track-tools">
            {track && <NotInterestedButton key={`dislike:${track.id}`} trackId={track.id} title={track.title} notInterested={track.manual_like === -1} />}
            {p.onInspectWhyTrack && (
              <button
                className="press-icon"
                title="Why this track?"
                aria-label="Why this track?"
                onClick={p.onInspectWhyTrack}
              >
                <Sparkles size={18} />
              </button>
            )}
            {track && p.onOpenAddToPlaylist && (
              <button
                className="press-icon"
                title={
                  p.isInPlaylist
                    ? "In playlist (click to manage)"
                    : "Add to playlist"
                }
                aria-label="Add to playlist"
                onClick={() =>
                  p.onOpenAddToPlaylist?.({
                    id: track.id,
                    title: track.title,
                    artist: track.artist_name,
                    album: track.album_title,
                    cover_art_url: p.coverArtUrl || track.cover_art_url,
                  })
                }
              >
                {p.isInPlaylist ? (
                  <BookmarkCheck size={18} />
                ) : (
                  <Bookmark size={18} />
                )}
              </button>
            )}
            {online && track && <DownloadButton key={`download:${track.id}`} trackId={track.id} sourceId={track.playback_source?.track_id} title={track.title} compact />}
            {p.onToggleLyrics && (
              <button
                className="press-icon"
                title={p.isLyricsActive ? "Hide Lyrics" : "Show Lyrics"}
                aria-label={p.isLyricsActive ? "Hide Lyrics" : "Show Lyrics"}
                aria-pressed={p.isLyricsActive}
                onClick={p.onToggleLyrics}
              >
                <Mic2 size={18} />
              </button>
            )}
            {p.onToggleFullscreen && (
              <button
                className="press-icon"
                title={p.isFullscreen ? "Exit Full Screen (F)" : "Full Screen (F)"}
                aria-label={p.isFullscreen ? "Exit Full Screen" : "Full Screen"}
                aria-keyshortcuts="f"
                onClick={p.onToggleFullscreen}
              >
                {p.isFullscreen ? (
                  <Minimize2 size={18} />
                ) : (
                  <Maximize2 size={18} />
                )}
              </button>
            )}
          </div>
        </div>
        <div className="press-deck-next">
          <button
            className="press-deck-next-heading"
            aria-label="Open Queue"
            onClick={p.onOpenQueue}
          >
            UP NEXT
            <ArrowUpRight size={18} />
          </button>
          {p.entries.slice(0, 3).map((entry, i) => {
            const item = p.tracks.find((t) => t.id === entry.trackId);
            return (
              <button
                key={entry.key}
                className="press-deck-next-song"
                onClick={() => p.onPlayIndex(entry.index)}
              >
                <span>{String(i + 1).padStart(2, "0")}</span>
                <strong>
                  {item?.title || entry.title || "Queued track"}
                  <small>{item?.artist_name || entry.artist}</small>
                </strong>
                <ArrowUpRight size={16} />
              </button>
            );
          })}
          {!p.entries.length && (
            <p className="press-next-empty">
              End of the queue. Find your next record.
            </p>
          )}
        </div>
        <div className="press-deck-bottom">
          <button
            className="press-flow"
            aria-label={p.flowActive && state.is_playing ? "Pause Flow" : "Start Flow"}
            aria-pressed={p.flowActive && state.is_playing}
            onClick={p.onToggleFlow}
          >
            <Wind size={23} />
            <span>FLOW</span>
            <small>{p.flowActive ? state.is_playing ? "ON" : "PAUSED" : "OFF"}</small>
          </button>
          <div className="press-volume">
            <button
              className="press-icon"
              aria-label={state.is_muted ? "Unmute" : "Mute"}
              onClick={p.onToggleMute}
            >
              {state.is_muted ? <VolumeX size={20} /> : <Volume2 size={20} />}
            </button>
            <input
              type="range"
              aria-label="Volume"
              min={0}
              max={1}
              step={0.01}
              value={state.is_muted ? 0 : state.volume}
              onChange={(e) => p.onVolumeChange(Number(e.target.value))}
            />
            <output>
              {Math.round((state.is_muted ? 0 : state.volume) * 100)}
            </output>
          </div>
        </div>
      </div>
      <div className="press-account-navigation">
        <button
          className="nav-button"
          aria-label="Settings"
          title="Settings"
          aria-current={p.view === "settings" ? "page" : undefined}
          onClick={() => p.onNavigate("settings")}
        >
          <Settings size={20} />
          <span>Settings</span>
        </button>
        <ProfileNavigation
          user={p.currentUser}
          active={p.view === "stats"}
          compact={false}
          onSelectView={p.onNavigate}
        />
      </div>
      <div className="press-mobile-track">
        <button aria-label="Open Now Playing" onClick={p.onOpenNowPlaying}>
          <Artwork
            src={p.coverArtUrl || track?.cover_art_url}
            trackId={track?.id}
          />
        </button>
        <span>
          {track?.title || "No track selected"}
          <small>{track?.artist_name}</small>
        </span>
        <a href={`#${p.deckId || "press-deck"}`}>
          THE DECK <ListMusic size={16} />
        </a>
      </div>
    </aside>
  );
});
