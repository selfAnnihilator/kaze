import React, { useState, useMemo } from "react";
import { Sparkles, Share2, DownloadCloud, Check, AlertCircle, X, Search } from "lucide-react";
import { DownloadButton } from "../DownloadButton";
import { useDebouncedValue } from "../../presentation/useDebouncedValue";
import { WindowedTable } from "../../presentation/WindowedTable";
import { JioSaavnPlaylistImport, UserProfile } from "../../types";
import { matchesPlaylistSearch } from "../../playlistSearch";

export interface ImportedPlaylistTrackInfo {
  id: string;
  title?: string;
  artist?: string;
  album?: string;
  duration_secs?: number;
  cover_art_url?: string;
  preview_url?: string;
}

export interface ImportPlaylistModalProps {
  onClose: () => void;
  onInspectJioSaavnPlaylist: (urlOrId: string) => Promise<JioSaavnPlaylistImport | null>;
  onSaveImportedPlaylist: (name: string, trackIds: string[], tracksInfo?: ImportedPlaylistTrackInfo[]) => Promise<void>;
  onSearchDirect?: (artist: string, title: string, album?: string) => void;
  currentUser?: UserProfile | null;
  onOpenAuthModal?: () => void;
}

/** JioSaavn / Spotify playlist import. Mounted only while open, so closing resets its state. */
export const ImportPlaylistModal: React.FC<ImportPlaylistModalProps> = ({
  onClose,
  onInspectJioSaavnPlaylist,
  onSaveImportedPlaylist,
  onSearchDirect,
  currentUser,
  onOpenAuthModal,
}) => {
  const [jiosaavnInput, setJioSaavnInput] = useState("");
  const [inspectingJioSaavn, setInspectingJioSaavn] = useState(false);
  const [jiosaavnResult, setJioSaavnResult] = useState<JioSaavnPlaylistImport | null>(null);
  const [jiosaavnMessage, setJioSaavnMessage] = useState<string | null>(null);
  const [jiosaavnFilterQuery, setJioSaavnFilterQuery] = useState("");

  const handleInspectJioSaavn = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!jiosaavnInput.trim()) return;
    setInspectingJioSaavn(true);
    setJioSaavnMessage(null);
    try {
      const res = await onInspectJioSaavnPlaylist(jiosaavnInput.trim());
      if (res) {
        setJioSaavnResult(res);
      } else {
        setJioSaavnMessage(
          "Could not retrieve playlist. Please verify the URL or ID.",
        );
      }
    } catch (err: any) {
      setJioSaavnMessage(
        typeof err === "string" ? err : err?.message || "Failed to fetch playlist.",
      );
    } finally {
      setInspectingJioSaavn(false);
    }
  };

  const handleSaveJioSaavnPlaylist = async (importAll: boolean = true) => {
    if (!jiosaavnResult) return;
    if (!currentUser && onOpenAuthModal) {
      onOpenAuthModal();
      return;
    }

    // Determine tracks to import
    const targetTracks = importAll
      ? jiosaavnResult.tracks
      : jiosaavnResult.tracks.filter(
          (t) => t.in_library && t.matched_local_track_id,
        );

    if (targetTracks.length === 0) {
      alert("No tracks to import.");
      return;
    }

    const trackIds = targetTracks.map((track) => track.track_id);

    const tracksInfo = targetTracks.map((t, idx) => ({
      id: trackIds[idx],
      title: t.title,
      artist: t.artist,
      duration_secs: t.duration_secs,
      album: t.album,
      cover_art_url: t.cover_art_url,
    }));

    await onSaveImportedPlaylist(jiosaavnResult.title, trackIds, tracksInfo);
    onClose();
  };

  const debouncedJioSaavnFilter = useDebouncedValue(jiosaavnFilterQuery);
  const displayedJioSaavnTracks = useMemo(() => {
    if (!jiosaavnResult) return [];
    if (!debouncedJioSaavnFilter.trim()) return jiosaavnResult.tracks;
    return jiosaavnResult.tracks.filter((track) =>
      matchesPlaylistSearch(track, debouncedJioSaavnFilter),
    );
  }, [jiosaavnResult, debouncedJioSaavnFilter]);

  const handleSearchMissing = async (
    trackTitle: string,
    artistName: string,
  ) => {
    onSearchDirect?.(artistName, trackTitle);
  };

  return (
    <div
      className="modal-overlay"
      onClick={onClose}
    >
      <div
        className="modal-content"
        style={{
          maxWidth: "800px",
          width: "95%",
          maxHeight: "88vh",
          display: "flex",
          flexDirection: "column",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: "16px",
          }}
        >
          <div
            style={{ display: "flex", alignItems: "center", gap: "10px" }}
          >
            <Share2 size={24} color="var(--accent-secondary)" />
            <h3 style={{ fontSize: "var(--fs-20)", fontWeight: "var(--weight-bold)" }}>
              Import playlist (up to 50 songs)
            </h3>
          </div>
          <button
            onClick={onClose}
            style={{
              background: "none",
              border: "none",
              color: "var(--text-muted)",
              cursor: "pointer",
            }}
          >
            <X size={20} />
          </button>
        </div>

        <p
          style={{
            fontSize: "var(--fs-14)",
            color: "var(--text-muted)",
            marginBottom: "16px",
            lineHeight: 1.5,
          }}
        >
          Paste a public JioSaavn or Spotify playlist link. Spotify songs
          are matched to playable JioSaavn recordings; Kaze then lets you
          save the selected tracks.
        </p>

        {/* Input Form */}
        <form
          onSubmit={handleInspectJioSaavn}
          style={{ display: "flex", gap: "10px", marginBottom: "20px" }}
        >
          <input
            type="text"
            placeholder="JioSaavn or Spotify playlist link"
            value={jiosaavnInput}
            onChange={(e) => setJioSaavnInput(e.target.value)}
            className="input-field"
            style={{ flex: 1, fontFamily: "var(--font-mono)", fontSize: "var(--fs-14)" }}
          />
          <button
            type="submit"
            disabled={inspectingJioSaavn || !jiosaavnInput.trim()}
            className="btn btn-primary"
          >
            <Sparkles size={15} />
            <span>
              {inspectingJioSaavn ? "Inspecting..." : "Inspect Playlist"}
            </span>
          </button>
        </form>

        {jiosaavnMessage && (
          <div
            style={{
              padding: "12px 16px",
              borderRadius: "8px",
              backgroundColor: "var(--danger-a12)",
              border: "1px solid var(--danger)",
              color: "var(--palette-red-200)",
              fontSize: "var(--fs-14)",
              marginBottom: "16px",
              display: "flex",
              alignItems: "center",
              gap: "8px",
            }}
          >
            <AlertCircle size={16} />
            <span>{jiosaavnMessage}</span>
          </div>
        )}

        {/* Inspected Playlist Result */}
        {jiosaavnResult && (
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              flex: 1,
              overflow: "hidden",
            }}
          >
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                padding: "14px",
                borderRadius: "8px",
                backgroundColor: "var(--bg-card)",
                border: "1px solid var(--border)",
                marginBottom: "16px",
                flexWrap: "wrap",
                gap: "12px",
              }}
            >
              <div>
                <div
                  style={{
                    fontWeight: "var(--weight-bold)",
                    fontSize: "var(--fs-18)",
                    color: "var(--text-main)",
                  }}
                >
                  {jiosaavnResult.title}
                </div>
                <div
                  style={{
                    display: "flex",
                    gap: "12px",
                    marginTop: "6px",
                    fontSize: "var(--fs-13)",
                  }}
                >
                  <span style={{ color: "var(--text-dim)" }}>
                    Total: {jiosaavnResult.total_tracks} tracks
                  </span>
                  <span
                    style={{ color: "var(--success)", fontWeight: "var(--weight-semibold)" }}
                  >
                    ● {jiosaavnResult.matched_tracks} Local
                  </span>
                  <span
                    style={{
                      color: "var(--accent-secondary, var(--accent))",
                      fontWeight: "var(--weight-semibold)",
                    }}
                  >
                    ● {jiosaavnResult.missing_tracks} Online
                  </span>
                </div>
                {(jiosaavnResult.source_total ?? 0) > 50 && (
                  <div
                    style={{
                      marginTop: "6px",
                      fontSize: "var(--fs-12)",
                      color: "var(--text-dim)",
                    }}
                  >
                    Only the first 50 of {jiosaavnResult.source_total} songs were checked.
                  </div>
                )}
                {!!jiosaavnResult.unmatched?.length && (
                  <details
                    style={{
                      marginTop: "6px",
                      fontSize: "var(--fs-13)",
                      color: "var(--text-muted)",
                    }}
                  >
                    <summary style={{ cursor: "pointer" }}>
                      {jiosaavnResult.unmatched.length} song
                      {jiosaavnResult.unmatched.length === 1 ? "" : "s"} not
                      found on JioSaavn
                    </summary>
                    <ul
                      style={{
                        margin: "6px 0 0",
                        paddingLeft: "18px",
                        maxHeight: "140px",
                        overflowY: "auto",
                      }}
                    >
                      {jiosaavnResult.unmatched.map((u, i) => (
                        <li key={`${u.title}-${i}`}>
                          {u.title}
                          {u.artist ? ` — ${u.artist}` : ""}
                        </li>
                      ))}
                    </ul>
                  </details>
                )}
              </div>

              <div>
                <button
                  className="btn btn-primary"
                  onClick={() => handleSaveJioSaavnPlaylist(true)}
                  title="Import this entire playlist into Kaze"
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "8px",
                  }}
                >
                  <Check size={16} />
                  <span>Add as Playlist</span>
                </button>
              </div>
            </div>

            {/* Track Search Bar in JioSaavn Inspection Modal */}
            <div style={{ position: "relative", marginBottom: "12px" }}>
              <Search
                size={14}
                color="var(--text-muted)"
                style={{
                  position: "absolute",
                  left: "12px",
                  top: "50%",
                  transform: "translateY(-50%)",
                  pointerEvents: "none",
                }}
              />
              <input
                type="text"
                placeholder="Search songs or artists in this playlist..."
                value={jiosaavnFilterQuery}
                onChange={(e) => setJioSaavnFilterQuery(e.target.value)}
                style={{
                  width: "100%",
                  padding: "8px 32px 8px 34px",
                  borderRadius: "8px",
                  border: "1px solid var(--border)",
                  backgroundColor: "var(--shade-25)",
                  color: "var(--text-main)",
                  fontSize: "var(--fs-13)",
                  outline: "none",
                }}
              />
              {jiosaavnFilterQuery && (
                <button
                  type="button"
                  onClick={() => setJioSaavnFilterQuery("")}
                  style={{
                    position: "absolute",
                    right: "10px",
                    top: "50%",
                    transform: "translateY(-50%)",
                    background: "none",
                    border: "none",
                    color: "var(--text-muted)",
                    cursor: "pointer",
                    padding: "2px",
                    display: "flex",
                    alignItems: "center",
                  }}
                  title="Clear search"
                >
                  <X size={13} />
                </button>
              )}
            </div>

            {/* Track inspection table */}
            <div style={{ overflowY: "auto", flex: 1 }}>
              <WindowedTable
                className="track-table"
                style={{ width: "100%" }}
                items={displayedJioSaavnTracks}
                rowHeight={72}
                renderItem={(tr, idx) => (
                  <tr key={tr.track_id || idx} className="track-row">
                    <td
                      style={{
                        textAlign: "center",
                        color: "var(--text-dim)",
                      }}
                    >
                      {idx + 1}
                    </td>
                    <td className="primary">{tr.title}</td>
                    <td>{tr.artist}</td>
                    <td style={{ textAlign: "center" }}>
                      {tr.in_library ? (
                        <span className="badge badge-exact">Local</span>
                      ) : (
                        <span
                          className="badge"
                          style={{
                            backgroundColor: "var(--accent-a15)",
                            color: "var(--accent-secondary, var(--accent))",
                          }}
                        >
                          Online
                        </span>
                      )}
                    </td>
                    <td style={{ textAlign: "center" }}>
                      {!tr.in_library && (
                        <div
                          style={{
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                          }}
                        >
                          {onSearchDirect ? (
                            <DownloadButton key={tr.track_id} trackId={tr.track_id} sourceId={tr.track_id?.startsWith("jiosaavn:") ? tr.track_id.slice(9) : undefined} title={tr.title} />
                          ) : (
                            <button
                              className="btn btn-secondary"
                              style={{
                                padding: "4px 10px",
                                fontSize: "var(--fs-12)",
                                display: "inline-flex",
                                alignItems: "center",
                                gap: "5px",
                              }}
                              onClick={() =>
                                handleSearchMissing(
                                  tr.title,
                                  tr.artist,
                                )
                              }
                              title="Search and download track"
                            >
                              <DownloadCloud size={13} />
                              <span>Download</span>
                            </button>
                          )}
                        </div>
                      )}
                    </td>
                  </tr>
                )}
              >
                <thead>
                  <tr>
                    <th style={{ width: "40px", textAlign: "center" }}>
                      #
                    </th>
                    <th>Track</th>
                    <th>Artist</th>
                    <th style={{ width: "120px", textAlign: "center" }}>
                      Status
                    </th>
                    <th style={{ width: "120px", textAlign: "center" }}>
                      Action
                    </th>
                  </tr>
                </thead>
              </WindowedTable>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
