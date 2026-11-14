import type { Track, DiscoveryRecommendation } from "../../../types";

export interface CollectionTrackItem {
  id: string;
  title: string;
  artist: string;
  album?: string;
  duration_secs: number;
  cover_art_url?: string;
  preview_url?: string;
  is_downloaded?: boolean;
  matched_local_track_id?: string;
  rawLocalTrack?: Track;
  rawRecommendation?: DiscoveryRecommendation;
}

export interface CollectionData {
  id: string;
  type: "playlist" | "mix" | "chart";
  title: string;
  subtitle: string;
  tag: string;
  bgGradient?: string;
  accentColor?: string;
  searchQuery?: string;
  playlistId?: string;
  curator?: string;
  tracks?: CollectionTrackItem[];
  cover_art_url?: string;
}
