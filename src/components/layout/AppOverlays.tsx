import { lazy, Suspense, type ComponentProps, type ReactNode } from "react";
import { PlaybackNotice } from "../PlaybackNotice";
import { RecommendationNotice } from "../RecommendationNotice";
import { RecommendationSetupGate } from "../MusicTaste";
import { OnboardingModal } from "../OnboardingModal";
import { ToastContainer } from "../notifications/ToastContainer";
import { UpdateBanner } from "../UpdateBanner";
import { ConfirmModal } from "../modals/ConfirmModal";

const AuthModal = lazy(() => import("../modals/AuthModal").then((m) => ({ default: m.AuthModal })));
const MobileOverlays = lazy(() => import("../../presentation/mobile/MobileOverlays").then((m) => ({ default: m.MobileOverlays })));
const AddToPlaylistModal = lazy(() => import("../modals/AddToPlaylistModal").then((m) => ({ default: m.AddToPlaylistModal })));

export interface AppOverlaysProps {
  userKey: string;
  /** The phone interface shows the full-screen sign-in sheet (Figma G6). */
  mobile?: boolean;
  toasts: ComponentProps<typeof ToastContainer>;
  playlistModal: ComponentProps<typeof AddToPlaylistModal>;
  auth: Omit<ComponentProps<typeof AuthModal>, "isOpen"> & { open: boolean };
  recommendationNotice: ComponentProps<typeof RecommendationNotice> | null;
  setupGate: Omit<ComponentProps<typeof RecommendationSetupGate>, "userId">;
  onboarding: ComponentProps<typeof OnboardingModal> | null;
  promptDialog: ReactNode;
}

/**
 * Everything that floats above the interface (toasts, sign in, add to playlist,
 * onboarding, notices, prompts and confirmations). Desktop and mobile both render it,
 * so neither interface can lose a dialog the hooks wait on. The phone draws its
 * own (Figma G-series) in MobileOverlays.
 */
export function AppOverlays(p: AppOverlaysProps) {
  if (p.mobile) return <Suspense fallback={null}><MobileOverlays {...p} /></Suspense>;
  return (
    <>
      <Suspense fallback={null}>
        <ToastContainer {...p.toasts} />
        <UpdateBanner />
        <AddToPlaylistModal {...p.playlistModal} />
        {p.auth.open && <AuthModal isOpen onClose={p.auth.onClose} onLogin={p.auth.onLogin} onSignUp={p.auth.onSignUp} />}
      </Suspense>
      <PlaybackNotice key={`playback-notice:${p.userKey}`} />
      {p.recommendationNotice && <RecommendationNotice key={p.userKey} {...p.recommendationNotice} />}
      <RecommendationSetupGate userId={p.userKey} key={`taste:${p.userKey}`} {...p.setupGate} />
      {p.onboarding && <OnboardingModal {...p.onboarding} />}
      {/* Yes/No questions (e.g. removing a download); store-driven, no App state */}
      <ConfirmModal />
      {p.promptDialog}
    </>
  );
}
