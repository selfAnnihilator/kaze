import { Artwork } from "../../../presentation/Artwork";
import { ArrowLeft, Heart } from "lucide-react";
import type { CollectionDetailViewContext } from "./context";

export function CollectionHero({ c }: { c: CollectionDetailViewContext }) {
  return (
    <>
      {c.withinArchive ? (
        <div className="press-archive-liked-heading">
          <h2>
            <Heart size={24} />{c.collection.title}
          </h2>
          <p>
            {c.isLoadingTracks
              ? "Loading your liked tracks…"
              : `${c.rawTracks.length} liked ${c.rawTracks.length === 1 ? "track" : "tracks"}`}
          </p>
        </div>
      ) : (
      <div
        className="collection-hero"
        style={{
          position: "relative",
          borderRadius: "14px",
          overflow: "hidden",
          background: c.bgGradient,
          padding: "28px 32px 36px 32px",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "24px",
          boxShadow: "0 8px 32px var(--shade-40)",
          minHeight: "260px",
        }}
      >
        {/* Back Button on top left */}
        <button
          type="button"
          onClick={c.onBack}
          style={{
            position: "absolute",
            top: "16px",
            left: "16px",
            display: "inline-flex",
            alignItems: "center",
            gap: "6px",
            backgroundColor: "var(--shade-65)",
            border: "1px solid var(--overlay-15)",
            color: "var(--text-main)",
            borderRadius: "20px",
            padding: "6px 14px",
            fontSize: "0.82rem",
            fontWeight: 600,
            cursor: "pointer",
            zIndex: 10,
            transition: "transform 0.15s ease, opacity 0.15s ease",
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = "var(--shade-70)";
            e.currentTarget.style.borderColor = "var(--overlay-30)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = "var(--shade-45)";
            e.currentTarget.style.borderColor = "var(--overlay-15)";
          }}
        >
          <ArrowLeft size={16} />
          <span>Back</span>
        </button>

        {/* Left: Text Metadata */}
        <div className="collection-copy" style={{ flex: 1, marginTop: "24px", zIndex: 2 }}>
          <div
            style={{
              fontSize: "0.8rem",
              fontWeight: 700,
              letterSpacing: "1.2px",
              color: "var(--overlay-85)",
              textTransform: "uppercase",
              marginBottom: "8px",
            }}
          >
            {c.collection.tag || "PUBLIC PLAYLIST"}
          </div>

          <h1
            style={{
              fontSize: "clamp(2rem, 4vw, 3.2rem)",
              fontWeight: 900,
              color: "var(--text-main)",
              lineHeight: 1.1,
              letterSpacing: "-0.5px",
              marginBottom: "12px",
              textShadow: "0 3px 12px var(--shade-40)",
            }}
          >
            {c.collection.title}
          </h1>

          <p
            style={{
              fontSize: "0.95rem",
              color: "var(--overlay-90)",
              maxWidth: "600px",
              lineHeight: 1.4,
              marginBottom: "14px",
              textShadow: "0 1px 4px var(--shade-40)",
            }}
          >
            {c.collection.subtitle}
          </p>

          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "8px",
              fontSize: "0.85rem",
              color: "var(--overlay-85)",
              flexWrap: "wrap",
            }}
          >
            <span style={{ fontWeight: 700, color: "var(--text-main)" }}>Kaze</span>
            <span>•</span>
            <span>
              {c.searchQuery.trim()
                ? `${c.tracks.length} of ${c.rawTracks.length} ${c.rawTracks.length === 1 ? "song" : "songs"}`
                : `${c.rawTracks.length} ${c.rawTracks.length === 1 ? "song" : "songs"}`}
            </span>
            {c.totalDurationSecs > 0 && (
              <>
                <span>•</span>
                <span>{c.formatTotalDuration(c.totalDurationSecs)}</span>
              </>
            )}
          </div>
        </div>

        <div className={c.interfaceStyle === "spatial" ? "spatial-collection-art" : "press-collection-art"}>
          <Artwork src={c.heroImage1} alt={c.collection.title} />
          <p>{c.collection.subtitle}</p>
        </div>
      </div>
      )}
    </>
  );
}
