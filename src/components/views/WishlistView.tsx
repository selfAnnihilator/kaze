import React, { useState } from "react";
import { WishlistItem } from "../../types";
import {
  Bookmark,
  Search,
  Plus,
  CloudDownload,
  CheckCircle2,
  XCircle,
  Clock,
} from "lucide-react";

interface WishlistViewProps {
  wishlist: WishlistItem[];
  onAddToWishlist: (title: string, artist: string, album?: string) => Promise<void>;
  onUpdateStatus: (
    id: string,
    status: "want" | "ignore" | "already_own" | "downloaded"
  ) => Promise<void>;
  onSearchSoulseek: (artist: string, title: string, album?: string) => void;
}

export const WishlistView: React.FC<WishlistViewProps> = ({
  wishlist,
  onAddToWishlist,
  onUpdateStatus,
  onSearchSoulseek,
}) => {
  const [filter, setFilter] = useState<string>("ALL");
  const [searchTerm, setSearchTerm] = useState("");
  const [showAddForm, setShowAddForm] = useState(false);
  const [newTitle, setNewTitle] = useState("");
  const [newArtist, setNewArtist] = useState("");
  const [newAlbum, setNewAlbum] = useState("");
  const [submitting, setSubmitting] = useState(false);

  const filteredItems = wishlist.filter((item) => {
    const matchesFilter =
      filter === "ALL" ? true : item.status.toUpperCase() === filter.toUpperCase();
    const matchesSearch =
      item.title.toLowerCase().includes(searchTerm.toLowerCase()) ||
      item.artist.toLowerCase().includes(searchTerm.toLowerCase()) ||
      (item.album && item.album.toLowerCase().includes(searchTerm.toLowerCase()));
    return matchesFilter && matchesSearch;
  });

  const handleAddSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTitle.trim() || !newArtist.trim()) return;

    setSubmitting(true);
    try {
      await onAddToWishlist(newTitle.trim(), newArtist.trim(), newAlbum.trim() || undefined);
      setNewTitle("");
      setNewArtist("");
      setNewAlbum("");
      setShowAddForm(false);
    } finally {
      setSubmitting(false);
    }
  };

  const getStatusBadge = (status: WishlistItem["status"]) => {
    switch (status) {
      case "WANT":
        return (
          <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium bg-amber-500/15 text-amber-400 border border-amber-500/30">
            <Clock size={12} /> Want
          </span>
        );
      case "DOWNLOADED":
        return (
          <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium bg-emerald-500/15 text-emerald-400 border border-emerald-500/30">
            <CheckCircle2 size={12} /> Downloaded
          </span>
        );
      case "ALREADY_OWN":
        return (
          <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-500/15 text-blue-400 border border-blue-500/30">
            <CheckCircle2 size={12} /> Owned
          </span>
        );
      case "IGNORE":
        return (
          <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium bg-zinc-500/15 text-zinc-400 border border-zinc-500/30">
            <XCircle size={12} /> Ignored
          </span>
        );
    }
  };

  return (
    <div className="p-8 space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold flex items-center gap-3">
            <Bookmark className="text-amber-400" /> Wishlist & Missing Music
          </h1>
          <p className="text-zinc-400 text-sm mt-1">
            Tracks you discovered that aren't yet in your local library. Find and acquire them via Soulseek.
          </p>
        </div>

        <button
          onClick={() => setShowAddForm(!showAddForm)}
          className="flex items-center gap-2 px-4 py-2 bg-emerald-500 hover:bg-emerald-400 text-black font-semibold rounded-lg shadow transition"
        >
          <Plus size={18} />
          <span>Add Track</span>
        </button>
      </div>

      {/* Add track inline modal/box */}
      {showAddForm && (
        <form
          onSubmit={handleAddSubmit}
          className="bg-zinc-900 border border-zinc-700 rounded-xl p-6 shadow-xl space-y-4 animate-in fade-in duration-200"
        >
          <h3 className="font-semibold text-lg text-white">Add Music to Wishlist</h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div>
              <label className="block text-xs text-zinc-400 uppercase font-semibold mb-1">
                Track Title *
              </label>
              <input
                type="text"
                required
                value={newTitle}
                onChange={(e) => setNewTitle(e.target.value)}
                placeholder="e.g. Master of Puppets"
                className="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm focus:outline-none focus:border-emerald-500"
              />
            </div>
            <div>
              <label className="block text-xs text-zinc-400 uppercase font-semibold mb-1">
                Artist Name *
              </label>
              <input
                type="text"
                required
                value={newArtist}
                onChange={(e) => setNewArtist(e.target.value)}
                placeholder="e.g. Metallica"
                className="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm focus:outline-none focus:border-emerald-500"
              />
            </div>
            <div>
              <label className="block text-xs text-zinc-400 uppercase font-semibold mb-1">
                Album (Optional)
              </label>
              <input
                type="text"
                value={newAlbum}
                onChange={(e) => setNewAlbum(e.target.value)}
                placeholder="e.g. Master of Puppets"
                className="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm focus:outline-none focus:border-emerald-500"
              />
            </div>
          </div>
          <div className="flex justify-end gap-3 pt-2">
            <button
              type="button"
              onClick={() => setShowAddForm(false)}
              className="px-4 py-2 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-sm font-medium transition"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={submitting}
              className="px-4 py-2 rounded-lg bg-emerald-500 hover:bg-emerald-400 text-black font-semibold text-sm transition disabled:opacity-50"
            >
              {submitting ? "Saving..." : "Save to Wishlist"}
            </button>
          </div>
        </form>
      )}

      {/* Filter and Search Bar */}
      <div className="flex flex-col sm:flex-row items-center justify-between gap-4">
        <div className="flex items-center gap-2 overflow-x-auto w-full sm:w-auto pb-1">
          {["ALL", "WANT", "DOWNLOADED", "ALREADY_OWN", "IGNORE"].map((tab) => (
            <button
              key={tab}
              onClick={() => setFilter(tab)}
              className={`px-3 py-1.5 rounded-lg text-xs font-semibold uppercase tracking-wider transition ${
                filter === tab
                  ? "bg-zinc-200 text-black"
                  : "bg-zinc-800/80 hover:bg-zinc-700 text-zinc-400 hover:text-white"
              }`}
            >
              {tab.replace("_", " ")}
            </button>
          ))}
        </div>

        <div className="relative w-full sm:w-72">
          <Search size={16} className="absolute left-3 top-2.5 text-zinc-500" />
          <input
            type="text"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            placeholder="Search wishlist..."
            className="w-full pl-9 pr-4 py-2 bg-zinc-900 border border-zinc-800 rounded-lg text-sm text-white placeholder-zinc-500 focus:outline-none focus:border-zinc-700"
          />
        </div>
      </div>

      {/* Wishlist Items List / Table */}
      {filteredItems.length === 0 ? (
        <div className="text-center py-16 bg-zinc-900/40 border border-zinc-800/50 rounded-2xl">
          <Bookmark size={48} className="mx-auto text-zinc-600 mb-3" />
          <p className="text-zinc-400 font-medium text-lg">No wishlist items found</p>
          <p className="text-zinc-600 text-sm mt-1">
            Explore recommendations in Discovery or add tracks you want to find!
          </p>
        </div>
      ) : (
        <div className="bg-zinc-900/60 border border-zinc-800/80 rounded-xl overflow-hidden shadow-sm">
          <table className="w-full text-left border-collapse text-sm">
            <thead>
              <tr className="border-b border-zinc-800 text-zinc-500 text-xs font-semibold uppercase">
                <th className="py-3.5 px-4">Track & Artist</th>
                <th className="py-3.5 px-4 hidden md:table-cell">Album</th>
                <th className="py-3.5 px-4">Status</th>
                <th className="py-3.5 px-4 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-zinc-800/60">
              {filteredItems.map((item) => (
                <tr key={item.id} className="hover:bg-zinc-800/40 transition group">
                  <td className="py-3.5 px-4">
                    <div className="font-medium text-zinc-100">{item.title}</div>
                    <div className="text-xs text-zinc-400">{item.artist}</div>
                  </td>
                  <td className="py-3.5 px-4 hidden md:table-cell text-zinc-400">
                    {item.album || "—"}
                  </td>
                  <td className="py-3.5 px-4">
                    <div className="flex items-center gap-2">
                      {getStatusBadge(item.status)}
                      <select
                        value={item.status.toLowerCase()}
                        onChange={(e) =>
                          onUpdateStatus(
                            item.id,
                            e.target.value as "want" | "ignore" | "already_own" | "downloaded"
                          )
                        }
                        className="opacity-0 group-hover:opacity-100 transition bg-zinc-800 border border-zinc-700 text-xs text-zinc-300 rounded px-1 py-0.5 focus:opacity-100"
                      >
                        <option value="want">Want</option>
                        <option value="downloaded">Downloaded</option>
                        <option value="already_own">Already Own</option>
                        <option value="ignore">Ignore</option>
                      </select>
                    </div>
                  </td>
                  <td className="py-3.5 px-4 text-right">
                    <div className="flex items-center justify-end gap-2">
                      <button
                        onClick={() => onSearchSoulseek(item.artist, item.title, item.album)}
                        className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-400 text-xs font-semibold transition border border-emerald-500/30"
                        title="Search and download on Soulseek"
                      >
                        <CloudDownload size={14} />
                        <span>Find on Soulseek</span>
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};
