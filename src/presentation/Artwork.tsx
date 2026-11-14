import { memo, useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore } from "react";
import { Disc3 } from "lucide-react";
import { loadCoverArt, subscribeCoverArt, coverArtGeneration, sizedCoverUrl } from "../services/coverArt";

type Resolved = { id: string; src: string; generation: number } | null;

/** The image to show: the given source, or this track's current resolved cover, sized to the frame. */
function artworkUrl({ src, trackId, resolved, generation, width }: {
  src?: string | null; trackId?: string; resolved: Resolved; generation: number; width: number;
}) {
  const current = resolved?.id === trackId && resolved?.generation === generation ? resolved?.src : undefined;
  const loaded = src || current;
  return loaded ? sizedCoverUrl(loaded, width || 256) : undefined;
}

export const Artwork = memo(function Artwork({
  src,
  trackId,
  alt = "",
  className = "",
}: {
  src?: string | null;
  trackId?: string;
  alt?: string;
  className?: string;
}) {
  const artGeneration = useSyncExternalStore(subscribeCoverArt, coverArtGeneration, coverArtGeneration);
  const [resolved, setResolved] = useState<Resolved>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const frame = useRef<HTMLDivElement>(null);
  // The frame's size is fixed by CSS; local covers are resized to it so a
  // 48px row never decodes a full-size image. A ref, not state: covers
  // arrive asynchronously, after the frame has been measured, so this costs
  // no extra render.
  const width = useRef(0);
  useLayoutEffect(() => {
    width.current = frame.current?.clientWidth || 0;
  }, []);
  useEffect(() => {
    if (
      src ||
      !trackId
    )
      return;
    const controller = new AbortController();
    loadCoverArt(trackId, controller.signal).then((url) => {
      if (!controller.signal.aborted)
        setResolved(url ? { id: trackId, src: url, generation: artGeneration } : null);
    });
    return () => {
      controller.abort();
    };
  }, [src, trackId, artGeneration]);
  const url = artworkUrl({ src, trackId, resolved, generation: artGeneration, width: width.current });
  const hasImage = !!url && failed !== url;
  return (
    <div
      ref={frame}
      className={`music-artwork${hasImage ? "" : " music-artwork--placeholder"} ${className}`}
    >
      {hasImage ? (
        <img
          src={url}
          alt={alt}
          loading="lazy"
          decoding="async"
          onError={() => setFailed(url)}
        />
      ) : (
        <Disc3 aria-label={alt || "Artwork unavailable"} size={40} />
      )}
    </div>
  );
});
