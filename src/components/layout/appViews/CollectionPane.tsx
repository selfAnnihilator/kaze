import type { CollectionData } from "../../views/CollectionDetailView";
import { CollectionDetailView } from "../../views/CollectionDetailView";
import type { AppViewsContext } from "./context";

/** The open album, playlist or mix. */
export function CollectionPane({ c, collection }: { c: AppViewsContext; collection: CollectionData }) {
  return (
    <CollectionDetailView
      key={`${collection.type}:${collection.id}`}
      collection={collection}
      withinArchive={c.isArchiveLiked}
      isLoadingTracks={c.isLoadingCollectionTracks}
      loadingTrackId={c.loadingTrackId}
      onBack={() => {
        if (!c.navigateBack())
          c.navigate({ ...c.destination, collection: null });
      }}
      onPlayTrack={c.handlePlayCollectionTrack}
      onPlayAll={c.handlePlayAllCollection}
      onShuffleAll={c.handleShuffleCollection}
      onAddToWishlist={(track) =>
        c.handleAddToWishlist(track.title, track.artist, track.album)
      }
      onSaveToPlaylists={c.handleSaveCollectionToPlaylists}
      currentPlayingTrackId={
        c.playbackState.is_playing
          ? c.playbackState.current_track?.id
          : undefined
      }
      isPlaying={c.isPlayingThisCollection}
      isSaved={c.isCollectionSaved}
      isShuffled={c.playbackState.is_shuffled}
      trackPlaylistMap={c.regularTrackPlaylistMap}
      userPlaylistIds={c.regularPlaylistIds}
      onAddToPlaylist={(track) =>
        c.handleOpenAddToPlaylistModal({
          id:
            track.matched_local_track_id &&
            !track.matched_local_track_id.startsWith("online:") &&
            !track.matched_local_track_id.startsWith("itunes:")
              ? track.matched_local_track_id
              : track.id,
          title: track.title,
          artist: track.artist,
          album: track.album,
          cover_art_url: track.cover_art_url,
        })
      }
      onRenamePlaylist={c.handleRenamePlaylist}
      onDeletePlaylist={c.handleDeletePlaylist}
      likedTrackIds={c.likedTrackIds}
      onToggleLike={c.handleToggleCollectionLike}
      queuedTrackIds={c.queuedTrackIds}
      onEnqueueTrack={(track) =>
        c.handleEnqueueRecommendation(
          track.rawRecommendation || {
            external_track_id: track.id,
            provider: track.matched_local_track_id
              ? "library"
              : "online",
            provider_id: track.id,
            title: track.title,
            artist: track.artist,
            album: track.album,
            duration_secs: track.duration_secs,
            cover_art_url: track.cover_art_url,
            preview_url: track.preview_url,
            matched_local_track_id: track.matched_local_track_id,
            match_status: track.matched_local_track_id
              ? "EXACT_MATCH"
              : "NOT_FOUND",
            recommendation_reason: "From collection",
            in_wishlist: false,
          },
        )
      }
    />
  );
}
