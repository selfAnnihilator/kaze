import { Heart } from "lucide-react";
import { TrackRow } from "./TrackRow";
import { joinMeta } from "./format";
import { openTrackSheet } from "./sheets/trackSheet";
import type { CollectionTrackItem } from "../../components/views/collection/types";
import type { MobileScreenProps } from "./types";
import { canRate } from "../../services/ratingRule";

const isRemote = (id?: string) => !id || id.startsWith("online:") || id.startsWith("itunes:");
/** The song's id in this library, if it has one (same rule as liking). */
const libraryId = (item: CollectionTrackItem) =>
  [item.matched_local_track_id, item.rawLocalTrack?.id, item.id].find((id) => !isRemote(id));

/** A collection song (Figma D6): sleeve, title, artist · album, heart, then ⋮ for songs in the library. */
export function CollectionRow({ item, c }: { item: CollectionTrackItem } & MobileScreenProps) {
  const localId = libraryId(item);
  const liked = c.likedTrackIds.has(localId || item.id);
  // A song from the user's own folders takes no like.
  const localFile = !!item.rawLocalTrack && !canRate(item.rawLocalTrack);
  return (
    <TrackRow
      noIndex
      title={item.title}
      subtitle={joinMeta(item.artist, item.album)}
      src={item.cover_art_url}
      trackId={localId}
      active={!!localId && localId === c.activePlayingTrackId}
      onPress={() => void c.handlePlayCollectionTrack(item)}
      trailing={
        <button type="button" className={`m-track-row__action-btn ${liked ? "is-red" : ""}`} aria-pressed={liked} aria-label={liked ? "Unlike" : "Like"} disabled={localFile}
          onClick={() => void c.handleToggleCollectionLike(item, liked)}>
          <Heart size={20} strokeWidth={2} fill={liked ? "currentColor" : "none"} />
        </button>
      }
      onMore={localId ? () => openTrackSheet("actions", { id: localId, title: item.title, artist: item.artist, album: item.album, cover: item.cover_art_url, localFile }) : undefined}
    />
  );
}
