import React, { memo, useEffect, useState } from "react";
import {
  ArrowDownRight,
  ArrowUpLeft,
  ListMusic,
  Pause,
  Play,
  Repeat,
  Repeat1,
  Shuffle,
  SkipBack,
  SkipForward,
  Volume2,
  VolumeX,
} from "lucide-react";
import { SpatialTrack, formatTime } from "./data";
import { Artwork, LikeButton } from "./Primitives";

interface PlayerProps {
  track: SpatialTrack;
  playing: boolean;
  liked: boolean;
  onToggle: () => void;
  onLike: () => void;
  onNext: () => void;
  onPrevious: () => void;
  onQueue: () => void;
  onNowPlaying: () => void;
  canNext: boolean;
  canPrevious: boolean;
  shuffle: boolean;
  repeat: number;
  onShuffle: () => void;
  onRepeat: () => void;
}

// Preview-only, user-driven seeking stays in this leaf. No clocks, audio,
// global playback emitter writes, or high-frequency root state are introduced.
const Seek = memo(function Seek({ track }: { track: SpatialTrack }) {
  const [position, setPosition] = useState(82);
  useEffect(() => setPosition(track.id === "migration-0" ? 82 : 0), [track.id]);
  return (
    <div className="sp-seek">
      <span>{formatTime(position)}</span>
      <input
        type="range"
        aria-label="Seek sample track"
        min="0"
        max={track.duration}
        step="1"
        value={position}
        onChange={(event) => setPosition(Number(event.target.value))}
        style={
          {
            "--seek": `${(position / track.duration) * 100}%`,
          } as React.CSSProperties
        }
      />
      <span>{formatTime(track.duration)}</span>
    </div>
  );
});

const Volume = memo(function Volume() {
  const [volume, setVolume] = useState(0.7);
  const [remembered, setRemembered] = useState(0.7);
  return (
    <div className="sp-volume">
      <button
        className="sp-icon"
        aria-label={volume ? "Mute sample volume" : "Unmute sample volume"}
        onClick={() => {
          if (volume) {
            setRemembered(volume);
            setVolume(0);
          } else setVolume(remembered || 0.7);
        }}
      >
        {volume ? <Volume2 size={18} /> : <VolumeX size={18} />}
      </button>
      <input
        aria-label="Sample volume"
        type="range"
        min="0"
        max="1"
        step="0.01"
        value={volume}
        onChange={(event) => setVolume(Number(event.target.value))}
      />
    </div>
  );
});

export const Transport = memo(function Transport({
  track,
  playing,
  liked,
  onToggle,
  onLike,
  onNext,
  onPrevious,
  onQueue,
  onNowPlaying,
  canNext,
  canPrevious,
  shuffle,
  repeat,
  onShuffle,
  onRepeat,
}: PlayerProps) {
  return (
    <footer className="sp-transport" aria-label="Sample player controls">
      <div className="sp-transport-meta">
        <button
          className="sp-mini-art"
          onClick={onNowPlaying}
          aria-label="Open Now Playing"
        >
          <Artwork src={track.artwork} title={track.album} />
        </button>
        <button className="sp-transport-title" onClick={onNowPlaying}>
          <strong>{track.title}</strong>
          <span>{track.artist}</span>
        </button>
        <LikeButton title={track.title} liked={liked} onClick={onLike} />
      </div>
      <div className="sp-transport-center">
        <div className="sp-transport-buttons">
          <button
            className={`sp-icon sp-extra-control ${shuffle ? "sp-active-control" : ""}`}
            aria-label="Shuffle sample selection"
            aria-pressed={shuffle}
            onClick={onShuffle}
          >
            <Shuffle size={17} />
          </button>
          <button
            className="sp-icon"
            aria-label="Previous track"
            disabled={!canPrevious}
            onClick={onPrevious}
          >
            <SkipBack size={19} fill="currentColor" />
          </button>
          <button
            className="sp-play"
            aria-label={playing ? "Pause sample track" : "Play sample track"}
            onClick={onToggle}
          >
            {playing ? (
              <Pause size={22} fill="currentColor" />
            ) : (
              <Play size={22} fill="currentColor" />
            )}
          </button>
          <button
            className="sp-icon"
            aria-label="Next track"
            disabled={!canNext}
            onClick={onNext}
          >
            <SkipForward size={19} fill="currentColor" />
          </button>
          <button
            className={`sp-icon sp-extra-control ${repeat ? "sp-active-control" : ""}`}
            aria-label={`Repeat: ${["off", "all", "one"][repeat]}`}
            onClick={onRepeat}
          >
            {repeat === 2 ? <Repeat1 size={17} /> : <Repeat size={17} />}
          </button>
        </div>
        <Seek track={track} />
      </div>
      <div className="sp-transport-end">
        <Volume />
        <button className="sp-queue-button" onClick={onQueue}>
          <ListMusic size={19} />
          <span>Queue</span>
        </button>
      </div>
    </footer>
  );
});

export const NowPlaying = memo(function NowPlaying({
  queue,
  cursor,
  onSelect,
  track,
  playing,
  liked,
  onToggle,
  onLike,
}: Pick<PlayerProps, "track" | "playing" | "liked" | "onToggle" | "onLike"> & {
  queue: SpatialTrack[];
  cursor: number;
  onSelect: (id: string) => void;
}) {
  return (
    <div className="sp-page sp-now-playing">
      <div className="sp-now-heading">
        <p className="sp-kicker">Right here, right now</p>
        <h1>Now Playing</h1>
        <p>{track.album}</p>
      </div>
      <div
        className="sp-orbit"
        aria-label="Previous, current and upcoming music"
      >
        <span className="sp-past-label">
          <ArrowUpLeft size={17} />
          Previously played
        </span>
        <span className="sp-future-label">
          Up next
          <ArrowDownRight size={17} />
        </span>
        {queue.map((item, index) => {
          const offset = index - cursor;
          if (Math.abs(offset) > 2) return null;
          return (
            <button
              key={item.id}
              className={`sp-orbit-cover ${offset === 0 ? "sp-orbit-current" : ""}`}
              aria-current={offset === 0 ? "true" : undefined}
              aria-label={`${offset === 0 ? "Pause or play" : offset < 0 ? "Play previous track" : "Play next track"}: ${item.title} by ${item.artist}`}
              onClick={() => (offset === 0 ? onToggle() : onSelect(item.id))}
              style={
                {
                  "--offset": offset,
                  "--distance": Math.abs(offset),
                  "--card-rotation": `${offset * 8}deg`,
                  "--card-x": `${offset * 252}px`,
                  "--card-y": `${Math.abs(offset) * 106}px`,
                  "--card-scale": 1 - Math.abs(offset) * 0.24,
                  opacity: 1 - Math.abs(offset) * 0.29,
                  zIndex: 3 - Math.abs(offset),
                } as React.CSSProperties
              }
            >
              <Artwork
                src={item.artwork}
                title={`${item.album} by ${item.artist}`}
                eager={offset === 0}
              />
              <span>
                {item.title}
                <small>{item.artist}</small>
              </span>
            </button>
          );
        })}
      </div>
      <div className="sp-now-meta" aria-live="polite">
        <div>
          <h2>{track.title}</h2>
          <p>{track.artist}</p>
        </div>
        <LikeButton title={track.title} liked={liked} onClick={onLike} />
        <button
          className="sp-play"
          aria-label={
            playing ? "Pause current sample track" : "Play current sample track"
          }
          onClick={onToggle}
        >
          {playing ? (
            <Pause size={22} fill="currentColor" />
          ) : (
            <Play size={22} fill="currentColor" />
          )}
        </button>
      </div>
      <p className="sp-now-description">A little space for the music.</p>
    </div>
  );
});
