import { useInterfaceStyle } from "../../presentation/InterfaceContext";
import { BrutalistAlbums } from "../../presentation/BrutalistAlbums";
import { WindowedGallery } from "../../presentation/WindowedGallery";
import React, { useState, useEffect, useRef, useCallback, useSyncExternalStore } from "react";
import { Disc, Calendar } from "lucide-react";
import { Album } from "../../types";
import { loadCoverArt, peekCoverArt, subscribeCoverArt, coverArtGeneration } from "../../services/coverArt";

// Single shared IntersectionObserver for all album cards to eliminate per-card observer overhead
const observerCallbacks = new Map<Element, () => void>();
let sharedCoverObserver: IntersectionObserver | null = null;

function getSharedCoverObserver(): IntersectionObserver | null {
  if (typeof window === "undefined" || !("IntersectionObserver" in window))
    return null;
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
      { rootMargin: "250px 0px" },
    );
  }
  return sharedCoverObserver;
}

interface AlbumCardProps {
  album: Album;
  onSelectAlbum: (albumId: string) => void;
}

const AlbumCard: React.FC<AlbumCardProps> = React.memo(
  ({ album, onSelectAlbum }) => {
    const cacheKey = album.first_track_id || album.id;
    const generation = useSyncExternalStore(subscribeCoverArt, coverArtGeneration, coverArtGeneration);
    const [coverUrl, setCoverUrl] = useState<string | null>(() => {
      if (album.cover_art_path) return album.cover_art_path;
      if (cacheKey) return peekCoverArt(cacheKey) || null;
      return null;
    });
    const [isVisible, setIsVisible] = useState(false);
    const cardRef = useRef<HTMLDivElement>(null);

    // Single shared intersection observer to detect when this card approaches the viewport
    useEffect(() => {
      if (coverUrl || isVisible || !cardRef.current) return;

      if (cacheKey && peekCoverArt(cacheKey)) {
        setCoverUrl(peekCoverArt(cacheKey)!);
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

    useEffect(() => { setCoverUrl(null); }, [generation]);

    // Request cover art only when card enters viewport and has a track ID with artwork
    useEffect(() => {
      if (!isVisible || coverUrl) return;

      if (cacheKey && peekCoverArt(cacheKey)) {
        setCoverUrl(peekCoverArt(cacheKey)!);
        return;
      }

      const targetTrackId = album.first_track_id;
      if (!targetTrackId) return;

      const controller = new AbortController();
      loadCoverArt(targetTrackId, controller.signal).then((url) => {
        if (!controller.signal.aborted && url) setCoverUrl(url);
      });
      return () => controller.abort();
    }, [isVisible, coverUrl, album.first_track_id, cacheKey]);

    const handleClick = useCallback(() => {
      onSelectAlbum(album.id);
    }, [album.id, onSelectAlbum]);

    return (
      <div
        ref={cardRef}
        className="album-card"
        role="button"
        tabIndex={0}
        aria-label={`Open album ${album.title}`}
        onKeyDown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            handleClick();
          }
        }}
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
              background:
                "linear-gradient(135deg, rgba(243, 112, 30, 0.15), rgba(232, 216, 201, 0.1))",
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
  },
);

interface AlbumsViewProps {
  albums: Album[];
  onSelectAlbum: (albumId: string) => void;
}

export const AlbumsView: React.FC<AlbumsViewProps> = React.memo(
  ({ albums, onSelectAlbum }) => {
    const style = useInterfaceStyle();
    if (style === "brutalist") return <BrutalistAlbums albums={albums} onSelectAlbum={onSelectAlbum} />;
    return (
      <div className="albums-view">
        <div className="view-header">
          <div>
            <h1 className="view-title">Albums</h1>
            <p style={{ color: "var(--text-muted)", marginTop: 8 }}>
              {albums.length} albums in your library
            </p>
          </div>
        </div>
        {!albums.length ? (
          <p className="interface-empty">
            Your record shelf is waiting. Add a music folder in Settings and
            rescan your library.
          </p>
        ) : (
          <WindowedGallery
            items={albums}
            label="Album shelves"
            layout={style === "spatial" ? "shelf" : undefined}
            renderItem={(album) => (
              <AlbumCard
                key={album.id}
                album={album}
                onSelectAlbum={onSelectAlbum}
              />
            )}
          />
        )}
      </div>
    );
  },
);
