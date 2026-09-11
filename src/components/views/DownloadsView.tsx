import React, { useState, useEffect } from "react";
import { DownloadTask, DownloadSearchResult } from "../../types";
import {
  DownloadCloud,
  Search,
  CheckCircle,
  AlertCircle,
  XCircle,
  Clock,
  HardDrive,
  RefreshCw,
  Users,
  Zap,
  ExternalLink,
  ArrowRight,
  Music,
  Check,
} from "lucide-react";

interface DownloadsViewProps {
  downloads: DownloadTask[];
  onSearchSoulseek: (artist: string, title: string, album?: string) => Promise<DownloadSearchResult[]>;
  onStartDownload: (searchResultId: string, wishlistId?: string) => Promise<void>;
  onCancelDownload: (taskId: string) => Promise<void>;
  onRefreshDownloads: () => Promise<void>;
  onLaunchSoulseek: (query?: string, filter?: string) => Promise<void>;
  onImportSoulseek: () => Promise<void>;
  initialSearch?: { artist: string; title: string; album?: string } | null;
}

export const DownloadsView: React.FC<DownloadsViewProps> = ({
  downloads,
  onSearchSoulseek,
  onStartDownload,
  onCancelDownload,
  onRefreshDownloads,
  onLaunchSoulseek,
  onImportSoulseek,
  initialSearch,
}) => {
  const [activeTab, setActiveTab] = useState<"search" | "tasks">("search");
  const [searchArtist, setSearchArtist] = useState(initialSearch?.artist || "");
  const [searchTitle, setSearchTitle] = useState(initialSearch?.title || "");
  const [searchAlbum, setSearchAlbum] = useState(initialSearch?.album || "");
  const [searchResults, setSearchResults] = useState<DownloadSearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [searchStatus, setSearchStatus] = useState<"idle" | "searching" | "succeeded" | "no_results" | "error">("idle");
  const [lastQuery, setLastQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<string>("ALL");
  const [downloadingIds, setDownloadingIds] = useState<Set<string>>(new Set());
  const [feedbackBanner, setFeedbackBanner] = useState<{
    type: "success" | "info";
    title: string;
    subtitle?: string;
  } | null>(null);

  // Auto-switch to search tab if initialSearch is provided
  useEffect(() => {
    if (initialSearch) {
      setSearchArtist(initialSearch.artist);
      setSearchTitle(initialSearch.title);
      setSearchAlbum(initialSearch.album || "");
      setActiveTab("search");
      handleSearch(initialSearch.artist, initialSearch.title, initialSearch.album);
    }
  }, [initialSearch]);

  const handleSearch = async (artist: string, title: string, album?: string) => {
    const qArtist = artist.trim();
    const qTitle = title.trim();
    if (!qArtist && !qTitle) return;

    setSearching(true);
    setSearchStatus("searching");
    const queryDisplay = [qArtist, qTitle].filter(Boolean).join(" - ");
    setLastQuery(queryDisplay);
    setFeedbackBanner(null);

    try {
      const results = await onSearchSoulseek(qArtist, qTitle, album?.trim() || undefined);
      setSearchResults(results);
      if (results.length > 0) {
        setSearchStatus("succeeded");
      } else {
        setSearchStatus("no_results");
      }
    } catch (err) {
      console.error("Failed to search music:", err);
      setSearchStatus("error");
    } finally {
      setSearching(false);
    }
  };

  const handleDownloadClick = async (result: DownloadSearchResult) => {
    setDownloadingIds((prev) => new Set(prev).add(result.id));
    try {
      await onStartDownload(result.id);
      setFeedbackBanner({
        type: "success",
        title: `Direct download started!`,
        subtitle: `\"${result.filename}\" is now downloading directly. Auto-importing to library upon completion.`,
      });
    } catch (err) {
      console.error("Failed to start download:", err);
    } finally {
      setDownloadingIds((prev) => {
        const next = new Set(prev);
        next.delete(result.id);
        return next;
      });
    }
  };

  const formatBytes = (bytes: number) => {
    if (!bytes || bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  };

  const filteredTasks = downloads.filter((task) => {
    if (statusFilter === "ALL") return true;
    if (statusFilter === "ACTIVE") return task.status === "DOWNLOADING" || task.status === "QUEUED";
    return task.status === statusFilter;
  });

  const activeDownloads = downloads.filter(
    (t) => t.status === "DOWNLOADING" || t.status === "QUEUED"
  );

  const getStatusBadge = (status: DownloadTask["status"], isDirect: boolean = true) => {
    const baseStyle: React.CSSProperties = {
      display: "inline-flex",
      alignItems: "center",
      gap: "6px",
      padding: "4px 10px",
      borderRadius: "9999px",
      fontSize: "0.75rem",
      fontWeight: 600,
      textTransform: "uppercase",
    };

    switch (status) {
      case "QUEUED":
        return (
          <span style={{ ...baseStyle, backgroundColor: "rgba(245, 158, 11, 0.15)", color: "#f59e0b", border: "1px solid rgba(245, 158, 11, 0.3)" }}>
            <Clock size={12} /> Queued
          </span>
        );
      case "DOWNLOADING":
        return (
          <span style={{ ...baseStyle, backgroundColor: "rgba(139, 92, 246, 0.2)", color: "#a78bfa", border: "1px solid rgba(139, 92, 246, 0.4)" }}>
            <DownloadCloud size={12} /> {isDirect ? "Direct Downloading" : "Transferring"}
          </span>
        );
      case "COMPLETED":
        return (
          <span style={{ ...baseStyle, backgroundColor: "rgba(16, 185, 129, 0.15)", color: "#10b981", border: "1px solid rgba(16, 185, 129, 0.3)" }}>
            <CheckCircle size={12} /> In Library
          </span>
        );
      case "FAILED":
        return (
          <span style={{ ...baseStyle, backgroundColor: "rgba(239, 68, 68, 0.15)", color: "#ef4444", border: "1px solid rgba(239, 68, 68, 0.3)" }}>
            <AlertCircle size={12} /> Failed
          </span>
        );
      case "CANCELLED":
        return (
          <span style={{ ...baseStyle, backgroundColor: "rgba(107, 114, 128, 0.15)", color: "#9ca3af", border: "1px solid rgba(107, 114, 128, 0.3)" }}>
            <XCircle size={12} /> Cancelled
          </span>
        );
    }
  };

  const findTaskForResult = (res: DownloadSearchResult): DownloadTask | undefined => {
    return downloads.find(
      (t) => t.filename === res.filename || (res.id.startsWith("ytdlp_") && t.filename.includes(res.username))
    );
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "24px" }}>
      {/* Top Header */}
      <div className="view-header" style={{ marginBottom: 0 }}>
        <div>
          <h1 className="view-title" style={{ display: "flex", alignItems: "center", gap: "12px" }}>
            <DownloadCloud size={28} color="var(--accent-light)" />
            <span>Music Downloads & Transfers</span>
          </h1>
          <p style={{ color: "var(--text-muted)", fontSize: "0.88rem", marginTop: "6px" }}>
            Search and download songs directly into your library in 320kbps MP3, or manage active transfers.
          </p>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
          <button
            onClick={() => onLaunchSoulseek()}
            className="btn btn-secondary"
            style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "0.85rem" }}
            title="Open external SoulseekQt application"
          >
            <ExternalLink size={15} />
            <span>Open SoulseekQt</span>
          </button>
          <button
            onClick={() => onImportSoulseek()}
            className="btn btn-secondary"
            style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "0.85rem" }}
            title="Scan ~/Soulseek Downloads/complete and auto-import into library"
          >
            <RefreshCw size={15} />
            <span>Import Completed</span>
          </button>
          <button
            onClick={() => onRefreshDownloads()}
            className="btn btn-secondary"
            style={{ padding: "8px 12px" }}
            title="Refresh status"
          >
            <RefreshCw size={15} />
          </button>
        </div>
      </div>

      {/* Main Tabs */}
      <div className="tab-bar" style={{ margin: 0 }}>
        <button
          onClick={() => setActiveTab("search")}
          className={`tab-btn ${activeTab === "search" ? "active" : ""}`}
        >
          <Search size={16} />
          <span>Direct Search & Download</span>
        </button>

        <button
          onClick={() => setActiveTab("tasks")}
          className={`tab-btn ${activeTab === "tasks" ? "active" : ""}`}
        >
          <HardDrive size={16} />
          <span>Transfers & Library Imports</span>
          {downloads.length > 0 && (
            <span
              style={{
                marginLeft: "6px",
                padding: "2px 8px",
                borderRadius: "12px",
                fontSize: "0.75rem",
                fontWeight: 700,
                backgroundColor: activeDownloads.length > 0 ? "var(--accent)" : "var(--bg-sidebar)",
                color: activeDownloads.length > 0 ? "#fff" : "var(--text-dim)",
              }}
            >
              {activeDownloads.length > 0 ? `${activeDownloads.length} active` : downloads.length}
            </span>
          )}
        </button>
      </div>

      {/* Notification Banner */}
      {feedbackBanner && (
        <div
          style={{
            backgroundColor: "rgba(139, 92, 246, 0.12)",
            border: "1px solid rgba(139, 92, 246, 0.4)",
            borderRadius: "10px",
            padding: "14px 18px",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: "14px",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
            <CheckCircle size={20} color="var(--accent-light)" style={{ flexShrink: 0 }} />
            <div>
              <div style={{ fontWeight: 600, fontSize: "0.92rem", color: "var(--text-main)" }}>
                {feedbackBanner.title}
              </div>
              {feedbackBanner.subtitle && (
                <div style={{ fontSize: "0.82rem", color: "var(--text-muted)", marginTop: "2px" }}>
                  {feedbackBanner.subtitle}
                </div>
              )}
            </div>
          </div>
          <button
            onClick={() => setActiveTab("tasks")}
            className="btn btn-primary"
            style={{ fontSize: "0.8rem", padding: "6px 12px", flexShrink: 0 }}
          >
            <span>View in Transfers</span>
            <ArrowRight size={14} />
          </button>
        </div>
      )}

      {/* TAB 1: SEARCH & DIRECT DOWNLOAD */}
      {activeTab === "search" && (
        <div style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
          {/* Visual Progress Pipeline Tracker */}
          <div className="content-card" style={{ padding: "16px 20px", marginBottom: 0 }}>
            <div
              style={{
                fontSize: "0.75rem",
                fontWeight: 700,
                textTransform: "uppercase",
                letterSpacing: "0.05em",
                color: "var(--text-dim)",
                marginBottom: "14px",
                display: "flex",
                alignItems: "center",
                gap: "8px",
              }}
            >
              <Zap size={14} color="var(--accent-light)" />
              <span>Direct In-App Music Acquisition Pipeline</span>
            </div>

            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))",
                gap: "12px",
              }}
            >
              {/* Step 1 */}
              <div
                style={{
                  padding: "12px 14px",
                  borderRadius: "8px",
                  border: `1px solid ${
                    searching
                      ? "var(--accent)"
                      : searchStatus === "succeeded"
                      ? "rgba(16, 185, 129, 0.4)"
                      : "var(--border)"
                  }`,
                  backgroundColor: searching
                    ? "rgba(139, 92, 246, 0.1)"
                    : searchStatus === "succeeded"
                    ? "rgba(16, 185, 129, 0.08)"
                    : "var(--bg-sidebar)",
                  display: "flex",
                  alignItems: "center",
                  gap: "12px",
                  transition: "all 0.2s ease",
                }}
              >
                <div style={{ flexShrink: 0 }}>
                  {searching ? (
                    <RefreshCw size={18} color="var(--accent-light)" className="animate-spin" />
                  ) : searchStatus === "succeeded" ? (
                    <Check size={18} color="#10b981" />
                  ) : (
                    <Search size={18} color="var(--text-dim)" />
                  )}
                </div>
                <div>
                  <div style={{ fontSize: "0.82rem", fontWeight: 700, color: "var(--text-main)" }}>
                    1. Online Search
                  </div>
                  <div style={{ fontSize: "0.74rem", color: "var(--text-muted)", marginTop: "2px" }}>
                    {searching
                      ? "Querying sources..."
                      : searchStatus === "succeeded"
                      ? `${searchResults.length} streams found`
                      : "Multi-source online search"}
                  </div>
                </div>
              </div>

              {/* Step 2 */}
              <div
                style={{
                  padding: "12px 14px",
                  borderRadius: "8px",
                  border: `1px solid ${
                    searchResults.length > 0 ? "rgba(16, 185, 129, 0.4)" : "var(--border)"
                  }`,
                  backgroundColor:
                    searchResults.length > 0 ? "rgba(16, 185, 129, 0.08)" : "var(--bg-sidebar)",
                  display: "flex",
                  alignItems: "center",
                  gap: "12px",
                  transition: "all 0.2s ease",
                }}
              >
                <div style={{ flexShrink: 0 }}>
                  {searchResults.length > 0 ? (
                    <Check size={18} color="#10b981" />
                  ) : (
                    <Music size={18} color="var(--text-dim)" />
                  )}
                </div>
                <div>
                  <div style={{ fontSize: "0.82rem", fontWeight: 700, color: "var(--text-main)" }}>
                    2. Audio Quality
                  </div>
                  <div style={{ fontSize: "0.74rem", color: "var(--text-muted)", marginTop: "2px" }}>
                    {searchResults.length > 0 ? "320 kbps MP3 Ready" : "High bitrate audio"}
                  </div>
                </div>
              </div>

              {/* Step 3 */}
              <div
                style={{
                  padding: "12px 14px",
                  borderRadius: "8px",
                  border: `1px solid ${
                    activeDownloads.length > 0
                      ? "var(--accent)"
                      : downloads.some((t) => t.status === "COMPLETED")
                      ? "rgba(16, 185, 129, 0.4)"
                      : "var(--border)"
                  }`,
                  backgroundColor:
                    activeDownloads.length > 0
                      ? "rgba(139, 92, 246, 0.12)"
                      : downloads.some((t) => t.status === "COMPLETED")
                      ? "rgba(16, 185, 129, 0.08)"
                      : "var(--bg-sidebar)",
                  display: "flex",
                  alignItems: "center",
                  gap: "12px",
                  transition: "all 0.2s ease",
                }}
              >
                <div style={{ flexShrink: 0 }}>
                  {activeDownloads.length > 0 ? (
                    <DownloadCloud size={18} color="var(--accent-light)" />
                  ) : downloads.some((t) => t.status === "COMPLETED") ? (
                    <Check size={18} color="#10b981" />
                  ) : (
                    <Zap size={18} color="var(--text-dim)" />
                  )}
                </div>
                <div>
                  <div style={{ fontSize: "0.82rem", fontWeight: 700, color: "var(--text-main)" }}>
                    3. Direct Download
                  </div>
                  <div style={{ fontSize: "0.74rem", color: "var(--text-muted)", marginTop: "2px" }}>
                    {activeDownloads.length > 0
                      ? `${activeDownloads.length} transfer(s) active`
                      : "Native background transfer"}
                  </div>
                </div>
              </div>

              {/* Step 4 */}
              <div
                style={{
                  padding: "12px 14px",
                  borderRadius: "8px",
                  border: `1px solid ${
                    downloads.some((t) => t.status === "COMPLETED")
                      ? "rgba(16, 185, 129, 0.4)"
                      : "var(--border)"
                  }`,
                  backgroundColor: downloads.some((t) => t.status === "COMPLETED")
                    ? "rgba(16, 185, 129, 0.08)"
                    : "var(--bg-sidebar)",
                  display: "flex",
                  alignItems: "center",
                  gap: "12px",
                  transition: "all 0.2s ease",
                }}
              >
                <div style={{ flexShrink: 0 }}>
                  {downloads.some((t) => t.status === "COMPLETED") ? (
                    <Check size={18} color="#10b981" />
                  ) : (
                    <CheckCircle size={18} color="var(--text-dim)" />
                  )}
                </div>
                <div>
                  <div style={{ fontSize: "0.82rem", fontWeight: 700, color: "var(--text-main)" }}>
                    4. Auto-Import
                  </div>
                  <div style={{ fontSize: "0.74rem", color: "var(--text-muted)", marginTop: "2px" }}>
                    {downloads.some((t) => t.status === "COMPLETED")
                      ? "Indexed in Library"
                      : "Instant library indexing"}
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* Search Input Form */}
          <form
            onSubmit={(e) => {
              e.preventDefault();
              handleSearch(searchArtist, searchTitle, searchAlbum);
            }}
            className="content-card"
            style={{ marginBottom: 0 }}
          >
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))",
                gap: "16px",
                marginBottom: "20px",
              }}
            >
              <div>
                <label
                  style={{
                    display: "block",
                    fontSize: "0.75rem",
                    fontWeight: 600,
                    textTransform: "uppercase",
                    letterSpacing: "0.05em",
                    color: "var(--text-dim)",
                    marginBottom: "8px",
                  }}
                >
                  Artist Name
                </label>
                <input
                  type="text"
                  value={searchArtist}
                  onChange={(e) => setSearchArtist(e.target.value)}
                  placeholder="e.g. Daft Punk"
                  className="input-field"
                  style={{ width: "100%" }}
                />
              </div>

              <div>
                <label
                  style={{
                    display: "block",
                    fontSize: "0.75rem",
                    fontWeight: 600,
                    textTransform: "uppercase",
                    letterSpacing: "0.05em",
                    color: "var(--text-dim)",
                    marginBottom: "8px",
                  }}
                >
                  Song / Track Title
                </label>
                <input
                  type="text"
                  value={searchTitle}
                  onChange={(e) => setSearchTitle(e.target.value)}
                  placeholder="e.g. One More Time"
                  className="input-field"
                  style={{ width: "100%" }}
                />
              </div>

              <div>
                <label
                  style={{
                    display: "block",
                    fontSize: "0.75rem",
                    fontWeight: 600,
                    textTransform: "uppercase",
                    letterSpacing: "0.05em",
                    color: "var(--text-dim)",
                    marginBottom: "8px",
                  }}
                >
                  Album (Optional)
                </label>
                <input
                  type="text"
                  value={searchAlbum}
                  onChange={(e) => setSearchAlbum(e.target.value)}
                  placeholder="e.g. Discovery"
                  className="input-field"
                  style={{ width: "100%" }}
                />
              </div>
            </div>

            <div style={{ display: "flex", justifyContent: "flex-end", alignItems: "center", gap: "12px" }}>
              <button
                type="button"
                onClick={() => {
                  const title = searchTitle.trim();
                  const artist = searchArtist.trim();
                  const query = title || artist;
                  const filter = title && artist ? artist : undefined;
                  onLaunchSoulseek(query || undefined, filter);
                }}
                className="btn btn-secondary"
                title="Search track in SoulseekQt and filter results by artist name"
                style={{ fontSize: "0.85rem" }}
              >
                <ExternalLink size={15} />
                <span>Search in SoulseekQt</span>
              </button>

              <button
                type="submit"
                disabled={searching || (!searchArtist.trim() && !searchTitle.trim())}
                className="btn btn-primary"
                style={{
                  fontSize: "0.88rem",
                  padding: "9px 20px",
                  opacity: searching || (!searchArtist.trim() && !searchTitle.trim()) ? 0.6 : 1,
                  cursor: searching || (!searchArtist.trim() && !searchTitle.trim()) ? "not-allowed" : "pointer",
                }}
              >
                {searching ? <RefreshCw size={16} className="animate-spin" /> : <Search size={16} />}
                <span>{searching ? "Searching Audio Sources..." : "Search & Download Directly"}</span>
              </button>
            </div>
          </form>

          {/* Search Status Feedback Cards */}
          {searchStatus === "searching" && (
            <div
              style={{
                backgroundColor: "rgba(139, 92, 246, 0.1)",
                border: "1px solid rgba(139, 92, 246, 0.3)",
                borderRadius: "10px",
                padding: "16px 20px",
                display: "flex",
                alignItems: "center",
                gap: "14px",
              }}
            >
              <RefreshCw size={20} color="var(--accent-light)" className="animate-spin" />
              <div>
                <div style={{ fontWeight: 600, fontSize: "0.9rem", color: "var(--text-main)" }}>
                  Searching high-speed audio sources...
                </div>
                <div style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginTop: "2px" }}>
                  Querying online databases for \"{lastQuery}\". Looking for 320kbps MP3 audio streams...
                </div>
              </div>
            </div>
          )}

          {searchStatus === "succeeded" && (
            <div
              style={{
                backgroundColor: "rgba(16, 185, 129, 0.1)",
                border: "1px solid rgba(16, 185, 129, 0.3)",
                borderRadius: "10px",
                padding: "14px 20px",
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: "14px",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                <CheckCircle size={18} color="#10b981" style={{ flexShrink: 0 }} />
                <div>
                  <span style={{ fontWeight: 600, fontSize: "0.9rem", color: "#10b981" }}>
                    Search Succeeded!
                  </span>
                  <span style={{ fontSize: "0.82rem", color: "var(--text-muted)", marginLeft: "8px" }}>
                    Found {searchResults.length} direct high-speed audio stream(s) for \"{lastQuery}\". Click Download below.
                  </span>
                </div>
              </div>
              <span
                style={{
                  fontSize: "0.75rem",
                  fontWeight: 600,
                  color: "#10b981",
                  backgroundColor: "rgba(16, 185, 129, 0.15)",
                  padding: "4px 10px",
                  borderRadius: "9999px",
                  flexShrink: 0,
                }}
              >
                Direct In-App
              </span>
            </div>
          )}

          {searchStatus === "no_results" && (
            <div
              style={{
                backgroundColor: "rgba(245, 158, 11, 0.1)",
                border: "1px solid rgba(245, 158, 11, 0.3)",
                borderRadius: "10px",
                padding: "16px 20px",
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: "14px",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                <AlertCircle size={20} color="#f59e0b" style={{ flexShrink: 0 }} />
                <div>
                  <div style={{ fontWeight: 600, fontSize: "0.9rem", color: "#f59e0b" }}>
                    No Direct Streams Found
                  </div>
                  <div style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginTop: "2px" }}>
                    Could not find direct streams for \"{lastQuery}\". Check your search terms or try SoulseekQt.
                  </div>
                </div>
              </div>
              <button
                onClick={() => {
                  const title = searchTitle.trim();
                  const artist = searchArtist.trim();
                  onLaunchSoulseek(title || artist || undefined, title && artist ? artist : undefined);
                }}
                className="btn btn-secondary"
                style={{ fontSize: "0.8rem", padding: "6px 12px", flexShrink: 0 }}
              >
                <ExternalLink size={14} />
                <span>Search SoulseekQt</span>
              </button>
            </div>
          )}

          {/* Active Direct Downloads Section on Search Tab */}
          {activeDownloads.length > 0 && (
            <div
              className="content-card"
              style={{
                borderColor: "var(--accent)",
                padding: "18px 20px",
                marginBottom: 0,
              }}
            >
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  marginBottom: "14px",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  <div
                    style={{
                      width: "8px",
                      height: "8px",
                      borderRadius: "50%",
                      backgroundColor: "var(--accent-light)",
                      boxShadow: "0 0 8px var(--accent)",
                    }}
                  />
                  <span
                    style={{
                      fontSize: "0.78rem",
                      fontWeight: 700,
                      textTransform: "uppercase",
                      letterSpacing: "0.05em",
                      color: "var(--accent-light)",
                    }}
                  >
                    Direct Downloads Active ({activeDownloads.length})
                  </span>
                </div>
                <button
                  onClick={() => setActiveTab("tasks")}
                  style={{
                    background: "none",
                    border: "none",
                    color: "var(--accent-light)",
                    fontSize: "0.8rem",
                    fontWeight: 600,
                    cursor: "pointer",
                    display: "flex",
                    alignItems: "center",
                    gap: "4px",
                  }}
                >
                  <span>View in Transfers</span>
                  <ArrowRight size={13} />
                </button>
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
                {activeDownloads.map((task) => {
                  const total = task.file_size || 0;
                  const progress = total > 0 ? Math.min(100, (task.bytes_downloaded / total) * 100) : 0;
                  return (
                    <div
                      key={task.id}
                      style={{
                        padding: "12px 14px",
                        backgroundColor: "var(--bg-sidebar)",
                        borderRadius: "8px",
                        border: "1px solid var(--border)",
                      }}
                    >
                      <div
                        style={{
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "space-between",
                          fontSize: "0.82rem",
                          marginBottom: "6px",
                        }}
                      >
                        <div
                          style={{
                            fontWeight: 600,
                            color: "var(--text-main)",
                            whiteSpace: "nowrap",
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            maxWidth: "400px",
                          }}
                        >
                          {task.title || task.filename}
                        </div>
                        <div
                          style={{
                            color: "var(--accent-light)",
                            fontFamily: "monospace",
                            fontWeight: 600,
                          }}
                        >
                          {progress > 0 ? `${progress.toFixed(1)}%` : "Connecting..."}
                        </div>
                      </div>
                      <div className="progress-track">
                        <div className="progress-fill" style={{ width: `${progress}%` }} />
                      </div>
                      <div
                        style={{
                          display: "flex",
                          justifyContent: "space-between",
                          fontSize: "0.72rem",
                          color: "var(--text-dim)",
                          marginTop: "6px",
                        }}
                      >
                        <span>
                          {formatBytes(task.bytes_downloaded)} / {formatBytes(total)}
                        </span>
                        <span>Auto-importing into library upon completion</span>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* Results Table */}
          {searchResults.length > 0 && (
            <div className="content-card" style={{ padding: 0, overflow: "hidden", marginBottom: 0 }}>
              <div
                style={{
                  padding: "14px 20px",
                  borderBottom: "1px solid var(--border)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                }}
              >
                <span
                  style={{
                    fontSize: "0.78rem",
                    fontWeight: 700,
                    textTransform: "uppercase",
                    letterSpacing: "0.05em",
                    color: "var(--text-dim)",
                  }}
                >
                  Found {searchResults.length} Audio Stream(s)
                </span>
                <span style={{ fontSize: "0.78rem", color: "var(--text-dim)" }}>
                  Downloads save directly to your local library folder
                </span>
              </div>

              <table className="track-table">
                <thead>
                  <tr>
                    <th style={{ width: "42%" }}>Track / Filename</th>
                    <th>Source & Quality</th>
                    <th>Format & Size</th>
                    <th style={{ textAlign: "right" }}>Action</th>
                  </tr>
                </thead>
                <tbody>
                  {searchResults.map((res) => {
                    const isStarting = downloadingIds.has(res.id);
                    const isDirect = res.provider === "yt-dlp" || res.id.startsWith("ytdlp_");
                    const task = findTaskForResult(res);

                    return (
                      <tr key={res.id} className="track-row">
                        <td className="primary" style={{ maxWidth: "300px" }}>
                          <div
                            style={{
                              whiteSpace: "nowrap",
                              overflow: "hidden",
                              textOverflow: "ellipsis",
                              fontSize: "0.88rem",
                            }}
                            title={res.filename}
                          >
                            {res.filename}
                          </div>
                        </td>
                        <td>
                          {isDirect ? (
                            <div>
                              <div
                                style={{
                                  display: "flex",
                                  alignItems: "center",
                                  gap: "6px",
                                  color: "#10b981",
                                  fontWeight: 600,
                                  fontSize: "0.8rem",
                                }}
                              >
                                <Zap size={13} color="#10b981" />
                                <span>Direct High-Speed</span>
                              </div>
                              <div
                                style={{
                                  fontSize: "0.72rem",
                                  color: "var(--text-dim)",
                                  whiteSpace: "nowrap",
                                  overflow: "hidden",
                                  textOverflow: "ellipsis",
                                  maxWidth: "200px",
                                  marginTop: "2px",
                                }}
                              >
                                {res.username}
                              </div>
                            </div>
                          ) : (
                            <div>
                              <div
                                style={{
                                  display: "flex",
                                  alignItems: "center",
                                  gap: "6px",
                                  fontSize: "0.8rem",
                                  color: "var(--text-muted)",
                                }}
                              >
                                <Users size={13} color="var(--text-dim)" />
                                <span>{res.username}</span>
                              </div>
                              <div
                                style={{
                                  fontSize: "0.72rem",
                                  color: "var(--text-dim)",
                                  display: "flex",
                                  alignItems: "center",
                                  gap: "4px",
                                  marginTop: "2px",
                                }}
                              >
                                <Zap size={11} color="#f59e0b" />
                                <span>{(res.speed_bps / 1024).toFixed(0)} KB/s</span>
                              </div>
                            </div>
                          )}
                        </td>
                        <td>
                          <div style={{ fontWeight: 600, color: "var(--text-main)", fontSize: "0.82rem" }}>
                            {res.format.toUpperCase()} {res.bitrate ? `${res.bitrate} kbps` : "320 kbps"}
                          </div>
                          <div style={{ fontSize: "0.72rem", color: "var(--text-dim)", marginTop: "2px" }}>
                            {formatBytes(res.file_size)}
                          </div>
                        </td>
                        <td style={{ textAlign: "right" }}>
                          {task?.status === "DOWNLOADING" ? (
                            <div style={{ display: "inline-flex", flexDirection: "column", alignItems: "flex-end", gap: "4px" }}>
                              <div
                                style={{
                                  display: "flex",
                                  alignItems: "center",
                                  gap: "6px",
                                  fontSize: "0.8rem",
                                  fontWeight: 600,
                                  color: "var(--accent-light)",
                                }}
                              >
                                <DownloadCloud size={14} />
                                <span>
                                  Downloading{" "}
                                  {task.file_size
                                    ? Math.min(100, Math.round((task.bytes_downloaded / task.file_size) * 100))
                                    : 0}
                                  %
                                </span>
                              </div>
                              <div style={{ width: "100px" }} className="progress-track">
                                <div
                                  className="progress-fill"
                                  style={{
                                    width: `${
                                      task.file_size
                                        ? Math.min(100, (task.bytes_downloaded / task.file_size) * 100)
                                        : 0
                                    }%`,
                                  }}
                                />
                              </div>
                              <button
                                onClick={() => setActiveTab("tasks")}
                                style={{
                                  background: "none",
                                  border: "none",
                                  color: "var(--text-dim)",
                                  fontSize: "0.72rem",
                                  cursor: "pointer",
                                  textDecoration: "underline",
                                  marginTop: "2px",
                                }}
                              >
                                View in Transfers
                              </button>
                            </div>
                          ) : task?.status === "COMPLETED" ? (
                            <div
                              style={{
                                display: "inline-flex",
                                alignItems: "center",
                                gap: "6px",
                                padding: "6px 12px",
                                borderRadius: "8px",
                                backgroundColor: "rgba(16, 185, 129, 0.15)",
                                color: "#10b981",
                                border: "1px solid rgba(16, 185, 129, 0.3)",
                                fontSize: "0.78rem",
                                fontWeight: 600,
                              }}
                            >
                              <CheckCircle size={14} />
                              <span>In Library</span>
                            </div>
                          ) : task?.status === "QUEUED" || isStarting ? (
                            <div
                              style={{
                                display: "inline-flex",
                                alignItems: "center",
                                gap: "6px",
                                padding: "6px 12px",
                                borderRadius: "8px",
                                backgroundColor: "rgba(245, 158, 11, 0.15)",
                                color: "#f59e0b",
                                border: "1px solid rgba(245, 158, 11, 0.3)",
                                fontSize: "0.78rem",
                                fontWeight: 600,
                              }}
                            >
                              <RefreshCw size={13} className="animate-spin" />
                              <span>Starting...</span>
                            </div>
                          ) : (
                            <button
                              onClick={() => handleDownloadClick(res)}
                              disabled={isStarting}
                              className="btn btn-primary"
                              style={{
                                fontSize: "0.8rem",
                                padding: "6px 14px",
                                display: "inline-flex",
                                alignItems: "center",
                                gap: "6px",
                              }}
                            >
                              <DownloadCloud size={14} />
                              <span>Download</span>
                            </button>
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}

      {/* TAB 2: DOWNLOAD TASKS & TRANSFERS */}
      {activeTab === "tasks" && (
        <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
          {/* Subfilter & Info */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              flexWrap: "wrap",
              gap: "12px",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
              {["ALL", "ACTIVE", "COMPLETED", "FAILED"].map((sub) => (
                <button
                  key={sub}
                  onClick={() => setStatusFilter(sub)}
                  className={`subtab-btn ${statusFilter === sub ? "active" : ""}`}
                >
                  {sub}
                </button>
              ))}
            </div>

            <span style={{ fontSize: "0.78rem", color: "var(--text-dim)" }}>
              Files auto-import into your local library upon 100% completion
            </span>
          </div>

          {filteredTasks.length === 0 ? (
            <div
              className="content-card"
              style={{
                textAlign: "center",
                padding: "60px 20px",
                borderStyle: "dashed",
              }}
            >
              <HardDrive size={48} color="var(--text-dim)" style={{ margin: "0 auto 14px auto" }} />
              <div style={{ fontSize: "1.1rem", fontWeight: 600, color: "var(--text-main)" }}>
                No transfers found
              </div>
              <p style={{ fontSize: "0.85rem", color: "var(--text-muted)", marginTop: "6px" }}>
                Search online or via Soulseek to find and download music directly to your library!
              </p>
              <button
                onClick={() => setActiveTab("search")}
                className="btn btn-primary"
                style={{ marginTop: "18px", fontSize: "0.85rem", display: "inline-flex", alignItems: "center", gap: "8px" }}
              >
                <Search size={15} />
                <span>Search & Download Now</span>
              </button>
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
              {filteredTasks.map((task) => {
                const total = task.file_size || 0;
                const progress = total > 0 ? Math.min(100, (task.bytes_downloaded / total) * 100) : 0;
                const isDirect = task.provider === "yt-dlp" || task.provider === "composite";

                return (
                  <div
                    key={task.id}
                    className="content-card"
                    style={{
                      padding: "16px 20px",
                      marginBottom: 0,
                      display: "flex",
                      flexDirection: "column",
                      gap: "12px",
                    }}
                  >
                    <div
                      style={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        flexWrap: "wrap",
                        gap: "10px",
                      }}
                    >
                      <div>
                        <div style={{ fontWeight: 600, color: "var(--text-main)", fontSize: "0.95rem", display: "flex", alignItems: "center", gap: "8px" }}>
                          <span>{task.title || task.filename}</span>
                          <span style={{ fontSize: "0.8rem", fontWeight: 400, color: "var(--text-dim)" }}>
                            by {task.artist || "Unknown Artist"}
                          </span>
                          {isDirect && (
                            <span
                              style={{
                                display: "inline-flex",
                                alignItems: "center",
                                gap: "4px",
                                fontSize: "0.7rem",
                                fontWeight: 700,
                                color: "#10b981",
                                backgroundColor: "rgba(16, 185, 129, 0.12)",
                                padding: "2px 8px",
                                borderRadius: "9999px",
                                border: "1px solid rgba(16, 185, 129, 0.25)",
                              }}
                            >
                              <Zap size={10} color="#10b981" /> Direct In-App
                            </span>
                          )}
                        </div>
                        <div
                          style={{
                            fontSize: "0.78rem",
                            color: "var(--text-dim)",
                            marginTop: "4px",
                            whiteSpace: "nowrap",
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            maxWidth: "500px",
                          }}
                        >
                          {task.destination_path || task.filename}
                        </div>
                      </div>

                      <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                        {getStatusBadge(task.status, isDirect)}

                        {(task.status === "DOWNLOADING" || task.status === "QUEUED") && (
                          <button
                            onClick={() => onCancelDownload(task.id)}
                            style={{
                              padding: "4px 10px",
                              fontSize: "0.75rem",
                              fontWeight: 600,
                              borderRadius: "6px",
                              backgroundColor: "rgba(239, 68, 68, 0.12)",
                              color: "#ef4444",
                              border: "1px solid rgba(239, 68, 68, 0.3)",
                              cursor: "pointer",
                            }}
                          >
                            Cancel
                          </button>
                        )}
                      </div>
                    </div>

                    {/* Progress Track */}
                    <div>
                      <div
                        style={{
                          display: "flex",
                          justifyContent: "space-between",
                          fontSize: "0.75rem",
                          color: "var(--text-dim)",
                          fontFamily: "monospace",
                          marginBottom: "4px",
                        }}
                      >
                        <span>
                          {formatBytes(task.bytes_downloaded)} / {formatBytes(total)}
                        </span>
                        <span style={{ color: "var(--text-main)", fontWeight: 600 }}>
                          {progress.toFixed(1)}%
                        </span>
                      </div>
                      <div className="progress-track">
                        <div
                          className="progress-fill"
                          style={{
                            width: `${progress}%`,
                            backgroundColor:
                              task.status === "COMPLETED"
                                ? "var(--success)"
                                : task.status === "FAILED"
                                ? "var(--danger)"
                                : "var(--accent)",
                          }}
                        />
                      </div>
                    </div>

                    {task.error_message && (
                      <div
                        style={{
                          fontSize: "0.78rem",
                          color: "#ef4444",
                          backgroundColor: "rgba(239, 68, 68, 0.1)",
                          padding: "8px 12px",
                          borderRadius: "6px",
                          border: "1px solid rgba(239, 68, 68, 0.2)",
                        }}
                      >
                        {task.error_message}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
