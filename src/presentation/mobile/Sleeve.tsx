import { memo } from "react";
import { Artwork } from "../Artwork";

/** Album sleeve: square cover with a mono caption. */
export const Sleeve = memo(function Sleeve({
  title, subtitle, src, trackId, size = "tile", onPress,
}: {
  title: string; subtitle?: string; src?: string | null; trackId?: string;
  size?: "tile" | "large"; onPress?: () => void;
}) {
  const body = (
    <>
      <Artwork src={src} trackId={trackId} alt="" className="m-sleeve__art" />
      {size === "tile" && (
        <span className="m-sleeve__caption">
          <span className="m-sleeve__title">{title}</span>
          {subtitle && <span className="m-sleeve__meta">{subtitle}</span>}
        </span>
      )}
    </>
  );
  if (!onPress) return <div className={`m-sleeve m-sleeve--${size}`}>{body}</div>;
  return (
    <button type="button" className={`m-sleeve m-sleeve--${size}`} onClick={onPress} aria-label={title}>
      {body}
    </button>
  );
});
