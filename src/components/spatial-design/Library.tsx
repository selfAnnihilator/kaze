import { memo, useMemo, useState } from "react";
import { List, RowComponentProps } from "react-window";
import {
  ArrowDown,
  ArrowUp,
  ArrowUpRight,
  GripVertical,
  Heart,
  Play,
} from "lucide-react";
import { ALBUMS, PLAYLISTS, SpatialTrack, TRACKS, formatTime } from "./data";
import { Artwork } from "./Primitives";

interface RowProps {
  tracks: SpatialTrack[];
  currentId: string;
  onPlay: (id: string) => void;
  onAlbum: (id: string) => void;
  onMove?: (source: string, destination: string) => void;
}

function TrackRow({
  ariaAttributes,
  index,
  style,
  tracks,
  currentId,
  onPlay,
  onAlbum,
  onMove,
}: RowComponentProps<RowProps>) {
  const track = tracks[index];
  return (
    <div
      {...ariaAttributes}
      style={style}
      className={`sp-track-row ${track.id === currentId ? "sp-current-row" : ""}`}
      draggable={Boolean(onMove)}
      onDragStart={(event) => {
        event.dataTransfer.setData("text/plain", track.id);
        event.dataTransfer.effectAllowed = "move";
      }}
      onDragOver={(event) => {
        if (onMove) event.preventDefault();
      }}
      onDrop={(event) => {
        if (onMove) {
          event.preventDefault();
          onMove(event.dataTransfer.getData("text/plain"), track.id);
        }
      }}
    >
      <span className="sp-track-number">
        {onMove ? (
          <GripVertical size={16} />
        ) : (
          String(index + 1).padStart(2, "0")
        )}
      </span>
      <button
        className="sp-track-art"
        onClick={() => onPlay(track.id)}
        aria-label={`Play ${track.title}`}
      >
        <Artwork src={track.artwork} title={track.album} />
        <Play size={15} fill="currentColor" />
      </button>
      <button className="sp-track-title" onClick={() => onPlay(track.id)}>
        <strong>{track.title}</strong>
        <span>{track.artist}</span>
      </button>
      <button className="sp-track-album" onClick={() => onAlbum(track.albumId)}>
        {track.album}
      </button>
      {onMove && (
        <div className="sp-order-actions">
          <button
            className="sp-icon"
            disabled={index === 0}
            aria-label={`Move ${track.title} earlier`}
            onClick={() => onMove(track.id, tracks[index - 1].id)}
          >
            <ArrowUp size={15} />
          </button>
          <button
            className="sp-icon"
            disabled={index === tracks.length - 1}
            aria-label={`Move ${track.title} later`}
            onClick={() => onMove(track.id, tracks[index + 1].id)}
          >
            <ArrowDown size={15} />
          </button>
        </div>
      )}
      <span className="sp-duration">{formatTime(track.duration)}</span>
    </div>
  );
}

export const TrackList = memo(function TrackList({
  tracks,
  currentId,
  onPlay,
  onAlbum,
  onMove,
  label = "Tracks",
}: RowProps & { label?: string }) {
  const rowProps = useMemo(
    () => ({ tracks, currentId, onPlay, onAlbum, onMove }),
    [tracks, currentId, onPlay, onAlbum, onMove],
  );
  return (
    <div className="sp-track-list" aria-label={label}>
      <div className="sp-track-columns" aria-hidden="true">
        <span>#</span>
        <span>Track / Artist</span>
        <span>Album</span>
        <span>Time</span>
      </div>
      {tracks.length ? (
        <List<RowProps>
          rowCount={tracks.length}
          rowHeight={76}
          rowComponent={TrackRow}
          rowProps={rowProps}
          overscanCount={3}
          style={{ height: Math.min(tracks.length * 76, 540), minHeight: 76 }}
          aria-label={label}
        />
      ) : (
        <div className="sp-empty">
          <Heart size={28} />
          <h3>No tracks here yet</h3>
          <p>Try another search or browse your albums.</p>
        </div>
      )}
    </div>
  );
});

interface BrowseProps {
  currentId: string;
  onPlay: (id: string) => void;
  onAlbum: (id: string) => void;
  onPlaylist: (id: string) => void;
  query: string;
  liked: Set<string>;
}

export const Library = memo(function Library({
  currentId,
  onPlay,
  onAlbum,
  onPlaylist,
  query,
  liked,
}: BrowseProps) {
  const [tab, setTab] = useState("Albums");
  const albums = useMemo(
    () =>
      ALBUMS.filter((album) =>
        `${album.title} ${album.artist}`
          .toLocaleLowerCase()
          .includes(query.toLocaleLowerCase()),
      ),
    [query],
  );
  const tracks = useMemo(
    () =>
      TRACKS.filter(
        (track) =>
          (tab !== "Liked" || liked.has(track.id)) &&
          `${track.title} ${track.artist} ${track.album}`
            .toLocaleLowerCase()
            .includes(query.toLocaleLowerCase()),
      ),
    [query, tab, liked],
  );
  return (
    <div className="sp-page sp-library">
      <div className="sp-page-heading">
        <p className="sp-kicker">The music you make yours</p>
        <h1>
          Your library<span className="sp-orange">.</span>
        </h1>
        <p>{ALBUMS.length} albums. A thousand places to go.</p>
      </div>
      <div className="sp-tabs" role="tablist" aria-label="Library categories">
        {["Albums", "Tracks", "Playlists", "Liked"].map((name) => (
          <button
            key={name}
            role="tab"
            aria-selected={tab === name}
            onClick={() => setTab(name)}
          >
            {name}
          </button>
        ))}
      </div>
      {tab === "Albums" && (
        <div className="sp-album-shelf">
          {albums.map((album, index) => (
            <article
              key={album.id}
              className={`sp-shelf-album sp-shelf-${index % 5}`}
            >
              <button
                className="sp-cover-button"
                onClick={() => onAlbum(album.id)}
                aria-label={`Open ${album.title} by ${album.artist}`}
              >
                <Artwork
                  src={album.artwork}
                  title={`${album.title} by ${album.artist}`}
                  eager={index < 3}
                />
                <span className="sp-cover-play">
                  <ArrowUpRight size={25} />
                </span>
              </button>
              <div className="sp-shelf-caption">
                <span className="sp-shelf-index">
                  {String(index + 1).padStart(2, "0")}
                </span>
                <div>
                  <button
                    className="sp-card-title"
                    onClick={() => onAlbum(album.id)}
                  >
                    {album.title}
                  </button>
                  <p>{album.artist}</p>
                </div>
              </div>
            </article>
          ))}
        </div>
      )}
      {tab === "Albums" && !albums.length && (
        <div className="sp-empty">
          <h3>No matching albums</h3>
          <p>Try a different artist or album name.</p>
        </div>
      )}
      {(tab === "Tracks" || tab === "Liked") && (
        <TrackList
          tracks={tracks}
          currentId={currentId}
          onPlay={onPlay}
          onAlbum={onAlbum}
          label={tab === "Liked" ? "Liked tracks" : "Library tracks"}
        />
      )}
      {tab === "Playlists" && (
        <div className="sp-playlists">
          {PLAYLISTS.map((playlist, index) => (
            <button
              key={playlist.id}
              className="sp-playlist"
              onClick={() => onPlaylist(playlist.id)}
            >
              <span className="sp-playlist-number">0{index + 1}</span>
              <Artwork src={playlist.cover} title={playlist.name} />
              <div>
                <h2>{playlist.name}</h2>
                <p>{playlist.description}</p>
                <span>{playlist.tracks.length} tracks</span>
              </div>
              <ArrowUpRight size={26} />
            </button>
          ))}
        </div>
      )}
    </div>
  );
});

export const Collection = memo(function Collection({
  id,
  playlist = false,
  currentId,
  onPlay,
  onAlbum,
  onPlaySelection,
}: { id: string; playlist?: boolean } & Pick<
  RowProps,
  "currentId" | "onPlay" | "onAlbum"
> & { onPlaySelection: (tracks: SpatialTrack[]) => void }) {
  const album = ALBUMS.find((item) => item.id === id) ?? ALBUMS[0];
  const list = PLAYLISTS.find((item) => item.id === id) ?? PLAYLISTS[0];
  const tracks = useMemo(
    () =>
      playlist
        ? list.tracks
        : TRACKS.filter((item) => item.albumId === album.id),
    [playlist, list, album],
  );
  const title = playlist ? list.name : album.title;
  return (
    <div className="sp-page sp-collection">
      <div className="sp-collection-heading">
        <p className="sp-kicker">
          {playlist ? "Playlist" : `${album.year} / ${album.genre}`}
        </p>
        <h1>{title}</h1>
        <p>{playlist ? list.description : album.artist}</p>
        <button className="sp-primary" onClick={() => onPlaySelection(tracks)}>
          <Play size={17} fill="currentColor" />
          Play {playlist ? "playlist" : "album"}
        </button>
        <span className="sp-collection-count">
          {tracks.length} sample tracks
        </span>
      </div>
      <div className="sp-collection-art">
        <Artwork
          src={playlist ? list.cover : album.artwork}
          title={title}
          eager
        />
        <span>{playlist ? "Made for the moment" : album.artist}</span>
      </div>
      <div className="sp-collection-tracks">
        <h2>Track list</h2>
        <TrackList
          tracks={tracks}
          currentId={currentId}
          onPlay={onPlay}
          onAlbum={onAlbum}
          label={`${title} tracks`}
        />
      </div>
      <div className="sp-collection-note">
        {playlist
          ? "Keep the feeling going."
          : "A record worth getting lost in."}
        <p>Every track has somewhere to take you.</p>
      </div>
    </div>
  );
});
