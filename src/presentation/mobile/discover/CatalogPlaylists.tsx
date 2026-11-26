import type { CatalogPlaylist } from "../../../navigationHistory";
import type { MobileScreenProps } from "../types";
import { Sleeve } from "../Sleeve";

export function CatalogPlaylistRow({ c, playlist: p }: MobileScreenProps & { playlist: CatalogPlaylist }) {
  return (
    <button key={p.id} type="button" className="m-search-artist" onClick={() => c.handleOpenCollection({
      id: `jiosaavn:playlist:${p.id}`, type: "chart", title: p.title, subtitle: "JioSaavn playlist",
      tag: "PLAYLIST", cover_art_url: p.image,
    })}>
      <Sleeve size="edge" src={p.image} title={p.title} />
      <span className="m-search-artist__text"><strong>{p.title}</strong><span className="m-kicker m-kicker--muted">JIOSAAVN PLAYLIST</span></span>
    </button>
  );
}
