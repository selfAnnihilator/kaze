import React, { useState, useEffect } from "react";
import {
  DownloadCloud,
  X,
  RefreshCw,
  Check,
  Bookmark,
  Radio,
  FileAudio,
} from "lucide-react";
import { DownloadSearchResult } from "../../types";

export interface DownloadModalTrack {
  title: string;
  artist: string;
  album?: string;
  externalTrackId?: string;
}

interface DownloadOptionsModalProps {
  isOpen: boolean;
  track: DownloadModalTrack | null;
  onClose: () => void;
  onSearchSoulseek: (
    artist: string,
    title: string,
    album?: string
  ) => Promise<DownloadSearchResult[]>;
  onStartDownload: (searchResultId: string, track: DownloadModalTrack) => Promise<void>;
  onDirectAudioDownload?: (track: DownloadModalTrack) => Promise<void>;
  onAddToWishlist?: (title: string, artist: string, album?: string) => void;
}

export const DownloadOptionsModal: React.FC<DownloadOptionsModalProps> = ({
  isOpen,
  track,
  onClose,
  onSearchSoulseek,
  onStartDownload,
  onDirectAudioDownload,
  onAddToWishlist,
}) => {
  const [isSearching, setIsSearching] = useState(false);
  const [results, setResults] = useState<DownloadSearchResult[]>([]);
  const [hasSearched, setHasSearched] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [isStartingDownload, setIsStartingDownload] = useState(false);
  const [wishlistAdded, setWishlistAdded] = useState(false);

  useEffect(() => {
    if (isOpen && track) {
      setResults([]);
      setHasSearched(false);
      setSelectedId(null);
      setIsStartingDownload(false);
      setWishlistAdded(false);
      performSearch(track);
    }
  }, [isOpen, track]);

  const performSearch = async (tr: DownloadModalTrack) => {
    setIsSearching(true);
    try {
      const res = await onSearchSoulseek(tr.artist, tr.title, tr.album);
      setResults(res || []);
    } catch (err) {
      console.error("Soulseek search failed:", err);
      setResults([]);
    } finally {
      setIsSearching(false);
      setHasSearched(true);
    }
  };

  if (!isOpen || !track) return null;

  const formatBytes = (bytes: number): string => {
    if (!bytes || bytes <= 0) return "Unknown size";
    const mb = bytes / (1024 * 1024);
    if (mb >= 1024) {
      return `${(mb / 1024).toFixed(2)} GB`;
    }
    return `${mb.toFixed(1)} MB`;
  };

  const handleSelectSoulseekOption = async (result: DownloadSearchResult) => {
    setSelectedId(result.id);
    setIsStartingDownload(true);
    try {
      await onStartDownload(result.id, track);
      onClose();
    } catch (err) {
      console.error("Failed to start download:", err);
      setIsStartingDownload(false);
    }
  };

  const handleSelectDirectOption = async () => {
    if (!onDirectAudioDownload) return;
    setIsStartingDownload(true);
    try {
      await onDirectAudioDownload(track);
      onClose();
    } catch (err) {
      console.error("Failed to start direct download:", err);
      setIsStartingDownload(false);
    }
  };

  const handleWishlistClick = () => {
    if (onAddToWishlist && track) {
      onAddToWishlist(track.title, track.artist, track.album);
      setWishlistAdded(true);
    }
  };

  return (
    <div
      className="modal-overlay"
      style={{
        position: "fixed",
        inset: 0,
        backgroundColor: "rgba(0, 0, 0, 0.75)",
        backdropFilter: "blur(8px)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 9000,
        padding: "20px",
      }}
      onClick={onClose}
    >
      <div
        className="modal-content"
        style={{
          backgroundColor: "var(--bg-card)",
          border: "1px solid var(--border)",
          borderRadius: "16px",
          padding: "24px",
          width: "100%",
          maxWidth: "580px",
          maxHeight: "85vh",
          display: "flex",
          flexDirection: "column",
          gap: "18px",
          boxShadow: "0 20px 50px rgba(0, 0, 0, 0.7)",
          animation: "modalFadeIn 0.2s ease-out",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div
          style={{
            display: "flex",
            alignItems: "flex-start",
            justifyContent: "space-between",
            borderBottom: "1px solid var(--border)",
            paddingBottom: "16px",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
            <div
              style={{
                width: "44px",
                height: "44px",
                borderRadius: "10px",
                backgroundColor: "rgba(243, 112, 30, 0.15)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                flexShrink: 0,
              }}
            >
              <DownloadCloud size={24} color="var(--accent-light)" />
            </div>
            <div>
              <h2
                style={{
                  fontSize: "1.15rem",
                  fontWeight: 800,
                  color: "#e8d8c9",
                  lineHeight: 1.2,
                  marginBottom: "3px",
                }}
              >
                {track.title}
              </h2>
              <p style={{ fontSize: "0.82rem", color: "var(--text-muted)" }}>
                by {track.artist} {track.album ? `• ${track.album}` : ""}
              </p>
            </div>
          </div>

          <button
            type="button"
            onClick={onClose}
            style={{
              background: "none",
              border: "none",
              color: "var(--text-dim)",
              cursor: "pointer",
              padding: "4px",
              display: "flex",
              alignItems: "center",
              borderRadius: "6px",
            }}
            title="Close"
          >
            <X size={18} />
          </button>
        </div>

        {/* Content Body */}
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "14px",
            overflowY: "auto",
            maxHeight: "50vh",
            paddingRight: "4px",
          }}
        >
          {isSearching ? (
            <div style={{ textAlign: "center", padding: "40px 10px", color: "var(--text-dim)" }}>
              <RefreshCw
                size={36}
                className="animate-spin"
                color="var(--accent-light)"
                style={{ margin: "0 auto 14px" }}
              />
              <div style={{ fontWeight: 700, fontSize: "0.95rem", color: "var(--text-main)", marginBottom: "4px" }}>
                Searching P2P Soulseek Network...
              </div>
              <p style={{ fontSize: "0.82rem", color: "var(--text-muted)", maxWidth: "360px", margin: "0 auto" }}>
                Querying peers for optimal bitrates, audio formats, and open transfer slots
              </p>
            </div>
          ) : results.length > 0 ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
              <div
                style={{
                  fontSize: "0.78rem",
                  fontWeight: 700,
                  letterSpacing: "0.8px",
                  textTransform: "uppercase",
                  color: "var(--text-dim)",
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                }}
              >
                <span>Available Soulseek Files ({results.length})</span>
                <button
                  type="button"
                  onClick={() => performSearch(track)}
                  style={{
                    background: "none",
                    border: "none",
                    color: "var(--accent-light)",
                    fontSize: "0.75rem",
                    cursor: "pointer",
                    display: "inline-flex",
                    alignItems: "center",
                    gap: "4px",
                  }}
                >
                  <RefreshCw size={11} />
                  <span>Re-scan</span>
                </button>
              </div>

              {results.map((res) => {
                const isSelected = selectedId === res.id && isStartingDownload;
                return (
                  <div
                    key={res.id}
                    style={{
                      backgroundColor: "var(--bg-main)",
                      border: "1px solid var(--border)",
                      borderRadius: "10px",
                      padding: "12px 14px",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      gap: "12px",
                      transition: "all 0.15s ease",
                    }}
                  >
                    <div style={{ minWidth: 0, flex: 1 }}>
                      <div
                        style={{
                          fontSize: "0.85rem",
                          fontWeight: 600,
                          color: "#e8d8c9",
                          whiteSpace: "nowrap",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          marginBottom: "6px",
                        }}
                        title={res.filename}
                      >
                        {res.filename}
                      </div>

                      <div style={{ display: "flex", alignItems: "center", gap: "8px", flexWrap: "wrap" }}>
                        <span
                          className="badge"
                          style={{
                            backgroundColor: "rgba(243, 112, 30, 0.15)",
                            color: "var(--accent-light)",
                            fontSize: "0.7rem",
                            padding: "2px 6px",
                          }}
                        >
                          {res.format.toUpperCase()}
                        </span>

                        {res.bitrate && (
                          <span style={{ fontSize: "0.74rem", color: "var(--text-muted)" }}>
                            {res.bitrate} kbps
                          </span>
                        )}

                        <span style={{ fontSize: "0.74rem", color: "var(--text-dim)" }}>•</span>

                        <span style={{ fontSize: "0.74rem", color: "var(--text-muted)" }}>
                          {formatBytes(res.file_size)}
                        </span>

                        <span style={{ fontSize: "0.74rem", color: "var(--text-dim)" }}>•</span>

                        <span
                          style={{
                            fontSize: "0.72rem",
                            color: res.slots_free ? "var(--accent-secondary)" : "#f3701e",
                            fontWeight: 500,
                          }}
                        >
                          {res.slots_free ? "Slot Free" : "Queued Slot"}
                        </span>
                      </div>
                    </div>

                    <button
                      type="button"
                      className="btn btn-primary"
                      disabled={isStartingDownload}
                      onClick={() => handleSelectSoulseekOption(res)}
                      style={{
                        padding: "7px 14px",
                        fontSize: "0.82rem",
                        display: "inline-flex",
                        alignItems: "center",
                        gap: "6px",
                        flexShrink: 0,
                      }}
                    >
                      {isSelected ? (
                        <>
                          <RefreshCw size={13} className="animate-spin" />
                          <span>Starting...</span>
                        </>
                      ) : (
                        <>
                          <DownloadCloud size={14} />
                          <span>Download</span>
                        </>
                      )}
                    </button>
                  </div>
                );
              })}
            </div>
          ) : hasSearched ? (
            <div
              style={{
                backgroundColor: "rgba(255, 255, 255, 0.02)",
                border: "1px dashed var(--border)",
                borderRadius: "10px",
                padding: "24px 16px",
                textAlign: "center",
              }}
            >
              <FileAudio size={32} color="var(--text-dim)" style={{ margin: "0 auto 10px" }} />
              <div style={{ fontWeight: 600, fontSize: "0.92rem", color: "var(--text-main)", marginBottom: "4px" }}>
                No Soulseek peer results found right now
              </div>
              <p style={{ fontSize: "0.8rem", color: "var(--text-muted)", maxWidth: "380px", margin: "0 auto 16px" }}>
                Peers might be offline or using different naming. You can download the high-quality web audio stream directly or add this song to your wishlist.
              </p>
            </div>
          ) : null}

          {/* Fallback Direct Stream Option */}
          {onDirectAudioDownload && (
            <div
              style={{
                backgroundColor: "rgba(139, 124, 246, 0.06)",
                border: "1px solid rgba(139, 124, 246, 0.2)",
                borderRadius: "10px",
                padding: "14px",
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: "12px",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                <div
                  style={{
                    width: "36px",
                    height: "36px",
                    borderRadius: "8px",
                    backgroundColor: "rgba(139, 124, 246, 0.15)",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    flexShrink: 0,
                  }}
                >
                  <Radio size={18} color="var(--accent-secondary)" />
                </div>
                <div>
                  <div style={{ fontSize: "0.88rem", fontWeight: 700, color: "#e8d8c9" }}>
                    Direct Audio Stream Download
                  </div>
                  <div style={{ fontSize: "0.76rem", color: "var(--text-muted)" }}>
                    High-quality web audio capture directly to library (instant start)
                  </div>
                </div>
              </div>

              <button
                type="button"
                className="btn btn-secondary"
                disabled={isStartingDownload}
                onClick={handleSelectDirectOption}
                style={{
                  padding: "7px 14px",
                  fontSize: "0.82rem",
                  borderColor: "rgba(139, 124, 246, 0.4)",
                  color: "var(--accent-secondary)",
                  display: "inline-flex",
                  alignItems: "center",
                  gap: "6px",
                  flexShrink: 0,
                }}
              >
                <DownloadCloud size={14} />
                <span>Download Direct</span>
              </button>
            </div>
          )}
        </div>

        {/* Footer Actions */}
        <div
          style={{
            borderTop: "1px solid var(--border)",
            paddingTop: "14px",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          {onAddToWishlist && (
            <button
              type="button"
              className="btn btn-secondary"
              onClick={handleWishlistClick}
              disabled={wishlistAdded}
              style={{
                fontSize: "0.8rem",
                display: "inline-flex",
                alignItems: "center",
                gap: "6px",
                color: wishlistAdded ? "var(--accent-secondary)" : "var(--text-muted)",
              }}
            >
              {wishlistAdded ? <Check size={14} /> : <Bookmark size={14} />}
              <span>{wishlistAdded ? "Added to Wishlist" : "Save to Wishlist"}</span>
            </button>
          )}

          <button
            type="button"
            className="btn btn-secondary"
            onClick={onClose}
            style={{ fontSize: "0.82rem", marginLeft: "auto" }}
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
};
