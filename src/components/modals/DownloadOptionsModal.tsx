import React, { useState, useEffect } from "react";
import {
  DownloadCloud,
  X,
  RefreshCw,
  Zap,
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
}

export const rankResultMatch = (
  result: DownloadSearchResult,
  targetArtist: string,
  targetTitle: string
): number => {
  const norm = (s: string) =>
    s
      .toLowerCase()
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .replace(/[^a-z0-9\s]/g, " ")
      .replace(/\s+/g, " ")
      .trim();

  const cleanFilename = norm(result.filename);
  const cleanArtist = norm(targetArtist);
  const cleanTitle = norm(targetTitle);

  const titleWords = cleanTitle.split(" ").filter((w) => w.length > 1);
  const artistWords = cleanArtist.split(" ").filter((w) => w.length > 1);
  const filenameWords = cleanFilename.split(" ").filter((w) => w.length > 1);

  let score = 0;

  // Full title match
  if (cleanFilename.includes(cleanTitle)) {
    score += 60;
  } else {
    // Word-by-word title match
    if (titleWords.length > 0) {
      const matches = titleWords.filter((w) => cleanFilename.includes(w)).length;
      score += (matches / titleWords.length) * 40;
    }
  }

  // Artist match
  if (
    cleanFilename.includes(cleanArtist) ||
    (result.username && norm(result.username).includes(cleanArtist))
  ) {
    score += 30;
  }

  // Heavy penalty for extraneous noise words in filename (e.g. "pingu", "goes", "to", "theme park", "vlog")
  const commonMusicNoise = new Set(["lofi", "remix", "ost", "edit", "audio", "flac", "mp3", "track", "official", "theme", "original"]);
  const extraWords = filenameWords.filter(
    (w) =>
      !titleWords.includes(w) &&
      !artistWords.includes(w) &&
      !commonMusicNoise.has(w) &&
      !/^\d+$/.test(w)
  );
  if (extraWords.length > 2) {
    score -= (extraWords.length - 2) * 18;
  }

  // Prefer lossless or 320k high bitrate
  if (result.format.toLowerCase() === "flac") {
    score += 10;
  } else if (result.bitrate && result.bitrate >= 320) {
    score += 6;
  }

  // Prefer slot free
  if (result.slots_free) {
    score += 5;
  }

  return score;
};

export const DownloadOptionsModal: React.FC<DownloadOptionsModalProps> = ({
  isOpen,
  track,
  onClose,
  onSearchSoulseek,
  onStartDownload,
  onDirectAudioDownload,
}) => {
  const [isSearching, setIsSearching] = useState(false);
  const [results, setResults] = useState<DownloadSearchResult[]>([]);
  const [hasSearched, setHasSearched] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [isStartingDownload, setIsStartingDownload] = useState(false);
  const [providerFilter, setProviderFilter] = useState<"all" | "direct" | "soulseek">("all");

  useEffect(() => {
    if (isOpen && track) {
      setResults([]);
      setHasSearched(false);
      setSelectedId(null);
      setIsStartingDownload(false);
      setProviderFilter("all");
      performSearch(track);
    }
  }, [isOpen, track]);

  const performSearch = async (tr: DownloadModalTrack) => {
    setIsSearching(true);
    try {
      const res = await onSearchSoulseek(tr.artist, tr.title, tr.album);
      const list = Array.isArray(res) ? [...res] : [];
      list.sort((a, b) => rankResultMatch(b, tr.artist, tr.title) - rankResultMatch(a, tr.artist, tr.title));
      setResults(list);
    } catch (err) {
      console.error("Download search failed:", err);
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

  const directResults = results.filter(
    (r) => r.provider === "yt-dlp" || r.id.startsWith("ytdlp_")
  );
  const soulseekResults = results.filter(
    (r) => r.provider !== "yt-dlp" && !r.id.startsWith("ytdlp_")
  );

  const displayedResults =
    providerFilter === "direct"
      ? directResults
      : providerFilter === "soulseek"
      ? soulseekResults
      : results;

  const handleSelectOption = async (result: DownloadSearchResult) => {
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
    // Check if we have a confident direct stream result (score >= 40)
    const directRes = results.filter(
      (r) => r.provider === "yt-dlp" || r.id.startsWith("ytdlp_")
    );
    const goodDirect = directRes.find(
      (r) => rankResultMatch(r, track.artist, track.title) >= 40
    );

    if (goodDirect) {
      await handleSelectOption(goodDirect);
      return;
    }

    // If direct match was poor or missing, check if we have an excellent match in results
    if (results.length > 0 && rankResultMatch(results[0], track.artist, track.title) >= 30) {
      await handleSelectOption(results[0]);
      return;
    }

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
          maxWidth: "600px",
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
                Searching Download Providers...
              </div>
              <p style={{ fontSize: "0.82rem", color: "var(--text-muted)", maxWidth: "420px", margin: "0 auto" }}>
                Querying Direct Audio streams and Soulseek P2P network for optimal audio quality and speeds
              </p>
            </div>
          ) : results.length > 0 ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
              {/* Provider Filter Bar & Controls */}
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  flexWrap: "wrap",
                  gap: "8px",
                }}
              >
                <div style={{ display: "flex", gap: "6px", alignItems: "center" }}>
                  <button
                    type="button"
                    onClick={() => setProviderFilter("all")}
                    style={{
                      background: providerFilter === "all" ? "var(--bg-active, #2a2826)" : "transparent",
                      border: "1px solid",
                      borderColor: providerFilter === "all" ? "var(--border-active, #555)" : "var(--border)",
                      borderRadius: "6px",
                      padding: "4px 9px",
                      fontSize: "0.75rem",
                      fontWeight: 600,
                      color: providerFilter === "all" ? "#f3701e" : "var(--text-muted)",
                      cursor: "pointer",
                    }}
                  >
                    All ({results.length})
                  </button>

                  {directResults.length > 0 && (
                    <button
                      type="button"
                      onClick={() => setProviderFilter("direct")}
                      style={{
                        background: providerFilter === "direct" ? "rgba(139, 92, 246, 0.2)" : "transparent",
                        border: "1px solid",
                        borderColor: providerFilter === "direct" ? "rgba(139, 92, 246, 0.5)" : "var(--border)",
                        borderRadius: "6px",
                        padding: "4px 9px",
                        fontSize: "0.75rem",
                        fontWeight: 600,
                        color: providerFilter === "direct" ? "#c4b5fd" : "var(--text-muted)",
                        cursor: "pointer",
                        display: "inline-flex",
                        alignItems: "center",
                        gap: "4px",
                      }}
                    >
                      <Zap size={11} />
                      Direct Stream ({directResults.length})
                    </button>
                  )}

                  {soulseekResults.length > 0 && (
                    <button
                      type="button"
                      onClick={() => setProviderFilter("soulseek")}
                      style={{
                        background: providerFilter === "soulseek" ? "rgba(243, 112, 30, 0.2)" : "transparent",
                        border: "1px solid",
                        borderColor: providerFilter === "soulseek" ? "rgba(243, 112, 30, 0.5)" : "var(--border)",
                        borderRadius: "6px",
                        padding: "4px 9px",
                        fontSize: "0.75rem",
                        fontWeight: 600,
                        color: providerFilter === "soulseek" ? "var(--accent-light)" : "var(--text-muted)",
                        cursor: "pointer",
                      }}
                    >
                      Soulseek ({soulseekResults.length})
                    </button>
                  )}
                </div>

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

              {/* Items List */}
              {displayedResults.map((res) => {
                const isSelected = selectedId === res.id && isStartingDownload;
                const isDirect = res.provider === "yt-dlp" || res.id.startsWith("ytdlp_");

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
                        {/* Provider Source Tag */}
                        {isDirect ? (
                          <span
                            style={{
                              display: "inline-flex",
                              alignItems: "center",
                              gap: "3px",
                              backgroundColor: "rgba(139, 92, 246, 0.16)",
                              color: "#c4b5fd",
                              fontSize: "0.68rem",
                              fontWeight: 700,
                              padding: "2px 7px",
                              borderRadius: "4px",
                              letterSpacing: "0.4px",
                              textTransform: "uppercase",
                            }}
                          >
                            <Zap size={10} />
                            Direct Stream
                          </span>
                        ) : (
                          <span
                            style={{
                              display: "inline-flex",
                              alignItems: "center",
                              gap: "3px",
                              backgroundColor: "rgba(243, 112, 30, 0.16)",
                              color: "var(--accent-light)",
                              fontSize: "0.68rem",
                              fontWeight: 700,
                              padding: "2px 7px",
                              borderRadius: "4px",
                              letterSpacing: "0.4px",
                              textTransform: "uppercase",
                            }}
                          >
                            Soulseek P2P
                          </span>
                        )}

                        <span
                          className="badge"
                          style={{
                            backgroundColor: "rgba(255, 255, 255, 0.08)",
                            color: "var(--text-main)",
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
                            color: isDirect
                              ? "var(--accent-secondary)"
                              : res.slots_free
                              ? "var(--accent-secondary)"
                              : "#f3701e",
                            fontWeight: 500,
                          }}
                        >
                          {isDirect
                            ? "Instant Start"
                            : res.slots_free
                            ? "Slot Free"
                            : "Queued Slot"}
                        </span>
                      </div>
                    </div>

                    <button
                      type="button"
                      className="btn btn-primary"
                      disabled={isStartingDownload}
                      onClick={() => handleSelectOption(res)}
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
                No search results found right now
              </div>
              <p style={{ fontSize: "0.8rem", color: "var(--text-muted)", maxWidth: "380px", margin: "0 auto 16px" }}>
                No matching tracks found across Direct Audio or Soulseek networks. You can try re-scanning or use Direct Audio Stream Download below.
              </p>
            </div>
          ) : null}

          {/* Direct Stream Option Box */}
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
                <Zap size={14} />
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
            justifyContent: "flex-end",
          }}
        >
          <button
            type="button"
            className="btn btn-secondary"
            onClick={onClose}
            style={{ fontSize: "0.82rem" }}
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
};
