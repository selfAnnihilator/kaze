import { RankedListCard } from "./RankedListCard";
import { Music, Play, Tag, Users } from "lucide-react";
import type { StatsViewContext } from "./context";

export function TopSongsArtists({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 480px), 1fr))",
        gap: "28px",
        marginBottom: "36px",
      }}
    >
      {/* Top 10 Songs */}
      <div
        style={{
          background: "var(--bg-card)",
          border: "1px solid rgba(255, 255, 255, 0.08)",
          borderRadius: "16px",
          padding: "24px",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "10px",
            marginBottom: "20px",
          }}
        >
          <div
            style={{
              width: "32px",
              height: "32px",
              borderRadius: "8px",
              background: "rgba(243, 112, 30, 0.15)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: "var(--accent-light)",
            }}
          >
            <Music size={18} />
          </div>
          <div>
            <h2 style={{ fontSize: "18px", fontWeight: 700, margin: 0 }}>
              Top 10 Songs
            </h2>
            <p
              style={{
                margin: 0,
                fontSize: "12px",
                color: "rgba(255, 255, 255, 0.4)",
              }}
            >
              Ranked by play score and listen duration
            </p>
          </div>
        </div>

        {!c.stats?.top_songs || c.stats.top_songs.length === 0 ? (
          <div
            style={{
              padding: "36px 0",
              textAlign: "center",
              color: "rgba(255, 255, 255, 0.4)",
              fontSize: "13px",
            }}
          >
            No songs listened to yet. Play some music to see your top songs!
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
            {c.stats.top_songs.map((song, idx) => {
              const rank = idx + 1;
              const rankColor =
                rank === 1 ? "#fbbf24" : rank === 2 ? "#e2e8f0" : rank === 3 ? "#d97706" : "rgba(255, 255, 255, 0.5)";
              return (
                <div
                  key={song.track_id}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    padding: "10px 14px",
                    borderRadius: "10px",
                    background: rank <= 3 ? "rgba(255, 255, 255, 0.03)" : "transparent",
                    border: rank <= 3 ? "1px solid rgba(255, 255, 255, 0.05)" : "none",
                    transition: "background 0.15s ease",
                  }}
                  className="stats-item-row"
                >
                  <span
                    style={{
                      width: "28px",
                      fontSize: "15px",
                      fontWeight: 800,
                      color: rankColor,
                      textAlign: "center",
                      flexShrink: 0,
                    }}
                  >
                    {rank}
                  </span>

                  <div style={{ flex: 1, minWidth: 0, margin: "0 14px" }}>
                    <div
                      style={{
                        fontSize: "14px",
                        fontWeight: 600,
                        color: "#e8d8c9",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                    >
                      {song.title}
                    </div>
                    <div
                      style={{
                        fontSize: "12px",
                        color: "rgba(255, 255, 255, 0.5)",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                    >
                      {song.artist_name || "Unknown Artist"}
                    </div>
                  </div>

                  <div style={{ textAlign: "right", marginRight: "12px" }}>
                    <div style={{ fontSize: "13px", fontWeight: 600, color: "#e8d8c9" }}>
                      {c.formatSeconds(song.total_seconds)}
                    </div>
                    <div style={{ fontSize: "11px", color: "rgba(255, 255, 255, 0.4)" }}>
                      {song.play_count} {song.play_count === 1 ? "play" : "plays"}
                    </div>
                  </div>

                  {c.onPlayTrack && (
                    <button
                      onClick={() => c.onPlayTrack?.(song.track_id)}
                      title={`Play ${song.title}`}
                      style={{
                        width: "32px",
                        height: "32px",
                        borderRadius: "50%",
                        background: "rgba(243, 112, 30, 0.2)",
                        border: "none",
                        color: "var(--accent-light)",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        cursor: "pointer",
                        flexShrink: 0,
                      }}
                    >
                      <Play size={14} fill="currentColor" />
                    </button>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Top 5 Artists + Top 5 Genres (stacked to match Top 10 Songs height) */}
      <div style={{ display: "flex", flexDirection: "column", gap: "28px" }}>
        <RankedListCard
          title="Top 5 Artists"
          subtitle="Your most listened to artists"
          icon={<Users size={18} />}
          iconBg="rgba(236, 72, 153, 0.15)"
          iconColor="#f472b6"
          rows={(c.stats?.top_artists ?? []).map((a) => ({ key: a.artist_id, name: a.name, total_seconds: a.total_seconds, play_count: a.play_count }))}
          emptyText="No artists recorded yet."
          formatSeconds={c.formatSeconds}
        />
        <RankedListCard
          title="Top 5 Genres"
          subtitle="Your most listened to genres"
          icon={<Tag size={18} />}
          iconBg="rgba(245, 158, 11, 0.15)"
          iconColor="#fbbf24"
          rows={(c.stats?.top_genres ?? []).map((g) => ({ key: g.genre_id, name: g.name, total_seconds: g.total_seconds, play_count: g.play_count }))}
          emptyText="No genres recorded yet."
          formatSeconds={c.formatSeconds}
        />
      </div>
    </div>
  );
}
