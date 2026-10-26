import { memo, useEffect, useState, useSyncExternalStore } from "react";
import { Disc3 } from "lucide-react";
import { loadCoverArt, subscribeCoverArt, coverArtGeneration } from "../services/coverArt";

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
  const [resolved, setResolved] = useState<{ id: string; src: string } | null>(
    null,
  );
  const [failed, setFailed] = useState<string | null>(null);
  useEffect(() => {
    if (
      src ||
      !trackId ||
      trackId.startsWith("online:") ||
      trackId.startsWith("itunes:")
    )
      return;
    const controller = new AbortController();
    loadCoverArt(trackId, controller.signal).then((url) => {
      if (!controller.signal.aborted)
        setResolved(url ? { id: trackId, src: url } : null);
    });
    return () => {
      controller.abort();
    };
  }, [src, trackId, artGeneration]);
  const url =
    src || (resolved && resolved.id === trackId ? resolved.src : undefined);
  const hasImage = !!url && failed !== url;
  return (
    <div
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
