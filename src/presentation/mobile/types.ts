import type { ReactNode } from "react";
import type { AppViewsContext } from "../../components/layout/appViews/context";
import type { NowPlayingBarProps } from "../../components/NowPlayingBar";

/** The player controls the mobile screens use. */
export type MobilePlayer = Pick<
  NowPlayingBarProps,
  | "playbackState" | "isLoading" | "currentTrack" | "coverArtUrl" | "onPlayPause" | "onNext"
  | "onPrevious" | "onSeek" | "onToggleRepeat" | "onLike" | "onRemoveFeedback" | "onLikeOnline"
  | "isOnlineLiked" | "onOpenAddToPlaylist" | "isInPlaylist" | "onVolumeChange" | "onToggleMute"
>;

/** Deck extras App owns: Flow, the playing origin and toasts. */
export interface MobileDeckExtras {
  flowActive: boolean;
  onToggleFlow: () => void;
  onOpenOrigin?: () => void;
  notify: (type: "success" | "error" | "info", title: string, message: string) => void;
}

export interface MobileProps {
  c: AppViewsContext;
  player: MobilePlayer;
  deck: MobileDeckExtras;
  /** Sign in, toasts, dialogs and notices, shared with the desktop interface. */
  overlays: ReactNode;
}

export interface MobileScreenProps {
  c: AppViewsContext;
}
