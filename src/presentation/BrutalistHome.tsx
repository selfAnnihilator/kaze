import { NotInterestedButton } from "../components/NotInterestedButton";
import { memo } from "react";
import {
  ArrowUpRight,
  Check,
  Info,
  ListPlus,
  Pause,
  Play,
  RefreshCw,
} from "lucide-react";
import type { HomeOverview, Track, FlowExplanation } from "../types";
import { Artwork } from "./Artwork";
import type { PlaybackArtworkTrack } from "./queuePresentation";
import { FamiliarityBar } from "./FamiliarityBar";
import { WindowedList } from "./WindowedList";

type Item = {
  id: string;
  title: string;
  artist?: string;
  cover?: string;
  explanation?: FlowExplanation | null;
};
export const BrutalistHome = memo(function BrutalistHome(p: {
  overview: HomeOverview | null;
  currentTrack?: Track | null;
  currentCover?: string | null;
  previousTrack?: Track;
  nextTrack?: PlaybackArtworkTrack;
  onPreviousTrack: () => void;
  onNextTrack: () => void;
  isLoading: boolean;
  isFlowPending?: boolean;
  isPlaying: boolean;
  flowAction: string;
  onFlow: () => void;
  flowSteering: number;
  onFlowSteeringChange: (value: number) => void;
  onRefresh: () => void;
  onDiscover?: () => void;
  onOpenAlbum?: (id: string) => void;
  onPlayTrack: (id: string, source?: string) => void;
  onTogglePlayback: () => void;
  onEnqueueTrack?: (id: string) => void;
  queuedTrackIds?: Set<string>;
  loadingTrackId?: string | null;
  onExplain: (item: {
    title: string;
    artist: string;
    explanation?: FlowExplanation | null;
  }) => void;
}) {
  const overview = p.overview;
  const sections: [string, Item[]][] = [
    [
      "Recent Rotation",
      (overview?.recent_rotation || []).map((t) => ({
        id: t.track_id,
        title: t.title,
        artist: t.artist,
        cover: t.cover_art_url,
      })),
    ],
    [
      "Rediscover",
      (overview?.rediscover || []).map((t) => ({
        id: t.track_id,
        title: t.title,
        artist: t.artist,
        cover: t.cover_art_url,
        explanation: t.explanation,
      })),
    ],
    [
      "For You",
      (overview?.for_you || []).map((t) => ({
        id: t.track_id,
        title: t.title,
        artist: t.artist_name,
        explanation: t.explanation,
      })),
    ],
  ];
  const moment = overview?.library_moment;
  const momentTrack = overview?.recent_rotation.find(
    (t) =>
      t.album?.toLowerCase() === moment?.album_title.toLowerCase() &&
      t.artist.toLowerCase() === moment?.artist_name.toLowerCase(),
  );
  const momentCover = moment?.cover_art_path || momentTrack?.cover_art_url;
  const momentPlayId = moment?.first_track_id || momentTrack?.track_id;
  const momentIsCurrent = !!momentPlayId && momentPlayId === p.currentTrack?.id;
  const momentLoading = !!momentPlayId && p.loadingTrackId === momentPlayId;
  return (
    <div className="home-view press-home">
      <section className="press-listen home-flow" aria-label="Your Flow">
        <div className="press-poster-title">
          <span className="press-micro">
            KAZE HOME / YOUR MUSIC, ALWAYS MOVING
          </span>
          <h1>FLOW</h1>
        </div>
        <div className="press-poster-body">
          <div className="press-home-artwork" aria-label="Playback artwork">
            <button
              type="button"
              className="press-home-adjacent press-home-previous"
              onClick={p.onPreviousTrack}
              disabled={!p.previousTrack}
              aria-label={
                p.previousTrack
                  ? `Previous track: ${p.previousTrack.title}`
                  : "No previous track"
              }
              title={
                p.previousTrack
                  ? `Previous: ${p.previousTrack.title}`
                  : "No previous track"
              }
            >
              <Artwork
                src={p.previousTrack?.cover_art_url}
                trackId={p.previousTrack?.id}
                alt={p.previousTrack?.title || "No previous track"}
              />
            </button>
            <button
              type="button"
              className="press-poster-cover"
              onClick={p.onTogglePlayback}
              disabled={
                !p.currentTrack || p.loadingTrackId === p.currentTrack.id
              }
              aria-label={
                p.currentTrack
                  ? `${p.isPlaying ? "Pause" : "Play"} current track: ${p.currentTrack.title}`
                  : "No current track"
              }
              title={
                p.currentTrack
                  ? `Current: ${p.currentTrack.title}`
                  : "No current track"
              }
            >
              <Artwork
                src={p.currentCover || p.currentTrack?.cover_art_url}
                trackId={p.currentTrack?.id}
                alt={p.currentTrack?.title || "No current track"}
              />
            </button>
            <button
              type="button"
              className="press-home-adjacent press-home-next"
              onClick={p.onNextTrack}
              disabled={!p.nextTrack}
              aria-label={
                p.nextTrack
                  ? `Next track: ${p.nextTrack.title}`
                  : "No next track"
              }
              title={
                p.nextTrack ? `Next: ${p.nextTrack.title}` : "No next track"
              }
            >
              <Artwork
                src={p.nextTrack?.cover_art_url}
                trackId={p.nextTrack?.id}
                alt={p.nextTrack?.title || "No next track"}
              />
            </button>
          </div>
          <div className="press-poster-annotation press-home-flow-copy">
            <span className="press-micro">
              FAMILIAR FAVOURITES. UNEXPECTED TURNS.
            </span>
            <h2>Start your Flow</h2>
            <p>
              {overview?.flow_session?.flow_label || "A mix shaped by you."}
            </p>
            <div className="press-home-flow-actions">
              <button
                className="press-solid-button"
                disabled={p.isFlowPending}
                aria-busy={!!p.isFlowPending}
                onClick={p.onFlow}
              >
                {p.isFlowPending ? (<RefreshCw size={18} className="spin-animation" />) : p.flowAction === "Pause Flow" ? (
                  <Pause size={18} />
                ) : (
                  <Play size={18} />
                )}
                {p.flowAction}
              </button>

            </div>
            <FamiliarityBar
              value={p.flowSteering}
              onChange={p.onFlowSteeringChange}
            />
          </div>
        </div>
        <div className="press-line-label">
          <span>
            {p.currentTrack
              ? `${p.currentTrack.title} / ${p.currentTrack.artist_name || "Unknown artist"}`
              : "SELECT A RECORD. START A NEW SIDE."}
          </span>
          <button
            aria-label="Refresh Home recommendations"
            onClick={p.onRefresh}
            disabled={p.isLoading}
          >
            <RefreshCw size={16} />
          </button>
        </div>
      </section>
      {sections.map(([title, items], i) => (
        <section
          className="press-home-selection"
          key={title}
          aria-labelledby={`press-home-list-${i}`}
        >
          <header className="press-home-list-heading">
            <span className="press-home-list-number" aria-hidden="true">
              {String(i + 1).padStart(2, "0")}
            </span>
            <div>
              <h2 id={`press-home-list-${i}`}>{title}</h2>
              <p>
                {
                  [
                    "Back to the records you’ve been playing.",
                    "Favourites ready for another spin.",
                    "Selected from your listening.",
                  ][i]
                }
              </p>
            </div>
            <span className="press-home-list-count">
              <strong>{items.length}</strong>RECORDS
            </span>
          </header>
          <div className="press-home-list-body">
            {items.length ? (
              <WindowedList
                items={items}
                label={title}
                rowHeight={88}
                renderItem={(t, index) => (
                  <div className="press-home-row">
                    <span className="press-row-number">
                      {String(index + 1).padStart(2, "0")}
                    </span>
                    <Artwork src={t.cover} trackId={t.id} />
                    <button
                      className="press-home-song"
                      onClick={() =>
                        t.id === p.currentTrack?.id
                          ? p.onTogglePlayback()
                          : p.onPlayTrack(
                              t.id,
                              `home:${title.toLowerCase().replace(/ /g, "_")}`,
                            )
                      }
                    >
                      <strong>{t.title}</strong>
                      <small>{t.artist || "Unknown artist"}</small>
                    </button>
                    {(t.explanation || title === "For You") && (
                      <button
                        className="press-icon"
                        aria-label="Why this track?"
                        title="Why this track?"
                        onClick={() =>
                          p.onExplain({
                            ...t,
                            artist: t.artist || "Unknown artist",
                          })
                        }
                      >
                        <Info size={18} />
                      </button>
                    )}
                    <NotInterestedButton trackId={t.id} title={t.title} />
                    {p.onEnqueueTrack && (
                      <button
                        className="press-icon"
                        aria-label={
                          p.queuedTrackIds?.has(t.id)
                            ? "Already in queue"
                            : "Add to queue"
                        }
                        disabled={p.queuedTrackIds?.has(t.id)}
                        onClick={() => p.onEnqueueTrack?.(t.id)}
                      >
                        {p.queuedTrackIds?.has(t.id) ? (
                          <Check size={18} />
                        ) : (
                          <ListPlus size={18} />
                        )}
                      </button>
                    )}
                    <button
                      className="press-icon"
                      aria-label={`${t.id === p.currentTrack?.id && p.isPlaying ? "Pause" : "Play"} ${t.title}`}
                      disabled={p.loadingTrackId === t.id}
                      onClick={() =>
                        t.id === p.currentTrack?.id
                          ? p.onTogglePlayback()
                          : p.onPlayTrack(t.id, "home")
                      }
                    >
                      {p.loadingTrackId === t.id ? (
                        <RefreshCw size={18} />
                      ) : t.id === p.currentTrack?.id && p.isPlaying ? (
                        <Pause size={18} />
                      ) : (
                        <Play size={18} />
                      )}
                    </button>
                  </div>
                )}
              />
            ) : (
              <p className="press-next-empty">
                {p.isLoading
                  ? "Loading your records…"
                  : title === "For You" ? "Your recommendations are getting ready. Try Refresh, or choose your music taste in Settings." : "Your listening will bring more records here."}
              </p>
            )}
          </div>
        </section>
      ))}
      <section className="press-home-moment">
        <div className="press-home-moment-copy">
          <div className="press-home-section-heading">
            <span className="press-micro">04 / FROM YOUR SHELVES</span>
            <h2>Library Moment</h2>
          </div>
          <h3>{moment?.album_title || "Your next favourite awaits"}</h3>
          <p className="press-home-moment-artist">
            {moment?.artist_name || "Add a music folder in Settings to begin."}
          </p>
          {moment?.subtitle && (
            <p className="press-home-moment-note">{moment.subtitle}</p>
          )}
          {moment && (
            <div className="press-home-moment-footer">
              <span className="press-micro">
                {moment.total_tracks}{" "}
                {moment.total_tracks === 1 ? "TRACK" : "TRACKS"}
                {moment.release_year ? ` / ${moment.release_year}` : ""}
              </span>
              <button
                className="press-solid-button"
                disabled={!p.onOpenAlbum}
                onClick={() => p.onOpenAlbum?.(moment.album_id)}
              >
                Open record
                <ArrowUpRight size={18} />
              </button>
            </div>
          )}
        </div>
        <button
          className="press-home-moment-art"
          disabled={!momentPlayId || momentLoading}
          onClick={() => {
            if (!momentPlayId) return;
            if (momentIsCurrent) p.onTogglePlayback();
            else p.onPlayTrack(momentPlayId, "home_library_moment");
          }}
          aria-label={moment ? `${momentLoading ? "Loading" : momentIsCurrent && p.isPlaying ? "Pause" : "Play"} ${moment.album_title}` : "No featured album"}
        >
          <Artwork
            src={momentCover}
            trackId={moment?.first_track_id || momentTrack?.track_id}
            alt={moment?.album_title || "No featured album"}
          />
        </button>
      </section>
      <section className="press-home-week">
        <div className="press-home-week-heading">
          <div className="press-home-section-heading">
            <span className="press-micro">05 / IN ROTATION</span>
            <h2>Your Week</h2>
          </div>
          <button className="press-text-button" onClick={p.onDiscover}>
            Keep exploring
            <ArrowUpRight size={18} />
          </button>
        </div>
        <dl className="press-home-week-stats">
          <div>
            <dt>Listening time</dt>
            <dd>{overview?.your_week?.formatted_time || "0m"}</dd>
          </div>
          <div>
            <dt>Tracks played</dt>
            <dd>{overview?.your_week?.track_count || 0}</dd>
          </div>
          <div>
            <dt>Artists</dt>
            <dd>{overview?.your_week?.artist_count || 0}</dd>
          </div>
        </dl>
      </section>
    </div>
  );
});
