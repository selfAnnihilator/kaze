import { useState } from "react";
import { MoreVertical, Pencil, Trash2 } from "lucide-react";
import { confirmAction } from "../../../services/confirmDialog";
import { usePromptDialog } from "../../../hooks/app/usePromptDialog";
import type { MobileScreenProps } from "../types";

type Ctx = MobileScreenProps["c"];

/**
 * Playlist Options (Figma D7): the ⋮ in an open playlist's back bar opens a
 * small menu with Rename (Name It prompt, G7) and Delete (confirm, G8).
 */
export function PlaylistOptions({ c, playlistId, name }: { c: Ctx; playlistId: string; name: string }) {
  const [open, setOpen] = useState(false);
  const { requestPrompt, promptDialog } = usePromptDialog();
  const rename = async () => {
    setOpen(false);
    const next = await requestPrompt("Rename playlist", "Name it.", name);
    if (next?.trim() && next.trim() !== name) await c.handleRenamePlaylist(playlistId, next.trim());
  };
  const remove = async () => {
    setOpen(false);
    const ok = await confirmAction({ title: `Delete “${name}”?`, message: "The playlist goes away. The songs stay in your library.", confirmLabel: "Delete", cancelLabel: "Keep" });
    if (ok) await c.handleDeletePlaylist(playlistId);
  };
  return (
    <>
      <button type="button" className="m-backbar__btn m-backbar__more" aria-label="Playlist options" aria-expanded={open} onClick={() => setOpen(!open)}>
        <MoreVertical size={20} strokeWidth={2} />
      </button>
      {open && (
        <>
          <button type="button" className="m-menu-scrim" aria-label="Close menu" onClick={() => setOpen(false)} />
          <div className="m-menu" role="menu">
            <span className="m-kicker m-menu__head">PLAYLIST OPTIONS</span>
            <button type="button" role="menuitem" className="m-menu__item" onClick={() => void rename()}><Pencil size={18} strokeWidth={2} /><span>Rename</span></button>
            <button type="button" role="menuitem" className="m-menu__item is-danger" onClick={() => void remove()}><Trash2 size={18} strokeWidth={2} /><span>Delete</span></button>
          </div>
        </>
      )}
      {promptDialog}
    </>
  );
}
