import type { DiscoveryRecommendation } from "../../../types";
import type { Dispatch } from "react";
import type { Playlist } from "../../../types";
import type { ReactNode, SetStateAction } from "react";
export type SearchCategoryTab = "all" | "songs" | "artists" | "albums" | "playlists";

export interface OnlineSearchResultsSectionContext {
  setActiveTab: Dispatch<SetStateAction<SearchCategoryTab>>;
  activeTab: SearchCategoryTab;
  searchResults: DiscoveryRecommendation[];
  distinctArtists: { artist: string; cover?: string | undefined; count: number; }[];
  distinctAlbums: { album: string; artist: string; cover?: string | undefined; }[];
  matchingPlaylists: Playlist[];
  searchQuery: string;
  onClearSearch: () => void;
  topResult: DiscoveryRecommendation | null;
  handleTogglePlay: (rec: DiscoveryRecommendation) => void;
  isTrackPlaying: (rec: DiscoveryRecommendation) => boolean;
  isTrackLoading: (rec: DiscoveryRecommendation) => boolean;
  onSelectArtist: ((artist: string) => void) | undefined;
  onAddToPlaylist: ((rec: DiscoveryRecommendation) => void) | undefined;
  songList: ReactNode;
}
