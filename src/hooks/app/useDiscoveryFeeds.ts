import { useCallback, useMemo, useRef, useState } from "react";
import type { MutableRefObject } from "react";
import { executeQuery } from "../../services/api";
import type { DiscoveryRecommendation, StatsOverview } from "../../types";

/**
 * Discovery feeds: personal recommendations and world trending (cached first,
 * then live), user-scoped via request ids, plus the stats overview query.
 */
export function useDiscoveryFeeds({ currentUserIdRef }: { currentUserIdRef: MutableRefObject<string> }) {
  const [isTrendingLoading, setIsTrendingLoading] = useState(true);
  const [trendingError, setTrendingError] = useState<string | null>(null);
  const [discoveryRecs, setDiscoveryRecs] = useState<DiscoveryRecommendation[]>([],
  );
  const [worldTrending, setWorldTrending] = useState<DiscoveryRecommendation[]>([],
  );

  const discoveryRequestRef = useRef(0);
  const chartRequestRef = useRef(0);
  const fetchPersonalRecommendations = useCallback(async (forceRefresh = false) => {
    const request = ++discoveryRequestRef.current;
    const userId = currentUserIdRef.current;
    const res = await executeQuery({
      query: "GetDiscoveryRecommendations",
      payload: { limit: 100, force_refresh: forceRefresh },
    });
    if (request === discoveryRequestRef.current && userId === currentUserIdRef.current && Array.isArray(res.data)) {
      setDiscoveryRecs(res.data);
    }
  }, []);

  const fetchTrending = useCallback(async (forceRefresh = false) => {
    const request = ++chartRequestRef.current;
    const user = currentUserIdRef.current;
    const current = () => request === chartRequestRef.current && user === currentUserIdRef.current;
    setIsTrendingLoading(true);
    setTrendingError(null);
    try {
      if (!forceRefresh) {
        try {
          const cached = await executeQuery({ query: "GetCachedWorldTrending", payload: { limit: 50 } });
          if (current() && Array.isArray(cached.data) && cached.data.length) setWorldTrending(cached.data);
        } catch (error) { console.error("Failed to read saved trending songs:", error); }
      }
      const res = await executeQuery({ query: "GetWorldTrending", payload: { limit: 50, force_refresh: forceRefresh } });
      if (current() && Array.isArray(res.data)) setWorldTrending(res.data);
    } catch (error) {
      if (current()) setTrendingError("Couldn't update trending songs. Try Refresh again.");
      console.error("Failed to fetch world charts:", error);
    } finally {
      if (current()) setIsTrendingLoading(false);
    }
  }, []);
  const refreshTrending = useCallback(() => fetchTrending(true), [fetchTrending]);
  const refreshPersonalRecommendations = useCallback((forceRefresh = false) => fetchPersonalRecommendations(forceRefresh), [fetchPersonalRecommendations]);
  const fetchDiscovery = useCallback(async (forceRefresh = false, refreshCharts = false) => {
    const results = await Promise.allSettled([
      fetchPersonalRecommendations(forceRefresh), fetchTrending(refreshCharts),
    ]);
    for (const result of results) if (result.status === "rejected") console.error("Failed to refresh discovery:", result.reason);
  }, [fetchPersonalRecommendations, fetchTrending]);

  const fetchStatsOverview = useCallback(
    async (year?: number, month?: number): Promise<StatsOverview | null> => {
      try {
        const payload: { year?: number; month?: number } = {};
        if (year !== undefined) payload.year = year;
        if (month !== undefined) payload.month = month;
        const res = await executeQuery({
          query: "GetStatsOverview",
          payload,
        });
        if (res && res.data) {
          return res.data as StatsOverview;
        }
      } catch (err) {
        console.error("Failed to fetch stats overview:", err);
      }
      return null;
    },
    [],
  );

  return useMemo(() => ({
    discoveryRequestRef, chartRequestRef, discoveryRecs, setDiscoveryRecs, worldTrending, setWorldTrending, isTrendingLoading, setIsTrendingLoading, trendingError, setTrendingError, fetchPersonalRecommendations, refreshTrending, refreshPersonalRecommendations, fetchDiscovery, fetchStatsOverview,
  }), [discoveryRequestRef, chartRequestRef, discoveryRecs, setDiscoveryRecs, worldTrending, setWorldTrending, isTrendingLoading, setIsTrendingLoading, trendingError, setTrendingError, fetchPersonalRecommendations, refreshTrending, refreshPersonalRecommendations, fetchDiscovery, fetchStatsOverview]);
}
