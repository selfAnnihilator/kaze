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
        borderRadius: "12px",
        padding: "10px",
        display: "flex",
        flexDirection: "column",
        cursor: "pointer",
        position: "relative",
        transition: "transform 0.2s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
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
          borderRadius: "8px",
          overflow: "hidden",
          background: "var(--bg-card)",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          padding: "16px",
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
              "opacity 0.2s ease, transform 0.2s ease, background-color 0.2s ease",
            zIndex: 3,
          }}
          title={`Play ${mix.name}`}
        >
          <Play size={17} fill="var(--text-main)" style={{ marginLeft: "2px" }} />
        </button>
      </div>

      <div
        style={{
          marginTop: "10px",
          fontWeight: 700,
          fontSize: "0.88rem",
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
          marginTop: "3px",
          fontWeight: 400,
          fontSize: "0.76rem",
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
