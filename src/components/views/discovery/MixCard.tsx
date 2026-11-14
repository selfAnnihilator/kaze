import { Play } from "lucide-react";
import React, { useState } from "react";

export interface MixItem {
  id: string;
  name: string;
  subtitle: string;
  bgGradient: string;
  accentColor: string;
  searchQuery?: string;
  playlistId?: string;
  cover_art_url?: string;
}

export const MixCard: React.FC<{
  mix: MixItem;
  onPlay: () => void;
  onOpen?: () => void;
}> = ({ mix, onPlay, onOpen }) => {
  const [isHovered, setIsHovered] = useState(false);

  return (
    <div
      className="mix-card"
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onClick={onOpen || onPlay}
      style={{
        backgroundColor: "var(--bg-card)",
        border: isHovered
          ? "1px solid var(--overlay-20)"
          : "1px solid var(--border)",
        borderRadius: "var(--radius-12)",
        padding: "var(--space-10)",
        display: "flex",
        flexDirection: "column",
        cursor: "pointer",
        position: "relative",
        transition: "transform var(--duration-base) var(--ease-standard), opacity var(--duration-base) var(--ease-standard)",
        boxShadow: isHovered
          ? "0 10px 24px var(--shade-40)"
          : "0 2px 8px var(--shade-15)",
        width: "100%",
        height: "100%",
      }}
    >
      <div
        style={{
          position: "relative",
          width: "100%",
          aspectRatio: "1 / 1",
          borderRadius: "var(--radius-8)",
          overflow: "hidden",
          background: "var(--bg-card)",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          padding: "var(--space-16)",
          textAlign: "center",
        }}
      >
        <img decoding="async" loading="lazy" src={mix.cover_art_url || "/kaze-playlist-default.svg"}
          alt={mix.name}
          style={{
            position: "absolute",
            inset: 0,
            width: "100%",
            height: "100%",
            objectFit: "cover",
          }}
        />

        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onPlay();
          }}
          style={{
            position: "absolute",
            bottom: "8px",
            left: "8px",
            width: "38px",
            height: "38px",
            borderRadius: "50%",
            backgroundColor: "var(--accent)",
            border: "none",
            color: "var(--text-main)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            boxShadow: "0 4px 14px var(--shade-60)",
            cursor: "pointer",
            opacity: isHovered ? 1 : 0,
            transform: isHovered ? "scale(1)" : "scale(0.85)",
            transition:
              "opacity var(--duration-base) ease, transform var(--duration-base) ease, background-color var(--duration-base) ease",
            zIndex: 3,
          }}
          title={`Play ${mix.name}`}
        >
          <Play size={17} fill="var(--text-main)" style={{ marginLeft: "var(--space-2)" }} />
        </button>
      </div>

      <div
        style={{
          marginTop: "var(--space-10)",
          fontWeight: "var(--weight-bold)",
          fontSize: "var(--fs-14)",
          color: "var(--text-main)",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
          lineHeight: "1.3",
        }}
        title={mix.name}
      >
        {mix.name}
      </div>

      <div
        style={{
          marginTop: "var(--space-4)",
          fontWeight: "var(--weight-regular)",
          fontSize: "var(--fs-12)",
          color: "var(--text-muted)",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
          lineHeight: "1.3",
        }}
        title={mix.subtitle}
      >
        {mix.subtitle}
      </div>
    </div>
  );
};
