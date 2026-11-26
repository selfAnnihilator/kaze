import type { CollectionData } from "../../components/views/collection/types";
import type { Playlist } from "../../types";

export const isPersonalPlaylist = (playlist: Playlist) =>
  playlist.is_smart_mix !== 1 && playlist.name !== "Liked Songs";

/** A playlist or smart mix as the collection the Collection pane opens. */
export function playlistCollection(p: Playlist): CollectionData {
  const mix = p.is_smart_mix === 1;
  return {
    id: p.id,
    type: mix ? "mix" : "playlist",
    title: p.name,
    subtitle: p.description || (mix ? "Smart mix" : "Created playlist"),
    tag: mix ? "SMART MIX" : "PUBLIC PLAYLIST",
    playlistId: p.id,
    cover_art_url: p.cover_art_url,
  };
}
