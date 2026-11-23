import { useState, type ComponentProps, type FormEvent } from "react";
import { Check, Plus } from "lucide-react";
import { MobileSheet, joinSub } from "./MobileSheet";
import type { AddToPlaylistModal } from "../../../components/modals/AddToPlaylistModal";

type Props = ComponentProps<typeof AddToPlaylistModal>;

/** Add to Playlist (Figma G2): tick playlists, or name a new one, then Done. */
export function AddToPlaylistSheet({ isOpen, onClose, track, playlists, trackPlaylistIds, onTogglePlaylist, onCreatePlaylist }: Props) {
  const [name, setName] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState(false);
  if (!isOpen || !track) return null;
  const toggle = async (id: string, member: boolean) => {
    setBusy(id);
    setError(false);
    try { await onTogglePlaylist(id, member); } catch { setError(true); } finally { setBusy(null); }
  };
  const create = async (e: FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    setBusy("new");
    try {
      const id = await onCreatePlaylist(name.trim());
      if (id) { await onTogglePlaylist(id, false); setName(""); }
    } catch { setError(true); } finally { setBusy(null); }
  };
  return (
    <MobileSheet kicker="ADD TO PLAYLIST" onClose={onClose}
      song={{ title: track.title, subtitle: joinSub(track.artist, track.album), cover: track.cover_art_url, trackId: track.id }}>
      {playlists.map((p) => {
        const member = trackPlaylistIds.includes(p.id);
        return (
          <button key={p.id} type="button" className="m-bsheet__row m-bsheet__row--pick" disabled={busy !== null} aria-pressed={member} onClick={() => void toggle(p.id, member)}>
            <span className={`m-check ${member ? "is-on" : ""}`}>{member && <Check size={14} strokeWidth={2} />}</span>
            <strong>{p.name}</strong><span className="m-profile-meta">{p.track_count ?? 0} TRACKS</span>
          </button>
        );
      })}
      {!playlists.length && <p className="m-bsheet__intro">No playlists yet. Name one below to make it.</p>}
      <form className="m-bsheet__create" onSubmit={(e) => void create(e)}>
        <label className="m-field m-field--sm">
          <Plus size={18} strokeWidth={2} className="m-field__icon" />
          <input className="m-field__input" placeholder="New playlist name…" value={name} onChange={(e) => setName(e.target.value)} aria-label="New playlist name" />
        </label>
        <button type="submit" className="m-btn m-btn--outline m-btn--sm" disabled={busy !== null || !name.trim()}>CREATE</button>
      </form>
      {error && <p className="m-set-note is-error m-bsheet__intro" role="alert">Couldn’t update your playlists. Try again.</p>}
      <div className="m-bsheet__foot"><button type="button" className="m-btn m-btn--grow" onClick={onClose}>DONE</button></div>
    </MobileSheet>
  );
}
