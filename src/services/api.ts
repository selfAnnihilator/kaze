import { Command, Query, QueryResponse } from "../types";

// Helper checking if running within Tauri desktop container
export const isTauri = (): boolean => {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
};

export async function dispatchCommand(command: Command): Promise<any> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("execute_command", { command });
  } else {
    console.log("[Mock Browser API] dispatchCommand:", command);
    return { status: "Ok" };
  }
}

export async function executeQuery(query: Query): Promise<QueryResponse> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke("execute_query", { query });
  } else {
    console.log("[Mock Browser API] executeQuery:", query);
    return mockQueryResponse(query);
  }
}

export async function subscribeBackendEvents(callback: (event: any) => void): Promise<() => void> {
  if (isTauri()) {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen("backend-event", (e) => callback(e.payload));
    return unlisten;
  } else {
    // In browser, emit periodic mock ticker event for testing if needed
    return () => {};
  }
}

function mockQueryResponse(query: Query): QueryResponse {
  switch (query.query) {
    case "GetOnboardingStatus":
      return {
        type: "OnboardingStatus",
        completed: true,
        default_music_dir: "/home/user/Music",
        configured_folders: [{ id: "f1", path: "/home/user/Music", track_count: 42 }],
      };
    case "GetPlaybackState":
      return {
        type: "PlaybackState",
        data: {
          is_playing: false,
          position_secs: 0,
          duration_secs: 240,
          volume: 0.8,
          is_muted: false,
          repeat_mode: "off",
          is_shuffled: false,
        },
      };
    case "GetTracks":
      return {
        type: "Tracks",
        data: [
          {
            id: "t1",
            title: "Bohemian Rhapsody",
            artist_name: "Queen",
            album_title: "A Night at the Opera",
            duration_secs: 354,
            format: "flac",
            has_cover_art: 1,
          },
          {
            id: "t2",
            title: "Time",
            artist_name: "Pink Floyd",
            album_title: "The Dark Side of the Moon",
            duration_secs: 413,
            format: "flac",
            has_cover_art: 1,
          },
        ],
      };
    case "GetWishlist":
      return {
        type: "Wishlist",
        data: [
          {
            id: "wl_1",
            title: "Comfortably Numb",
            artist: "Pink Floyd",
            album: "The Wall",
            status: "WANT",
            created_at: Date.now() / 1000,
          },
        ],
      };
    case "GetDownloads":
      return {
        type: "Downloads",
        data: [],
      };
    default:
      return { type: "Empty", data: [] };
  }
}
