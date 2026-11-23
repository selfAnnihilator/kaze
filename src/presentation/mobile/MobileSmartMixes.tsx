import { memo, useState } from "react";
import { ArrowUpRight, BookmarkCheck, BookmarkPlus, Play } from "lucide-react";
import { playlistCollection } from "./playlistCollection";
import { Sleeve } from "./Sleeve";
import type { Playlist } from "../../types";
import type { MobileScreenProps } from "./types";

type Ctx = MobileScreenProps["c"];

/** One mix per name (the generator can repeat a name). */
function uniqueMixes(playlists: Playlist[]): Playlist[] {
  const seen = new Set<string>();
  return playlists.filter((p) => {
    const key = p.name.trim().toLowerCase();
    if (p.is_smart_mix !== 1 || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

/** Save a mix as one of your playlists (same steps as the PC). */
function useSaveMix(c: Ctx) {
  const [saved, setSaved] = useState<Set<string>>(new Set());
  const [saving, setSaving] = useState<string | null>(null);
  const save = async (mix: Playlist) => {
    setSaving(mix.id);
    try {
      const tracks = await c.handleFetchPlaylistTracks(mix.id);
      await c.handleSaveImportedPlaylist(mix.name.trim(), tracks.map((t) => t.id));
      setSaved((prev) => new Set(prev).add(mix.id));
    } catch {
      /* the playlist hook reports failures */
    } finally {
      setSaving(null);
    }
  };
  return { saved, saving, save };
}

function MixCard({ mix, c, saver }: { mix: Playlist; c: Ctx; saver: ReturnType<typeof useSaveMix> }) {
  const isSaved = saver.saved.has(mix.id);
  return (
    <article className="m-mix">
      <div className="m-mix__top">
        <Sleeve size="mix" src={mix.cover_art_url} trackId={mix.first_track_id} title={mix.name} />
        <div className="m-mix__text">
          <span className="m-mix__tag">SMART MIX</span>
          <h2 className="m-mix__title">{mix.name}</h2>
          <span className="m-profile-meta">{mix.track_count ?? 0} TRACKS</span>
        </div>
      </div>
      {(mix.generation_reason || mix.description) && <p className="m-mix__why">{mix.generation_reason || mix.description}</p>}
      <div className="m-mix__actions">
        <button type="button" className="m-btn m-btn--sm m-mix__play" onClick={() => void c.handlePlayPlaylist(mix.id)}>
          <Play size={18} strokeWidth={2} /><span>PLAY MIX</span>
        </button>
        <button type="button" className="m-btn m-btn--outline m-btn--sm" disabled={isSaved || saver.saving === mix.id} onClick={() => void saver.save(mix)}>
          {isSaved ? <BookmarkCheck size={18} strokeWidth={2} /> : <BookmarkPlus size={18} strokeWidth={2} />}
          <span>{isSaved ? "SAVED" : saver.saving === mix.id ? "SAVING…" : "SAVE"}</span>
        </button>
        <button type="button" className="m-btn m-btn--outline m-btn--sm m-mix__open" onClick={() => c.handleOpenCollection(playlistCollection(mix))} aria-label={`Open ${mix.name}`}>
          <ArrowUpRight size={20} strokeWidth={2} />
        </button>
      </div>
    </article>
  );
}

/** Smart Mixes (Figma D5): mix cards with why, Play Mix, Save and Open. */
export const MobileSmartMixes = memo(function MobileSmartMixes({ c }: MobileScreenProps) {
  const mixes = uniqueMixes(c.playlists);
  const saver = useSaveMix(c);
  return (
    <div className="m-set">
      <div className="m-set-hero m-set-hero--plain">
        <span className="m-kicker">03 // LIBRARY / MORE</span>
        <h1 className="m-display m-mixes__title">SMART MIXES.</h1>
      </div>
      <span className="m-profile-meta m-mixes__sub">CURATED SMART MIXES &amp; RECOMMENDATIONS</span>
      <div className="m-mixes">
        {mixes.map((mix) => <MixCard key={mix.id} mix={mix} c={c} saver={saver} />)}
        {!mixes.length && <p className="m-empty-msg">No smart mixes yet. Play songs for a while and Kaze will build mixes from what you love.</p>}
      </div>
    </div>
  );
});
