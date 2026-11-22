import type { AppViewsContext } from "../../components/layout/appViews/context";
import type { NowPlayingBarProps } from "../../components/NowPlayingBar";

/** The player controls the mobile screens use. */
export type MobilePlayer = Pick<
  NowPlayingBarProps,
  | "playbackState" | "isLoading" | "currentTrack" | "coverArtUrl" | "onPlayPause" | "onNext"
  | "onPrevious" | "onSeek" | "onToggleRepeat" | "onLike" | "onRemoveFeedback"
>;

export interface MobileProps {
  c: AppViewsContext;
  player: MobilePlayer;
}

export interface MobileScreenProps {
  c: AppViewsContext;
}
