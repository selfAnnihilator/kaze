import { isPlayingLiked } from "./likeTrack";
import { showPhoneToast } from "./phoneToasts";
import type { MobilePlayer, MobileScreenProps } from "./types";

/** Guard the phone's like actions before they call the shared handlers. */
export function mobileLikeActions(c: MobileScreenProps["c"], player: MobilePlayer) {
  if (c.currentUser) return { c, player };

  const requireSignIn = () => showPhoneToast("Sign in to like songs", undefined, "warning");
  const onLikeOnline = player.onLikeOnline;
  return {
    c: {
      ...c,
      handleLike: async () => requireSignIn(),
      handleToggleCollectionLike: async (track, liked) => {
        if (liked) await c.handleToggleCollectionLike(track, liked);
        else requireSignIn();
      },
    } satisfies MobileScreenProps["c"],
    player: {
      ...player,
      onLike: requireSignIn,
      onLikeOnline: onLikeOnline && ((track) => {
        if (isPlayingLiked(player)) onLikeOnline(track);
        else requireSignIn();
      }),
    } satisfies MobilePlayer,
  };
}
