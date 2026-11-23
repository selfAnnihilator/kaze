import { memo } from "react";
import { ListPlus, Check, Play } from "lucide-react";
import { SectionHead } from "./SectionHead";
import { Sleeve } from "./Sleeve";
import { joinMeta } from "./format";
import type { MobileScreenProps } from "./types";
import { queueTrack } from "./phoneToasts";
import { useFitText } from "./fitText";

interface HomeItem {
  id: string;
  title: string;
  meta: string;
  cover?: string;
}

const SHOWN = 5;

/** Home row (Figma B2): 54px sleeve, 18px title, queue icon and a boxed play button. */
const HomeRow = memo(function HomeRow({
  item, queued, onPlay, onQueue,
}: {
  item: HomeItem; queued: boolean; onPlay: () => void; onQueue: () => void;
}) {
  return (
    <div className="m-home-row">
      <button type="button" className="m-home-row__main" onClick={onPlay} aria-label={`Play ${item.title}`}>
        <Sleeve size="row" src={item.cover} trackId={item.id} title={item.title} />
        <span className="m-home-row__text">
          <span className="m-home-row__title">{item.title}</span>
          <span className="m-home-row__meta">{item.meta}</span>
        </span>
      </button>
      <button type="button" className="m-home-row__icon" onClick={onQueue} disabled={queued}
        aria-label={queued ? "Already in queue" : "Add to queue"}>
        {queued ? <Check size={18} strokeWidth={2} /> : <ListPlus size={18} strokeWidth={2} />}
      </button>
      <button type="button" className="m-home-row__play" onClick={onPlay} aria-label={`Play ${item.title}`}>
        <Play size={18} strokeWidth={2} />
      </button>
    </div>
  );
});

function Section({
  c, num, sub, title, items,
}: MobileScreenProps & {
  num: string; sub: string; title: string; items: HomeItem[];
}) {
  if (!items.length) return null;
  return (
    <section>
      <SectionHead kicker={`${num} // ${sub}`} title={title} />
      {items.slice(0, SHOWN).map((item) => (
        <HomeRow
          key={item.id}
          item={item}
          queued={c.queuedTrackIds.has(item.id)}
          onPlay={() => void c.handlePlayTrack(item.id)}
          onQueue={() => void queueTrack(c, item.id)}
        />
      ))}
    </section>
  );
}

/** A Your Week number: one line, shrinking to fit its cell instead of wrapping. */
function WeekValue({ value }: { value: string }) {
  const ref = useFitText<HTMLSpanElement>(value);
  return <span ref={ref} className="m-home-week__value">{value}</span>;
}

/** Home below Flow (Figma B2 and B3): rotation, rediscover, for you, library moment, your week. */
export const HomeSections = memo(function HomeSections({ c }: MobileScreenProps) {
  const overview = c.homeOverview;
  if (!overview) return null;

  const rotation: HomeItem[] = overview.recent_rotation.map((t) => ({
    id: t.track_id, title: t.title, meta: joinMeta(t.artist, t.album), cover: t.cover_art_url,
  }));
  const rediscover: HomeItem[] = overview.rediscover.map((t) => ({
    id: t.track_id, title: t.title, meta: joinMeta(t.artist, t.album), cover: t.cover_art_url,
  }));
  const forYou: HomeItem[] = overview.for_you.map((t) => ({
    id: t.track_id, title: t.title, meta: joinMeta(t.artist_name, t.album_title),
  }));
  const moment = overview.library_moment;
  const week = overview.your_week;

  return (
    <>
      <Section c={c} num="02" sub="BACK IN GOOD COMPANY" title="RECENT ROTATION" items={rotation} />
      <Section c={c} num="03" sub="STILL SOUNDS LIKE THE FIRST TIME" title="REDISCOVER" items={rediscover} />
      <Section c={c} num="04" sub="PICKED FROM YOUR LISTENING" title="FOR YOU" items={forYou} />

      {moment && (
        <section>
          <SectionHead kicker="05 // FROM YOUR SHELVES" title="LIBRARY MOMENT" />
          <div className="m-home-moment">
            <Sleeve size="moment" src={moment.cover_art_path} trackId={moment.first_track_id} title={moment.album_title} />
            <div className="m-home-moment__info">
              <h3 className="m-home-moment__title">{moment.album_title.toUpperCase()}</h3>
              <span className="m-kicker m-kicker--muted">{joinMeta(moment.artist_name.toUpperCase(), moment.release_year ? String(moment.release_year) : undefined)}</span>
              <span className="m-kicker m-kicker--muted">{moment.total_tracks} TRACKS</span>
              <button type="button" className="m-btn m-btn--sm m-home-moment__play"
                disabled={!moment.first_track_id}
                onClick={() => moment.first_track_id && void c.handlePlayTrack(moment.first_track_id)}>
                <Play size={18} strokeWidth={2} />
                <span>PLAY ALBUM</span>
              </button>
            </div>
          </div>
        </section>
      )}

      {week && (
        <section>
          <SectionHead kicker="06 // THIS WEEK" title="YOUR WEEK" />
          <div className="m-home-week">
            {[
              [week.formatted_time.toUpperCase(), "TIME LISTENED"],
              [String(week.track_count), "TRACKS PLAYED"],
              [String(week.artist_count), "ARTISTS EXPLORED"],
            ].map(([value, label]) => (
              <div key={label} className="m-home-week__cell">
                <WeekValue value={value} />
                <span className="m-kicker m-kicker--muted m-home-week__label">{label}</span>
              </div>
            ))}
          </div>
        </section>
      )}

    </>
  );
});
