import { useEffect, useRef, useState } from "react";
import { subscribeBackendEvents } from "../../services/api";
import { ownBackendSubscription } from "../../services/backendSubscription";
import { handleAccountEvent } from "./backendEvents/accountEvents";
import type { BackendEventContext, EventTimers } from "./backendEvents/context";
import { handleLibraryEvent } from "./backendEvents/libraryEvents";
import { handlePlaybackEvent } from "./backendEvents/playbackEvents";

/**
 * Subscribes to backend events for the app's lifetime. Each domain's events are
 * handled in `backendEvents/<domain>Events.ts`; on reconnect, everything is refetched.
 */
export function useBackendEvents(ctx: Omit<BackendEventContext, "timers">) {
  const [connectionError, setConnectionError] = useState(false);
  const retryConnection = useRef<() => void>(() => {});
  const { addAppNotification, discoveryRequestRef, fetchAlbums, fetchCurrentUser, fetchDiscovery, fetchDownloads, fetchFlowSession, fetchHomeOverview, fetchOnboardingStatus, fetchPersonalRecommendations, fetchPlaybackState, fetchPlaylists, fetchTrackPlaylistMemberships, fetchTracks, fetchWishlist, playbackSessionId, resetNavigation } = ctx;
  useEffect(() => {
    let subscriptionDisposed = false;
    const timers: EventTimers = {};
    const context = { ...ctx, timers };

    const owner = ownBackendSubscription(() => subscribeBackendEvents((event: any) => {
      if (subscriptionDisposed || !event || !event.event) return;

      handlePlaybackEvent(event, context) || handleLibraryEvent(event, context) || handleAccountEvent(event, context);
    }), () => {
      // Reconcile events missed while disconnected through guarded query owners.
      playbackSessionId.current = null;
      void fetchPlaybackState();
      void fetchCurrentUser();
      void fetchTracks(); void fetchAlbums();
      void fetchPlaylists(); void fetchTrackPlaylistMemberships();
      void fetchWishlist(); void fetchDownloads();
      void fetchDiscovery(); void fetchHomeOverview();
    }, setConnectionError);
    retryConnection.current = owner.retry;

    return () => {
      subscriptionDisposed = true;
      clearTimeout(timers.taste);
      ++discoveryRequestRef.current;
      owner.dispose();
    };
  }, [
    fetchPersonalRecommendations,
    fetchDiscovery,
    fetchTracks,
    fetchAlbums,
    fetchOnboardingStatus,
    fetchDownloads,
    fetchWishlist,
    addAppNotification,
    fetchPlaylists,
    resetNavigation,
    fetchTrackPlaylistMemberships,
    fetchFlowSession,
    fetchHomeOverview,
    fetchPlaybackState,
  ]);

  return { connectionError, retryConnection };
}
