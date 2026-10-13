import type { ViewType } from "./components/Sidebar";
import type { CollectionData } from "./components/views/CollectionDetailView";
import type { DiscoveryRecommendation } from "./types";

export interface NavigationDestination {
  view: ViewType;
  collection: CollectionData | null;
  lyrics: boolean;
  libraryQuery: string;
  libraryAlbumId?: string;
  searchQuery: string;
  searchResults: DiscoveryRecommendation[] | null;
}
export interface NavigationHistory { entries: NavigationDestination[]; index: number }
export const viewDestination = (view: ViewType): NavigationDestination => ({ view, collection: null, lyrics: false, libraryQuery: "", searchQuery: "", searchResults: null });
const key = (d: NavigationDestination) => JSON.stringify([d.view, d.collection?.type, d.collection?.id, d.collection?.playlistId, d.collection?.searchQuery, d.lyrics, d.libraryQuery, d.libraryAlbumId, d.searchQuery]);
export function visitDestination(h: NavigationHistory, d: NavigationDestination): NavigationHistory {
  if (key(h.entries[h.index]) === key(d)) return replaceDestination(h, d);
  const entries = [...h.entries.slice(0, h.index + 1), d].slice(-100);
  return { entries, index: entries.length - 1 };
}
export function replaceDestination(h: NavigationHistory, d: NavigationDestination): NavigationHistory {
  return { ...h, entries: h.entries.map((entry, index) => index === h.index ? d : entry) };
}
export function stepHistory(h: NavigationHistory, step: -1 | 1): NavigationHistory {
  const index = h.index + step;
  return index < 0 || index >= h.entries.length ? h : { ...h, index };
}

export function navigationShortcut(event: Pick<KeyboardEvent, "key" | "altKey" | "ctrlKey" | "metaKey" | "shiftKey" | "defaultPrevented">, editable: boolean): -1 | 1 | null {
  if (event.defaultPrevented || editable || !event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return null;
  return event.key === "ArrowLeft" ? -1 : event.key === "ArrowRight" ? 1 : null;
}
