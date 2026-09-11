import React from "react";
import { Download, Sparkles, Check, Bookmark } from "lucide-react";
import { DiscoveryRecommendation } from "../../types";

interface DiscoveryViewProps {
  recommendations: DiscoveryRecommendation[];
  onAddToWishlist: (rec: DiscoveryRecommendation) => void;
  onSearchSoulseek: (artist: string, title: string) => void;
}

export const DiscoveryView: React.FC<DiscoveryViewProps> = ({
  recommendations,
  onAddToWishlist,
  onSearchSoulseek,
}) => {
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

  return (
    <div>
      <div className="view-header">
        <div>
          <h1 className="view-title">Music Discovery</h1>
          <p style={{ color: "var(--text-dim)", fontSize: "0.88rem", marginTop: "4px" }}>
            Tracks outside your collection recommended based on your listening affinities
          </p>
        </div>
      </div>

      {recommendations.length === 0 ? (
        <div style={{ textAlign: "center", padding: "60px 0", color: "var(--text-dim)" }}>
          <Sparkles size={36} color="var(--accent-light)" style={{ marginBottom: "12px" }} />
          <p style={{ fontSize: "1.1rem", marginBottom: "6px" }}>No discovery recommendations yet</p>
          <p style={{ fontSize: "0.88rem" }}>
            Listen to your local library to build your taste profile, or configure external providers in Settings.
          </p>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
          {recommendations.map((rec) => (
            <div
              key={rec.external_track_id}
              style={{
                backgroundColor: "var(--bg-card)",
                borderRadius: "10px",
                padding: "16px 20px",
                border: "1px solid var(--border)",
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: "16px",
              }}
            >
              <div style={{ flex: 1 }}>
                <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "4px" }}>
                  <span style={{ fontWeight: 600, fontSize: "1rem" }}>{rec.title}</span>
                  <span className={getBadgeClass(rec.match_status)}>
                    {getBadgeLabel(rec.match_status)}
                  </span>
                </div>
                <div style={{ fontSize: "0.88rem", color: "var(--text-muted)", marginBottom: "6px" }}>
                  {rec.artist} {rec.album ? `— ${rec.album}` : ""}
                </div>
                <div style={{ fontSize: "0.78rem", color: "var(--text-dim)", display: "flex", alignItems: "center", gap: "6px" }}>
                  <Sparkles size={12} color="var(--accent-light)" />
                  <span>{rec.recommendation_reason}</span>
                </div>
              </div>

              <div style={{ display: "flex", gap: "10px", alignItems: "center" }}>
                <button
                  className="btn btn-secondary"
                  onClick={() => onSearchSoulseek(rec.artist, rec.title)}
                  title="Search Soulseek network for this track"
                >
                  <Download size={15} />
                  <span>Search</span>
                </button>

                {rec.in_wishlist ? (
                  <button className="btn btn-secondary" disabled style={{ opacity: 0.6, cursor: "default" }}>
                    <Check size={15} color="var(--success)" />
                    <span>In Wishlist</span>
                  </button>
                ) : (
                  <button className="btn btn-primary" onClick={() => onAddToWishlist(rec)}>
                    <Bookmark size={15} />
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
