import { dispatchCommand, executeQuery } from "../../services/api";
import type { Track } from "../../types";
import { showPhoneToast } from "./phoneToasts";

/**
 * Plays a whole album in track order (Library › Albums: tapping the card or
 * PLAY, Figma D1). Stops at the album's end like any played list.
 */
export async function playAlbum(albumId: string) {
  try {
    const response = await executeQuery({ query: "GetAlbumDetails", payload: { album_id: albumId } });
    const tracks: Track[] = Array.isArray(response.data?.tracks) ? response.data.tracks : [];
    const ids = tracks.map((track) => track.id);
    if (!ids.length) throw Error("empty album");
    await dispatchCommand({ command: "PlayTrackList", payload: { track_ids: ids, start_track_id: null, shuffle: false } });
  } catch {
    // Playback events update the Deck; only a failure needs a word.
    showPhoneToast("Couldn’t play this album");
  }
}
