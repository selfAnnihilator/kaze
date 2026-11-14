import { RankedListCard } from "./RankedListCard";
import { Tag, Users } from "lucide-react";
import type { StatsViewContext } from "./context";

export function ArchivedTopLists({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 480px), 1fr))",
        gap: "28px",
      }}
    >
      {/* Top 10 Songs of the Year */}
      <div
        style={{
          background: "var(--bg-card)",
          border: "1px solid var(--overlay-8)",
          borderRadius: "16px",
          padding: "24px",
        }}
      >
        <h3 style={{ fontSize: "18px", fontWeight: 700, margin: "0 0 16px 0" }}>
          All-Time Top 10 Songs
        </h3>
        {!c.stats?.top_songs || c.stats.top_songs.length === 0 ? (
          <div
            style={{
              padding: "36px 0",
              textAlign: "center",
              color: "var(--overlay-40)",
              fontSize: "13px",
            }}
          >
            No songs recorded yet.
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
            {c.stats.top_songs.map((song, idx) => (
              <div
                key={song.track_id}
                style={{
                  display: "flex",
                  alignItems: "center",
                  padding: "10px 14px",
                  borderRadius: "10px",
                  background: idx < 3 ? "var(--overlay-3)" : "transparent",
                }}
              >
                <span
                  style={{
                    width: "28px",
                    fontSize: "15px",
                    fontWeight: 800,
                    color: idx === 0 ? "var(--palette-amber-450)" : idx === 1 ? "var(--palette-slate-100)" : idx === 2 ? "var(--palette-orange-550)" : "var(--overlay-50)",
                    textAlign: "center",
                  }}
                >
                  {idx + 1}
                </span>
                <div style={{ flex: 1, minWidth: 0, margin: "0 14px" }}>
                  <div style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-main)" }}>
                    {song.title}
                  </div>
                  <div style={{ fontSize: "12px", color: "var(--overlay-50)" }}>
                    {song.artist_name || "Unknown Artist"}
                  </div>
                </div>
                <div style={{ textAlign: "right" }}>
                  <div style={{ fontSize: "13px", fontWeight: 600, color: "var(--text-main)" }}>
                    {c.formatSeconds(song.total_seconds)}
                  </div>
                  <div style={{ fontSize: "11px", color: "var(--overlay-40)" }}>
                    {song.play_count} plays
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Top 5 Artists + Top 5 Genres (all-time) */}
      <div style={{ display: "flex", flexDirection: "column", gap: "28px" }}>
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
