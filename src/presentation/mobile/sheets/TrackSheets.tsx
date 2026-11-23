import { useState } from "react";
import { BookmarkPlus, ChevronRight, Disc3, Download, Heart, ListPlus, ListStart, Music, Search, ThumbsDown, TrendingDown, UserX } from "lucide-react";
import { dispatchCommand } from "../../../services/api";
import { recommendationPreferences } from "../../../services/recommendationPreferences";
import { useMobileNav } from "../mobileNav";
import { MobileSheet, SheetRow, joinSub } from "./MobileSheet";
import { closeTrackSheet, openTrackSheet, useTrackSheet, type SheetTrack } from "./trackSheet";
import type { Command } from "../../../types";
import type { MobileScreenProps } from "../types";
import { queueRecommendation, queueTrack } from "../phoneToasts";

type Ctx = MobileScreenProps["c"];
const ICON = { size: 20, strokeWidth: 2 } as const;

const songOf = (t: SheetTrack) => ({ title: t.title, subtitle: joinSub(t.artist, t.album), cover: t.cover, trackId: t.id });

/** Rows for songs in this library: like, playlists, queue, play next, not interested. */
function LocalRows({ t, c, liked, then }: { t: SheetTrack; c: Ctx; liked: boolean; then: (work: () => unknown) => () => void }) {
  return (
    <>
      <SheetRow icon={<Heart {...ICON} fill={liked ? "currentColor" : "none"} />} label="Like" note={liked ? "LIKED" : undefined} tone={liked ? "red" : undefined}
        onPress={() => void (liked ? c.handleRemoveFeedback(t.id) : c.handleLike(t.id))} />
      <SheetRow icon={<BookmarkPlus {...ICON} />} label="Add to playlist" onPress={then(() => c.handleOpenAddToPlaylistModal({ id: t.id, title: t.title, artist: t.artist, album: t.album, cover_art_url: t.cover ?? undefined }))} />
      <SheetRow icon={<ListPlus {...ICON} />} label="Add to queue" onPress={then(() => queueTrack(c, t.id))} />
      <SheetRow icon={<ListStart {...ICON} />} label="Play next" onPress={then(() => queueTrack(c, t.id, true))} />
      <SheetRow icon={<ThumbsDown {...ICON} />} label="Not interested" trailing={<ChevronRight size={16} strokeWidth={2} />} onPress={() => openTrackSheet("not_interested", t)} />
    </>
  );
}

/** Track Actions (Figma G1). */
function TrackActions({ t, c }: { t: SheetTrack; c: Ctx }) {
  const nav = useMobileNav();
  const liked = c.likedTrackIds.has(t.id);
  const album = t.album ? c.albums.find((a) => a.title === t.album && a.artist_name === t.artist) : undefined;
  const then = (work: () => unknown) => () => { closeTrackSheet(); void work(); };
  const moreBy = () => {
    nav.goTab("discover");
    c.setGlobalSearchQuery(t.artist || "");
    void c.handleGlobalOnlineSearch(t.artist || "");
  };
  return (
    <MobileSheet kicker="TRACK" onClose={closeTrackSheet} song={songOf(t)}>
      {!t.online && <LocalRows t={t} c={c} liked={liked} then={then} />}
      {t.rec && <SheetRow icon={<ListPlus {...ICON} />} label="Add to queue" onPress={then(() => queueRecommendation(c, t.rec!))} />}
      {t.online && <SheetRow icon={<Download {...ICON} />} label="Download" onPress={then(() => dispatchCommand({ command: "DownloadTrack", payload: { track_id: t.id } }))} />}
      {album && <SheetRow icon={<Disc3 {...ICON} />} label="Go to album" onPress={then(() => nav.openAlbum(album.id))} />}
      {t.artist && <SheetRow icon={<Search {...ICON} />} label={`More by ${t.artist}`} onPress={then(moreBy)} />}
    </MobileSheet>
  );
}

/** Not Interested (Figma G3): not this song, not this artist, or less like this. */
function NotInterested({ t, c }: { t: SheetTrack; c: Ctx }) {
  const [error, setError] = useState(false);
  const genre = c.tracks.find((x) => x.id === t.id)?.genre_name;
  const run = async (command: Command, after?: () => void) => {
    try {
      await dispatchCommand(command);
      after?.();
      closeTrackSheet();
    } catch {
      setError(true);
    }
  };
  const payload = { track_id: t.id };
  return (
    <MobileSheet kicker="NOT INTERESTED" onClose={closeTrackSheet} song={songOf(t)}>
      <p className="m-bsheet__intro">Kaze will stop recommending this in Flow, For You and mixes.</p>
      <SheetRow icon={<Music {...ICON} />} label="Not this song"
        onPress={() => void run({ command: "DislikeTrack", payload }, () => recommendationPreferences.update([t.id], true))} />
      <SheetRow icon={<UserX {...ICON} />} label="Not this artist" note={t.artist?.toUpperCase()} onPress={() => void run({ command: "BlockArtist", payload })} />
      <SheetRow icon={<TrendingDown {...ICON} />} label="Less like this" note={genre?.toUpperCase()} onPress={() => void run({ command: "LessLikeThis", payload })} />
      {error && <p className="m-set-note is-error m-bsheet__intro" role="alert">Couldn’t save your choice. Try again.</p>}
      <div className="m-bsheet__foot"><button type="button" className="m-btn m-btn--outline m-btn--grow" onClick={closeTrackSheet}>CANCEL</button></div>
    </MobileSheet>
  );
}

/** Whichever track sheet is open. */
export function TrackSheets({ c }: { c: Ctx }) {
  const open = useTrackSheet();
  if (!open) return null;
  return open.kind === "actions" ? <TrackActions t={open.track} c={c} /> : <NotInterested t={open.track} c={c} />;
}
