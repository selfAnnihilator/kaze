import React, { useState } from "react";
import { DownloadCloud, Sparkles, Check, Bookmark, RefreshCw, Flame, Radio, Music2 } from "lucide-react";
import { DiscoveryRecommendation } from "../../types";

interface DiscoveryViewProps {
  recommendations: DiscoveryRecommendation[];
  onAddToWishlist: (rec: DiscoveryRecommendation) => void;
  onSearchDirect: (artist: string, title: string) => void;
  onRefresh?: () => Promise<void>;
}

export const DiscoveryView: React.FC<DiscoveryViewProps> = ({
  recommendations,
  onAddToWishlist,
  onSearchDirect,
  onRefresh,
}) => {
  const [filter, setFilter] = useState<"ALL" | "TRENDING" | "GENRE" | "SIMILAR">("ALL");
  const [refreshing, setRefreshing] = useState(false);

  const handleRefreshClick = async () => {
    if (!onRefresh) return;
    setRefreshing(true);
    try {
      await onRefresh();
    } finally {
      setRefreshing(false);
    }
  };

  const getBadgeClass = (status: string) => {
    switch (status) {
      case "EXACT_MATCH":
        return "badge badge-exact";
      case "LIKELY_MATCH":
        return "badge badge-likely";
      case "POSSIBLE_MATCH":
        return "badge badge-possible";
      default:
        return "badge badge-missing";
    }
  };

  const getBadgeLabel = (status: string) => {
    switch (status) {
      case "EXACT_MATCH":
        return "In Library";
      case "LIKELY_MATCH":
        return "Likely Owned";
      case "POSSIBLE_MATCH":
        return "Alternate Version";
      default:
        return "Missing Track";
    }
  };

  const filteredRecs = recommendations.filter((rec) => {
    if (filter === "ALL") return true;
    const r = rec.recommendation_reason.toLowerCase();
    if (filter === "TRENDING") return r.includes("trending") || r.includes("chart");
    if (filter === "GENRE") return r.includes("genre") || r.includes("popular in");
    if (filter === "SIMILAR") return r.includes("similar") || r.includes("listening") || r.includes("library artist");
    return true;
  });

  const trendingCount = recommendations.filter((r) => {
    const s = r.recommendation_reason.toLowerCase();
    return s.includes("trending") || s.includes("chart");
  }).length;

  const genreCount = recommendations.filter((r) => {
    const s = r.recommendation_reason.toLowerCase();
    return s.includes("genre") || s.includes("popular in");
  }).length;

  const similarCount = recommendations.filter((r) => {
    const s = r.recommendation_reason.toLowerCase();
    return s.includes("similar") || s.includes("listening") || s.includes("library artist");
  }).length;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
      <div className="view-header" style={{ marginBottom: 0 }}>
        <div>
          <h1 className="view-title" style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <Sparkles size={26} color="var(--accent-light)" />
            <span>Music Discovery</span>
          </h1>
          <p style={{ color: "var(--text-muted)", fontSize: "0.88rem", marginTop: "4px" }}>
            Fresh music recommended from trending charts, your top genres, and artist listening affinities.
          </p>
        </div>

        {onRefresh && (
          <button
            onClick={handleRefreshClick}
            disabled={refreshing}
            className="btn btn-secondary"
            style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "0.85rem" }}
            title="Refresh discovery recommendations"
          >
            <RefreshCw size={15} className={refreshing ? "animate-spin" : ""} />
            <span>{refreshing ? "Refreshing..." : "Refresh Discovery"}</span>
          </button>
        )}
      </div>

      {recommendations.length > 0 && (
        <div style={{ display: "flex", alignItems: "center", gap: "8px", flexWrap: "wrap" }}>
          <button
            onClick={() => setFilter("ALL")}
            className={`subtab-btn ${filter === "ALL" ? "active" : ""}`}
          >
            All Recommendations ({recommendations.length})
          </button>
          {trendingCount > 0 && (
            <button
              onClick={() => setFilter("TRENDING")}
              className={`subtab-btn ${filter === "TRENDING" ? "active" : ""}`}
              style={{ display: "inline-flex", alignItems: "center", gap: "5px" }}
            >
              <Flame size={12} color="#f59e0b" />
              <span>Trending Hits ({trendingCount})</span>
            </button>
          )}
          {genreCount > 0 && (
            <button
              onClick={() => setFilter("GENRE")}
              className={`subtab-btn ${filter === "GENRE" ? "active" : ""}`}
              style={{ display: "inline-flex", alignItems: "center", gap: "5px" }}
            >
              <Radio size={12} color="var(--accent-light)" />
              <span>Top Genres ({genreCount})</span>
            </button>
          )}
          {similarCount > 0 && (
            <button
              onClick={() => setFilter("SIMILAR")}
              className={`subtab-btn ${filter === "SIMILAR" ? "active" : ""}`}
              style={{ display: "inline-flex", alignItems: "center", gap: "5px" }}
            >
              <Sparkles size={12} color="#10b981" />
              <span>Similar Artists ({similarCount})</span>
            </button>
          )}
        </div>
      )}

      {filteredRecs.length === 0 ? (
        <div
          className="content-card"
          style={{ textAlign: "center", padding: "60px 20px", color: "var(--text-dim)", borderStyle: "dashed" }}
        >
          <Sparkles size={40} color="var(--accent-light)" style={{ marginBottom: "14px" }} />
          <p style={{ fontSize: "1.1rem", fontWeight: 600, color: "var(--text-main)", marginBottom: "6px" }}>
            No discovery recommendations found in this view
          </p>
          <p style={{ fontSize: "0.88rem", color: "var(--text-muted)", maxWidth: "500px", margin: "0 auto" }}>
            Listen to your local library or click Refresh Discovery to discover fresh trending and genre tracks.
          </p>
          {onRefresh && (
            <button
              onClick={handleRefreshClick}
              className="btn btn-primary"
              style={{ marginTop: "18px", fontSize: "0.85rem", display: "inline-flex", alignItems: "center", gap: "8px" }}
            >
              <RefreshCw size={15} />
              <span>Refresh Discovery Now</span>
            </button>
          )}
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
          {filteredRecs.map((rec) => (
            <div
              key={rec.external_track_id}
              className="content-card"
              style={{
                marginBottom: 0,
                padding: "14px 18px",
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: "16px",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "14px", flex: 1, minWidth: 0 }}>
                {rec.cover_art_url ? (
                  <img
                    src={rec.cover_art_url}
                    alt={rec.title}
                    style={{
                      width: "48px",
                      height: "48px",
                      borderRadius: "6px",
                      objectFit: "cover",
                      flexShrink: 0,
                      backgroundColor: "var(--bg-sidebar)",
                    }}
                  />
                ) : (
                  <div
                    style={{
                      width: "48px",
                      height: "48px",
                      borderRadius: "6px",
                      backgroundColor: "var(--bg-sidebar)",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      flexShrink: 0,
                      border: "1px solid var(--border)",
                    }}
                  >
                    <Music2 size={22} color="var(--accent-light)" />
                  </div>
                )}

                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "3px" }}>
                    <span
                      style={{
                        fontWeight: 600,
                        fontSize: "0.95rem",
                        color: "var(--text-main)",
                        whiteSpace: "nowrap",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                      }}
                      title={rec.title}
                    >
                      {rec.title}
                    </span>
                    <span className={getBadgeClass(rec.match_status)}>
                      {getBadgeLabel(rec.match_status)}
                    </span>
                  </div>

                  <div
                    style={{
                      fontSize: "0.85rem",
                      color: "var(--text-muted)",
                      marginBottom: "4px",
                      whiteSpace: "nowrap",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                    }}
                  >
                    {rec.artist} {rec.album ? `— ${rec.album}` : ""}
                  </div>

                  <div
                    style={{
                      fontSize: "0.76rem",
                      color: "var(--text-dim)",
                      display: "flex",
                      alignItems: "center",
                      gap: "6px",
                    }}
                  >
                    <Sparkles size={11} color="var(--accent-light)" />
                    <span>{rec.recommendation_reason}</span>
                  </div>
                </div>
              </div>

              <div style={{ display: "flex", gap: "8px", alignItems: "center", flexShrink: 0 }}>
                <button
                  className="btn btn-secondary"
                  onClick={() => onSearchDirect(rec.artist, rec.title)}
                  title="Search & download directly in-app"
                  style={{ fontSize: "0.8rem", padding: "6px 12px", display: "inline-flex", alignItems: "center", gap: "6px" }}
                >
                  <DownloadCloud size={14} color="var(--accent-light)" />
                  <span>Download Direct</span>
                </button>

                {rec.in_wishlist ? (
                  <button
                    className="btn btn-secondary"
                    disabled
                    style={{ opacity: 0.6, cursor: "default", fontSize: "0.8rem", padding: "6px 12px" }}
                  >
                    <Check size={14} color="var(--success)" />
                    <span>In Wishlist</span>
                  </button>
                ) : (
                  <button
                    className="btn btn-primary"
                    onClick={() => onAddToWishlist(rec)}
                    style={{ fontSize: "0.8rem", padding: "6px 12px", display: "inline-flex", alignItems: "center", gap: "6px" }}
                  >
                    <Bookmark size={14} />
                    <span>Add to Wishlist</span>
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
