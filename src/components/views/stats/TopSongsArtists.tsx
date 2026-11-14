import { RankedListCard } from "./RankedListCard";
import { Music, Play, Tag, Users } from "lucide-react";
import type { StatsViewContext } from "./context";

export function TopSongsArtists({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 480px), 1fr))",
        gap: "var(--space-28)",
        marginBottom: "var(--space-36)",
      }}
    >
      {/* Top 10 Songs */}
      <div
        style={{
          background: "var(--bg-card)",
          border: "1px solid var(--overlay-8)",
          borderRadius: "var(--radius-16)",
          padding: "var(--space-24)",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "var(--space-10)",
            marginBottom: "var(--space-20)",
          }}
        >
          <div
            style={{
              width: "32px",
              height: "32px",
              borderRadius: "var(--radius-8)",
              background: "var(--accent-a15)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: "var(--accent-light)",
            }}
          >
            <Music size={18} />
          </div>
          <div>
            <h2 style={{ fontSize: "var(--fs-18)", fontWeight: "var(--weight-bold)", margin: 0 }}>
              Top 10 Songs
            </h2>
            <p
              style={{
                margin: 0,
                fontSize: "var(--fs-12)",
                color: "var(--overlay-40)",
              }}
            >
              Ranked by play score and listen duration
            </p>
          </div>
        </div>

        {!c.stats?.top_songs || c.stats.top_songs.length === 0 ? (
          <div
            style={{
              padding: "var(--space-36) 0",
              textAlign: "center",
              color: "var(--overlay-40)",
              fontSize: "var(--fs-13)",
            }}
          >
            No songs listened to yet. Play some music to see your top songs!
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-8)" }}>
            {c.stats.top_songs.map((song, idx) => {
              const rank = idx + 1;
              const rankColor =
                rank === 1 ? "var(--palette-amber-450)" : rank === 2 ? "var(--palette-slate-100)" : rank === 3 ? "var(--palette-orange-550)" : "var(--overlay-50)";
              return (
                <div
                  key={song.track_id}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    padding: "var(--space-10) var(--space-14)",
                    borderRadius: "var(--radius-10)",
                    background: rank <= 3 ? "var(--overlay-3)" : "transparent",
                    border: rank <= 3 ? "1px solid var(--overlay-5)" : "none",
                    transition: "background var(--duration-fast) ease",
                  }}
                  className="stats-item-row"
                >
                  <span
                    style={{
                      width: "28px",
                      fontSize: "var(--fs-15)",
                      fontWeight: "var(--weight-extrabold)",
                      color: rankColor,
                      textAlign: "center",
                      flexShrink: 0,
                    }}
                  >
                    {rank}
                  </span>

                  <div style={{ flex: 1, minWidth: 0, margin: "0 var(--space-14)" }}>
                    <div
                      style={{
                        fontSize: "var(--fs-14)",
                        fontWeight: "var(--weight-semibold)",
                        color: "var(--text-main)",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                    >
                      {song.title}
                    </div>
                    <div
                      style={{
                        fontSize: "var(--fs-12)",
                        color: "var(--overlay-50)",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                    >
                      {song.artist_name || "Unknown Artist"}
                    </div>
                  </div>

                  <div style={{ textAlign: "right", marginRight: "var(--space-12)" }}>
                    <div style={{ fontSize: "var(--fs-13)", fontWeight: "var(--weight-semibold)", color: "var(--text-main)" }}>
                      {c.formatSeconds(song.total_seconds)}
                    </div>
                    <div style={{ fontSize: "var(--fs-11)", color: "var(--overlay-40)" }}>
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
                        background: "var(--accent-a20)",
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
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-28)" }}>
        <RankedListCard
          title="Top 5 Artists"
          subtitle="Your most listened to artists"
          icon={<Users size={18} />}
          iconBg="var(--palette-pink-400-a15)"
          iconColor="var(--palette-pink-300)"
          rows={(c.stats?.top_artists ?? []).map((a) => ({ key: a.artist_id, name: a.name, total_seconds: a.total_seconds, play_count: a.play_count }))}
          emptyText="No artists recorded yet."
          formatSeconds={c.formatSeconds}
        />
        <RankedListCard
          title="Top 5 Genres"
          subtitle="Your most listened to genres"
          icon={<Tag size={18} />}
          iconBg="var(--palette-orange-500-a15)"
          iconColor="var(--palette-amber-450)"
          rows={(c.stats?.top_genres ?? []).map((g) => ({ key: g.genre_id, name: g.name, total_seconds: g.total_seconds, play_count: g.play_count }))}
          emptyText="No genres recorded yet."
          formatSeconds={c.formatSeconds}
        />
      </div>
    </div>
  );
}
