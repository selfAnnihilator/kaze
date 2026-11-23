import { Sleeve } from "../Sleeve";
import type { ProfileStats } from "./useProfileStats";

const count = (n: number) => n.toLocaleString("en-US");

/** Year in Review (Figma F3): an earlier year's hours, tracks and artists, and its song of the year. */
export function YearReview({ p }: { p: ProfileStats }) {
  const s = p.stats;
  if (!s) return null;
  const top = s.top_songs[0];
  return (
    <section className="m-review">
      <span className="m-review__tag">ARCHIVED YEAR</span>
      <strong className="m-review__year">{p.viewingYear}</strong>
      <h2 className="m-display m-review__title">YEAR IN REVIEW.</h2>
      <div className="m-review__boxes">
        <div className="m-review__box"><span className="m-profile-meta">LISTENED</span><strong>{Math.round(s.total_year_seconds / 3600)}H</strong></div>
        <div className="m-review__box"><span className="m-profile-meta">TRACKS</span><strong>{count(s.top_songs.length)}</strong></div>
        <div className="m-review__box"><span className="m-profile-meta">ARTISTS</span><strong>{count(s.top_artists.length)}</strong></div>
      </div>
      {top ? (
        <div className="m-review__song">
          <Sleeve size="review" trackId={top.track_id} title={top.title} />
          <span className="m-review__song-text">
            <span className="m-kicker">SONG OF THE YEAR</span>
            <strong>{top.title}</strong>
            <span className="m-profile-meta">{[top.artist_name?.toUpperCase(), `${count(top.play_count)} PLAYS`].filter(Boolean).join(" · ")}</span>
          </span>
        </div>
      ) : <p className="m-empty-msg">Nothing was played in {p.viewingYear}.</p>}
    </section>
  );
}
