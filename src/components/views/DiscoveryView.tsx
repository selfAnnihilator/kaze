import { DiscoveryChapters, DiscoveryPreviewTarget, type DiscoveryPreview } from "../../presentation/DiscoveryChapters";
import { useDiscoveryMemory } from "../../presentation/ViewMemory";
import React, { useState, useRef, useEffect, useMemo } from "react";
import { DiscoveryRecommendation, Track, Playlist, DownloadTask } from "../../types";
import { executeQuery } from "../../services/api";
import { CollectionData } from "./CollectionDetailView";
import { OnlineSearchResultsSection } from "./OnlineSearchResultsSection";
import { splitDiscoveryRecommendations } from "../../forYouRecommendations";
import { DiscoveryTrackCard } from "./discovery/DiscoveryTrackCard";
import { MixItem } from "./discovery/MixCard";
import { ChartItem } from "./discovery/ChartCard";
import type { DiscoveryViewContext } from "./discovery/context";
import { DiscoveryHero } from "./discovery/DiscoveryHero";
import { OfflineDiscovery } from "./discovery/OfflineDiscovery";
import { TrendingShelf } from "./discovery/TrendingShelf";
import { ForYouShelf } from "./discovery/ForYouShelf";
import { MixesShelf } from "./discovery/MixesShelf";
import { ChartsShelf } from "./discovery/ChartsShelf";
import { useDiscoverySearch } from "./discovery/useDiscoverySearch";

interface DiscoveryViewProps {
  recommendations: DiscoveryRecommendation[];
  worldTrending: DiscoveryRecommendation[];
  isTrendingLoading?: boolean;
  trendingError?: string | null;
  onRefreshTrending?: () => Promise<void>;
  localTracks: Track[];
  playlists?: Playlist[];
  onPlayPlaylist?: (id: string) => void;
  onAddToWishlist: (rec: DiscoveryRecommendation) => void;
  onSearchDirect: (artist: string, title: string) => void;
  onRefresh?: (force?: boolean) => Promise<void>;
  onPlayOnlineTrack: (rec: DiscoveryRecommendation) => void;
  onStopTrack?: (rec: DiscoveryRecommendation) => void;
  activeOnlineTrackId?: string | null;
  isOnlinePlaying?: boolean;
  loadingTrackId?: string | null;
  currentLocalTrack?: Track | null;
  isLocalPlaying?: boolean;
  onOpenCollection?: (collection: CollectionData) => void;
  onPlayCollection?: (collection: CollectionData) => Promise<void>;
  downloads?: DownloadTask[];
  downloadTargets?: Record<string, { title: string; artist: string }>;
  searchQuery?: string;
  setSearchQuery?: (q: string) => void;
  searchResults?: DiscoveryRecommendation[] | null;
  setSearchResults?: (res: DiscoveryRecommendation[] | null) => void;
  isSearchingOnline?: boolean;
  onSearchOnline?: (queryOverride?: string) => Promise<void>;
  onClearSearch?: () => void;
  trackPlaylistMap?: Record<string, string[]>;
  onAddToPlaylist?: (rec: DiscoveryRecommendation) => void;
  onGoToLibrary?: () => void;
  queuedTrackIds?: Set<string>;
  onEnqueueTrack?: (rec: DiscoveryRecommendation) => void;
  likedTrackIds?: Set<string>;
}

export const DiscoveryView: React.FC<DiscoveryViewProps> = React.memo(({
  recommendations,
  worldTrending,
  isTrendingLoading = false,
  trendingError = null,
  onRefreshTrending,
  localTracks,
  playlists = [],
  onPlayPlaylist,
  onAddToWishlist,
  onSearchDirect,
  onRefresh: _onRefresh,
  onPlayOnlineTrack,
  onStopTrack,
  activeOnlineTrackId,
  isOnlinePlaying = false,
  loadingTrackId = null,
  currentLocalTrack,
  isLocalPlaying = false,
  onOpenCollection,
  onPlayCollection,
  downloads = [],
  downloadTargets,
  searchQuery: propSearchQuery,
  setSearchQuery: propSetSearchQuery,
  searchResults: propSearchResults,
  setSearchResults: propSetSearchResults,
  isSearchingOnline: propIsSearchingOnline,
  onSearchOnline: propOnSearchOnline,
  onClearSearch: propOnClearSearch,
  trackPlaylistMap,
  onAddToPlaylist,
  onGoToLibrary,
  queuedTrackIds,
  onEnqueueTrack,
  likedTrackIds,
}) => {
  const viewMemory = useDiscoveryMemory();
  const [, setVisibleCount] = useState(15);
  const [isRefreshingTrending, setIsRefreshingTrending] = useState(false);
  const [isOnline, setIsOnline] = useState(
    typeof navigator !== "undefined" ? navigator.onLine : true,
  );

  const handleRefreshTrending = async () => {
    if (isRefreshingTrending || isTrendingLoading || !(onRefreshTrending || _onRefresh)) return;
    setIsRefreshingTrending(true);
    try {
      if (onRefreshTrending) await onRefreshTrending();
      else await _onRefresh?.(true);
      setVisibleCount(15);
      if (songsScrollRef.current) {
        songsScrollRef.current.scrollLeft = 0;
      }
      viewMemory.shelves.set(0, 0);
    } catch (err) {
      console.error("Failed to refresh trending:", err);
    } finally {
      setIsRefreshingTrending(false);
    }
  };

  const playlistTrackIds = useMemo(
    () =>
      new Set(
        Object.entries(trackPlaylistMap || {})
          .filter(([, playlistIds]) => playlistIds.length > 0)
          .map(([trackId]) => trackId),
      ),
    [trackPlaylistMap],
  );
  const { forYou } = useMemo(
    () =>
      splitDiscoveryRecommendations({
        recommendations,
        playlistTrackIds,
        likedTrackIds: likedTrackIds || new Set<string>(),
        downloads,
              downloadTargets,
        localTracks,
      }),
    [
      recommendations,
      playlistTrackIds,
      likedTrackIds,
      downloads,
      downloadTargets,
      localTracks,
    ],
  );
  const trending = worldTrending;

  const handleSongsScroll = () => {
    const el = songsScrollRef.current;
    if (!el) return;
    if (el.scrollLeft + el.clientWidth >= el.scrollWidth - 350) {
      setVisibleCount((prev) => Math.min(prev + 10, trending.length));
    }
  };

  useEffect(() => {
    const handleOnline = () => {
      setIsOnline(true);
      void _onRefresh?.();
      void onRefreshTrending?.();
    };
    const handleOffline = () => {
      setIsOnline(false);
    };
    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);
    return () => {
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
    };
  }, [_onRefresh, onRefreshTrending]);

  const { searchQuery, setSearchQuery, searchResults, searching, searchActive, handleSearchOnline, handleClearSearch } = useDiscoverySearch({
    searchQuery: propSearchQuery, setSearchQuery: propSetSearchQuery, searchResults: propSearchResults,
    setSearchResults: propSetSearchResults, isSearchingOnline: propIsSearchingOnline,
    onSearchOnline: propOnSearchOnline, onClearSearch: propOnClearSearch,
  });

  const songsScrollRef = useRef<HTMLDivElement>(null);
  const forYouScrollRef = useRef<HTMLDivElement>(null);
  const mixesScrollRef = useRef<HTMLDivElement>(null);
  const chartsScrollRef = useRef<HTMLDivElement>(null);
  const discoveryRef = useRef<HTMLDivElement>(null);

  const handleScroll = (
    ref: React.RefObject<HTMLDivElement | null>,
    direction: "left" | "right",
  ) => {
    if (ref.current) {
      const amount = direction === "left" ? -500 : 500;
      ref.current.scrollBy({ left: amount, behavior: "smooth" });
    }
  };

  const seenMixNames = new Set<string>();
  const userSmartMixes: MixItem[] = (playlists || [])
    .filter((p) => {
      if (p.is_smart_mix !== 1) return false;
      const key = p.name.trim().toLowerCase();
      if (seenMixNames.has(key)) return false;
      seenMixNames.add(key);
      return true;
    })
    .map((p, idx) => {
      const gradients = [
        "linear-gradient(135deg, #4b607f 0%, #8b7cf6 100%)",
        "linear-gradient(135deg, #2a211b 0%, #f3701e 100%)",
        "linear-gradient(135deg, #1a1714 0%, #4b607f 100%)",
        "linear-gradient(135deg, #2a211b 0%, #8b7cf6 100%)",
        "linear-gradient(135deg, #f3701e 0%, #8b7cf6 100%)",
      ];
      return {
        id: p.id,
        name: p.name,
        subtitle:
          p.description ||
          p.generation_reason ||
          `${p.track_count || 0} tracks`,
        bgGradient: gradients[idx % gradients.length],
        accentColor: "var(--accent-light)",
        playlistId: p.id,
        cover_art_url: p.cover_art_url,
      };
    });

  // Every mix shown here is generated for the active account from its library
  // and listening profile. Generic search presets do not belong in For You.
  const [refreshedMixes, setRefreshedMixes] = useState<Playlist[] | null>(null);
  const allMixes = refreshedMixes ? refreshedMixes.filter(p => p.is_smart_mix === 1).map(p => ({ id: `smart-mix:${p.id}`, name: p.name, subtitle: p.description || "Made for you", tag: "SMART MIX", bgGradient: "var(--bg-surface)", accentColor: "var(--accent)", playlistId: p.id, cover_art_url: p.cover_art_url })) : userSmartMixes;

  const [topCharts, setTopCharts] = useState<ChartItem[]>([]);
  const refreshCharts = async (force = false) => {
    const result = await executeQuery({ query: force ? "RefreshCharts" : "GetCharts" });
    if (Array.isArray(result.data)) setTopCharts(result.data.map((chart: {id: string; title: string; image?: string}, index: number) => ({
      id: `jiosaavn:playlist:${chart.id}`, title: chart.title, subtitle: "Top songs",
      chartNumber: String(index + 1).padStart(2, "0"), region: "", image: chart.image,
      bgGradient: "var(--bg-surface)", badgeBg: "var(--accent)",
    })));
  };
  useEffect(() => { let mounted = true; executeQuery({ query: "GetCharts" }).then(result => {
    if (mounted && Array.isArray(result.data)) setTopCharts(result.data.map((chart: { id: string; title: string; image?: string }, index: number) => ({ id: `jiosaavn:playlist:${chart.id}`, title: chart.title, subtitle: "Top songs", chartNumber: String(index + 1).padStart(2, "0"), region: "", image: chart.image, bgGradient: "var(--bg-surface)", badgeBg: "var(--accent)" })));
  }).catch(console.error); return () => { mounted = false; }; }, []);
  const refreshMixes = async () => { const result = await executeQuery({ query: "RefreshSmartMixes" }); if (Array.isArray(result.data)) setRefreshedMixes(result.data); };

  const handleOpenMix = (mix: MixItem) => {
    if (onOpenCollection) {
      onOpenCollection({
        id: mix.id,
        type: "mix",
        title: mix.name,
        subtitle: mix.subtitle,
        tag: "SMART MIX",
        bgGradient: mix.bgGradient,
        accentColor: mix.accentColor,
        searchQuery: mix.searchQuery,
        playlistId: mix.playlistId,
        cover_art_url: mix.cover_art_url,
      });
    } else {
      handlePlayMixItem(mix);
    }
  };

  const chartToCollection = (chart: ChartItem): CollectionData => ({
    id: chart.id,
    type: "chart",
    title: chart.title,
    subtitle: chart.subtitle,
    tag: `TOP CHART • ${chart.region.toUpperCase()}`,
    bgGradient: chart.bgGradient,
    accentColor: chart.badgeBg,
  });

  const handleOpenChart = (chart: ChartItem) => {
    if (onOpenCollection) {
      onOpenCollection(chartToCollection(chart));
    } else {
      handlePlayChartItem(chart);
    }
  };

  const handlePlayMixItem = (mix: MixItem) => {
    if (mix.playlistId && onPlayPlaylist) {
      onPlayPlaylist(mix.playlistId);
    } else if (mix.searchQuery) {
      setSearchQuery(mix.searchQuery);
      handleSearchOnline(mix.searchQuery);
    }
  };

  const handlePlayChartItem = (chart: ChartItem) => {
    if (onPlayCollection) {
      void onPlayCollection(chartToCollection(chart));
      return;
    }
    onOpenCollection?.(chartToCollection(chart));
  };

  const songPlayback = (rec: DiscoveryRecommendation) => {
    const isLoading =
      loadingTrackId === rec.external_track_id ||
      (!!rec.matched_local_track_id &&
        loadingTrackId === rec.matched_local_track_id);
    const hasRealLocalMatch =
      !!rec.matched_local_track_id &&
      !rec.matched_local_track_id.startsWith("itunes:") &&
      !rec.matched_local_track_id.startsWith("online:");
    const isPlayingLocal =
      isLocalPlaying &&
      !!currentLocalTrack &&
      currentLocalTrack.format !== "online" &&
      !currentLocalTrack.file_path.startsWith("online://") &&
      ((hasRealLocalMatch &&
        rec.matched_local_track_id === currentLocalTrack.id) ||
        rec.external_track_id === currentLocalTrack.id ||
        (rec.title.toLowerCase().trim() ===
          currentLocalTrack.title.toLowerCase().trim() &&
          !!currentLocalTrack.artist_name &&
          rec.artist.toLowerCase().trim() ===
            currentLocalTrack.artist_name.toLowerCase().trim()));
    const isPlayingOnline =
      activeOnlineTrackId === rec.external_track_id && isOnlinePlaying;
    const isPlaying = isPlayingLocal || isPlayingOnline;

    return { isLoading, isPlaying };
  };

  const songPreview = (rec: DiscoveryRecommendation): DiscoveryPreview => {
    const { isLoading, isPlaying } = songPlayback(rec);
    return {
      id: rec.external_track_id, title: rec.title, artist: rec.artist, album: rec.album,
      cover: rec.cover_art_url, loading: isLoading, playing: isPlaying,
      onPlay: () => {
        if (isLoading) return;
        if (isPlaying) (onStopTrack || onPlayOnlineTrack)(rec);
        else onPlayOnlineTrack(rec);
      },
    };
  };
  const mixPreview = (mix: MixItem): DiscoveryPreview => ({
    id: mix.id, title: mix.name, artist: mix.subtitle, cover: mix.cover_art_url,
    artworkLabel: "MIX", onPlay: () => handlePlayMixItem(mix), onOpen: () => handleOpenMix(mix),
  });
  const chartPreview = (chart: ChartItem): DiscoveryPreview => ({
    id: chart.id, title: chart.title, artist: chart.subtitle, cover: chart.image, artworkLabel: chart.chartNumber,
    onPlay: () => handlePlayChartItem(chart), onOpen: () => handleOpenChart(chart),
  });

  const renderSongCard = (rec: DiscoveryRecommendation) => {
    const { isLoading, isPlaying } = songPlayback(rec);
    return (
      <DiscoveryPreviewTarget key={rec.external_track_id} preview={songPreview(rec)} className="discovery-track-card">
        <DiscoveryTrackCard
          rec={rec}
          isPlaying={isPlaying}
          isLoading={isLoading}
          isSelected={
            isPlaying ||
            isLoading ||
            activeOnlineTrackId === rec.external_track_id
          }
          onPlay={onPlayOnlineTrack}
          onStop={onStopTrack || onPlayOnlineTrack}
          onArtistClick={(artist) => {
            setSearchQuery(artist);
            handleSearchOnline(artist);
          }}
          onAddToWishlist={onAddToWishlist}
          onDownload={onSearchDirect}
          downloads={downloads}
          trackPlaylistMap={trackPlaylistMap}
          onAddToPlaylist={onAddToPlaylist}
          isQueued={queuedTrackIds?.has(
            rec.matched_local_track_id || rec.external_track_id,
          )}
          onEnqueue={onEnqueueTrack}
          isLiked={Boolean(
            (rec.matched_local_track_id &&
              likedTrackIds?.has(rec.matched_local_track_id)) ||
            (rec.external_track_id &&
              likedTrackIds?.has(rec.external_track_id)),
          )}
        />
      </DiscoveryPreviewTarget>
    );
  };

  if (!isOnline) {
    return (
      <OfflineDiscovery onGoToLibrary={onGoToLibrary} />
    );
  }

  const c: DiscoveryViewContext = {
    handleScroll, songsScrollRef, trending, trendingError, isTrendingLoading, handleSongsScroll,
    viewMemory, renderSongCard, forYou, forYouScrollRef, mixesScrollRef, allMixes, mixPreview,
    handlePlayMixItem, handleOpenMix, chartsScrollRef, topCharts, chartPreview, handlePlayChartItem,
    handleOpenChart,
  };
  return (
    <div
      ref={discoveryRef}
      className={`discovery-view${searchActive ? " discovery-search-active" : ""}`}
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "28px",
        paddingBottom: "40px",
      }}
    >
      {/* SEPARATE SEARCH SECTION (Displayed exclusively when search results are active) */}
      {searchActive ? (
        <OnlineSearchResultsSection
          searchQuery={searchQuery}
          searchResults={searchResults ?? []}
          isSearching={searching}
          onClearSearch={handleClearSearch}
          onPlayOnlineTrack={onPlayOnlineTrack}
          onStopTrack={onStopTrack}
          activeOnlineTrackId={activeOnlineTrackId}
          isOnlinePlaying={isOnlinePlaying}
          isOnlineLoading={loadingTrackId !== null}
          loadingTrackId={loadingTrackId}
          currentLocalTrack={currentLocalTrack}
          isLocalPlaying={isLocalPlaying}
          onAddToPlaylist={onAddToPlaylist}
          onAddToWishlist={onAddToWishlist}
          trackPlaylistMap={trackPlaylistMap}
          playlists={playlists}
          onSelectArtist={(artist) => {
            setSearchQuery(artist);
            handleSearchOnline(artist);
          }}
        />
      ) : (
        <>
          <DiscoveryHero />
          <DiscoveryChapters selections={[
            { title: "Trending songs", subtitle: "Popular songs from the charts.", previews: trending.map(songPreview), onRefresh: handleRefreshTrending, isRefreshing: isTrendingLoading },
            { title: "For you", subtitle: "A little familiar. A little unexpected.", previews: forYou.map(songPreview), onRefresh: () => _onRefresh?.(true) || Promise.resolve() },
            { title: "Mixes for you", subtitle: "Personal blends shaped by your listening.", previews: allMixes.map(mixPreview), onRefresh: refreshMixes },
            { title: "Top charts", subtitle: "Follow the sound around the world.", previews: topCharts.map(chartPreview), onRefresh: () => refreshCharts(true) },
          ]}>
            {/* SECTION 1: Trending Songs */}
            <TrendingShelf c={c} />

            {/* SECTION 2: For You (new songs based on listening taste) */}
            <ForYouShelf c={c} />

            {/* SECTION 3: Mixes For You (Horizontal Scroll) */}
            <MixesShelf c={c} />

            {/* SECTION 3: Top Charts (Horizontal Scroll) */}
            <ChartsShelf c={c} />
          </DiscoveryChapters>
        </>
      )}
    </div>
  );
});
