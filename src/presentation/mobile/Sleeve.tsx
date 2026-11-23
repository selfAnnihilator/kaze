import { memo } from "react";
import { Artwork } from "../Artwork";

export type SleeveSize =
  | "mini" | "side" | "artist" | "mix" | "mixcard" | "toprow" | "albumcard" | "review" | "row" | "edge" | "tile" | "preview" | "moment" | "collection" | "playlist" | "card" | "hero" | "album" | "deck" | "large";

/** Covers the backend can find by id (online placeholders have none). */
const lookupable = (id?: string): id is string => !!id && !id.startsWith("online:") && !id.startsWith("itunes:");

/**
 * Brutalist vinyl cardboard sleeve (Figma 1:7804):
 * Sand cardboard (#c6c0b4), groove circle (#5b5852), dark vinyl core (#1a1b1c) & coral spindle (#fb615a).
 */
export const Sleeve = memo(function Sleeve({
  title, subtitle, src, trackId, size = "tile", onPress,
}: {
  title?: string; subtitle?: string; src?: string | null; trackId?: string;
  size?: SleeveSize; onPress?: () => void;
}) {
  const body = (
    <>
      <div className={`m-sleeve__disc m-sleeve__disc--${size}`}>
        {src ? (
          <Artwork src={src} trackId={trackId} alt={title ?? ""} className="m-sleeve__artwork" />
        ) : (
          <>
            <div className="m-sleeve__cardboard">
              <div className="m-sleeve__groove">
                <div className="m-sleeve__vinyl-core">
                  <div className="m-sleeve__spindle" />
                </div>
              </div>
            </div>
            {/* No image given: ask the backend for this song's cover, like the PC does. */}
            {lookupable(trackId) && <Artwork trackId={trackId} alt={title ?? ""} className="m-sleeve__artwork is-lookup" />}
          </>
        )}
      </div>
      {size === "tile" && (title || subtitle) && (
        <span className="m-sleeve__caption">
          {title && <span className="m-sleeve__title">{title}</span>}
          {subtitle && <span className="m-sleeve__meta">{subtitle}</span>}
        </span>
      )}
    </>
  );

  if (!onPress) {
    return <div className={`m-sleeve m-sleeve--${size}`}>{body}</div>;
  }
  return (
    <button type="button" className={`m-sleeve m-sleeve--${size} is-clickable`} onClick={onPress} aria-label={title ?? "Play album"}>
      {body}
    </button>
  );
});
