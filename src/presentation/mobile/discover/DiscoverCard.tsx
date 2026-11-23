import { memo } from "react";
import { Bookmark, BookmarkCheck, ListPlus, Play } from "lucide-react";
import { DownloadButton } from "../../../components/DownloadButton";
import { Sleeve } from "../Sleeve";
import { joinMeta } from "../format";
import type { DiscoveryRecommendation } from "../../../types";
import type { MobileScreenProps } from "../types";
import { queueRecommendation } from "../phoneToasts";

const pad = (n: number) => String(n).padStart(2, "0");

/** The id a playlist stores for this song: its local match when real, else the online id (as on the PC). */
export const playlistTrackId = (rec: DiscoveryRecommendation) => {
  const local = rec.matched_local_track_id;
  return local && !local.startsWith("online:") && !local.startsWith("itunes:") ? local : rec.external_track_id;
};

/** Add to playlist, like the PC's Discover card; filled when the song is already in one. */
function PlaylistButton({ rec, c }: { rec: DiscoveryRecommendation } & MobileScreenProps) {
  const id = playlistTrackId(rec);
  const saved = (c.regularTrackPlaylistMap[id]?.length ?? 0) > 0;
  const Icon = saved ? BookmarkCheck : Bookmark;
  return (
    <button type="button" className={`m-discover-card__btn ${saved ? "is-on" : ""}`} aria-label={saved ? "In a playlist, add to another" : "Add to playlist"}
      onClick={() => c.handleOpenAddToPlaylistModal({ id, title: rec.title, artist: rec.artist, album: rec.album, cover_art_url: rec.cover_art_url })}>
      <Icon size={18} strokeWidth={2} />
    </button>
  );
}

export /** Discover card (Figma C1): rule + red index, 150px sleeve, title, artist and four actions (play, queue, playlist, download). */
const DiscoverCard = memo(function DiscoverCard({
  rec, index, c,
}: { rec: DiscoveryRecommendation; index: number } & MobileScreenProps) {
  return (
    <div className="m-discover-card">
      <div className="m-discover-card__header">
        <span className="m-kicker">{pad(index)}</span>
      </div>
      <Sleeve size="card" src={rec.cover_art_url} title={rec.title} onPress={() => void c.handlePlayOnlineTrack(rec)} />
      <span className="m-discover-card__title">{rec.title}</span>
      <span className="m-discover-card__artist">{joinMeta(rec.artist, rec.album) || "Unknown artist"}</span>
      <div className="m-discover-card__actions">
        <button type="button" className="m-discover-card__btn" onClick={() => void c.handlePlayOnlineTrack(rec)} aria-label={`Play ${rec.title}`}>
          <Play size={18} strokeWidth={2} />
        </button>
        <button type="button" className="m-discover-card__btn" onClick={() => void queueRecommendation(c, rec)} aria-label="Add to queue">
          <ListPlus size={18} strokeWidth={2} />
        </button>
        <PlaylistButton rec={rec} c={c} />
        {!rec.matched_local_track_id && (
          <span className="m-discover-card__download">
            <DownloadButton trackId={rec.external_track_id} sourceId={rec.playback_source?.track_id} title={rec.title} compact />
          </span>
        )}
      </div>
    </div>
  );
});
