import { isOnlineTrack } from "../../trackKind";
import type { MobilePlayer } from "./types";
import { canRate } from "../../services/ratingRule";

/** Liked, for a local song (its feedback) or an online one (App's online-like state). */
export const isPlayingLiked = (player: MobilePlayer) =>
  player.currentTrack?.manual_like === 1 || !!player.isOnlineLiked;

/** Like or unlike the playing song the way the desktop Deck does (online songs use their own path). */
export function togglePlayingLike(player: MobilePlayer) {
  const track = player.currentTrack;
  if (!track || !canRate(track)) return;
  if (isOnlineTrack(track) && player.onLikeOnline) {
    player.onLikeOnline({
      id: track.id,
      title: track.title,
      artist: track.artist_name || "Unknown Artist",
      album: track.album_title,
      cover_art_url: track.cover_art_url,
      preview_url: track.preview_url,
      duration_secs: track.duration_secs,
    });
  } else if (isPlayingLiked(player)) player.onRemoveFeedback(track.id);
  else player.onLike(track.id);
}
