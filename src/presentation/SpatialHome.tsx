import { NotInterestedButton } from "../components/NotInterestedButton";
import { memo } from "react";
import {
  ArrowDownRight,
  ArrowUpRight,
  Check,
  Info,
  ListPlus,
  Pause,
  Play,
  RefreshCw,
} from "lucide-react";
import type { FlowExplanation, HomeOverview, Track } from "../types";
import { Artwork } from "./Artwork";
import type { PlaybackArtworkTrack } from "./queuePresentation";
import { FamiliarityDial } from "./FamiliarityDial";
import { WindowedList } from "./WindowedList";
import { presentFlowExplanation } from "../flowExplanation";
import { isCurrentPlaybackTrack } from "../localTrackMatch";

type MusicItem = {
  id: string;
  title: string;
  artist: string;
  album?: string;
  cover?: string;
  explanation?: FlowExplanation | null;
};
type Props = {
  overview: HomeOverview | null;
  currentTrack?: Track | null;
  currentCover?: string | null;
  previousTrack?: Track;
  nextTrack?: PlaybackArtworkTrack;
  onPreviousTrack: () => void;
  onNextTrack: () => void;
  isLoading: boolean;
  isPlaying: boolean;
  flowAction: string;
  onFlow: () => void;
  onStopFlow: () => void;
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
};

/** The approved Home composition, populated exclusively from the real overview. */
export const SpatialHome = memo(function SpatialHome(props: Props) {
  const {
    overview,
    currentTrack,
    isPlaying,
    isLoading,
    onPlayTrack,
    onEnqueueTrack,
    onExplain,
  } = props;
  const recent: MusicItem[] = (overview?.recent_rotation || []).map((item) => ({
    id: item.track_id,
    title: item.title,
    artist: item.artist,
    album: item.album,
    cover: item.cover_art_url,
  }));
  const rediscover: MusicItem[] = (overview?.rediscover || []).map((item) => ({
    id: item.track_id,
    title: item.title,
    artist: item.artist,
    album: item.album,
    cover: item.cover_art_url,
    explanation: item.explanation,
  }));
  const play = (item: MusicItem, source: string) =>
    item.id === currentTrack?.id
      ? props.onTogglePlayback()
      : onPlayTrack(item.id, source);
  const label = (item: MusicItem) =>
    `${props.loadingTrackId === item.id ? "Loading" : item.id === currentTrack?.id && isPlaying ? "Pause" : "Play"} ${item.title}`;
  const moment = overview?.library_moment;
  const momentTrack =
    currentTrack &&
    moment &&
    currentTrack.album_title?.toLowerCase() === moment.album_title.toLowerCase()
      ? currentTrack.id
      : recent.find(
          (item) =>
            item.album?.toLowerCase() === moment?.album_title.toLowerCase(),
        )?.id;
  const actions = (item: MusicItem, source: string) => (
    <div className="spatial-music-actions">
      {item.explanation && (
        <button
          className="player-icon-btn"
          title="Why this track?"
          aria-label="Why this track?"
          onClick={() => onExplain(item)}
        >
          <Info size={15} />
        </button>
      )}
      <NotInterestedButton trackId={item.id} title={item.title} />
      {onEnqueueTrack && (
        <button
          className="player-icon-btn"
          title={
            props.queuedTrackIds?.has(item.id)
              ? "Already in queue"
              : "Add to queue"
          }
          onClick={() => onEnqueueTrack(item.id)}
        >
          {props.queuedTrackIds?.has(item.id) ? (
            <Check size={15} />
          ) : (
            <ListPlus size={15} />
          )}
        </button>
      )}
      <button
        className="player-icon-btn"
        aria-label={label(item)}
        disabled={props.loadingTrackId === item.id}
        onClick={() => play(item, source)}
      >
        {props.loadingTrackId === item.id ? (
          <RefreshCw size={15} className="spin-animation" />
        ) : item.id === currentTrack?.id && isPlaying ? (
          <Pause size={15} />
        ) : (
          <Play size={15} />
        )}
      </button>
    </div>
  );
  const row = (item: MusicItem, source: string) => (
    <div className="spatial-music-row">
      <Artwork src={item.cover} trackId={item.id} />
      <button
        className="spatial-music-title"
        onClick={() => play(item, source)}
      >
        <strong>{item.title}</strong>
        <span>{item.artist}</span>
      </button>
      {actions(item, source)}
    </div>
  );
  return (
    <div className="home-view spatial-home">
      <header className="spatial-home-heading">
        <h1>
          Good to see you<span>.</span>
        </h1>
        <div>
          <p>Your music, in motion.</p>
          <button
            className="player-icon-btn"
            title="Refresh Home recommendations"
            aria-label="Refresh Home recommendations"
            onClick={props.onRefresh}
            disabled={isLoading}
          >
            <RefreshCw
              size={15}
              className={isLoading ? "spin-animation" : ""}
            />
          </button>
        </div>
      </header>
      <section
        className="home-flow spatial-flow"
        aria-labelledby="spatial-flow-title"
      >
        <div className="spatial-flow-copy">
          <span className="interface-eyebrow">Made for wherever you are</span>
          <h2 id="spatial-flow-title">
            Find
            <br />
            your <em>flow.</em>
          </h2>
          <p>
            Familiar favourites.
            <br />
            Unexpected turns. All you.
          </p>
          <div className="spatial-flow-actions">
            <button
              className="btn btn-primary"
              onClick={props.onFlow}
              disabled={isLoading}
            >
              {props.flowAction === "Pause Flow" ? (
                <Pause size={18} fill="currentColor" />
              ) : (
                <Play size={18} fill="currentColor" />
              )}
              {props.flowAction}
            </button>
            {overview?.flow_session?.is_active && (
              <button
                className="spatial-text-action"
                onClick={props.onStopFlow}
              >
                Stop Flow
              </button>
            )}
          </div>
          <span
            className="spatial-flow-caption"
            title={
              overview?.flow_session?.last_track_title
                ? `Last in Flow: ${overview.flow_session.last_track_title} · ${overview.flow_session.last_track_artist || ""}`
                : undefined
            }
          >
            {overview?.flow_session?.flow_label || "A mix that moves with you."}
          </span>
        </div>
        <div className="spatial-flow-artwork">
          <button
            type="button"
            className="spatial-flow-back spatial-flow-back-0"
            disabled={!props.previousTrack}
            onClick={props.onPreviousTrack}
            aria-label={props.previousTrack ? `Previous track: ${props.previousTrack.title}` : "No previous track"}
            title={props.previousTrack ? `Previous: ${props.previousTrack.title}` : "No previous track"}
          >
            <Artwork
              src={props.previousTrack?.cover_art_url}
              trackId={props.previousTrack?.id}
              alt={props.previousTrack?.title || "No previous track"}
            />
          </button>
          <button
            type="button"
            className="spatial-flow-front"
            disabled={!currentTrack || props.loadingTrackId === currentTrack.id}
            onClick={props.onTogglePlayback}
            aria-label={currentTrack ? `${isPlaying ? "Pause" : "Play"} current track: ${currentTrack.title}` : "No current track"}
            title={currentTrack ? `Current: ${currentTrack.title}` : "No current track"}
          >
            <Artwork
              src={props.currentCover || currentTrack?.cover_art_url}
              trackId={currentTrack?.id}
              alt={currentTrack?.title || "No current track"}
            />
          </button>
          <button
            type="button"
            className="spatial-flow-back spatial-flow-back-1"
            disabled={!props.nextTrack}
            onClick={props.onNextTrack}
            aria-label={props.nextTrack ? `Next track: ${props.nextTrack.title}` : "No next track"}
            title={props.nextTrack ? `Next: ${props.nextTrack.title}` : "No next track"}
          >
            <Artwork
              src={props.nextTrack?.cover_art_url}
              trackId={props.nextTrack?.id}
              alt={props.nextTrack?.title || "No next track"}
            />
          </button>
          <div className="spatial-flow-art-caption">
            <span>Start with a feeling</span>
            <ArrowDownRight size={20} />
          </div>
          <div className="spatial-flow-dial">
            <FamiliarityDial
              value={props.flowSteering}
              onChange={props.onFlowSteeringChange}
            />
          </div>
        </div>
        <div className="spatial-flow-wordmark" aria-hidden="true">
          FLOW
        </div>
      </section>
      <section
        className="home-rotation spatial-rotation"
        aria-labelledby="spatial-rotation-title"
      >
        <div className="spatial-section-title">
          <div>
            <p className="interface-eyebrow">Back in good company</p>
            <h2 id="spatial-rotation-title" aria-label="Recent Rotation">
              Your recent rotation
            </h2>
          </div>
          {props.onDiscover && (
            <button className="spatial-text-action" onClick={props.onDiscover}>
              Keep exploring
              <ArrowUpRight size={17} />
            </button>
          )}
        </div>
        {recent.length ? (
          <div className="spatial-rotation-covers">
            {recent.slice(0, 4).map((item) => (
              <article className="spatial-rotation-item" key={item.id}>
                <button
                  className="spatial-music-cover"
                  onClick={() => play(item, "recent_rotation")}
                  aria-label={label(item)}
                >
                  <Artwork
                    src={item.cover}
                    trackId={item.id}
                    alt={item.album || item.title}
                  />
                </button>
                <div className="spatial-music-caption">
                  <strong>{item.title}</strong>
                  <span>{item.artist}</span>
                </div>
                {actions(item, "recent_rotation")}
              </article>
            ))}
          </div>
        ) : (
          <p className="interface-empty">
            No recent listening history yet. Start your Flow or play tracks from
            your library to build rotation.
          </p>
        )}
        {recent.length > 4 && (
          <details className="spatial-more-music">
            <summary>More from your recent rotation</summary>
            <WindowedList
              items={recent.slice(4)}
              rowHeight={68}
              label="More recent rotation"
              renderItem={(item) => row(item, "recent_rotation")}
            />
          </details>
        )}
      </section>
      <section
        className="home-rediscover spatial-rediscovery"
        aria-labelledby="spatial-rediscover-title"
      >
        <div className="spatial-rediscovery-copy">
          <p className="interface-eyebrow">Still sounds like the first time</p>
          <h2 id="spatial-rediscover-title" aria-label="Rediscover">
            Worth another
            <br />
            listen.
          </h2>
          <p>Some records never really leave you.</p>
          {rediscover[0] && (
            <>
              <button
                className="spatial-text-action"
                onClick={() => play(rediscover[0], "rediscover")}
              >
                {label(rediscover[0])}
                <ArrowUpRight size={18} />
              </button>
              <p className="spatial-track-reason">
                {presentFlowExplanation(rediscover[0].explanation).title}
              </p>
              {actions(rediscover[0], "rediscover")}
            </>
          )}
        </div>
        {rediscover[0] ? (
          <article className="spatial-rediscovery-art">
            <button
              className="spatial-music-cover"
              onClick={() => play(rediscover[0], "rediscover")}
              aria-label={label(rediscover[0])}
            >
              <Artwork
                src={rediscover[0].cover}
                trackId={rediscover[0].id}
                alt={rediscover[0].album || rediscover[0].title}
              />
            </button>
            <div className="spatial-music-caption">
              <strong>{rediscover[0].title}</strong>
              <span>{rediscover[0].artist}</span>
            </div>
          </article>
        ) : (
          <p className="interface-empty">
            As you build your listening library, forgotten favourites will
            surface here.
          </p>
        )}
        <aside className="spatial-library-moment">
          <h3 className="interface-eyebrow">Library Moment</h3>
          {moment ? (
            <>
              <button
                className="spatial-music-cover"
                disabled={!props.onOpenAlbum}
                onClick={() => props.onOpenAlbum?.(moment.album_id)}
                aria-label={`Open ${moment.album_title}`}
              >
                <Artwork
                  src={moment.cover_art_path}
                  trackId={momentTrack}
                  alt={`${moment.album_title} by ${moment.artist_name}`}
                />
              </button>
              <h2>
                {moment.album_title}
                <ArrowUpRight size={20} />
              </h2>
              <p>{moment.artist_name}</p>
              <p>{moment.subtitle || `${moment.total_tracks} tracks`}</p>
            </>
          ) : (
            <p>Add music folders to your library to unlock album spotlights.</p>
          )}
        </aside>
      </section>
      {rediscover.length > 1 && (
        <details className="spatial-more-music">
          <summary>More to rediscover</summary>
          <WindowedList
            items={rediscover.slice(1)}
            rowHeight={68}
            label="More to rediscover"
            renderItem={(item) => row(item, "rediscover")}
          />
        </details>
      )}
      <div className="spatial-home-extras">
        <section>
          <h2>For You</h2>
          <p>Songs informed by your listening.</p>
          {overview?.for_you.length ? (
            overview.for_you.slice(0, 5).map((rec) => {
              const isCurrent = isCurrentPlaybackTrack(rec, currentTrack);
              const item = {
                id: rec.track_id,
                title: rec.title,
                artist: rec.artist_name || "Unknown artist",
                explanation: rec.explanation,
              };
              return (
                <div className="spatial-music-row" key={rec.track_id}>
                  <div className="spatial-music-caption">
                    <strong>{rec.title}</strong>
                    <span>{rec.artist_name || "Unknown artist"}</span>
                  </div>
                  <NotInterestedButton trackId={rec.track_id} title={rec.title} />
                  <button
                    className="player-icon-btn"
                    aria-label={`Why: ${presentFlowExplanation(rec.explanation).title}`}
                    title={`Why: ${presentFlowExplanation(rec.explanation).title}`}
                    onClick={() => onExplain(item)}
                  >
                    <Info size={14} />
                  </button>
                  <button
                    className="player-icon-btn"
                    aria-label={`${isCurrent ? (isPlaying ? "Pause" : "Resume") : "Play"} ${rec.title}`}
                    onClick={() =>
                      isCurrent
                        ? props.onTogglePlayback()
                        : onPlayTrack(rec.track_id, "for_you")
                    }
                  >
                    {isCurrent && isPlaying ? (
                      <Pause size={15} />
                    ) : (
                      <Play size={15} />
                    )}
                  </button>
                </div>
              );
            })
          ) : (
            <p className="interface-empty">
              Start playing tracks to establish your musical taste profile.
            </p>
          )}
        </section>
        <section className="spatial-week">
          <h2>Your Week</h2>
          <p>Listening summary for the past 7 days.</p>
          <dl>
            <div>
              <dt>Time listened</dt>
              <dd>{overview?.your_week?.formatted_time || "0m"}</dd>
            </div>
            <div>
              <dt>Tracks played</dt>
              <dd>{overview?.your_week?.track_count || 0}</dd>
            </div>
            <div>
              <dt>Artists explored</dt>
              <dd>{overview?.your_week?.artist_count || 0}</dd>
            </div>
          </dl>
        </section>
      </div>
    </div>
  );
});
