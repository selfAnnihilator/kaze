import { memo, type ReactNode } from "react";
import { AudioLines, MoreVertical } from "lucide-react";
import { Sleeve } from "./Sleeve";
import { formatDuration } from "./format";

export interface TrackRowProps {
  index?: number | string;
  title: string;
  subtitle?: string;
  durationSecs?: number;
  src?: string | null;
  trackId?: string;
  active?: boolean;
  /** Hide the index column (Figma rows on Discover, Liked and collections). */
  noIndex?: boolean;
  onPress: () => void;
  onMore?: () => void;
  trailing?: ReactNode;
}

/**
 * Brutalist Track Row (Figma 1:7801 & 1:7802):
 * 67px height, 24px index or audio-lines indicator, 44x44 sleeve,
 * Inter 16px bold title, Inter 12px meta, Courier 11px duration, 1px line divider.
 */
export const TrackRow = memo(function TrackRow({
  index, title, subtitle, durationSecs, src, trackId, active = false, noIndex = false, onPress, onMore, trailing,
}: TrackRowProps) {
  const formattedIndex = typeof index === "number" ? String(index).padStart(2, "0") : index;

  return (
    <div className={`m-track-row ${active ? "is-active" : ""}`}>
      <button type="button" className="m-track-row__main" onClick={onPress} aria-label={`Play ${title}`}>
        {!noIndex && (
          <div className="m-track-row__lead">
            {active ? (
              <AudioLines size={18} className="m-track-row__wave" strokeWidth={2} />
            ) : (
              <span className="m-track-row__index">{formattedIndex ?? "·"}</span>
            )}
          </div>
        )}
        <Sleeve size="mini" src={src} trackId={trackId} title={title} />
        <div className="m-track-row__text">
          <span className="m-track-row__title">{title}</span>
          {subtitle && <span className="m-track-row__meta">{subtitle}</span>}
        </div>
        {durationSecs ? (
          <span className="m-track-row__time">{formatDuration(durationSecs)}</span>
        ) : null}
      </button>
      {trailing && <div className="m-track-row__trailing">{trailing}</div>}
      {onMore && (
        <button type="button" className="m-track-row__more" onClick={onMore} aria-label="More options">
          <MoreVertical size={20} strokeWidth={2} />
        </button>
      )}
    </div>
  );
});
