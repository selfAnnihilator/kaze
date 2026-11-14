import { ImportPlaylistModal } from "../ImportPlaylistModal";
import type { PlaylistsViewContext } from "./context";

export function JioSaavnImportModal({ c }: { c: PlaylistsViewContext }) {
  return (
    <>
      {c.showJioSaavnModal && (
        <ImportPlaylistModal
          onClose={() => c.setShowJioSaavnModal(false)}
          onInspectJioSaavnPlaylist={c.onInspectJioSaavnPlaylist}
          onSaveImportedPlaylist={c.onSaveImportedPlaylist}
          onSearchDirect={c.onSearchDirect}
          currentUser={c.currentUser}
          onOpenAuthModal={c.onOpenAuthModal}
        />
      )}
    </>
  );
}
