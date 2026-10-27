/** Prune only recommendation payloads; explicit search/library state is separate. */
export function removeRecommendationTracks<T>(value: T, ids: ReadonlySet<string>): T {
  const visit = (value: unknown): unknown => {
    if (Array.isArray(value)) return value.filter(item => !isHidden(item)).map(visit);
    if (value && typeof value === "object") return Object.fromEntries(Object.entries(value).map(([key, child]) => [key, visit(child)]));
    return value;
  };
  const isHidden = (value: unknown) => {
    if (!value || typeof value !== "object") return false;
    const item = value as Record<string, unknown>;
    const id = item.track_id ?? item.external_track_id ?? item.id;
    return typeof item.title === "string" && typeof id === "string" && ids.has(id);
  };
  return visit(value) as T;
}
