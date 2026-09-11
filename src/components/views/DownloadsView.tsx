import React, { useState } from "react";
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
  const [activeTab, setActiveTab] = useState<"tasks" | "search">("tasks");
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
  React.useEffect(() => {
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
        subtitle: `"${result.filename}" is now downloading directly. Auto-importing to library upon completion.`,
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
    switch (status) {
      case "QUEUED":
        return (
          <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium bg-amber-500/15 text-amber-400 border border-amber-500/30">
            <Clock size={12} /> Queued
          </span>
        );
      case "DOWNLOADING":
        return (
          <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium bg-cyan-500/15 text-cyan-400 border border-cyan-500/30 animate-pulse">
            <DownloadCloud size={12} /> {isDirect ? "Direct Downloading" : "Transferring"}
          </span>
        );
      case "COMPLETED":
        return (
          <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium bg-emerald-500/15 text-emerald-400 border border-emerald-500/30">
            <CheckCircle size={12} /> In Library
          </span>
        );
      case "FAILED":
        return (
          <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium bg-rose-500/15 text-rose-400 border border-rose-500/30">
            <AlertCircle size={12} /> Failed
          </span>
        );
      case "CANCELLED":
        return (
          <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium bg-zinc-600/15 text-zinc-400 border border-zinc-600/30">
            <XCircle size={12} /> Cancelled
          </span>
        );
    }
  };

  // Helper matching a search result to any active or completed download task
  const findTaskForResult = (res: DownloadSearchResult): DownloadTask | undefined => {
    return downloads.find(
      (t) => t.filename === res.filename || (res.id.startsWith("ytdlp_") && t.filename.includes(res.username))
    );
  };

  return (
    <div className="p-8 space-y-6">
      {/* Top Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold flex items-center gap-3">
            <DownloadCloud className="text-cyan-400" /> Music Downloads
          </h1>
          <p className="text-zinc-400 text-sm mt-1">
            Search and download songs directly into your library with 1-click in 320kbps MP3, or connect with SoulseekQt.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => onLaunchSoulseek()}
            className="btn btn-primary"
            style={{ display: "flex", alignItems: "center", gap: "6px" }}
          >
            <ExternalLink size={14} />
            <span>Open SoulseekQt</span>
          </button>
          <button
            onClick={() => onImportSoulseek()}
            className="btn btn-secondary"
            style={{ display: "flex", alignItems: "center", gap: "6px" }}
            title="Scan ~/Soulseek Downloads/complete and import into library"
          >
            <RefreshCw size={14} />
            <span>Import Downloads</span>
          </button>
          <button
            onClick={() => onRefreshDownloads()}
            className="p-2 text-zinc-400 hover:text-white bg-zinc-800 hover:bg-zinc-700 rounded-lg transition"
            title="Refresh downloads"
          >
            <RefreshCw size={16} />
          </button>
        </div>
      </div>

      {/* Main Tabs */}
      <div className="flex items-center gap-3 border-b border-zinc-800 pb-2">
        <button
          onClick={() => setActiveTab("search")}
          className={`px-4 py-2 rounded-lg text-sm font-semibold transition flex items-center gap-2 ${
            activeTab === "search"
              ? "bg-zinc-800 text-white shadow border border-zinc-700"
              : "text-zinc-400 hover:text-white"
          }`}
        >
          <Search size={15} />
          <span>Search & Download</span>
        </button>

        <button
          onClick={() => setActiveTab("tasks")}
          className={`px-4 py-2 rounded-lg text-sm font-semibold transition flex items-center gap-2 ${
            activeTab === "tasks"
              ? "bg-zinc-800 text-white shadow border border-zinc-700"
              : "text-zinc-400 hover:text-white"
          }`}
        >
          <HardDrive size={15} />
          <span>Transfers ({downloads.length})</span>
          {activeDownloads.length > 0 && (
            <span className="px-2 py-0.5 rounded-full text-xs font-bold bg-cyan-500 text-black animate-pulse">
              {activeDownloads.length} active
            </span>
          )}
        </button>
      </div>

      {/* Notification Banner */}
      {feedbackBanner && (
        <div className="p-4 rounded-xl bg-cyan-950/40 border border-cyan-500/40 flex items-center justify-between gap-3 shadow-lg">
          <div className="flex items-center gap-3">
            <CheckCircle className="text-cyan-400 shrink-0" size={20} />
            <div>
              <div className="text-sm font-semibold text-white">{feedbackBanner.title}</div>
              {feedbackBanner.subtitle && (
                <div className="text-xs text-zinc-300 mt-0.5">{feedbackBanner.subtitle}</div>
              )}
            </div>
          </div>
          <button
            onClick={() => setActiveTab("tasks")}
            className="px-3 py-1.5 rounded-lg bg-cyan-500 hover:bg-cyan-400 text-black text-xs font-bold transition shrink-0 flex items-center gap-1.5"
          >
            <span>View in Transfers</span>
            <ArrowRight size={13} />
          </button>
        </div>
      )}

      {/* TAB 1: SEARCH & DIRECT DOWNLOAD */}
      {activeTab === "search" && (
        <div className="space-y-6">
          {/* Visual Progress Pipeline Tracker */}
          <div className="bg-zinc-900/80 border border-zinc-800 rounded-xl p-4">
            <div className="text-xs font-semibold uppercase tracking-wider text-zinc-400 mb-3 flex items-center gap-2">
              <Zap size={14} className="text-cyan-400" />
              <span>Direct In-App Music Acquisition Pipeline</span>
            </div>
            <div className="grid grid-cols-1 sm:grid-cols-4 gap-3">
              {/* Step 1 */}
              <div
                className={`p-3 rounded-lg border flex items-center gap-3 transition ${
                  searching
                    ? "bg-cyan-500/10 border-cyan-500/40 text-cyan-300 shadow-sm animate-pulse"
                    : searchStatus === "succeeded"
                    ? "bg-emerald-500/10 border-emerald-500/30 text-emerald-300"
                    : "bg-zinc-900 border-zinc-800 text-zinc-400"
                }`}
              >
                <div className="shrink-0">
                  {searching ? (
                    <RefreshCw className="animate-spin text-cyan-400" size={18} />
                  ) : searchStatus === "succeeded" ? (
                    <Check className="text-emerald-400" size={18} />
                  ) : (
                    <Search size={18} />
                  )}
                </div>
                <div>
                  <div className="text-xs font-bold">1. Online Search</div>
                  <div className="text-[11px] opacity-80">
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
                className={`p-3 rounded-lg border flex items-center gap-3 transition ${
                  searchResults.length > 0
                    ? "bg-emerald-500/10 border-emerald-500/30 text-emerald-300"
                    : "bg-zinc-900 border-zinc-800 text-zinc-400"
                }`}
              >
                <div className="shrink-0">
                  {searchResults.length > 0 ? (
                    <Check className="text-emerald-400" size={18} />
                  ) : (
                    <Music size={18} />
                  )}
                </div>
                <div>
                  <div className="text-xs font-bold">2. Stream Quality</div>
                  <div className="text-[11px] opacity-80">
                    {searchResults.length > 0 ? "320 kbps MP3 Ready" : "High bitrate audio"}
                  </div>
                </div>
              </div>

              {/* Step 3 */}
              <div
                className={`p-3 rounded-lg border flex items-center gap-3 transition ${
                  activeDownloads.length > 0
                    ? "bg-cyan-500/10 border-cyan-500/40 text-cyan-300 animate-pulse"
                    : downloads.some((t) => t.status === "COMPLETED")
                    ? "bg-emerald-500/10 border-emerald-500/30 text-emerald-300"
                    : "bg-zinc-900 border-zinc-800 text-zinc-400"
                }`}
              >
                <div className="shrink-0">
                  {activeDownloads.length > 0 ? (
                    <DownloadCloud className="text-cyan-400 animate-bounce" size={18} />
                  ) : downloads.some((t) => t.status === "COMPLETED") ? (
                    <Check className="text-emerald-400" size={18} />
                  ) : (
                    <Zap size={18} />
                  )}
                </div>
                <div>
                  <div className="text-xs font-bold">3. Direct Download</div>
                  <div className="text-[11px] opacity-80">
                    {activeDownloads.length > 0
                      ? `${activeDownloads.length} transfer(s) active`
                      : "Native background transfer"}
                  </div>
                </div>
              </div>

              {/* Step 4 */}
              <div
                className={`p-3 rounded-lg border flex items-center gap-3 transition ${
                  downloads.some((t) => t.status === "COMPLETED")
                    ? "bg-emerald-500/10 border-emerald-500/30 text-emerald-300"
                    : "bg-zinc-900 border-zinc-800 text-zinc-400"
                }`}
              >
                <div className="shrink-0">
                  {downloads.some((t) => t.status === "COMPLETED") ? (
                    <Check className="text-emerald-400" size={18} />
                  ) : (
                    <CheckCircle size={18} />
                  )}
                </div>
                <div>
                  <div className="text-xs font-bold">4. Auto-Import</div>
                  <div className="text-[11px] opacity-80">
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
            className="bg-zinc-900 border border-zinc-800 rounded-xl p-5 shadow space-y-4"
          >
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div>
                <label className="block text-xs font-semibold text-zinc-400 uppercase mb-1">
                  Artist
                </label>
                <input
                  type="text"
                  value={searchArtist}
                  onChange={(e) => setSearchArtist(e.target.value)}
                  placeholder="e.g. Daft Punk"
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm focus:outline-none focus:border-cyan-500"
                />
              </div>
              <div>
                <label className="block text-xs font-semibold text-zinc-400 uppercase mb-1">
                  Title
                </label>
                <input
                  type="text"
                  value={searchTitle}
                  onChange={(e) => setSearchTitle(e.target.value)}
                  placeholder="e.g. One More Time"
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm focus:outline-none focus:border-cyan-500"
                />
              </div>
              <div>
                <label className="block text-xs font-semibold text-zinc-400 uppercase mb-1">
                  Album (Optional)
                </label>
                <input
                  type="text"
                  value={searchAlbum}
                  onChange={(e) => setSearchAlbum(e.target.value)}
                  placeholder="e.g. Discovery"
                  className="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm focus:outline-none focus:border-cyan-500"
                />
              </div>
            </div>

            <div className="flex justify-end gap-3">
              <button
                type="button"
                onClick={() => {
                  const title = searchTitle.trim();
                  const artist = searchArtist.trim();
                  const query = title || artist;
                  const filter = title && artist ? artist : undefined;
                  onLaunchSoulseek(query || undefined, filter);
                }}
                className="flex items-center gap-2 px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-white font-medium text-sm rounded-lg transition"
                title="Search track in SoulseekQt and filter results by artist name"
              >
                <ExternalLink size={15} />
                <span>Search in SoulseekQt</span>
              </button>

              <button
                type="submit"
                disabled={searching || (!searchArtist.trim() && !searchTitle.trim())}
                className="flex items-center gap-2 px-5 py-2 bg-cyan-500 hover:bg-cyan-400 text-black font-semibold text-sm rounded-lg transition disabled:opacity-50 shadow"
              >
                {searching ? <RefreshCw className="animate-spin" size={16} /> : <Search size={16} />}
                <span>{searching ? "Searching Sources..." : "Search & Download Directly"}</span>
              </button>
            </div>
          </form>

          {/* Search Status Banners */}
          {searchStatus === "searching" && (
            <div className="p-4 rounded-xl bg-cyan-950/30 border border-cyan-800/40 flex items-center gap-3">
              <RefreshCw className="animate-spin text-cyan-400 shrink-0" size={20} />
              <div>
                <div className="text-sm font-semibold text-cyan-200">
                  Searching high-speed audio sources...
                </div>
                <div className="text-xs text-cyan-400/80 mt-0.5">
                  Querying for "{lastQuery}". Searching 320kbps MP3 audio streams...
                </div>
              </div>
            </div>
          )}

          {searchStatus === "succeeded" && (
            <div className="p-3.5 rounded-xl bg-emerald-950/30 border border-emerald-800/40 flex items-center justify-between gap-3">
              <div className="flex items-center gap-2.5">
                <CheckCircle className="text-emerald-400 shrink-0" size={18} />
                <div>
                  <span className="text-sm font-semibold text-emerald-200">Search Succeeded!</span>
                  <span className="text-xs text-emerald-400/80 ml-2">
                    Found {searchResults.length} direct high-speed audio stream(s) for "{lastQuery}". Click Download below.
                  </span>
                </div>
              </div>
              <span className="text-[11px] font-semibold text-emerald-400 bg-emerald-900/40 px-2.5 py-1 rounded-full border border-emerald-800/50">
                Direct In-App
              </span>
            </div>
          )}

          {searchStatus === "no_results" && (
            <div className="p-4 rounded-xl bg-amber-950/30 border border-amber-800/40 flex items-center justify-between gap-3">
              <div className="flex items-center gap-2.5">
                <AlertCircle className="text-amber-400 shrink-0" size={18} />
                <div>
                  <div className="text-sm font-semibold text-amber-200">No Direct Audio Streams Found</div>
                  <div className="text-xs text-amber-400/80 mt-0.5">
                    Could not find direct streams for "{lastQuery}". Try alternate spelling or search via SoulseekQt.
                  </div>
                </div>
              </div>
              <button
                onClick={() => {
                  const title = searchTitle.trim();
                  const artist = searchArtist.trim();
                  onLaunchSoulseek(title || artist || undefined, title && artist ? artist : undefined);
                }}
                className="px-3 py-1.5 bg-amber-500/20 hover:bg-amber-500/30 text-amber-300 text-xs font-semibold rounded-lg border border-amber-500/30 transition flex items-center gap-1.5 shrink-0"
              >
                <ExternalLink size={13} />
                <span>Search SoulseekQt</span>
              </button>
            </div>
          )}

          {/* Active Direct Downloads Section on Search Tab */}
          {activeDownloads.length > 0 && (
            <div className="bg-zinc-900/90 border border-cyan-500/40 rounded-xl p-4 shadow-lg space-y-3">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="relative flex h-2.5 w-2.5">
                    <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-cyan-400 opacity-75"></span>
                    <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-cyan-500"></span>
                  </span>
                  <span className="text-xs font-bold text-cyan-400 uppercase tracking-wide">
                    Direct Downloads Active ({activeDownloads.length})
                  </span>
                </div>
                <button
                  onClick={() => setActiveTab("tasks")}
                  className="text-xs text-cyan-400 hover:text-cyan-300 font-semibold flex items-center gap-1"
                >
                  <span>View in Transfers</span>
                  <ArrowRight size={13} />
                </button>
              </div>

              <div className="space-y-2">
                {activeDownloads.map((task) => {
                  const total = task.file_size || 0;
                  const progress = total > 0 ? Math.min(100, (task.bytes_downloaded / total) * 100) : 0;
                  return (
                    <div
                      key={task.id}
                      className="p-3 bg-zinc-800/60 rounded-lg border border-zinc-700/50 space-y-1.5"
                    >
                      <div className="flex items-center justify-between text-xs">
                        <div className="font-semibold text-zinc-200 truncate max-w-md">
                          {task.title || task.filename}
                        </div>
                        <div className="text-cyan-400 font-mono font-semibold">
                          {progress > 0 ? `${progress.toFixed(1)}%` : "Connecting..."}
                        </div>
                      </div>
                      <div className="w-full bg-zinc-700/50 rounded-full h-1.5 overflow-hidden">
                        <div
                          className="h-full bg-cyan-500 transition-all duration-300"
                          style={{ width: `${progress}%` }}
                        />
                      </div>
                      <div className="flex justify-between text-[11px] text-zinc-400">
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
            <div className="bg-zinc-900/60 border border-zinc-800 rounded-xl overflow-hidden shadow">
              <div className="px-4 py-3 border-b border-zinc-800 flex items-center justify-between">
                <span className="text-xs font-semibold uppercase tracking-wider text-zinc-400">
                  Found {searchResults.length} Audio Streams
                </span>
                <span className="text-xs text-zinc-500">
                  Downloads save directly to your local library folder
                </span>
              </div>
              <table className="w-full text-left border-collapse text-sm">
                <thead>
                  <tr className="border-b border-zinc-800 text-zinc-500 text-xs font-semibold uppercase">
                    <th className="py-3 px-4">Track / Filename</th>
                    <th className="py-3 px-4 hidden md:table-cell">Source / Quality</th>
                    <th className="py-3 px-4">Audio Format & Size</th>
                    <th className="py-3 px-4 text-right">Download Action</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-zinc-800/60">
                  {searchResults.map((res) => {
                    const isStarting = downloadingIds.has(res.id);
                    const isDirect = res.provider === "yt-dlp" || res.id.startsWith("ytdlp_");
                    const task = findTaskForResult(res);

                    return (
                      <tr key={res.id} className="hover:bg-zinc-800/40 transition">
                        <td className="py-3.5 px-4 max-w-sm sm:max-w-md truncate">
                          <div className="font-medium text-xs text-zinc-200 truncate" title={res.filename}>
                            {res.filename}
                          </div>
                        </td>
                        <td className="py-3.5 px-4 hidden md:table-cell text-xs text-zinc-400">
                          {isDirect ? (
                            <div className="space-y-0.5">
                              <div className="flex items-center gap-1.5 text-emerald-400 font-semibold">
                                <Zap size={12} className="text-emerald-400 fill-emerald-400" />
                                <span>Direct In-App High-Speed</span>
                              </div>
                              <div className="text-zinc-500 text-[11px] truncate max-w-xs">
                                {res.username}
                              </div>
                            </div>
                          ) : (
                            <div className="space-y-0.5">
                              <div className="flex items-center gap-1.5">
                                <Users size={12} className="text-zinc-500" />
                                <span>{res.username}</span>
                              </div>
                              <div className="flex items-center gap-1 text-zinc-500 text-[11px]">
                                <Zap size={11} className="text-amber-500" />
                                <span>{(res.speed_bps / 1024).toFixed(0)} KB/s</span>
                              </div>
                            </div>
                          )}
                        </td>
                        <td className="py-3.5 px-4 text-xs">
                          <div className="font-semibold text-zinc-300">
                            {res.format.toUpperCase()} {res.bitrate ? `${res.bitrate} kbps` : "320 kbps"}
                          </div>
                          <div className="text-zinc-500">{formatBytes(res.file_size)}</div>
                        </td>
                        <td className="py-3.5 px-4 text-right">
                          {task?.status === "DOWNLOADING" ? (
                            <div className="inline-flex flex-col items-end gap-1">
                              <div className="flex items-center gap-1 text-xs font-semibold text-cyan-400">
                                <DownloadCloud size={13} className="animate-bounce" />
                                <span>
                                  Downloading{" "}
                                  {task.file_size
                                    ? Math.min(100, Math.round((task.bytes_downloaded / task.file_size) * 100))
                                    : 0}
                                  %
                                </span>
                              </div>
                              <div className="w-24 bg-zinc-800 rounded-full h-1.5 overflow-hidden">
                                <div
                                  className="h-full bg-cyan-500 transition-all duration-300"
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
                                className="text-[10px] text-zinc-400 hover:text-cyan-300 underline"
                              >
                                View in Transfers
                              </button>
                            </div>
                          ) : task?.status === "COMPLETED" ? (
                            <div className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 text-xs font-semibold">
                              <CheckCircle size={13} />
                              <span>In Library</span>
                            </div>
                          ) : task?.status === "QUEUED" || isStarting ? (
                            <div className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-amber-500/15 text-amber-400 border border-amber-500/30 text-xs font-semibold">
                              <RefreshCw size={13} className="animate-spin" />
                              <span>Starting...</span>
                            </div>
                          ) : (
                            <button
                              onClick={() => handleDownloadClick(res)}
                              disabled={isStarting}
                              className="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg bg-cyan-500 hover:bg-cyan-400 text-black text-xs font-semibold transition disabled:opacity-50 shadow-sm"
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
        <div className="space-y-4">
          {/* Subfilter */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              {["ALL", "ACTIVE", "COMPLETED", "FAILED"].map((sub) => (
                <button
                  key={sub}
                  onClick={() => setStatusFilter(sub)}
                  className={`px-3 py-1 rounded-md text-xs font-semibold uppercase transition ${
                    statusFilter === sub
                      ? "bg-zinc-200 text-black font-bold"
                      : "bg-zinc-900 text-zinc-400 hover:text-white border border-zinc-800"
                  }`}
                >
                  {sub}
                </button>
              ))}
            </div>

            <span className="text-xs text-zinc-500">
              Files auto-import into your local library upon 100% completion
            </span>
          </div>

          {filteredTasks.length === 0 ? (
            <div className="text-center py-16 bg-zinc-900/40 border border-zinc-800/50 rounded-2xl">
              <HardDrive size={48} className="mx-auto text-zinc-600 mb-3" />
              <p className="text-zinc-400 font-medium text-lg">No transfers found</p>
              <p className="text-zinc-600 text-sm mt-1">
                Search online or via Soulseek to find and download music directly to your library!
              </p>
              <button
                onClick={() => setActiveTab("search")}
                className="mt-4 inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-cyan-500 text-black text-xs font-bold hover:bg-cyan-400 transition"
              >
                <Search size={14} />
                <span>Search & Download Now</span>
              </button>
            </div>
          ) : (
            <div className="space-y-3">
              {filteredTasks.map((task) => {
                const total = task.file_size || 0;
                const progress = total > 0 ? Math.min(100, (task.bytes_downloaded / total) * 100) : 0;
                const isDirect = task.provider === "yt-dlp" || task.provider === "composite";

                return (
                  <div
                    key={task.id}
                    className="p-4 bg-zinc-900/70 border border-zinc-800 rounded-xl hover:border-zinc-700 transition space-y-3"
                  >
                    <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
                      <div>
                        <div className="font-semibold text-zinc-100 flex items-center gap-2">
                          <span>{task.title || task.filename}</span>
                          <span className="text-xs font-normal text-zinc-500">
                            by {task.artist || "Unknown"}
                          </span>
                          {isDirect && (
                            <span className="inline-flex items-center gap-1 text-[10px] font-semibold text-emerald-400 bg-emerald-950/40 px-2 py-0.5 rounded-full border border-emerald-800/40">
                              <Zap size={10} className="fill-emerald-400" /> Direct In-App
                            </span>
                          )}
                        </div>
                        <div className="text-xs text-zinc-400 mt-0.5 truncate max-w-xl">
                          {task.destination_path || task.filename}
                        </div>
                      </div>

                      <div className="flex items-center gap-3">
                        {getStatusBadge(task.status, isDirect)}

                        {(task.status === "DOWNLOADING" || task.status === "QUEUED") && (
                          <button
                            onClick={() => onCancelDownload(task.id)}
                            className="px-2.5 py-1 text-xs text-rose-400 hover:text-rose-300 bg-rose-500/10 hover:bg-rose-500/20 rounded-md border border-rose-500/20 transition"
                          >
                            Cancel
                          </button>
                        )}
                      </div>
                    </div>

                    {/* Progress Bar */}
                    <div className="space-y-1">
                      <div className="flex justify-between text-xs text-zinc-500 font-mono">
                        <span>
                          {formatBytes(task.bytes_downloaded)} / {formatBytes(total)}
                        </span>
                        <span className="text-zinc-300 font-semibold">{progress.toFixed(1)}%</span>
                      </div>
                      <div className="w-full bg-zinc-800 rounded-full h-1.5 overflow-hidden">
                        <div
                          className={`h-full transition-all duration-300 ${
                            task.status === "COMPLETED"
                              ? "bg-emerald-500"
                              : task.status === "FAILED"
                              ? "bg-rose-500"
                              : "bg-cyan-500"
                          }`}
                          style={{ width: `${progress}%` }}
                        />
                      </div>
                    </div>

                    {task.error_message && (
                      <div className="text-xs text-rose-400 bg-rose-500/10 px-3 py-1.5 rounded-lg border border-rose-500/20">
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
