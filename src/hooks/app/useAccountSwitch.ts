import { useEffect, type MutableRefObject } from "react";
import { recommendationPreferences } from "../../services/recommendationPreferences";

interface AccountSwitchDeps {
  currentUserIdRef: MutableRefObject<string>;
  discoveryRequestRef: MutableRefObject<number>;
  chartRequestRef: MutableRefObject<number>;
  /** Empties the previous account's state. */
  clear: () => void;
  /** Refetches the new account's data (discovery runs after these). */
  refetch: ReadonlyArray<() => unknown>;
}

/** When the signed-in account changes, drops the old account's data and loads the new one's. */
export function useAccountSwitch(userIdOrNull: string | undefined, fetchDiscovery: () => unknown, deps: AccountSwitchDeps) {
  const userId = userIdOrNull || "default";
  useEffect(() => {
    const { currentUserIdRef, discoveryRequestRef, chartRequestRef, clear, refetch } = deps;
    if (userId === currentUserIdRef.current) return;
    currentUserIdRef.current = userId;
    recommendationPreferences.reset();
    clear();
    for (const fetch of refetch) void fetch();
    void recommendationPreferences.refresh().catch(console.error);
    // Stale discovery and chart responses for the old account are ignored.
    ++discoveryRequestRef.current;
    ++chartRequestRef.current;
    void fetchDiscovery();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- runs per account, like before
  }, [userId, fetchDiscovery]);
}
