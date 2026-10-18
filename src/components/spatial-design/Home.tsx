import { memo } from "react";
import {
  ArrowDownRight,
  ArrowUpRight,
  MoveUpRight,
  Pause,
  Play,
} from "lucide-react";
import { ALBUMS, PLAYLISTS, SpatialTrack, TRACKS } from "./data";
import { Artwork, MusicCard } from "./Primitives";

export const Home = memo(function Home({
  onPlay,
  onAlbum,
  onPlaylist,
  onDiscover,
  flow,
  onFlow,
  current,
}: {
  onPlay: (id: string) => void;
  onAlbum: (id: string) => void;
  onPlaylist: (id: string) => void;
  onDiscover: () => void;
  flow: boolean;
  onFlow: () => void;
  current: SpatialTrack;
}) {
  return (
    <div className="sp-page sp-home">
      <header className="sp-home-heading">
        <p>Your music, in motion.</p>
        <h1>
          Good to see you<span className="sp-orange">.</span>
        </h1>
      </header>
      <section className="sp-flow" aria-labelledby="flow-heading">
        <div className="sp-flow-copy">
          <span className="sp-kicker">Made for wherever you are</span>
          <h2 id="flow-heading">
            Find
            <br />
            your <em>flow.</em>
          </h2>
          <p>
            Familiar favourites.
            <br />
            Unexpected turns. All you.
          </p>
          <button className="sp-primary" onClick={onFlow}>
            {flow ? (
              <Pause size={18} fill="currentColor" />
            ) : (
              <Play size={18} fill="currentColor" />
            )}
            {flow ? "Pause Flow preview" : "Start Flow"}
          </button>
          <span className="sp-flow-caption">
            {flow
              ? "Flow preview is on. Follow the music."
              : "A mix that moves with you."}
          </span>
        </div>
        <div className="sp-flow-art">
          <button
            className="sp-flow-back sp-flow-back-left"
            onClick={() => onAlbum("modal-soul")}
            aria-label="Open Modal Soul"
          >
            <Artwork src={ALBUMS[1].artwork} title="Modal Soul by Nujabes" />
          </button>
          <button
            className="sp-flow-back sp-flow-back-right"
            onClick={() => onAlbum("con-todo")}
            aria-label="Open Con Todo El Mundo"
          >
            <Artwork
              src={ALBUMS[2].artwork}
              title="Con Todo El Mundo by Khruangbin"
            />
          </button>
          <button
            className="sp-flow-front"
            onClick={() => onPlay(current.id)}
            aria-label={`Play ${current.title}`}
          >
            <Artwork
              src={current.artwork}
              title={`${current.album} by ${current.artist}`}
              eager
            />
          </button>
          <div className="sp-flow-art-caption">
            <span>Start with a feeling</span>
            <ArrowDownRight size={20} />
          </div>
        </div>
        <div className="sp-flow-wordmark" aria-hidden="true">
          FLOW
        </div>
      </section>
      <section className="sp-rotation" aria-labelledby="rotation-heading">
        <div className="sp-section-title">
          <div>
            <p className="sp-kicker">Back in good company</p>
            <h2 id="rotation-heading">Your recent rotation</h2>
          </div>
          <button className="sp-text-action" onClick={onDiscover}>
            Keep exploring
            <ArrowUpRight size={17} />
          </button>
        </div>
        <div className="sp-rotation-covers">
          {[TRACKS[9], TRACKS[6], TRACKS[3], TRACKS[21]].map((track, index) => (
            <div
              key={track.id}
              className={`sp-rotation-item sp-rotation-item-${index}`}
            >
              <MusicCard track={track} onPlay={onPlay} onAlbum={onAlbum} />
            </div>
          ))}
        </div>
      </section>
      <section className="sp-rediscover">
        <div className="sp-rediscover-copy">
          <p className="sp-kicker">Still sounds like the first time</p>
          <h2>
            Worth another
            <br />
            listen.
          </h2>
          <p>Some records never really leave you.</p>
          <button
            className="sp-text-action"
            onClick={() => onAlbum("moon-safari")}
          >
            Rediscover Moon Safari
            <MoveUpRight size={18} />
          </button>
        </div>
        <button
          className="sp-rediscover-art"
          onClick={() => onAlbum("moon-safari")}
          aria-label="Open Moon Safari by Air"
        >
          <Artwork src={ALBUMS[4].artwork} title="Moon Safari by Air" />
          <span>
            Air<span>Moon Safari / 1998</span>
          </span>
        </button>
        <div className="sp-home-playlist">
          <p className="sp-kicker">A playlist for later</p>
          <button onClick={() => onPlaylist(PLAYLISTS[0].id)}>
            <Artwork src={PLAYLISTS[0].cover} title="After hours playlist" />
            <h3>
              After
              <br />
              hours
              <ArrowUpRight size={22} />
            </h3>
          </button>
          <p>Keep the night a little longer.</p>
        </div>
      </section>
    </div>
  );
});
