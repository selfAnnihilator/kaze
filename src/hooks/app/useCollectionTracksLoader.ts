import { useEffect, useState } from "react";
import type { SetStateAction } from "react";
import { executeQuery } from "../../services/api";
import { isOnlineTrack } from "../../trackKind";
import type { DiscoveryRecommendation, Track } from "../../types";
import type { CollectionData, CollectionTrackItem } from "../../components/views/CollectionDetailView";

/** Loads the open collection's tracks when it has none yet (playlist, mix, chart, album). */
export function useCollectionTracksLoader(
  activeCollection: CollectionData | null,
  setActiveCollection: (value: SetStateAction<CollectionData | null>) => void,
) {
  const [isLoadingCollectionTracks, setIsLoadingCollectionTracks] = useState<boolean>(false);
  useEffect(() => {
    const collection = activeCollection;
    if (!collection || collection.tracks !== undefined) {
      setIsLoadingCollectionTracks(false);
      return;
    }
    let cancelled = false;
    const load = async () => {
      setIsLoadingCollectionTracks(true);
      try {
        if (collection.playlistId) {
          const res = await executeQuery({
            query: "GetPlaylistTracks",
            payload: { playlist_id: collection.playlistId },
          });
          const plTracks: Track[] = (res.data as any) || [];
          const items: CollectionTrackItem[] = plTracks.map((t) => {
            const isOnline = !!isOnlineTrack(t);
            return {
              id: t.id,
              title: t.title,
              artist: t.artist_name || "Unknown Artist",
              album: t.album_title,
              duration_secs: t.duration_secs ?? 0,
              cover_art_url: t.cover_art_url,
              preview_url: t.preview_url,
              is_downloaded: !isOnline,
              matched_local_track_id: isOnline ? undefined : t.id,
              rawLocalTrack: isOnline ? undefined : t,
              rawRecommendation: isOnline
                ? {
                    external_track_id: t.id,
                    provider: t.playback_source?.provider || "online",
                    provider_id: t.playback_source?.track_id || t.id,
                    playback_source: t.playback_source,
                    title: t.title,
                    artist: t.artist_name || "Unknown Artist",
                    album: t.album_title,
                    duration_secs: t.duration_secs ?? 0,
                    cover_art_url: t.cover_art_url,
                    preview_url: t.preview_url,
                    match_status: "NOT_FOUND",
                    recommendation_reason: "",
                    in_wishlist: false,
                  }
                : undefined,
            };
          });
          setActiveCollection((prev) =>
            !cancelled && prev === collection
              ? { ...prev, tracks: items }
              : prev,
          );
        } else if (collection.type === "chart" || collection.searchQuery) {
          const res = await executeQuery(
            collection.type === "chart"
              ? {
                  query: "GetChartSongs",
                  payload: { chart_id: collection.id, limit: 50 },
                }
              : {
                  query: "SearchOnlineMusic",
                  payload: { query: collection.searchQuery!, limit: 50 },
                },
          );
          const onlineRecs: DiscoveryRecommendation[] = (res.data as any) || [];
          const items: CollectionTrackItem[] = onlineRecs.map((r) => {
            const validLocalId =
              r.matched_local_track_id &&
              !r.matched_local_track_id.startsWith("online:") &&
              !r.matched_local_track_id.startsWith("itunes:")
                ? r.matched_local_track_id
                : undefined;
            return {
              id: r.external_track_id,
              title: r.title,
              artist: r.artist,
              album: r.album,
              duration_secs: r.duration_secs ?? 0,
              cover_art_url: r.cover_art_url,
              preview_url: r.preview_url,
              is_downloaded: !!validLocalId,
              matched_local_track_id: validLocalId,
              rawRecommendation: r,
            };
          });
          setActiveCollection((prev) =>
            !cancelled && prev === collection
              ? { ...prev, tracks: items }
              : prev,
          );
        }
      } catch (err) {
        console.error("Failed to load collection tracks:", err);
      } finally {
        if (!cancelled) setIsLoadingCollectionTracks(false);
      }
    };
    void load();
    return () => {
      cancelled = true;
    };
  }, [activeCollection, setActiveCollection]);
  return isLoadingCollectionTracks;
}
