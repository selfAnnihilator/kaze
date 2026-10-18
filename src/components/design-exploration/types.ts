export type DesignModelId = "standard" | "drift" | "broadsheet" | "gallery";

export type AppViewId =
  | "home"
  | "discovery"
  | "library"
  | "albums"
  | "playlists"
  | "liked"
  | "downloads"
  | "stats"
  | "settings"
  | "now-playing"
  | "queue"
  | "album-detail";

export interface DesignModelMeta {
  id: DesignModelId;
  name: string;
  codename: string;
  tagline: string;
  theme: string;
  layoutDescription: string;
  accent: string;
  keyShortcut: string;
}

export interface ExplorationTrack {
  id: string;
  title: string;
  artist: string;
  album: string;
  year?: number;
  durationSecs: number;
  genre: string;
  bpm?: number;
  key?: string;
  coverArt: string;
  accentColor: string;
  gradient: string;
  lyricsSnippet?: string;
  linerNotes?: string;
  isLiked?: boolean;
}

export interface ExplorationPlaylist {
  id: string;
  name: string;
  description: string;
  trackCount: number;
  coverArt: string;
  isSmartMix: boolean;
  accentColor?: string;
  tracks: ExplorationTrack[];
}

export interface ExplorationAlbum {
  id: string;
  title: string;
  artist: string;
  year: number;
  trackCount: number;
  coverArt: string;
  genre: string;
  tracks: ExplorationTrack[];
}

export interface CategoryStage {
  id: string;
  number: string;
  title: string;
  subtitle: string;
  tag: string;
  description: string;
  tracks: ExplorationTrack[];
}

export interface EditorialFeature {
  issueNumber: string;
  title: string;
  subtitle: string;
  essay: string;
  curator: string;
  curatorRole: string;
  publishedDate: string;
  primaryTrack: ExplorationTrack;
  secondaryTracks: ExplorationTrack[];
  quote: string;
  quoteAuthor: string;
}

export interface ExplorationStats {
  totalHours: number;
  topGenre: string;
  topArtists: Array<{ name: string; hours: number; affinity: number; image?: string }>;
  recentRotation: ExplorationTrack[];
  weeklyListening: Array<{ day: string; hours: number }>;
  acousticEras: Array<{ era: string; description: string; percentage: number }>;
}

export interface ExplorationAppContext {
  currentView: AppViewId;
  setCurrentView: (view: AppViewId) => void;
  tracks: ExplorationTrack[];
  playlists: ExplorationPlaylist[];
  albums: ExplorationAlbum[];
  currentTrack: ExplorationTrack;
  isPlaying: boolean;
  playbackSecs: number;
  volume: number;
  isShuffled: boolean;
  repeatMode: "off" | "all" | "one";
  isLiked: boolean;
  searchQuery: string;
  setSearchQuery: (q: string) => void;
  onPlayTrack: (track: ExplorationTrack) => void;
  onTogglePlayPause: () => void;
  onNextTrack: () => void;
  onPrevTrack: () => void;
  onSeek: (seconds: number) => void;
  onVolumeChange: (vol: number) => void;
  onToggleShuffle: () => void;
  onToggleRepeat: () => void;
  onToggleLike: (trackId: string) => void;
  onOpenAlbum?: (album: ExplorationAlbum) => void;
  onOpenPlaylist?: (playlist: ExplorationPlaylist) => void;
}
