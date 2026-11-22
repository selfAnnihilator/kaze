import { memo, type ReactNode } from "react";
import { Artwork } from "../Artwork";
import { formatDuration } from "./format";

export interface TrackRowProps {
  title: string;
  subtitle?: string;
  durationSecs?: number;
  src?: string | null;
  trackId?: string;
  active?: boolean;
  onPress: () => void;
  trailing?: ReactNode;
}

/** One track: cover, title and meta, duration, optional trailing action. */
export const TrackRow = memo(function TrackRow({
  title, subtitle, durationSecs, src, trackId, active = false, onPress, trailing,
}: TrackRowProps) {
  return (
    <div className={`m-track-row ${active ? "is-active" : ""}`}>
      <button type="button" className="m-track-row__main" onClick={onPress} aria-label={`Play ${title}`}>
        <Artwork src={src} trackId={trackId} alt="" className="m-track-row__art" />
        <span className="m-track-row__text">
          <span className="m-track-row__title">{title}</span>
          {subtitle && <span className="m-track-row__meta">{subtitle}</span>}
        </span>
        {durationSecs ? <span className="m-track-row__time">{formatDuration(durationSecs)}</span> : null}
      </button>
      {trailing}
    </div>
  );
});
