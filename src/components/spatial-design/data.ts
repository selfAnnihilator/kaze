import artwork from "./artwork.json";

export interface SpatialTrack {
  id: string;
  title: string;
  artist: string;
  albumId: string;
  album: string;
  artwork: string;
  duration: number;
  genre: string;
}

const titles: Record<string, string[]> = {
  migration: ["Kerala", "Break Apart", "Bambro Koyo Ganda"],
  "modal-soul": ["Feather", "Ordinary Joe", "Reflection Eternal"],
  "con-todo": ["Friday Morning", "Cómo Me Quieres", "August 10"],
  "in-rainbows": ["Weird Fishes / Arpeggi", "Nude", "Reckoner"],
  "moon-safari": ["La femme d’argent", "All I Need", "You Make It Easy"],
  mezzanine: ["Teardrop", "Angel", "Inertia Creeps"],
  dummy: ["Glory Box", "Sour Times", "Roads"],
  currents: ["Let It Happen", "Eventually", "Yes I’m Changing"],
  bloom: ["Myth", "Lazuli", "Wild"],
  jubilee: ["Paprika", "Be Sweet", "Kokomo, IN"],
  "black-sands": ["Kiara", "Kong", "Black Sands"],
  awake: ["Awake", "Montana", "Dye"],
};

export const ALBUMS = artwork;
// The catalogue and durations are a finite visual fixture, never backend data.
export const TRACKS: SpatialTrack[] = ALBUMS.flatMap((album, albumIndex) =>
  (titles[album.id] ?? [album.title]).map((title, index) => ({
    id: `${album.id}-${index}`,
    title,
    artist: album.artist,
    albumId: album.id,
    album: album.title,
    artwork: album.artwork,
    genre: album.genre,
    duration: [237, 288, 321, 264, 218, 305][(albumIndex + index) % 6],
  })),
);

export const INITIAL_QUEUE = [
  TRACKS[3],
  TRACKS[9],
  TRACKS[0],
  TRACKS[6],
  TRACKS[15],
  TRACKS[33],
  TRACKS[24],
  TRACKS[30],
  TRACKS[18],
  TRACKS[27],
  TRACKS[12],
  TRACKS[21],
];
export const PLAYLISTS = [
  {
    id: "after-hours",
    name: "After hours",
    description: "For when the city starts to quiet down.",
    tracks: [
      TRACKS[0],
      TRACKS[3],
      TRACKS[15],
      TRACKS[18],
      TRACKS[12],
      TRACKS[24],
    ],
    cover: ALBUMS[5].artwork,
  },
  {
    id: "slow-sundays",
    name: "Slow Sundays",
    description: "No particular place to be.",
    tracks: [TRACKS[6], TRACKS[3], TRACKS[12], TRACKS[27], TRACKS[33]],
    cover: ALBUMS[2].artwork,
  },
  {
    id: "on-repeat",
    name: "On repeat",
    description: "The tracks you keep coming back to.",
    tracks: [TRACKS[9], TRACKS[21], TRACKS[24], TRACKS[30], TRACKS[0]],
    cover: ALBUMS[3].artwork,
  },
];

export const CHAPTERS = [
  {
    title: "For you",
    subtitle: "A little familiar. A little unexpected.",
    description: "Follow the feeling, see where it takes you.",
    album: ALBUMS[2],
    tracks: [
      TRACKS[6],
      TRACKS[0],
      TRACKS[3],
      TRACKS[12],
      TRACKS[30],
      TRACKS[33],
    ],
  },
  {
    title: "Hidden gems",
    subtitle: "The good ones you haven’t met yet.",
    description: "Take the long way to your next favourite.",
    album: ALBUMS[1],
    tracks: [
      TRACKS[4],
      TRACKS[19],
      TRACKS[14],
      TRACKS[31],
      TRACKS[25],
      TRACKS[8],
    ],
  },
  {
    title: "New to you",
    subtitle: "Fresh additions to your rotation.",
    description: "An open door to a different sound.",
    album: ALBUMS[9],
    tracks: [
      TRACKS[27],
      TRACKS[21],
      TRACKS[33],
      TRACKS[24],
      TRACKS[15],
      TRACKS[9],
    ],
  },
  {
    title: "Because you liked Bonobo",
    subtitle: "More colour. More room to wander.",
    description: "From warm beats to the spaces in between.",
    album: ALBUMS[10],
    tracks: [
      TRACKS[30],
      TRACKS[33],
      TRACKS[12],
      TRACKS[6],
      TRACKS[3],
      TRACKS[18],
    ],
  },
];

export function formatTime(value: number) {
  const seconds = Math.max(0, Math.floor(value));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

export function standardHref() {
  const url = new URL(window.location.href);
  url.searchParams.delete("design");
  url.searchParams.delete("lab");
  url.searchParams.set("model", "standard");
  url.hash = "";
  return `${url.pathname}${url.search}${url.hash}`;
}
