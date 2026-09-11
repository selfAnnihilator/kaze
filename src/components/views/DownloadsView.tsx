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
  const [statusFilter, setStatusFilter] = useState<string>("ALL");
  const [downloadingIds, setDownloadingIds] = useState<Set<string>>(new Set());

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
    if (!artist.trim() && !title.trim()) return;
    setSearching(true);
    try {
      const results = await onSearchSoulseek(artist.trim(), title.trim(), album?.trim() || undefined);
      setSearchResults(results);
    } catch (err) {
      console.error("Failed to search Soulseek:", err);
    } finally {
      setSearching(false);
    }
  };

  const handleDownloadClick = async (resultId: string) => {
    setDownloadingIds((prev) => new Set(prev).add(resultId));
    try {
      await onStartDownload(resultId);
      setActiveTab("tasks");
    } finally {
      setDownloadingIds((prev) => {
        const next = new Set(prev);
        next.delete(resultId);
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

  const getStatusBadge = (status: DownloadTask["status"]) => {
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
            <DownloadCloud size={12} /> Downloading
          </span>
        );
      case "COMPLETED":
        return (
          <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium bg-emerald-500/15 text-emerald-400 border border-emerald-500/30">
            <CheckCircle size={12} /> Completed
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

  return (
    <div className="p-8 space-y-6">
      {/* Top Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold flex items-center gap-3">
            <DownloadCloud className="text-cyan-400" /> Soulseek P2P Downloads
          </h1>
          <p className="text-zinc-400 text-sm mt-1">
            Search peer-to-peer files, monitor transfers, and automatically import completed tracks to your local library.
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
      <div className="flex items-center gap-2 border-b border-zinc-800 pb-2">
        <button
          onClick={() => setActiveTab("tasks")}
          className={`px-4 py-2 rounded-lg text-sm font-semibold transition ${
            activeTab === "tasks"
              ? "bg-zinc-800 text-white shadow"
              : "text-zinc-400 hover:text-white"
          }`}
        >
          Transfers ({downloads.length})
        </button>
        <button
          onClick={() => setActiveTab("search")}
          className={`px-4 py-2 rounded-lg text-sm font-semibold transition flex items-center gap-1.5 ${
            activeTab === "search"
              ? "bg-zinc-800 text-white shadow"
              : "text-zinc-400 hover:text-white"
          }`}
        >
          <Search size={14} /> Search Network
        </button>
      </div>

      {/* TAB 1: DOWNLOAD TASKS */}
      {activeTab === "tasks" && (
        <div className="space-y-4">
          {/* Subfilter */}
          <div className="flex items-center gap-2">
            {["ALL", "ACTIVE", "COMPLETED", "FAILED"].map((sub) => (
              <button
                key={sub}
                onClick={() => setStatusFilter(sub)}
                className={`px-3 py-1 rounded-md text-xs font-semibold uppercase transition ${
                  statusFilter === sub
                    ? "bg-zinc-200 text-black"
                    : "bg-zinc-900 text-zinc-400 hover:text-white border border-zinc-800"
                }`}
              >
                {sub}
              </button>
            ))}
          </div>

          {filteredTasks.length === 0 ? (
            <div className="text-center py-16 bg-zinc-900/40 border border-zinc-800/50 rounded-2xl">
              <HardDrive size={48} className="mx-auto text-zinc-600 mb-3" />
              <p className="text-zinc-400 font-medium text-lg">No transfers found</p>
              <p className="text-zinc-600 text-sm mt-1">
                Search Soulseek to find and download music directly to your library!
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {filteredTasks.map((task) => {
                const total = task.file_size || 0;
                const progress = total > 0 ? Math.min(100, (task.bytes_downloaded / total) * 100) : 0;

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
                        </div>
                        <div className="text-xs text-zinc-400 mt-0.5 truncate max-w-xl">
                          {task.filename}
                        </div>
                      </div>

                      <div className="flex items-center gap-3">
                        {getStatusBadge(task.status)}

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
                        <span>{progress.toFixed(1)}%</span>
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

      {/* TAB 2: SOULSEEK NETWORK SEARCH */}
      {activeTab === "search" && (
        <div className="space-y-6">
          {/* Search Form */}
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
                className="flex items-center gap-2 px-5 py-2 bg-cyan-500 hover:bg-cyan-400 text-black font-semibold text-sm rounded-lg transition disabled:opacity-50"
              >
                {searching ? <RefreshCw className="animate-spin" size={16} /> : <Search size={16} />}
                <span>{searching ? "Searching Soulseek..." : "Search P2P Network"}</span>
              </button>
            </div>
          </form>

          {/* Results */}
          {searchResults.length > 0 && (
            <div className="bg-zinc-900/60 border border-zinc-800 rounded-xl overflow-hidden">
              <table className="w-full text-left border-collapse text-sm">
                <thead>
                  <tr className="border-b border-zinc-800 text-zinc-500 text-xs font-semibold uppercase">
                    <th className="py-3 px-4">Filename</th>
                    <th className="py-3 px-4 hidden md:table-cell">User / Speed</th>
                    <th className="py-3 px-4">Quality & Size</th>
                    <th className="py-3 px-4 text-right">Action</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-zinc-800/60">
                  {searchResults.map((res) => {
                    const isStarting = downloadingIds.has(res.id);
                    return (
                      <tr key={res.id} className="hover:bg-zinc-800/40 transition">
                        <td className="py-3 px-4 max-w-sm sm:max-w-md truncate">
                          <div className="font-mono text-xs text-zinc-200 truncate" title={res.filename}>
                            {res.filename}
                          </div>
                        </td>
                        <td className="py-3 px-4 hidden md:table-cell text-xs text-zinc-400">
                          <div className="flex items-center gap-1.5">
                            <Users size={12} className="text-zinc-500" />
                            <span>{res.username}</span>
                          </div>
                          <div className="flex items-center gap-1 text-zinc-500">
                            <Zap size={11} className="text-amber-500" />
                            <span>{(res.speed_bps / 1024).toFixed(0)} KB/s</span>
                          </div>
                        </td>
                        <td className="py-3 px-4 text-xs">
                          <div className="font-semibold text-zinc-300">
                            {res.format.toUpperCase()} {res.bitrate ? `${res.bitrate} kbps` : ""}
                          </div>
                          <div className="text-zinc-500">{formatBytes(res.file_size)}</div>
                        </td>
                        <td className="py-3 px-4 text-right">
                          <button
                            onClick={() => handleDownloadClick(res.id)}
                            disabled={isStarting}
                            className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-cyan-500/10 hover:bg-cyan-500/20 text-cyan-400 text-xs font-semibold border border-cyan-500/30 transition disabled:opacity-50"
                          >
                            <DownloadCloud size={14} />
                            <span>{isStarting ? "Queuing..." : "Download"}</span>
                          </button>
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
    </div>
  );
};
