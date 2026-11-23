import { Sleeve } from "../Sleeve";
import { hoursMinutes, type ProfileStats } from "./useProfileStats";
import type { MobileScreenProps } from "../types";

type Ctx = MobileScreenProps["c"];
const pad = (n: number) => String(n).padStart(2, "0");

function ListHead({ num, title, hint }: { num: string; title: string; hint?: string }) {
  return (
    <div className="m-profile-sec">
      <div className="m-profile-sec__text"><span className="m-kicker">{num} //</span><h2 className="m-profile-sec__title">{title}</h2></div>
      {hint && <span className="m-profile-meta m-profile-sec__hint">{hint}</span>}
    </div>
  );
}

const dayLabel = (iso: string) =>
  new Date(`${iso}T00:00:00`).toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" }).replace(/,/g, "").toUpperCase();

/**
 * Top 10 songs, top 5 artists, top genres and the top day (Figma F2). Each list
 * keeps the height Figma shows and scrolls inside it to reach the rest.
 */
export function ProfileTopLists({ c, p }: { c: Ctx; p: ProfileStats }) {
  const s = p.stats;
  const songs = s?.top_songs ?? [];
  const artists = s?.top_artists ?? [];
  const genres = s?.top_genres ?? [];
  const genreTotal = genres.reduce((sum, g) => sum + g.total_seconds, 0) || 1;
  const topDay = s?.top_days[0];
  return (
    <>
      <ListHead num="01" title="TOP 10 SONGS" hint={songs.length ? "TAP TO PLAY" : undefined} />
      {songs.length > 0 && (
        <div className="m-profile-scroll m-profile-scroll--songs" role="list" aria-label="Top songs">
          {songs.map((song, i) => (
            <button key={song.track_id} type="button" role="listitem" className="m-profile-rank" onClick={() => c.handlePlayTrack(song.track_id)}>
              <span className="m-profile-rank__num">{pad(i + 1)}</span>
              <span className="m-profile-rank__text"><strong>{song.title}</strong><span className="m-profile-meta">{(song.artist_name || "Unknown artist").toUpperCase()}</span></span>
              <span className="m-profile-meta">{song.play_count} PLAYS</span>
            </button>
          ))}
        </div>
      )}
      {!songs.length && <p className="m-empty-msg">No plays yet. Play a few songs and your top 10 shows up here.</p>}

      <ListHead num="02" title="TOP 5 ARTISTS" />
      {artists.length ? (
        <div className="m-profile-artists">
          {artists.map((a, i) => (
            <div key={a.artist_id} className="m-profile-artist">
              <Sleeve size="artist" title={a.name} />
              <strong>{a.name}</strong>
              <span className="m-profile-meta is-red">#{i + 1}</span>
            </div>
          ))}
        </div>
      ) : <p className="m-empty-msg">No artists yet. Your most-played artists show up here.</p>}

      <ListHead num="03" title="TOP GENRES & DAYS" />
      <div className="m-profile-genres">
        {genres.length > 0 && <div className="m-profile-scroll m-profile-scroll--genres">{genres.map((g) => {
          const share = g.total_seconds / genreTotal;
          return (
            <div key={g.genre_id} className="m-profile-genre">
              <span className="m-profile-genre__name">{g.name.toUpperCase()}</span>
              <span className="m-profile-genre__track"><span className="m-profile-genre__fill" style={{ transform: `scaleX(${share})` }} /></span>
              <span className="m-profile-meta">{Math.round(share * 100)}%</span>
            </div>
          );
        })}</div>}
        {!genres.length && <p className="m-set-note">No genres yet. They come from the songs you play.</p>}
        {topDay && <span className="m-profile-topday">TOP DAY — {dayLabel(topDay.day_date)} · {hoursMinutes(topDay.total_seconds)}</span>}
      </div>
    </>
  );
}
