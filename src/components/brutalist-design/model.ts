import {
  CHAPTERS,
  INITIAL_QUEUE,
  TRACKS,
  SpatialTrack,
} from "../spatial-design/data";

// Read-only catalogue reuse. This preview has no production playback imports.
export {
  ALBUMS,
  CHAPTERS,
  PLAYLISTS,
  TRACKS,
  formatTime,
  standardHref,
} from "../spatial-design/data";
export type { SpatialTrack as Track } from "../spatial-design/data";
export type Page =
  "listen" | "discover" | "library" | "queue" | "collection" | "search";
export type Destination = { page: Page; id?: string };
export interface DeckState {
  queue: SpatialTrack[];
  cursor: number;
  playing: boolean;
  flow: boolean;
  repeat: boolean;
  shuffle: boolean;
  liked: string[];
}
export const initialDeck: DeckState = {
  queue: INITIAL_QUEUE,
  cursor: 2,
  playing: false,
  flow: false,
  repeat: false,
  shuffle: false,
  liked: ["migration-0", "modal-soul-0"],
};
export type Action =
  | { type: "play"; id: string }
  | { type: "selection"; tracks: SpatialTrack[] }
  | {
      type:
        | "toggle"
        | "next"
        | "previous"
        | "like"
        | "flow"
        | "repeat"
        | "shuffle"
        | "clear";
    }
  | { type: "move"; id: string; offset: number }
  | { type: "remove"; id: string };

export function deckReducer(state: DeckState, action: Action): DeckState {
  switch (action.type) {
    case "play": {
      const index = state.queue.findIndex((track) => track.id === action.id);
      if (index >= 0) return { ...state, cursor: index, playing: true };
      const track = TRACKS.find((item) => item.id === action.id);
      return track
        ? {
            ...state,
            queue: [...state.queue, track],
            cursor: state.queue.length,
            playing: true,
          }
        : state;
    }
    case "selection":
      return action.tracks.length
        ? {
            ...state,
            queue: action.tracks,
            cursor: 0,
            playing: true,
            flow: false,
            shuffle: false,
          }
        : state;
    case "toggle":
      return { ...state, playing: !state.playing };
    case "next":
      return {
        ...state,
        cursor: state.repeat
          ? (state.cursor + 1) % state.queue.length
          : Math.min(state.cursor + 1, state.queue.length - 1),
      };
    case "previous":
      return { ...state, cursor: Math.max(0, state.cursor - 1) };
    case "like": {
      const id = state.queue[state.cursor].id;
      return {
        ...state,
        liked: state.liked.includes(id)
          ? state.liked.filter((item) => item !== id)
          : [...state.liked, id],
      };
    }
    case "repeat":
      return { ...state, repeat: !state.repeat };
    case "shuffle": {
      const ahead = state.queue.slice(state.cursor + 1);
      // Stable permutation keeps StrictMode updater calls deterministic.
      const reordered = state.shuffle
        ? [...ahead].sort((a, b) => a.id.localeCompare(b.id))
        : [
            ...ahead.filter((_, i) => i % 2),
            ...ahead.filter((_, i) => !(i % 2)),
          ];
      return {
        ...state,
        shuffle: !state.shuffle,
        queue: [...state.queue.slice(0, state.cursor + 1), ...reordered],
      };
    }
    case "flow":
      return state.flow
        ? { ...state, flow: false }
        : {
            ...state,
            flow: true,
            playing: true,
            shuffle: false,
            cursor: 0,
            queue: [
              ...new Map(
                CHAPTERS.flatMap((chapter) => chapter.tracks).map((track) => [
                  track.id,
                  track,
                ]),
              ).values(),
            ],
          };
    case "clear":
      return { ...state, queue: state.queue.slice(0, state.cursor + 1) };
    case "move": {
      const index = state.queue.findIndex((track) => track.id === action.id);
      const target = index + action.offset;
      if (
        index <= state.cursor ||
        target <= state.cursor ||
        target >= state.queue.length
      )
        return state;
      const queue = [...state.queue];
      [queue[index], queue[target]] = [queue[target], queue[index]];
      return { ...state, queue };
    }
    case "remove":
      return {
        ...state,
        queue: state.queue.filter(
          (track, index) => index <= state.cursor || track.id !== action.id,
        ),
      };
  }
}
