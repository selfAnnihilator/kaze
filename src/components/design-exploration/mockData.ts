import type { DesignModelMeta, ExplorationTrack, CategoryStage, EditorialFeature } from "./types.ts";

// Helper to generate abstract artistic SVG artwork data URIs
function createArtSvg(
  bg: string,
  fg: string,
  symbol: "circle" | "waveform" | "grid" | "monolith" | "orbit" | "sun" | "lines",
  seed = 1
): string {
  const shapes: Record<string, string> = {
    circle: `<circle cx="150" cy="150" r="85" fill="${fg}" fill-opacity="0.22" stroke="${fg}" stroke-width="2"/>
             <circle cx="150" cy="150" r="42" fill="${fg}" fill-opacity="0.7"/>
             <circle cx="150" cy="150" r="10" fill="#ffffff" fill-opacity="0.9"/>`,
    waveform: `<path d="M40 150 Q 80 70, 110 150 T 170 150 T 230 150 T 260 150" fill="none" stroke="${fg}" stroke-width="6" stroke-linecap="round"/>
               <path d="M40 170 Q 90 120, 130 170 T 210 170 T 260 170" fill="none" stroke="${fg}" stroke-width="3" stroke-opacity="0.5" stroke-linecap="round"/>`,
    grid: `<line x1="60" y1="60" x2="240" y2="240" stroke="${fg}" stroke-width="2" stroke-opacity="0.4"/>
           <line x1="60" y1="240" x2="240" y2="60" stroke="${fg}" stroke-width="2" stroke-opacity="0.4"/>
           <rect x="90" y="90" width="120" height="120" fill="none" stroke="${fg}" stroke-width="3" rx="8"/>`,
    monolith: `<rect x="110" y="50" width="80" height="200" fill="${fg}" fill-opacity="0.85" rx="4"/>
               <line x1="80" y1="250" x2="220" y2="250" stroke="${fg}" stroke-width="3"/>
               <circle cx="150" cy="35" r="12" fill="${fg}"/>`,
    orbit: `<ellipse cx="150" cy="150" rx="100" ry="45" fill="none" stroke="${fg}" stroke-width="3" transform="rotate(-25 150 150)"/>
            <circle cx="150" cy="150" r="32" fill="${fg}"/>
            <circle cx="210" cy="120" r="10" fill="#ffffff"/>`,
    sun: `<circle cx="150" cy="140" r="60" fill="${fg}"/>
          <path d="M60 210 L240 210 M75 225 L225 225 M95 240 L205 240" stroke="${fg}" stroke-width="4" stroke-opacity="0.7"/>`,
    lines: `<line x1="50" y1="80" x2="250" y2="80" stroke="${fg}" stroke-width="4"/>
            <line x1="50" y1="120" x2="190" y2="120" stroke="${fg}" stroke-width="4"/>
            <line x1="50" y1="160" x2="230" y2="160" stroke="${fg}" stroke-width="4"/>
            <line x1="50" y1="200" x2="160" y2="200" stroke="${fg}" stroke-width="4"/>`,
  };

  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 300" width="300" height="300">
    <defs>
      <linearGradient id="bg${seed}" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" stop-color="${bg}"/>
        <stop offset="100%" stop-color="#0c0a09"/>
      </linearGradient>
    </defs>
    <rect width="300" height="300" fill="url(#bg${seed})"/>
    ${shapes[symbol] || shapes.circle}
  </svg>`;

  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

export const DESIGN_MODELS: DesignModelMeta[] = [
  {
    id: "drift",
    name: "A: Drift",
    codename: "Kinetic Spatial Music",
    tagline: "Warm, flowing music player with diagonal Now Playing and spring-physics motion",
    theme: "Deep Warm Charcoal & Album-Derived Amber",
    layoutDescription: "Narrow icon rail navigation, asymmetric compositions, mixed-scale artwork, diagonal drift Now Playing with past/present/future spatial layout.",
    accent: "#D4883A",
    keyShortcut: "1",
  },
  {
    id: "broadsheet",
    name: "B: Broadsheet",
    codename: "Music Brutalism / Zine",
    tagline: "Bold typography, zero-radius edges, player integrated into navigation column",
    theme: "True Black & Single Cadmium Red",
    layoutDescription: "Wide left column with integrated player and queue, exposed grid borders, oversized track numbers, hard-edge artwork, zine-poster composition.",
    accent: "#E8431A",
    keyShortcut: "2",
  },
  {
    id: "gallery",
    name: "C: Gallery",
    codename: "Art Exhibition Music",
    tagline: "Warm gallery lighting, serif headings, generous breathing space, art-first presentation",
    theme: "Espresso & Cream with Amber-Gold",
    layoutDescription: "Refined serif sidebar navigation, wide player bar, gallery-lighting glow on artwork, featured rotating album, slow contemplative motion.",
    accent: "#C7903E",
    keyShortcut: "3",
  },
];

export const MOCK_TRACKS: ExplorationTrack[] = [
  {
    id: "exp-01",
    title: "Celestial Drift",
    artist: "Kurogane Sound",
    album: "Architects of the Horizon",
    year: 2026,
    durationSecs: 268,
    genre: "Downtempo",
    bpm: 94,
    key: "F# min",
    coverArt: createArtSvg("#2b1b17", "#f3701e", "orbit", 1),
    accentColor: "#f3701e",
    gradient: "linear-gradient(135deg, #f3701e33 0%, #1e130f 100%)",
    lyricsSnippet: "Across the overpass, amber streetlamps bleeding through the rain...",
    linerNotes: "Recorded at Echo Chamber B, Shibuya. Mastered to half-inch tape.",
    isLiked: true,
  },
  {
    id: "exp-02",
    title: "Subsurface Resonance",
    artist: "Aethelgard & Sylvan",
    album: "Geology of Sound",
    year: 2025,
    durationSecs: 312,
    genre: "Ambient House",
    bpm: 118,
    key: "D min",
    coverArt: createArtSvg("#14232a", "#38bdf8", "waveform", 2),
    accentColor: "#38bdf8",
    gradient: "linear-gradient(135deg, #38bdf833 0%, #0d171d 100%)",
    lyricsSnippet: "Only the reverberation of basalt beneath forty fathoms...",
    linerNotes: "Hydrophone recordings from the Mariana trench merged with Buchla synths.",
    isLiked: false,
  },
  {
    id: "exp-03",
    title: "Shinjuku Monolith",
    artist: "Tetsuo Kanno",
    album: "Monochrome Cityscape",
    year: 2026,
    durationSecs: 215,
    genre: "Industrial Minimalist",
    bpm: 126,
    key: "A min",
    coverArt: createArtSvg("#23201d", "#d4a373", "monolith", 3),
    accentColor: "#d4a373",
    gradient: "linear-gradient(135deg, #d4a37333 0%, #181513 100%)",
    lyricsSnippet: "Glass and steel reflecting the noon sky, pedestrians in synchronized phase...",
    linerNotes: "Composed on three analog drum synthesizers through a plate reverb.",
    isLiked: true,
  },
  {
    id: "exp-04",
    title: "Solar Flare Anthem",
    artist: "Vespera Collective",
    album: "Heliospheric Waves",
    year: 2024,
    durationSecs: 245,
    genre: "Space Disco",
    bpm: 122,
    key: "E maj",
    coverArt: createArtSvg("#331828", "#f72585", "sun", 4),
    accentColor: "#f72585",
    gradient: "linear-gradient(135deg, #f7258533 0%, #1a0f16 100%)",
    lyricsSnippet: "Golden coronal arches burning through the void...",
    linerNotes: "Vintage Roland Juno-106 bassline recorded directly to analog desk.",
    isLiked: false,
  },
  {
    id: "exp-05",
    title: "Velvet Static & Rain",
    artist: "Mirei Hashimoto",
    album: "Afterhours in Roppongi",
    year: 2026,
    durationSecs: 198,
    genre: "City Pop",
    bpm: 88,
    key: "Bb maj",
    coverArt: createArtSvg("#1b241e", "#2dd4bf", "circle", 5),
    accentColor: "#2dd4bf",
    gradient: "linear-gradient(135deg, #2dd4bf33 0%, #0d1512 100%)",
    lyricsSnippet: "Two cups of lukewarm coffee, smoke rising in lazy coils...",
    linerNotes: "Fender Rhodes Mark I played with vintage felt hammers.",
    isLiked: true,
  },
  {
    id: "exp-06",
    title: "Quantum State Vector",
    artist: "Decay Mode",
    album: "Non-Euclidean Geometries",
    year: 2026,
    durationSecs: 284,
    genre: "Experimental Bass",
    bpm: 140,
    key: "C min",
    coverArt: createArtSvg("#161a29", "#00f5d4", "grid", 6),
    accentColor: "#00f5d4",
    gradient: "linear-gradient(135deg, #00f5d433 0%, #0a0d17 100%)",
    lyricsSnippet: "Probabilities collapse along the boundary...",
    linerNotes: "Custom C++ physical modeling modules at 192kHz.",
    isLiked: false,
  },
  {
    id: "exp-07",
    title: "Autumnal Etude in G Minor",
    artist: "Claire Delacroix",
    album: "Chamber Works Vol. IV",
    year: 2025,
    durationSecs: 342,
    genre: "Modern Classical",
    bpm: 72,
    key: "G min",
    coverArt: createArtSvg("#2d1f1c", "#e2a876", "lines", 7),
    accentColor: "#e2a876",
    gradient: "linear-gradient(135deg, #e2a87633 0%, #1a1210 100%)",
    lyricsSnippet: "[Instrumental — Cello and prepared piano]",
    linerNotes: "Recorded at Sainte-Chapelle, Paris. Neumann M49 pair.",
    isLiked: true,
  },
  {
    id: "exp-08",
    title: "Hyperdrive Velocity",
    artist: "Vector 99",
    album: "Neon Interceptor",
    year: 2026,
    durationSecs: 230,
    genre: "Darksynth",
    bpm: 132,
    key: "D min",
    coverArt: createArtSvg("#22152b", "#b5179e", "waveform", 8),
    accentColor: "#b5179e",
    gradient: "linear-gradient(135deg, #b5179e33 0%, #150d1b 100%)",
    lyricsSnippet: "Redline tachometer pinning against the glass...",
    linerNotes: "Mastered for high-output subwoofers with linear-phase transients.",
    isLiked: false,
  },
];

export const CATEGORY_STAGES: CategoryStage[] = [
  {
    id: "stage-01",
    number: "01",
    title: "For You",
    subtitle: "Curated from your listening history",
    tag: "PERSONALIZED",
    description: "Tracks chosen based on your recent rotation, favorite genres, and listening patterns.",
    tracks: [MOCK_TRACKS[0], MOCK_TRACKS[1], MOCK_TRACKS[4], MOCK_TRACKS[6]],
  },
  {
    id: "stage-02",
    number: "02",
    title: "Hidden Gems",
    subtitle: "Lesser-known tracks in genres you love",
    tag: "DISCOVERY",
    description: "Deep cuts and overlooked releases from artists adjacent to your favorites.",
    tracks: [MOCK_TRACKS[2], MOCK_TRACKS[5], MOCK_TRACKS[7], MOCK_TRACKS[0]],
  },
  {
    id: "stage-03",
    number: "03",
    title: "New Releases",
    subtitle: "Fresh music from artists you follow",
    tag: "THIS WEEK",
    description: "Recent releases from artists in your library and related recommendations.",
    tracks: [MOCK_TRACKS[4], MOCK_TRACKS[2], MOCK_TRACKS[0], MOCK_TRACKS[3]],
  },
  {
    id: "stage-04",
    number: "04",
    title: "Because You Liked Kurogane Sound",
    subtitle: "Similar artists and sound profiles",
    tag: "RECOMMENDED",
    description: "If you enjoyed Architects of the Horizon, these tracks share a similar sonic palette.",
    tracks: [MOCK_TRACKS[1], MOCK_TRACKS[6], MOCK_TRACKS[5], MOCK_TRACKS[2]],
  },
  {
    id: "stage-05",
    number: "05",
    title: "Trending",
    subtitle: "What's popular right now",
    tag: "POPULAR",
    description: "The most played and shared tracks across the community this week.",
    tracks: [MOCK_TRACKS[7], MOCK_TRACKS[3], MOCK_TRACKS[5], MOCK_TRACKS[1]],
  },
];

export const EDITORIAL_FEATURE: EditorialFeature = {
  issueNumber: "N° 48",
  title: "The Physicality of Sound in a Weightless Era",
  subtitle: "Why the modern listener craves analog friction, tape flutter, and deliberate listening rituals.",
  essay: `In an era where millions of audio files are instantly summoned by algorithms with zero latency, music has paradoxically shed its physical weight. When everything is frictionless, nothing leaves a mark.

The renaissance of deliberate music consumption is not mere nostalgia; it is an instinctual rebellion against ephemeral streams. To listen to an album from beginning to end—to feel the intentional spacing between tracks, the warm saturation of magnetic tape, the microscopic human irregularities—is to anchor oneself in time.`,
  curator: "Kenzo Miyamoto",
  curatorRole: "Music Curator",
  publishedDate: "October 15, 2026",
  primaryTrack: MOCK_TRACKS[0],
  secondaryTracks: [MOCK_TRACKS[1], MOCK_TRACKS[2], MOCK_TRACKS[4], MOCK_TRACKS[6]],
  quote: "Music that is not anchored to a ritual becomes mere wallpaper for the restless mind.",
  quoteAuthor: "Tetsuo Kanno",
};

export const MOCK_ALBUMS = [
  {
    id: "alb-01",
    title: "Architects of the Horizon",
    artist: "Kurogane Sound",
    year: 2026,
    trackCount: 10,
    coverArt: MOCK_TRACKS[0].coverArt,
    genre: "Downtempo",
    tracks: [MOCK_TRACKS[0], MOCK_TRACKS[2], MOCK_TRACKS[4]],
  },
  {
    id: "alb-02",
    title: "Geology of Sound",
    artist: "Aethelgard & Sylvan",
    year: 2025,
    trackCount: 8,
    coverArt: MOCK_TRACKS[1].coverArt,
    genre: "Ambient House",
    tracks: [MOCK_TRACKS[1], MOCK_TRACKS[6]],
  },
  {
    id: "alb-03",
    title: "Monochrome Cityscape",
    artist: "Tetsuo Kanno",
    year: 2026,
    trackCount: 12,
    coverArt: MOCK_TRACKS[2].coverArt,
    genre: "Industrial Minimalist",
    tracks: [MOCK_TRACKS[2], MOCK_TRACKS[3]],
  },
  {
    id: "alb-04",
    title: "Heliospheric Waves",
    artist: "Vespera Collective",
    year: 2024,
    trackCount: 9,
    coverArt: MOCK_TRACKS[3].coverArt,
    genre: "Space Disco",
    tracks: [MOCK_TRACKS[3], MOCK_TRACKS[7]],
  },
  {
    id: "alb-05",
    title: "Afterhours in Roppongi",
    artist: "Mirei Hashimoto",
    year: 2026,
    trackCount: 7,
    coverArt: MOCK_TRACKS[4].coverArt,
    genre: "City Pop",
    tracks: [MOCK_TRACKS[4], MOCK_TRACKS[0]],
  },
  {
    id: "alb-06",
    title: "Non-Euclidean Geometries",
    artist: "Decay Mode",
    year: 2026,
    trackCount: 11,
    coverArt: MOCK_TRACKS[5].coverArt,
    genre: "Experimental",
    tracks: [MOCK_TRACKS[5], MOCK_TRACKS[7]],
  },
];

export const MOCK_PLAYLISTS = [
  {
    id: "pl-01",
    name: "Midnight Drift",
    description: "Deep analog low-end for late-night focus and solitary listening.",
    trackCount: 24,
    coverArt: MOCK_TRACKS[0].coverArt,
    isSmartMix: false,
    accentColor: "#f3701e",
    tracks: [MOCK_TRACKS[0], MOCK_TRACKS[2], MOCK_TRACKS[4], MOCK_TRACKS[7]],
  },
  {
    id: "pl-02",
    name: "Deep Work Flow",
    description: "Adaptive instrumental frequency lock from your top listening patterns.",
    trackCount: 18,
    coverArt: MOCK_TRACKS[1].coverArt,
    isSmartMix: true,
    accentColor: "#38bdf8",
    tracks: [MOCK_TRACKS[1], MOCK_TRACKS[5], MOCK_TRACKS[6]],
  },
  {
    id: "pl-03",
    name: "Tape Saturated Melancholia",
    description: "Warm cassette flutters, Rhodes keys, and rain on window panes.",
    trackCount: 32,
    coverArt: MOCK_TRACKS[4].coverArt,
    isSmartMix: false,
    accentColor: "#2dd4bf",
    tracks: [MOCK_TRACKS[4], MOCK_TRACKS[6], MOCK_TRACKS[0]],
  },
  {
    id: "pl-04",
    name: "Studio Masters",
    description: "Lossless uncompressed master tapes and direct desk recordings.",
    trackCount: 15,
    coverArt: MOCK_TRACKS[5].coverArt,
    isSmartMix: true,
    accentColor: "#00f5d4",
    tracks: [MOCK_TRACKS[5], MOCK_TRACKS[2], MOCK_TRACKS[7]],
  },
];

export const MOCK_STATS = {
  totalHours: 142.8,
  topGenre: "Downtempo & Ambient",
  topArtists: [
    { name: "Kurogane Sound", hours: 48.2, affinity: 96 },
    { name: "Aethelgard & Sylvan", hours: 34.5, affinity: 88 },
    { name: "Tetsuo Kanno", hours: 28.1, affinity: 82 },
    { name: "Mirei Hashimoto", hours: 19.4, affinity: 74 },
    { name: "Decay Mode", hours: 12.6, affinity: 65 },
  ],
  recentRotation: MOCK_TRACKS.slice(0, 5),
  weeklyListening: [
    { day: "Mon", hours: 4.2 },
    { day: "Tue", hours: 5.8 },
    { day: "Wed", hours: 6.4 },
    { day: "Thu", hours: 4.9 },
    { day: "Fri", hours: 7.2 },
    { day: "Sat", hours: 8.5 },
    { day: "Sun", hours: 5.1 },
  ],
  acousticEras: [
    { era: "Morning (06:00–11:00)", description: "Ambient & Classical", percentage: 38 },
    { era: "Afternoon (12:00–18:00)", description: "Uptempo & Electronic", percentage: 42 },
    { era: "Night (20:00–02:00)", description: "Darksynth & Lo-Fi", percentage: 20 },
  ],
};
