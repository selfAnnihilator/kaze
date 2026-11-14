import { memo, useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore } from "react";
import { Disc3 } from "lucide-react";
import { loadCoverArt, subscribeCoverArt, coverArtGeneration, sizedCoverUrl } from "../services/coverArt";

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
  const [resolved, setResolved] = useState<{ id: string; src: string; generation: number } | null>(
    null,
  );
  const [failed, setFailed] = useState<string | null>(null);
  const frame = useRef<HTMLDivElement>(null);
  // Measured once: the frame's size is fixed by CSS, and local covers are
  // resized to it so a 48px row never decodes a full-size image.
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    setWidth(frame.current?.clientWidth || 256);
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
  const loaded =
    src || (resolved && resolved.id === trackId && resolved.generation === artGeneration ? resolved.src : undefined);
  const url = loaded && width ? sizedCoverUrl(loaded, width) : undefined;
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
