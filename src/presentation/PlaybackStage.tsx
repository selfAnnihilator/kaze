import { memo } from "react";
import { SkipBack, SkipForward } from "lucide-react";
import type { Track } from "../types";
import { Artwork } from "./Artwork";

/** Composition only. Every transport action is supplied by the real App. */
export const PlaybackStage = memo(function PlaybackStage({
  track,
  cover,
  previous,
  next,
  playing,
  loading,
  index,
  onPrevious,
  onNext,
}: {
  track?: Track;
  cover?: string | null;
  previous?: Track;
  next?: Pick<Track, "title" | "artist_name" | "id" | "cover_art_url">;
  playing: boolean;
  loading?: boolean;
  index?: number | null;
  onPrevious: () => void;
  onNext: () => void;
}) {
  return (
    <section className="playback-stage" aria-label="Now playing">
      <div className="stage-heading">
        <p className="interface-eyebrow">
          {loading
            ? "Loading / buffering"
            : playing
              ? "Now playing"
              : track
                ? "Paused"
                : "Your listening space"}
        </p>
        <span className="stage-index">
          {index == null ? "♪" : String(index + 1).padStart(2, "0")}
        </span>
      </div>
      <div className="stage-sequence">
        {[
          {
            slot: "past",
            track: previous,
            action: onPrevious,
            label: "Previous",
          },
          { slot: "current", track, action: undefined, label: "" },
          { slot: "future", track: next, action: onNext, label: "Next" },
        ].map((sleeve, position, all) => {
          const identity = sleeve.track?.id || `empty-${sleeve.slot}`;
          const duplicate = all.some(
            (other, index) =>
              index < position && other.track?.id === sleeve.track?.id,
          );
          return (
            <div
              key={duplicate ? `${identity}-${sleeve.slot}` : identity}
              className={`stage-sleeve stage-${sleeve.slot}`}
              role={sleeve.action ? "button" : undefined}
              tabIndex={sleeve.action ? 0 : undefined}
              title={sleeve.action ? `${sleeve.label} Track` : undefined}
              onClick={sleeve.action}
              onKeyDown={(event) => {
                if (
                  sleeve.action &&
                  (event.key === "Enter" || event.key === " ")
                ) {
                  event.preventDefault();
                  event.stopPropagation();
                  sleeve.action();
                }
              }}
            >
              <Artwork
                src={
                  sleeve.slot === "current"
                    ? cover || sleeve.track?.cover_art_url
                    : sleeve.track?.cover_art_url
                }
                trackId={sleeve.track?.id}
                alt={
                  sleeve.slot === "current"
                    ? track?.album_title || track?.title || ""
                    : ""
                }
              />
              {sleeve.action && (
                <>
                  <span>
                    {sleeve.slot === "past" && <SkipBack size={18} />}
                    {sleeve.label}
                    {sleeve.slot === "future" && <SkipForward size={18} />}
                  </span>
                  {sleeve.track && <small>{sleeve.track.title}</small>}
                </>
              )}
            </div>
          );
        })}
      </div>
      <div className="stage-caption">
        <h1 title={track?.title}>{track?.title || "Make room for music."}</h1>
        <p>{track?.artist_name || "Choose a track, or start your Flow."}</p>
        {track?.album_title && <small>{track.album_title}</small>}
      </div>
    </section>
  );
});
