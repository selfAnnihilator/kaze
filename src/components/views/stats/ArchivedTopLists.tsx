import { RankedListCard } from "./RankedListCard";
import { Tag, Users } from "lucide-react";
import type { StatsViewContext } from "./context";

export function ArchivedTopLists({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 480px), 1fr))",
        gap: "var(--space-28)",
      }}
    >
      {/* Top 10 Songs of the Year */}
      <div
        style={{
          background: "var(--bg-card)",
          border: "1px solid var(--overlay-8)",
          borderRadius: "16px",
          padding: "var(--space-24)",
        }}
      >
        <h3 style={{ fontSize: "var(--fs-18)", fontWeight: "var(--weight-bold)", margin: "0 0 var(--space-16) 0" }}>
          All-Time Top 10 Songs
        </h3>
        {!c.stats?.top_songs || c.stats.top_songs.length === 0 ? (
          <div
            style={{
              padding: "var(--space-36) 0",
              textAlign: "center",
              color: "var(--overlay-40)",
              fontSize: "var(--fs-13)",
            }}
          >
            No songs recorded yet.
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-8)" }}>
            {c.stats.top_songs.map((song, idx) => (
              <div
                key={song.track_id}
                style={{
                  display: "flex",
                  alignItems: "center",
                  padding: "var(--space-10) var(--space-14)",
                  borderRadius: "10px",
                  background: idx < 3 ? "var(--overlay-3)" : "transparent",
                }}
              >
                <span
                  style={{
                    width: "28px",
                    fontSize: "var(--fs-15)",
                    fontWeight: "var(--weight-extrabold)",
                    color: idx === 0 ? "var(--palette-amber-450)" : idx === 1 ? "var(--palette-slate-100)" : idx === 2 ? "var(--palette-orange-550)" : "var(--overlay-50)",
                    textAlign: "center",
                  }}
                >
                  {idx + 1}
                </span>
                <div style={{ flex: 1, minWidth: 0, margin: "0 var(--space-14)" }}>
                  <div style={{ fontSize: "var(--fs-14)", fontWeight: "var(--weight-semibold)", color: "var(--text-main)" }}>
                    {song.title}
                  </div>
                  <div style={{ fontSize: "var(--fs-12)", color: "var(--overlay-50)" }}>
                    {song.artist_name || "Unknown Artist"}
                  </div>
                </div>
                <div style={{ textAlign: "right" }}>
                  <div style={{ fontSize: "var(--fs-13)", fontWeight: "var(--weight-semibold)", color: "var(--text-main)" }}>
                    {c.formatSeconds(song.total_seconds)}
                  </div>
                  <div style={{ fontSize: "var(--fs-11)", color: "var(--overlay-40)" }}>
                    {song.play_count} plays
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Top 5 Artists + Top 5 Genres (all-time) */}
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
