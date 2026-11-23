import type { ToastAction } from "../../../types";
import type { Dispatch, MutableRefObject, SetStateAction } from "react";
import type { AddToPlaylistModalTrack } from "../../../components/modals/AddToPlaylistModal";
import type { RecommendationNoticeValue } from "../../../components/RecommendationNotice";
import type { QueueEntry } from "../../../presentation/queuePresentation";
import type { DiscoveryRecommendation, DownloadTask, HomeOverview, PlaybackState, Playlist, Track, UserProfile, WishlistItem } from "../../../types";

/** A backend event as delivered by `subscribeBackendEvents`. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type BackendEvent = { event: string; payload?: any };

type Setter<T> = Dispatch<SetStateAction<T>>;
type Fetcher = () => unknown;
import type { PlayingOrigin } from "../../../playingOrigin";

/** Timers shared by event handlers for one subscription (cleared on unsubscribe). */
export interface EventTimers { taste?: ReturnType<typeof setTimeout> }

/** Everything the event handlers read or update. All values are stable setters, refs and fetchers. */
export interface BackendEventContext {
  addAppNotification: (type: "success" | "error" | "info" | "warning", title: string, message: string, action?: ToastAction) => void;
  chartRequestRef: MutableRefObject<number>;
  currentUserIdRef: MutableRefObject<string>;
  discoveryRequestRef: MutableRefObject<number>;
  fetchAlbums: Fetcher;
  fetchCurrentUser: Fetcher;
  fetchDiscovery: (forceRefresh?: boolean, refreshCharts?: boolean) => unknown;
  fetchDownloads: Fetcher;
  fetchFlowSession: () => Promise<unknown>;
  fetchHomeOverview: Fetcher;
  fetchOnboardingStatus: Fetcher;
  fetchPersonalRecommendations: (forceRefresh?: boolean) => Promise<unknown>;
  fetchPlaybackState: () => Promise<void>;
  fetchPlaylists: Fetcher;
  fetchTrackPlaylistMemberships: Fetcher;
  fetchTracks: Fetcher;
  fetchWishlist: Fetcher;
  flowSessionRequestRef: MutableRefObject<number>;
  homeRequestRef: MutableRefObject<number>;
  libraryAccountRef: MutableRefObject<string>;
  membershipRequest: MutableRefObject<number>;
  playbackSessionId: MutableRefObject<string | null>;
  playbackStateRevision: MutableRefObject<number>;
  playlistRequest: MutableRefObject<number>;
  resetNavigation: () => void;
  setCurrentUser: Setter<UserProfile | null>;
  setDiscoveryRecs: Setter<DiscoveryRecommendation[]>;
  setDownloads: Setter<DownloadTask[]>;
  setHomeOverview: Setter<HomeOverview | null>;
  setIsPlaylistModalOpen: Setter<boolean>;
  setIsScanning: Setter<boolean>;
  setIsTrendingLoading: Setter<boolean>;
  setLoadingTrackId: Setter<string | null>;
  setPlaybackState: Setter<PlaybackState>;
  setPlayingOrigin: Setter<PlayingOrigin>;
  setPlayingPlaylistId: Setter<string | null>;
  setPlaylistModalTrack: Setter<AddToPlaylistModalTrack | null>;
  setPlaylists: Setter<Playlist[]>;
  setQueueEntries: Setter<QueueEntry[]>;
  setQueuedTrackIds: Setter<Set<string>>;
  setRecommendationNotice: Setter<RecommendationNoticeValue | null>;
  setTrackPlaylistMap: Setter<Record<string, string[]>>;
  setTrendingError: Setter<string | null>;
  setWishlist: Setter<WishlistItem[]>;
  setWorldTrending: Setter<DiscoveryRecommendation[]>;
  tracksRef: MutableRefObject<Track[]>;
  userQueryRevision: MutableRefObject<number>;
  timers: EventTimers;
}
