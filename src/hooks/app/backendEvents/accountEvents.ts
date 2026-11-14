// Account, session, profile, taste and recommendation-feedback events.
import { removeRecommendationTracks } from "../../../recommendationFeedback";
import { recommendationPreferences } from "../../../services/recommendationPreferences";
import type { EntityFeedbackRow } from "../../../types";
import type { BackendEvent, BackendEventContext } from "./context";

/** Handles one account event; returns false when the event belongs elsewhere. */
export function handleAccountEvent(event: BackendEvent, ctx: BackendEventContext): boolean {
  const { chartRequestRef, currentUserIdRef, discoveryRequestRef, fetchDiscovery, fetchDownloads, fetchFlowSession, fetchHomeOverview, fetchPersonalRecommendations, fetchPlaylists, fetchTrackPlaylistMemberships, fetchWishlist, flowSessionRequestRef, homeRequestRef, libraryAccountRef, membershipRequest, playlistRequest, resetNavigation, setCurrentUser, setDiscoveryRecs, setDownloads, setHomeOverview, setIsPlaylistModalOpen, setIsTrendingLoading, setPlaybackState, setPlayingOrigin, setPlayingPlaylistId, setPlaylistModalTrack, setPlaylists, setRecommendationNotice, setTrackPlaylistMap, setTrendingError, setWishlist, setWorldTrending, userQueryRevision, timers } = ctx;
  switch (event.event) {
    case "RecommendationFeedbackChanged": {
      if (event.payload?.user_id !== currentUserIdRef.current) break;
      ++homeRequestRef.current; ++discoveryRequestRef.current; ++chartRequestRef.current;
      setIsTrendingLoading(false);
      recommendationPreferences.update(event.payload?.track_ids || [], !!event.payload?.disliked);
      const ids = new Set<string>(event.payload?.track_ids || []);
      if (event.payload?.disliked) {
        setDiscoveryRecs(previous => removeRecommendationTracks(previous, ids));
        setWorldTrending(previous => removeRecommendationTracks(previous, ids));
        setHomeOverview(previous => removeRecommendationTracks(previous, ids));
        setRecommendationNotice({ id: event.payload.track_id, title: event.payload.title });
      } else {
        setRecommendationNotice(previous => previous && ids.has(previous.id) ? null : previous);
        void fetchDiscovery(false, false);
      }
      setPlaybackState(previous => previous.current_track && ids.has(previous.current_track.id)
        ? { ...previous, current_track: { ...previous.current_track, manual_like: event.payload.manual_like } } : previous);
      void fetchPlaylists();
      break;
    }
    case "EntityFeedbackChanged": {
      if (event.payload?.user_id !== currentUserIdRef.current) break;
      ++homeRequestRef.current; ++discoveryRequestRef.current; ++chartRequestRef.current;
      setIsTrendingLoading(false);
      void recommendationPreferences.refresh();
      const rows: EntityFeedbackRow[] = event.payload?.rows || [];
      if (!event.payload?.cleared && rows.length) {
        setRecommendationNotice({ id: `entity:${rows.map(row => `${row.kind}:${row.key}`).join("|")}`, title: rows[0].display, entities: rows });
      }
      void fetchDiscovery(false, false);
      void fetchPlaylists();
      break;
    }
    case "TasteProfileChanged": {
      if (event.payload?.user_id !== currentUserIdRef.current) break;
      // The writer emits this after commit: no race with history persistence.
      // Coalesce rapid skips without subscribing App to position ticks.
      ++discoveryRequestRef.current;
      ++homeRequestRef.current;
      clearTimeout(timers.taste);
      timers.taste = setTimeout(() => {
        void fetchPersonalRecommendations().catch(console.error);
        void fetchHomeOverview();
      }, 250);
      break;
    }

    case "SessionChanged": {
      const nextAccount = event.payload?.user?.id || "default";
      if (nextAccount !== libraryAccountRef.current) {
        resetNavigation();
        setHomeOverview(null);
        setIsPlaylistModalOpen(false); setPlaylistModalTrack(null);
        setPlayingPlaylistId(null);
        setPlayingOrigin(origin => origin?.type === "playlist" ? null : origin);
      }
      ++userQueryRevision.current;
      libraryAccountRef.current = nextAccount;
      ++playlistRequest.current; ++membershipRequest.current;
      setPlaylists([]); setTrackPlaylistMap({});
      setWishlist([]); setDownloads([]);
      void fetchWishlist(); void fetchDownloads();
      recommendationPreferences.reset();
      void recommendationPreferences.refresh().catch(console.error);
      currentUserIdRef.current = event.payload?.user?.id || "default";
      ++chartRequestRef.current; setWorldTrending([]); setIsTrendingLoading(true); setTrendingError(null); setRecommendationNotice(null);
      ++discoveryRequestRef.current;
      setDiscoveryRecs([]);
      clearTimeout(timers.taste);
      void fetchDiscovery(false, false);
      ++homeRequestRef.current;
      ++flowSessionRequestRef.current;
      void fetchHomeOverview();
      void fetchFlowSession().catch(console.error);
      const user = event.payload?.user;
      if (user) {
        setCurrentUser(user);
        fetchPlaylists();
        fetchTrackPlaylistMemberships();
      } else {
        setCurrentUser(null);
        setPlaylists((prev) => prev.filter((p) => p.is_smart_mix === 1));
        setTrackPlaylistMap({});
      }
      break;
    }

    case "UserProfileUpdated": {
      const user = event.payload?.user;
      if (user) {
        setCurrentUser(user);
      }
      break;
    }

    case "UserLoggedOut": {
      window.location.reload();
      break;
    }

    default:
      return false;
  }
  return true;
}
