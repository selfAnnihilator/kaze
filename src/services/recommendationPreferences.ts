import { useCallback, useSyncExternalStore } from "react";
import { executeQuery } from "./api";
const values = new Map<string, boolean>();
const listeners = new Set<() => void>();
let revision = 0;
const notify = () => listeners.forEach(listener => listener());
export const recommendationPreferences = {
  reset() { ++revision; values.clear(); notify(); },
  update(ids: string[], blocked: boolean) { ++revision; ids.forEach(id => values.set(id, blocked)); notify(); },
  async refresh() {
    const request = ++revision;
    const result = await executeQuery({ query: "GetRecommendationFeedback" });
    if (request !== revision || !Array.isArray(result.data)) return;
    values.clear(); result.data.forEach((id: string) => values.set(id, true));
    notify();
  },
};
export function useNotInterested(id: string, fallback = false) {
  const read = useCallback(() => values.get(id) ?? fallback, [id, fallback]);
  return useSyncExternalStore(useCallback((listener: () => void) => { listeners.add(listener); return () => { listeners.delete(listener); }; }, []), read, read);
}
