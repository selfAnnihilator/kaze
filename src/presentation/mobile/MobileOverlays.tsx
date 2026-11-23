import { MobilePlaybackNotice, MobileRecommendationNotice, MobileToasts, MobileUpdateBanner } from "./MobileNotices";
import { MobileAuthSheet } from "./MobileAuthSheet";
import { MobileConfirm } from "./MobilePrompts";
import { MobileTasteGate } from "./settings/MobileMusicTaste";
import { MobileOnboarding } from "./settings/MobileOnboarding";
import { AddToPlaylistSheet } from "./sheets/AddToPlaylistSheet";
import type { AppOverlaysProps } from "../../components/layout/AppOverlays";

/**
 * Everything that floats above the phone interface, drawn from Figma: sign in
 * (G6), add to playlist (G2), confirmations (G7–G9),
 * the first-run screens (A2/A3), and the banners, notices and toasts (G10).
 */
export function MobileOverlays(p: AppOverlaysProps) {
  return (
    <>
      <MobileUpdateBanner />
      <MobileToasts {...p.toasts}>
        {p.recommendationNotice && <MobileRecommendationNotice key={p.userKey} {...p.recommendationNotice} />}
        <MobilePlaybackNotice key={`playback-notice:${p.userKey}`} />
      </MobileToasts>
      <AddToPlaylistSheet {...p.playlistModal} />
      <MobileTasteGate userId={p.userKey} key={`taste:${p.userKey}`} {...p.setupGate} />
      {p.onboarding && <MobileOnboarding {...p.onboarding} />}
      {p.promptDialog}
      <MobileConfirm />
      {p.auth.open && <MobileAuthSheet onClose={p.auth.onClose} onLogin={p.auth.onLogin} onSignUp={p.auth.onSignUp} />}
    </>
  );
}
