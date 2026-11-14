import { useEffect } from "react";

/**
 * Runs every fetcher in order on mount, and again whenever one of them changes
 * identity (e.g. a new account). The list must keep a constant length.
 */
export function useLoadAll(fetchers: ReadonlyArray<() => unknown>) {
  // eslint-disable-next-line react-hooks/exhaustive-deps -- the fetchers are the dependencies
  useEffect(() => { for (const fetch of fetchers) void fetch(); }, fetchers);
}
