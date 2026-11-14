import { Check, Info, ListPlus, Pause, Play, RefreshCw } from "lucide-react";
import type { FlowExplanation } from "../types";
import { NotInterestedButton } from "../components/NotInterestedButton";
import { Artwork } from "./Artwork";

export type MusicItem = {
  id: string;
  title: string;
  artist: string;
  album?: string;
  cover?: string;
  explanation?: FlowExplanation | null;
};

export interface SpatialMusicItemProps {
  item: MusicItem;
  /** The accessible play button label ("Play …", "Pause …" or "Loading …"). */
  label: string;
  playing: boolean;
  loading: boolean;
  queued: boolean;
  onPlay: () => void;
  onExplain: (item: MusicItem) => void;
  onEnqueueTrack?: (id: string) => void;
}

/** Why, not-interested, queue and play buttons for one Home track. */
export function SpatialMusicActions({ item, label, playing, loading, queued, onPlay, onExplain, onEnqueueTrack }: SpatialMusicItemProps) {
  return (
    <div className="spatial-music-actions">
      {item.explanation && (
        <button
          className="player-icon-btn"
          title="Why this track?"
          aria-label="Why this track?"
          onClick={() => onExplain(item)}
        >
          <Info size={15} />
        </button>
      )}
      <NotInterestedButton trackId={item.id} title={item.title} />
      {onEnqueueTrack && (
        <button
          className="player-icon-btn"
          title={queued ? "Already in queue" : "Add to queue"}
          onClick={() => onEnqueueTrack(item.id)}
        >
          {queued ? <Check size={15} /> : <ListPlus size={15} />}
        </button>
      )}
      <button className="player-icon-btn" aria-label={label} disabled={loading} onClick={onPlay}>
        {loading ? (
          <RefreshCw size={15} className="spin-animation" />
        ) : playing ? (
          <Pause size={15} />
        ) : (
          <Play size={15} />
        )}
      </button>
    </div>
  );
}

/** A Home list row: artwork, title and artist, then the track's actions. */
export function SpatialMusicRow(props: SpatialMusicItemProps) {
  const { item, onPlay } = props;
  return (
    <div className="spatial-music-row">
      <Artwork src={item.cover} trackId={item.id} />
      <button className="spatial-music-title" onClick={onPlay}>
        <strong>{item.title}</strong>
        <span>{item.artist}</span>
      </button>
      <SpatialMusicActions {...props} />
    </div>
  );
}
