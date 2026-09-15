import React, { useState, useEffect, useRef, useCallback } from "react";
import { Disc, Calendar } from "lucide-react";
import { Album } from "../../types";
import { executeQuery } from "../../services/api";

// In-memory frontend cache for album covers to avoid re-extracting from disk on every view switch
const albumCoverCache = new Map<string, string>();

// Concurrency limiter to prevent IPC congestion and memory spikes
const MAX_CONCURRENT_COVER_REQUESTS = 4;
let activeRequests = 0;
const requestQueue: (() => void)[] = [];

function enqueueCoverRequest<T>(fn: () => Promise<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    const run = () => {
      activeRequests++;
      fn()
        .then(resolve)
        .catch(reject)
        .finally(() => {
          activeRequests--;
          if (requestQueue.length > 0) {
            const next = requestQueue.shift();
            next?.();
          }
        });
    };
    if (activeRequests < MAX_CONCURRENT_COVER_REQUESTS) {
      run();
    } else {
      requestQueue.push(run);
    }
  });
}

// Single shared IntersectionObserver for all album cards to eliminate per-card observer overhead
const observerCallbacks = new Map<Element, () => void>();
let sharedCoverObserver: IntersectionObserver | null = null;

function getSharedCoverObserver(): IntersectionObserver | null {
  if (typeof window === "undefined" || !("IntersectionObserver" in window)) return null;
  if (!sharedCoverObserver) {
    sharedCoverObserver = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            const cb = observerCallbacks.get(entry.target);
            if (cb) {
              cb();
              observerCallbacks.delete(entry.target);
              sharedCoverObserver?.unobserve(entry.target);
            }
          }
        }
      },
      { rootMargin: "250px 0px" }
    );
  }
  return sharedCoverObserver;
}

interface AlbumCardProps {
  album: Album;
  onSelectAlbum: (albumId: string) => void;
}

const AlbumCard: React.FC<AlbumCardProps> = React.memo(({ album, onSelectAlbum }) => {
  const cacheKey = album.id || album.title;
  const [coverUrl, setCoverUrl] = useState<string | null>(() => {
    if (album.cover_art_path) return album.cover_art_path;
    if (cacheKey && albumCoverCache.has(cacheKey)) {
      return albumCoverCache.get(cacheKey)!;
    }
    return null;
  });
  const [isVisible, setIsVisible] = useState(false);
  const cardRef = useRef<HTMLDivElement>(null);

  // Single shared intersection observer to detect when this card approaches the viewport
  useEffect(() => {
    if (coverUrl || isVisible || !cardRef.current) return;

    if (cacheKey && albumCoverCache.has(cacheKey)) {
      setCoverUrl(albumCoverCache.get(cacheKey)!);
      return;
    }

    const el = cardRef.current;
    const observer = getSharedCoverObserver();
    if (!observer) {
      setIsVisible(true);
      return;
    }

    observerCallbacks.set(el, () => {
      setIsVisible(true);
    });
    observer.observe(el);

    return () => {
      observerCallbacks.delete(el);
      observer.unobserve(el);
    };
  }, [cacheKey, coverUrl, isVisible]);

  // Request cover art only when card enters viewport and has a track ID with artwork
  useEffect(() => {
    if (!isVisible || coverUrl) return;

    if (cacheKey && albumCoverCache.has(cacheKey)) {
      setCoverUrl(albumCoverCache.get(cacheKey)!);
      return;
    }

    const targetTrackId = album.first_track_id;
    if (!targetTrackId) return;

    let isMounted = true;
    enqueueCoverRequest(() =>
      executeQuery({
        query: "GetTrackCoverArt",
        payload: { track_id: targetTrackId },
      })
    )
      .then((res: any) => {
        if (isMounted && res.data) {
          setCoverUrl(res.data);
          if (cacheKey) {
            albumCoverCache.set(cacheKey, res.data);
          }
        }
      })
      .catch(() => {});

    return () => {
      isMounted = false;
    };
  }, [isVisible, coverUrl, album.first_track_id, cacheKey]);

  const handleClick = useCallback(() => {
    onSelectAlbum(album.id);
  }, [album.id, onSelectAlbum]);

  return (
    <div
      ref={cardRef}
      className="album-card"
      onClick={handleClick}
    >
      <div
        style={{
          width: "100%",
          aspectRatio: "1/1",
          borderRadius: "8px",
          backgroundColor: "rgba(255, 255, 255, 0.04)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          marginBottom: "12px",
          overflow: "hidden",
          position: "relative",
          flexShrink: 0,
        }}
      >
        {/* Constant background gradient fallback: prevents layout shifts */}
        <div
          style={{
            position: "absolute",
            inset: 0,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            background: "linear-gradient(135deg, rgba(243, 112, 30, 0.15), rgba(232, 216, 201, 0.1))",
          }}
        >
          <Disc size={44} color="var(--accent-light)" />
        </div>

        {/* Cover image overlay */}
        {coverUrl && (
          <img
            src={coverUrl}
            alt={album.title}
            loading="lazy"
            decoding="async"
            style={{
              position: "absolute",
              inset: 0,
              width: "100%",
              height: "100%",
              objectFit: "cover",
              display: "block",
            }}
          />
        )}
      </div>

      <div
        style={{
          fontWeight: 700,
          fontSize: "0.95rem",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
        }}
        title={album.title}
      >
        {album.title}
      </div>
      <div
        style={{
          fontSize: "0.82rem",
          color: "var(--text-muted)",
          marginTop: "2px",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
        }}
        title={album.artist_name || "Unknown Artist"}
      >
        {album.artist_name || "Unknown Artist"}
      </div>
      <div
        style={{
          fontSize: "0.75rem",
          color: "var(--text-dim)",
          marginTop: "6px",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        {album.track_count !== undefined ? (
          <span>
            {album.track_count} {album.track_count === 1 ? "track" : "tracks"}
          </span>
        ) : (
          <span />
        )}
        {album.year && (
          <div style={{ display: "flex", alignItems: "center", gap: "4px" }}>
            <Calendar size={12} />
            <span>{album.year}</span>
          </div>
        )}
      </div>
    </div>
  );
});

interface AlbumsViewProps {
  albums: Album[];
  onSelectAlbum: (albumId: string) => void;
}

const INITIAL_BATCH = 48;
const BATCH_STEP = 36;

export const AlbumsView: React.FC<AlbumsViewProps> = React.memo(({ albums, onSelectAlbum }) => {
  const [visibleCount, setVisibleCount] = useState(INITIAL_BATCH);
  const loadMoreRef = useRef<HTMLDivElement>(null);
  const isLoadingMoreRef = useRef(false);

  // Progressive infinite scroll sentinel with debounce to prevent cascading loop
  useEffect(() => {
    if (visibleCount >= albums.length || !loadMoreRef.current) return;

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && !isLoadingMoreRef.current) {
          isLoadingMoreRef.current = true;
          setVisibleCount((prev) => Math.min(prev + BATCH_STEP, albums.length));
          setTimeout(() => {
            isLoadingMoreRef.current = false;
          }, 180);
        }
      },
      { rootMargin: "150px" }
    );

    observer.observe(loadMoreRef.current);
    return () => observer.disconnect();
  }, [visibleCount, albums.length]);

  const visibleAlbums = albums.slice(0, visibleCount);

  return (
    <div>
      <div className="view-header">
        <div>
          <h1 className="view-title">Albums</h1>
          <p style={{ color: "var(--text-dim)", fontSize: "0.88rem", marginTop: "4px" }}>
            {albums.length} album{albums.length === 1 ? "" : "s"} in library
          </p>
        </div>
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))",
          gap: "24px",
        }}
      >
        {visibleAlbums.map((album) => (
          <AlbumCard
            key={album.id}
            album={album}
            onSelectAlbum={onSelectAlbum}
          />
        ))}
      </div>

      {/* Sentinel for progressive infinite loading */}
      {visibleCount < albums.length && (
        <div
          ref={loadMoreRef}
          style={{
            height: "40px",
            margin: "20px 0",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
          }}
        />
      )}
    </div>
  );
});
